// The one file that knows MCP (ADR 0009, "Operations, not MCP"): an upstream that speaks MCP over Streamable HTTP, as two
// functions, list and call. JSON-RPC initialize, then tools/list or tools/call, with the credential as a bearer. An answer may
// be JSON or an event stream. Redirects are refused (one could lead to a private address) and every request has a timeout.
// Errors carry the upstream's status and its own text, uncut: the caller scrubs the credential out first and only then
// shortens it. An answer over MAX_BYTES is refused (upstream_too_large) and none of it is kept. Every request goes to a Dest
// that egress.ts checked just before: a pinned one is sent to its checked address, with the name as Host and TLS server name.
import type { ListedOp } from './catalogue';
import type { Dest } from './egress';

type TooLarge = 'upstream_too_large';
export type Listed = { ok: true; ops: ListedOp[] } | { ok: false; status: number; error: string; code?: TooLarge };
export interface Called { ok: boolean; status: number; result?: Record<string, unknown>; error?: string; code?: TooLarge }

const TIMEOUT_MS = 10_000;
const MAX_BYTES = 1024 * 1024;

class TooLargeError extends Error { override name = 'TooLargeError'; }

/** The body as text, read no further than MAX_BYTES: a longer one (or a longer declared length) throws TooLargeError. */
async function bounded(res: Response): Promise<string> {
  if (Number(res.headers.get('content-length') ?? 0) > MAX_BYTES) throw new TooLargeError();
  const reader = res.body?.getReader();
  const parts: Uint8Array[] = [];
  let n = 0;
  for (;;) {
    const chunk = reader === undefined ? { done: true as const } : await reader.read();
    if (chunk.done) break;
    n += chunk.value.length;
    if (n > MAX_BYTES) { await reader!.cancel(); throw new TooLargeError(); }
    parts.push(chunk.value);
  }
  const all = new Uint8Array(n);
  parts.reduce((at, p) => { all.set(p, at); return at + p.length; }, 0);
  return new TextDecoder().decode(all);
}
type Rpc = { status: number; body?: Record<string, unknown>; text: string; session: string | null };

/** The JSON-RPC message of an answer: the body itself, or the last `data:` line of an event stream. */
function messageOf(type: string, text: string): Record<string, unknown> | undefined {
  const raw = type.includes('text/event-stream') ? text.split('\n').filter((l) => l.startsWith('data:')).map((l) => l.slice(5).trim()).pop() : text;
  try {
    const v: unknown = JSON.parse(raw ?? '');
    return v !== null && typeof v === 'object' && !Array.isArray(v) ? (v as Record<string, unknown>) : undefined;
  } catch { return undefined; }
}

async function rpc(dest: Dest, token: string, session: string | null, method: string, params: unknown, id?: number): Promise<Rpc> {
  const headers: Record<string, string> = { Authorization: `Bearer ${token}`, 'Content-Type': 'application/json', Accept: 'application/json, text/event-stream' };
  if (session !== null) headers['Mcp-Session-Id'] = session;
  if (dest.host !== undefined) headers.Host = dest.host;
  const init = {
    method: 'POST', headers, redirect: 'error', signal: AbortSignal.timeout(TIMEOUT_MS),
    body: JSON.stringify(id === undefined ? { jsonrpc: '2.0', method, params } : { jsonrpc: '2.0', id, method, params }),
    ...(dest.serverName === undefined ? {} : { tls: { serverName: dest.serverName } }), // Bun: verify the certificate against the name
  };
  const res = await fetch(dest.url, init as RequestInit);
  const text = await bounded(res);
  return { status: res.status, body: messageOf(res.headers.get('content-type') ?? '', text), text, session: res.headers.get('mcp-session-id') ?? session };
}

/**
 * The upstream's own words for a failed answer: its JSON-RPC error message, else its HTTP status and body. A JSON body is
 * re-serialized from its parsed form, so escapes such as \u sequences are decoded before the caller scrubs it.
 */
function errorOf(r: Rpc): string | null {
  const e = r.body?.error as { message?: unknown } | undefined;
  if (e !== undefined) return String(e.message ?? 'error');
  if (r.status < 200 || r.status > 299) return `HTTP ${r.status}: ${r.body === undefined ? r.text : JSON.stringify(r.body)}`;
  return r.body === undefined ? 'The upstream answered with no JSON-RPC message.' : null;
}

/** initialize plus the initialized notification; the session id, if the upstream gave one, rides on every later request. */
async function open(dest: Dest, token: string): Promise<Rpc> {
  const init = await rpc(dest, token, null, 'initialize', { protocolVersion: '2025-03-26', capabilities: {}, clientInfo: { name: 'jellymoon', version: '0' } }, 1);
  if (errorOf(init) === null) await rpc(dest, token, init.session, 'notifications/initialized', {});
  return init;
}

function failure(e: unknown): { status: number; error: string; code?: TooLarge } {
  if (e instanceof TooLargeError) return { status: 502, error: `The upstream answered with more than ${MAX_BYTES} bytes; none of it was kept.`, code: 'upstream_too_large' };
  return { status: 502, error: e instanceof Error && e.name === 'TimeoutError' ? 'The upstream did not answer in time.' : 'The upstream could not be reached.' };
}

export async function list(dest: Dest, token: string): Promise<Listed> {
  try {
    const init = await open(dest, token);
    const r = errorOf(init) === null ? await rpc(dest, token, init.session, 'tools/list', {}, 2) : init;
    const err = errorOf(r);
    if (err !== null) return { ok: false, status: r.status, error: err };
    const tools = ((r.body?.result as { tools?: unknown[] } | undefined)?.tools ?? []) as Array<{ name?: unknown; inputSchema?: { properties?: object; required?: unknown[] } }>;
    return { ok: true, ops: tools.map((t) => ({ name: String(t.name), args: Object.keys(t.inputSchema?.properties ?? {}), required: (t.inputSchema?.required ?? []).map(String) })) };
  } catch (e) { return { ok: false, ...failure(e) }; }
}

export async function call(dest: Dest, token: string, name: string, args: Record<string, unknown>): Promise<Called> {
  try {
    const init = await open(dest, token);
    const r = errorOf(init) === null ? await rpc(dest, token, init.session, 'tools/call', { name, arguments: args }, 2) : init;
    const err = errorOf(r);
    if (err !== null) return { ok: false, status: r.status, error: err };
    const res = (r.body?.result ?? {}) as { content?: unknown; structuredContent?: unknown; isError?: unknown };
    const result = res.structuredContent !== undefined && typeof res.structuredContent === 'object' ? (res.structuredContent as Record<string, unknown>) : { content: res.content ?? [] };
    if (res.isError === true) return { ok: false, status: r.status, error: JSON.stringify(res.content ?? '') };
    return { ok: true, status: r.status, result };
  } catch (e) { return { ok: false, ...failure(e) }; }
}
