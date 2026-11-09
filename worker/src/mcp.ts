// The MCP adapter at POST /mcp (docs/protocol.md): stateless Streamable HTTP, answered as JSON. It adds no behaviour. Each tool
// is one operation (gen/ts/tools.json), and a call is that operation's own HTTP route, sent through the same front door with the
// caller's own token, so authentication, the pod's rules and the errors are exactly the HTTP ones. The one thing it adds is a
// count: before a run's tool call is dispatched, `seen` tells the pod, which is where a receipt's toolCalls comes from. A call the
// pod could not count is refused before it takes effect, so the agent is never told of an error for something that happened.
import tools from '../../gen/ts/tools.json';
import { ERROR_STATUS, OPERATIONS, TOOL_PREFIX } from '../../gen/ts/protocol';
import type { Who } from './wire';

type Id = string | number | null;
type Dispatch = (req: Request) => Promise<Response>;
/** Called before a run's tools/call is dispatched, refused or not; true when the pod counted it. */
type Seen = (runId: string, podId: string) => Promise<boolean>;
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

/** True when the pod counted the call: a throw, or an answer that is not a count, is a call that was not counted. */
async function tally(seen: Seen, who: Who): Promise<boolean> {
  if (who.kind !== 'run') return true;
  try { return (await seen(who.runId, who.podId)) === true; } catch { return false; }
}

async function callTool(id: Id, who: Who, params: Args, origin: string, authorization: string, dispatch: Dispatch, seen: Seen): Promise<Response> {
  if (!(await tally(seen, who))) return refuse(id, -32603, 'The call was not made: the pod could not count it. Try again.');
  const args = (typeof params.arguments === 'object' && params.arguments !== null ? params.arguments : {}) as Args;
  const req = operation(String(params.name), who, args, origin, authorization);
  if (typeof req === 'string') return refuse(id, -32602, req);
  const res = await dispatch(req);
  const text = await res.text();
  let structured: unknown;
  try { structured = JSON.parse(text); } catch { structured = undefined; }
  return ok(id, { content: [{ type: 'text', text }], structuredContent: structured, isError: !res.ok });
}

/** One JSON-RPC message from an authenticated caller. */
export async function mcp(req: Request, origin: string, who: Who, dispatch: Dispatch, seen: Seen, live: Live): Promise<Response> {
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
