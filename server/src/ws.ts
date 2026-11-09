// GET /v1/pods/:pod/ws. The token rides in Authorization on the upgrade, or a browser's session cookie does (never the query string); hello is sent at once; the client then subscribes.
// Frames are handled one at a time per socket, in arrival order, and only after hello has gone out.
import type { Hono } from 'hono';
import { upgradeWebSocket } from 'hono/bun';
import type { ErrorBody } from '../../gen/ts/protocol';
import { isoOf } from '../../podcore/src/index';
import { parseFrame } from './frames';
import type { Hub } from './hub';
import { hashToken } from './ids';
import { authOf, fail, presented } from './kit';
import type { Conn, Opts, PodRuntime, Who } from './pod';
import type { Cmd } from '../../podcore/src/index';
import { findToken } from './store';
import { hello } from './views';

/** `token` is the presented token's hash: logout closes the sockets it opened. */
interface Session { pod: PodRuntime; who: Who; token: string }
/** What a ruling carries into the pod besides the core command: the connector gate's claim and release (ADR 0009). */
type OnRule = (pod: PodRuntime, cmd: Extract<Cmd, { type: 'rule' }>) => Opts;
const sessions = new WeakMap<Request, Session>();

const frame = (hub: Hub, pod: PodRuntime, type: string, data: unknown): string =>
  JSON.stringify({ v: 1, type, podId: pod.id, ts: isoOf(hub.now()), data });

async function onFrame(hub: Hub, s: Session, conn: Conn, text: string, onRule: OnRule): Promise<void> {
  const refuse = (e: ErrorBody): void => conn.send(frame(hub, s.pod, 'error', { code: e.code, message: e.message, ...(e.ref === undefined ? {} : { ref: e.ref }) } satisfies ErrorBody));
  const p = parseFrame(s.pod.id, s.who, text);
  if ('subscribe' in p) return s.pod.subscribe(conn, p.subscribe);
  if ('code' in p) return refuse(p);
  const reply = await s.pod.exec(p.cmd, p.cmd.type === 'rule' ? onRule(s.pod, p.cmd) : {});
  if (reply && !reply.ok) refuse(reply.error);
}

export function wsRoute(app: Hono, hub: Hub, onRule: OnRule): void {
  app.get('/v1/pods/:pod/ws', async (c, next) => {
    if (c.req.header('upgrade')?.toLowerCase() !== 'websocket') return next();
    const p = await authOf(hub, c, true); // a browser's cookie only from this server's own Origin (cross-site socket hijacking)
    if (p instanceof Response) return p;
    if (p === null) return fail(c, 'unauthorized', 'Send a valid bearer token.');
    if (p.kind === 'run') return fail(c, 'forbidden', 'A run token does not open the pod socket.');
    const pod = hub.pods.get(c.req.param('pod') ?? '');
    if (pod === undefined) return fail(c, 'not_found', 'No such pod.');
    const who: Who = p.kind === 'user' ? p : { kind: 'computer', computerId: p.computer.id, owner: p.owner };
    if (!pod.isMember(who.kind === 'user' ? who.actor.id : who.owner.id)) return fail(c, 'forbidden', 'Only members of this pod may connect.');
    sessions.set(c.req.raw, { pod, who, token: hashToken(presented(c, true)?.token ?? '') });
    return next();
  }, upgradeWebSocket((c) => {
    const s = sessions.get(c.req.raw) as Session;
    const conn: Conn = { who: s.who, send: () => undefined, subscribed: false, replaying: false, buf: [], sent: 0 };
    const crash = (err: unknown): void => { console.error('frame failed', err); conn.send(frame(hub, s.pod, 'error', { code: 'internal', message: 'The pod failed on that frame.' })); };
    let chain: Promise<void> = Promise.resolve();
    let close = (): void => undefined;
    let live = true;
    return {
      onOpen(_e, ws) {
        conn.send = (t) => { try { ws.send(t); } catch { /* the socket closed under us */ } };
        close = () => ws.close(1008, 'signed out');
        hub.addSocket(s.token, close);
        const you = s.who.kind === 'user' ? s.who.actor : s.who.owner;
        // A logout that revoked the token after the upgrade authenticated it closed nothing, since this socket was not yet
        // registered: the token is checked again now that it is, and a dead one is closed before hello (ADR 0008).
        // The computer is marked online before hello, so hello shows the presence this socket just made.
        chain = findToken(hub.db, s.token, hub.now()).then(async (row) => {
          if (row === undefined) { live = false; return close(); }
          await s.pod.attach(conn);
          conn.send(frame(hub, s.pod, 'hello', hello(s.pod.state, s.pod.meta, you)));
        }).catch((err) => {
          // A check that could not be made fails closed: the socket never runs a frame unless its token was read live.
          live = false;
          crash(err);
          ws.close(1011, 'internal error');
        });
      },
      onMessage(e) { chain = chain.then(() => (live ? onFrame(hub, s, conn, String(e.data), onRule) : undefined)).catch(crash); },
      onClose() {
        hub.dropSocket(s.token, close);
        chain = chain.then(() => (live ? s.pod.detach(conn) : undefined)).then(() => undefined).catch(crash);
      },
    };
  }));
}
