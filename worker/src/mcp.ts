// The MCP adapter at POST /mcp (docs/protocol.md): stateless Streamable HTTP, answered as JSON. It adds no behaviour. Each tool
// is one operation (gen/ts/tools.json), and a call is that operation's own HTTP route, sent through the same front door with the
// caller's own token, so authentication, the pod's rules and the errors are exactly the HTTP ones.
import tools from '../../gen/ts/tools.json';
import { OPERATIONS, TOOL_PREFIX } from '../../gen/ts/protocol';
import type { Who } from './wire';

type Id = string | number | null;
type Dispatch = (req: Request) => Promise<Response>;
type Args = Record<string, unknown>;
const PROTOCOL = '2025-03-26';
const ops = OPERATIONS as Record<string, { method: string; path: string }>;

const ok = (id: Id, result: unknown): Response => Response.json({ jsonrpc: '2.0', id, result });
const refuse = (id: Id, code: number, message: string): Response => Response.json({ jsonrpc: '2.0', id, error: { code, message } });

/** The operation's HTTP request: {podId} is the run's pod (a user token names it), other {names} come from the arguments, the rest is the query or body. */
function operation(name: string, who: Who, args: Args, origin: string, authorization: string): Request | string {
  const op = name.startsWith(TOOL_PREFIX) ? ops[name.slice(TOOL_PREFIX.length)] : undefined;
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

async function callTool(id: Id, who: Who, params: Args, origin: string, authorization: string, dispatch: Dispatch): Promise<Response> {
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
export async function mcp(req: Request, origin: string, who: Who, dispatch: Dispatch): Promise<Response> {
  let msg: { id?: Id; method?: unknown; params?: Args };
  try { msg = (await req.json()) as typeof msg; } catch { return refuse(null, -32700, 'The body is not JSON.'); }
  if (msg === null || typeof msg !== 'object' || Array.isArray(msg) || typeof msg.method !== 'string') return refuse(null, -32600, 'Send one JSON-RPC request.');
  const id = msg.id ?? null;
  const params = msg.params ?? {};
  if (msg.id === undefined) return new Response(null, { status: 202 }); // a notification takes no answer
  switch (msg.method) {
    case 'initialize': return ok(id, { protocolVersion: typeof params.protocolVersion === 'string' ? params.protocolVersion : PROTOCOL, capabilities: { tools: {} }, serverInfo: { name: 'jellymoon', version: '0' } });
    case 'ping': return ok(id, {});
    case 'tools/list': return ok(id, { tools: tools.tools });
    case 'tools/call': return callTool(id, who, params, origin, req.headers.get('Authorization') ?? '', dispatch);
    default: return refuse(id, -32601, `No such method: ${msg.method}`);
  }
}
