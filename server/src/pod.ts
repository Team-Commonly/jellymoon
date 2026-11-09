// One pod in memory: its core state, a FIFO that serialises apply + persist (the pod's only writer), and its sockets.
// Order of a command: apply the core, write state + events in one transaction, then fan out, then answer.
import type { Actor, AnyEvent, ErrorBody } from '../../gen/ts/protocol';
import { apply, nextDeadline } from '../../podcore/src/index';
import type { Cmd, Out, PodState, Result } from '../../podcore/src/index';
import type { Db, Tx } from './db';
import { eventsAfter, loadPod, savePod } from './store';
import type { PodMeta } from './store';

export type Reply = { ok: true; result: Result } | { ok: false; error: ErrorBody };

export type Who = { kind: 'user'; actor: Actor } | { kind: 'computer'; computerId: string; owner: Actor };

/** A socket on the pod. `sent` is the last seq it has been given; `buf` holds live events while a replay runs. */
export interface Conn {
  who: Who;
  send(text: string): void;
  subscribed: boolean;
  replaying: boolean;
  buf: AnyEvent[];
  sent: number;
}

export interface Deps { db: Db; now(): number; rearm(): void }

/**
 * `pre` runs first inside the FIFO and may answer instead; `meta`, `extra` and `after` run only when the command succeeded.
 * `extra` runs inside the command's transaction; `after` runs once it has committed, still inside the FIFO and before any
 * event fans out (a released connector write, ADR 0009: members hear of the ruling only once its call has been made).
 */
export interface Opts {
  pre?: (s: PodState) => Promise<Reply | null>;
  meta?: (m: PodMeta) => void;
  extra?: (t: Tx) => Promise<void>;
  after?: () => Promise<void>;
}

const REPLAY_BATCH = 500;
const replyOf = (out: Out[]): Reply | undefined => {
  const r = out.filter((o): o is Extract<Out, { kind: 'reply' }> => o.kind === 'reply').pop();
  return r === undefined ? undefined : r.ok ? { ok: true, result: r.result } : { ok: false, error: r.error };
};

export class PodRuntime {
  readonly conns = new Set<Conn>();
  deadline: number | null;
  private tail: Promise<unknown> = Promise.resolve();
  private saved: string;

  constructor(readonly id: string, public state: PodState, public meta: PodMeta, private deps: Deps, persisted: boolean) {
    this.deadline = nextDeadline(state);
    this.saved = persisted ? JSON.stringify(state) : '';
  }

  /** A computer's socket opened: it is online here. The core announces it and stamps it for staleness. */
  attach(c: Conn): Promise<unknown> {
    this.conns.add(c);
    return c.who.kind === 'computer' ? this.exec({ type: 'computer_online', computerId: c.who.computerId }) : Promise.resolve();
  }

  /** A socket closed. A computer's last socket to this pod gone means it is offline here. */
  detach(c: Conn): Promise<unknown> {
    this.conns.delete(c);
    const w = c.who;
    if (w.kind !== 'computer' || [...this.conns].some((x) => x.who.kind === 'computer' && x.who.computerId === w.computerId)) return Promise.resolve();
    return this.exec({ type: 'computer_offline', computerId: w.computerId });
  }

  isMember(userId: string): boolean { return this.state.members.some((m) => m.id === userId); }

  exec(cmd: Cmd, o: Opts = {}): Promise<Reply | undefined> {
    const run = this.tail.then(() => this.step(cmd, o));
    this.tail = run.catch(() => undefined);
    return run;
  }

  private async step(cmd: Cmd, o: Opts): Promise<Reply | undefined> {
    try {
      return await this.run(cmd, o);
    } finally {
      // A failed write leaves the memory as it was and the old deadline in place; the timer is re-armed either way.
      this.deps.rearm();
    }
  }

  private async run(cmd: Cmd, o: Opts): Promise<Reply | undefined> {
    const early = await o.pre?.(this.state);
    if (early) return early;
    const { state, out } = apply(this.state, cmd, this.deps.now());
    const reply = replyOf(out);
    const good = reply?.ok === true;
    const events = out.flatMap((x) => (x.kind === 'event' ? [x.event] : []));
    const meta = good && o.meta ? structuredClone(this.meta) : this.meta;
    if (meta !== this.meta) o.meta?.(meta);
    const text = JSON.stringify(state);
    // A pod not yet stored (saved is empty) is written only by a command that succeeded: a failed pod_create leaves no row.
    const changed = text !== this.saved && (this.saved !== '' || good);
    if (events.length > 0 || changed || meta !== this.meta || (good && o.extra)) {
      await this.deps.db.tx(async (t) => {
        await savePod(t, { id: this.id, state, meta }, events);
        if (good) await o.extra?.(t);
      });
    }
    this.state = state;
    this.meta = meta;
    this.saved = text;
    this.deadline = nextDeadline(state);
    if (good && o.after) await o.after().catch(() => console.error('a command\'s after-commit step failed', this.id));
    this.fanOut(out);
    return reply;
  }

  /** Test route (/__test/evict): forgets everything held in memory and restores from storage, as a woken hibernated object does. Sockets stay attached. */
  evict(): Promise<void> {
    const run = this.tail.then(async () => {
      const row = await loadPod(this.deps.db, this.id);
      if (row === undefined) return;
      this.state = row.state;
      this.meta = row.meta;
      this.saved = JSON.stringify(row.state);
      this.deadline = nextDeadline(row.state);
      await this.step({ type: 'boot', staleMs: 0 }, {});
    });
    this.tail = run.catch(() => undefined);
    return run;
  }

  private fanOut(out: Out[]): void {
    for (const o of out) {
      if (o.kind === 'event') this.deliver(o.event);
      else if (o.kind === 'live') {
        // Presence is current state: no seq, never replayed. Every subscribed socket gets it now.
        const text = JSON.stringify(o.event);
        for (const c of this.conns) if (c.subscribed) c.send(text);
      } else if (o.kind === 'to-computer') {
        const text = JSON.stringify(o.event);
        for (const c of this.conns) if (c.who.kind === 'computer' && c.who.computerId === o.computerId) c.send(text);
      }
    }
  }

  private deliver(e: AnyEvent): void {
    for (const c of this.conns) {
      if (!c.subscribed) continue;
      if (c.replaying) c.buf.push(e);
      else this.give(c, e);
    }
  }

  private give(c: Conn, e: AnyEvent): void {
    if (e.seq === undefined || e.seq <= c.sent) return;
    c.send(JSON.stringify(e));
    c.sent = e.seq;
  }

  /** Replays everything after `since` from the log, then goes live; events that land meanwhile are buffered, so there is no gap. */
  async subscribe(c: Conn, since: number): Promise<void> {
    if (c.replaying) return;
    Object.assign(c, { subscribed: true, replaying: true, buf: [], sent: since });
    try {
      for (let n = REPLAY_BATCH; n === REPLAY_BATCH;) {
        const batch = await eventsAfter(this.deps.db, this.id, c.sent, REPLAY_BATCH);
        for (const e of batch) this.give(c, e);
        n = batch.length;
      }
    } catch (err) {
      // A gap the client cannot see is worse than no subscription: end it and drop the buffer. The caller sends the error frame, and a later subscribe replays from the client's own sinceSeq.
      Object.assign(c, { subscribed: false, replaying: false, buf: [] });
      throw err;
    }
    const buf = c.buf;
    Object.assign(c, { replaying: false, buf: [] });
    for (const e of buf) this.give(c, e);
  }
}
