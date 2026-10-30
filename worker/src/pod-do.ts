// One Durable Object per pod. It holds the core state and the sinceSeq log in its own SQLite and is the pod's only writer.
// Order of a command, with no await between the first and the last storage write: apply the core, write state and events
// in one transactionSync, fan out to the sockets, set the alarm. The reply follows, held back by the output gate until durable.
// Sockets use the Hibernation API: ctx.acceptWebSocket with tags, auth in the attachment, handlers below. Never ws.accept().
import { apply, emptyPod, isoOf, nextDeadline } from '../../podcore/src/index';
import type { Cmd, Out, PodState } from '../../podcore/src/index';
import type { AnyEvent, ErrorBody } from '../../gen/ts/protocol';
import type { Env } from './env';
import { parseFrame, runFrame } from './frames';
import { internalRoute } from './internal';
import type { Starter } from './internal';
import { podRoute } from './pod-routes';
import { hello } from './views';
import { decodeWho, fail, WHO_HEADER } from './wire';
import type { Opts, PodMeta, Reply, Who } from './wire';

const SCHEMA = [
  'create table if not exists kv (k text primary key, v text not null)',
  'create table if not exists events (seq integer primary key, envelope text not null)',
  'create table if not exists invites (code text primary key, uses_left integer not null, expires_at real not null)',
];
const REPLAY_BATCH = 500;

/** Kept on the socket (it survives hibernation): who connected, and whether it has subscribed. */
interface Att { who: Extract<Who, { kind: 'user' | 'computer' }>; subscribed: boolean }

const replyOf = (out: Out[]): Reply | undefined => {
  const r = out.filter((o): o is Extract<Out, { kind: 'reply' }> => o.kind === 'reply').pop();
  return r === undefined ? undefined : r.ok ? { ok: true, result: r.result } : { ok: false, error: r.error };
};

export class PodDO implements Starter {
  state: PodState = emptyPod('');
  meta: PodMeta = { name: '', createdAt: '', harness: {} };
  readonly sql: SqlStorage;
  private saved = '';
  private armed: number | null | undefined;

  constructor(protected ctx: DurableObjectState, protected env: Env) {
    this.sql = ctx.storage.sql;
    ctx.blockConcurrencyWhile(async () => this.restore());
  }

  get id(): string { return this.state.podId; }

  /** The only clock read: once per command. A test subclass can fix it. */
  now(): number { return Date.now(); }

  /** When the alarm should fire. A test subclass with a fixed clock returns null. */
  protected alarmAt(): number | null { return nextDeadline(this.state); }

  /** Called with the rows a command wrote; the test subclass counts them. */
  protected note(_type: string, _rowsWritten: number): void { /* production keeps no counters */ }

  begin(podId: string): void { this.state = emptyPod(podId); }

  /** Loads state, then applies `boot {staleMs: 0}`: every command is persisted, heartbeats too, so nothing is stale. */
  private restore(): void {
    for (const q of SCHEMA) this.sql.exec(q);
    const rows = this.sql.exec<{ k: string; v: string }>("select k, v from kv where k in ('state', 'meta')").toArray();
    const state = rows.find((r) => r.k === 'state');
    if (state === undefined) return;
    this.state = JSON.parse(state.v) as PodState;
    this.saved = state.v;
    const meta = rows.find((r) => r.k === 'meta');
    if (meta !== undefined) this.meta = JSON.parse(meta.v) as PodMeta;
    this.exec({ type: 'boot', staleMs: 0 });
  }

  private put(k: string, v: string): number {
    const c = this.sql.exec('insert or replace into kv (k, v) values (?, ?)', k, v);
    c.toArray();
    return c.rowsWritten;
  }

  exec(cmd: Cmd, o: Opts = {}): Reply | undefined {
    const early = o.pre?.(this.state);
    if (early) return early;
    const { state, out } = apply(this.state, cmd, this.now());
    const reply = replyOf(out);
    const good = reply?.ok === true;
    const events = out.flatMap((x) => (x.kind === 'event' ? [x.event] : []));
    const meta = good && o.meta ? structuredClone(this.meta) : this.meta;
    if (meta !== this.meta) o.meta?.(meta);
    const text = JSON.stringify(state);
    if (events.length > 0 || text !== this.saved || meta !== this.meta || (good && o.extra)) {
      let rows = 0;
      this.ctx.storage.transactionSync(() => {
        rows += this.put('state', text);
        if (meta !== this.meta) rows += this.put('meta', JSON.stringify(meta));
        for (const e of events) {
          const c = this.sql.exec('insert into events (seq, envelope) values (?, ?)', e.seq, JSON.stringify(e));
          c.toArray();
          rows += c.rowsWritten;
        }
        if (good) o.extra?.();
      });
      this.note(cmd.type, rows);
    }
    this.state = state;
    this.meta = meta;
    this.saved = text;
    this.fanOut(out);
    this.arm();
    return reply;
  }

