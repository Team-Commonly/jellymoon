// Twin of worker/src/mcp.ts: a change to one is a change to the other (the boundary rules keep them from importing each other),
// except for the tool-call records this one also makes (ADR 0006, "Evidence"); the Worker does not serve evidence.
// The MCP adapter at POST /mcp (docs/protocol.md): stateless Streamable HTTP, answered as JSON. It adds no behaviour. Each tool
// is one operation (gen/ts/tools.json), and a call is that operation's own HTTP route, sent through the same front door with the
// caller's own token, so authentication, the pod's rules and the errors are exactly the HTTP ones. The one thing it adds is a
// count: before a run's tool call is dispatched, `seen` tells the pod, which is where a receipt's toolCalls comes from. A call the
// pod could not count is refused before it takes effect, so the agent is never told of an error for something that happened.
import tools from '../../gen/ts/tools.json';
import { ERROR_STATUS, OPERATIONS, TOOL_PREFIX } from '../../gen/ts/protocol';
import type { Hono } from 'hono';
import { runLive } from '../../podcore/src/index';
import type { Hub, Principal as Who } from './hub';
import { fail } from './kit';
import { stampCalls } from './evidence';
import type { Finish, Seen } from './evidence';

type Id = string | number | null;
type Dispatch = (req: Request) => Promise<Response>;
/** Asked before any method but tools/call of a run's token is answered: false when the run has ended or was stopped, so its token is dead here too. */
type Live = (runId: string, podId: string) => Promise<boolean>;
type Args = Record<string, unknown>;
const PROTOCOL = '2025-03-26';
const ops = OPERATIONS as Record<string, { method: string; path: string }>;

const ok = (id: Id, result: unknown): Response => Response.json({ jsonrpc: '2.0', id, result });
const refuse = (id: Id, code: number, message: string): Response => Response.json({ jsonrpc: '2.0', id, error: { code, message } });

/** The operation's HTTP request: {podId} is the run's pod (a user token names it), other {names} come from the arguments, the rest is the query or body. */
function operation(name: string, who: Who, args: Args, origin: string, authorization: string): Request | string {
  const key = name.slice(TOOL_PREFIX.length);
  const op = name.startsWith(TOOL_PREFIX) && Object.hasOwn(ops, key) ? ops[key] : undefined; // own keys only: `constructor` is not a tool
  if (op === undefined) return `No such tool: ${name}`;
  const podId = who.kind === 'run' ? who.podId : args.podId;
  if (typeof podId !== 'string') return 'podId is required with a user token.';
  const rest = { ...args };
  delete rest.podId;
  const path = op.path.replace('{podId}', encodeURIComponent(podId)).replace(/\{(\w+)\}/g, (_, k: string) => {
    const v = rest[k];
    delete rest[k];
    return encodeURIComponent(String(v));
  });
  const get = op.method === 'GET';
  const query = get && Object.keys(rest).length > 0 ? `?${new URLSearchParams(Object.entries(rest).map(([k, v]) => [k, String(v)]))}` : '';
  const headers = { Authorization: authorization, 'Content-Type': 'application/json' };
  return new Request(origin + path + query, { method: op.method, headers, body: get ? undefined : JSON.stringify(rest) });
}

const nothing: Finish = () => Promise.resolve();

/** How to finish the call's record, or null when the pod did not count the call: a throw, or an answer that is not a count, is a call that was not counted. A user's call is not counted. */
async function tally(seen: Seen, who: Who, name: unknown): Promise<Finish | null> {
  if (who.kind !== 'run') return nothing;
  try { return await seen(who.runId, who.podId, name); } catch { return null; }
}

async function callTool(id: Id, who: Who, params: Args, origin: string, authorization: string, dispatch: Dispatch, seen: Seen): Promise<Response> {
  const finish = await tally(seen, who, params.name);
  if (finish === null) return refuse(id, -32603, 'The call was not made: the pod could not count it. Try again.');
  const args = (typeof params.arguments === 'object' && params.arguments !== null ? params.arguments : {}) as Args;
  const req = operation(String(params.name), who, args, origin, authorization);
  if (typeof req === 'string') {
    await finish('error', args, req);
    return refuse(id, -32602, req);
  }
  const res = await dispatch(req);
  const text = await res.text();
  await finish(res.ok ? 'ok' : 'error', args, text);
  let structured: unknown;
  try { structured = JSON.parse(text); } catch { structured = undefined; }
  return ok(id, { content: [{ type: 'text', text }], structuredContent: structured, isError: !res.ok });
}

/** One JSON-RPC message from an authenticated caller. */
async function mcp(req: Request, origin: string, who: Who, dispatch: Dispatch, seen: Seen, live: Live): Promise<Response> {
  let msg: { id?: Id; method?: unknown; params?: Args };
  try { msg = (await req.json()) as typeof msg; } catch { return refuse(null, -32700, 'The body is not JSON.'); }
  if (msg === null || typeof msg !== 'object' || Array.isArray(msg) || typeof msg.method !== 'string') return refuse(null, -32600, 'Send one JSON-RPC request.');
  if (who.kind === 'run' && msg.method !== 'tools/call' && !(await live(who.runId, who.podId))) return Response.json({ code: 'unauthorized', message: 'This run has ended; its token no longer works.' }, { status: ERROR_STATUS.unauthorized });
  const id = msg.id ?? null;
  const params = msg.params ?? {};
  if (msg.id === undefined) return new Response(null, { status: 202 }); // a notification takes no answer
  switch (msg.method) {
    case 'initialize': return ok(id, { protocolVersion: typeof params.protocolVersion === 'string' ? params.protocolVersion : PROTOCOL, capabilities: { tools: {} }, serverInfo: { name: 'jellymoon', version: '0' } });
    case 'ping': return ok(id, {});
    case 'tools/list': return ok(id, { tools: tools.tools });
    case 'tools/call': return callTool(id, who, params, origin, req.headers.get('Authorization') ?? '', dispatch, seen);
    default: return refuse(id, -32601, `No such method: ${msg.method}`);
  }
}

/** The requests this adapter dispatched itself. A header could be forged by any caller; an object identity cannot (the Worker strips its x-jm-mcp header at the front door for the same reason). */
const carried = new WeakSet<Request>();
export const viaMcp = (req: Request): boolean => carried.has(req);

/** POST /mcp behind the same front door as every route: a run or user token (401 without a valid one, 403 for a computer's); each tool call re-enters `app`. */
export function mcpRoute(app: Hono, hub: Hub): void {
  const dispatch = (r: Request): Promise<Response> => { carried.add(r); return Promise.resolve(app.fetch(r)); };
  const seen = stampCalls(hub);
  const live = async (runId: string, podId: string): Promise<boolean> => { const pod = hub.pods.get(podId); return pod !== undefined && runLive(pod.state, runId); };
  app.post('/mcp', async (c) => {
    const who = await hub.authenticate(c.req.header('Authorization'));
    if (who === null) return fail(c, 'unauthorized', 'Send a valid bearer token.');
    if (who.kind === 'computer') return fail(c, 'forbidden', 'This kind of token may not call this route.');
    return mcp(c.req.raw, new URL(c.req.url).origin, who, dispatch, seen, live);
  });
}