  eventsAfter(since: number, limit: number): AnyEvent[] {
    return this.sql.exec<{ envelope: string }>('select envelope from events where seq > ? order by seq limit ?', since, limit).toArray().map((r) => JSON.parse(r.envelope) as AnyEvent);
  }

  /** One alarm, at the core's next deadline, set only when it moved. */
  private arm(): void {
    const at = this.alarmAt();
    if (at === this.armed) return;
    this.armed = at;
    void (at === null ? this.ctx.storage.deleteAlarm() : this.ctx.storage.setAlarm(at));
  }

  private fanOut(out: Out[]): void {
    for (const o of out) {
      if (o.kind === 'event') {
        const text = JSON.stringify(o.event);
        for (const ws of this.ctx.getWebSockets()) if ((ws.deserializeAttachment() as Att | null)?.subscribed) this.send(ws, text);
      } else if (o.kind === 'to-computer') {
        const text = JSON.stringify(o.event);
        for (const ws of this.ctx.getWebSockets(`c:${o.computerId}`)) this.send(ws, text);
      }
    }
  }

  private send(ws: WebSocket, text: string): void {
    try { ws.send(text); } catch { /* the socket closed under us */ }
  }

  private frame(type: string, data: unknown): string {
    return JSON.stringify({ v: 1, type, podId: this.id, ts: isoOf(this.now()), data });
  }

  async fetch(req: Request): Promise<Response> {
    try {
      const path = new URL(req.url).pathname;
      if (path.startsWith('/_in/')) return (await internalRoute(this, req)) ?? fail('not_found', 'No such route.');
      const who = decodeWho(req.headers.get(WHO_HEADER));
      if (who === null) return fail('unauthorized', 'Send a valid bearer token.');
      if (path.endsWith('/ws') && req.headers.get('upgrade')?.toLowerCase() === 'websocket') return this.upgrade(who);
      return (await podRoute(this, req, who)) ?? fail('not_found', 'No such route.');
    } catch (e) {
      console.error('pod request failed', e);
      return fail('internal', 'The pod failed on that request.');
    }
  }

  private upgrade(who: Who): Response {
    if (who.kind === 'run') return fail('forbidden', 'A run token does not open the pod socket.');
    const member = who.kind === 'user' ? who.actor : who.owner;
    if (!this.state.members.some((m) => m.id === member.id)) return fail('forbidden', 'Only members of this pod may connect.');
    const [client, server] = Object.values(new WebSocketPair());
    this.ctx.acceptWebSocket(server, [who.kind === 'user' ? `u:${who.actor.id}` : `c:${who.computerId}`]);
    server.serializeAttachment({ who, subscribed: false } satisfies Att);
    const online = new Set(this.ctx.getWebSockets().flatMap((ws) => {
      const a = ws.deserializeAttachment() as Att | null;
      return a?.who.kind === 'computer' ? [a.who.computerId] : [];
    }));
    this.send(server, this.frame('hello', hello(this.state, this.meta, member, online)));
    return new Response(null, { status: 101, webSocket: client });
  }

  webSocketMessage(ws: WebSocket, message: string | ArrayBuffer): void {
    const att = ws.deserializeAttachment() as Att | null;
    if (att === null) { ws.close(1011, 'no session'); return; }
    const p = parseFrame(this.id, att.who, typeof message === 'string' ? message : new TextDecoder().decode(message));
    if ('subscribe' in p) { this.subscribe(ws, att, p.subscribe); return; }
    const refusal = 'cmd' in p ? runFrame(this, p.cmd) : p;
    if (refusal) this.send(ws, this.frame('error', { code: refusal.code, message: refusal.message } satisfies ErrorBody));
  }

  /** Replays the log after `since`, then marks the socket live. No await in between, so no event can land in the gap. */
  private subscribe(ws: WebSocket, att: Att, since: number): void {
    for (let from = since, n = REPLAY_BATCH; n === REPLAY_BATCH;) {
      const batch = this.eventsAfter(from, REPLAY_BATCH);
      for (const e of batch) this.send(ws, JSON.stringify(e));
      n = batch.length;
      from = batch.length > 0 ? (batch[batch.length - 1].seq ?? from) : from;
    }
    ws.serializeAttachment({ ...att, subscribed: true } satisfies Att);
  }

  webSocketClose(ws: WebSocket, code: number): void {
    try { ws.close(code >= 1000 && code < 5000 && code !== 1005 && code !== 1006 ? code : 1000, 'closing'); } catch { /* already closed */ }
  }

  webSocketError(): void { /* a failed socket is dropped by the runtime; the pod's state is not tied to it */ }

  alarm(): void {
    this.armed = null;
    this.exec({ type: 'tick' });
  }
}
