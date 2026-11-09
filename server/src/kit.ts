// Shared HTTP plumbing: error bodies, body parsing, tiny validators, and route wrappers. Every wrapper authenticates before
// anything else, then checks the token kind, then (for a pod route) the pod, then the body against the schema.
import type { Context } from 'hono';
import { getCookie } from 'hono/cookie';
import { ERROR_STATUS } from '../../gen/ts/protocol';
import type { ErrorCode } from '../../gen/ts/protocol';
import type { Caller } from '../../podcore/src/index';
import type { Hub, Principal } from './hub';
import type { PodRuntime, Reply } from './pod';
import { check } from './validate';

export type Body = Record<string, unknown>;

export function fail(c: Context, code: ErrorCode, message: string, ref?: string): Response {
  return c.json(ref === undefined ? { code, message } : { code, message, ref }, ERROR_STATUS[code]);
}

/** An empty body is `{}`; anything that is not a JSON object is null. */
export async function parse(c: Context): Promise<Body | null> {
  const text = await c.req.text();
  if (text === '') return {};
  try {
    const v: unknown = JSON.parse(text);
    return v !== null && typeof v === 'object' && !Array.isArray(v) ? (v as Body) : null;
  } catch { return null; }
}

export const str = (v: unknown, min: number, max: number): string | undefined => (typeof v === 'string' && v.length >= min && v.length <= max ? v : undefined);
export const pick = <T extends string>(v: unknown, list: readonly T[]): T | undefined => list.find((x) => x === v);
export const bad = (c: Context, what: string): Response => fail(c, 'invalid_input', what);
export const intIn = (v: unknown, dflt: number, min: number, max: number): number | null => {
  const n = v === undefined ? dflt : v;
  return typeof n === 'number' && Number.isInteger(n) && n >= min && n <= max ? n : null;
};

export function respond(c: Context, reply: Reply | undefined, shape?: (r: unknown) => unknown): Response {
  if (reply === undefined) return fail(c, 'internal', 'The pod gave no answer.');
  if (!reply.ok) return fail(c, reply.error.code, reply.error.message, reply.error.ref);
  return c.json(shape ? shape(reply.result) : (reply.result ?? {}));
}

type Of<K extends Principal['kind']> = Extract<Principal, { kind: K }>;

export const SESSION = 'jm_session';
export const JSON_TYPE = /^application\/json\s*(;|$)/i;
/** The server's own origin for each request, set before any route runs (buildApp); a request with none matches no Origin. */
export const ownOrigin = new WeakMap<Request, string>();

/** The token a request presents: its Bearer, else, where a person may call, the session cookie. */
export function presented(c: Context, cookie: boolean): { token: string; byCookie: boolean } | null {
  const h = c.req.header('Authorization');
  if (h !== undefined) return { token: h.startsWith('Bearer ') ? h.slice(7).trim() : '', byCookie: false };
  const s = cookie ? getCookie(c, SESSION) : undefined;
  return s ? { token: s, byCookie: true } : null;
}

/**
 * The one authentication function (ADR 0008, "Browser session"). Bearer first; else, when `cookie`, the session cookie, which
 * must hold a jmu_. A cookie request that changes state, or opens the socket, must carry this server's own Origin, and a state
 * change must be JSON (forbidden otherwise). Null is no valid caller.
 */
export async function authOf(hub: Hub, c: Context, cookie: boolean): Promise<Principal | null | Response> {
  const p = presented(c, cookie);
  if (p === null || !p.byCookie) return hub.authenticate(p === null ? undefined : `Bearer ${p.token}`);
  const write = !['GET', 'HEAD', 'OPTIONS'].includes(c.req.method);
  const own = ownOrigin.get(c.req.raw);
  if ((write || c.req.header('upgrade') !== undefined) && (own === undefined || c.req.header('Origin') !== own)) return fail(c, 'forbidden', 'A browser session works only from this server\'s own pages.');
  if (write && !JSON_TYPE.test(c.req.header('Content-Type') ?? '')) return fail(c, 'forbidden', 'A browser session sends JSON.');
  const who = await hub.authenticate(`Bearer ${p.token}`);
  return who?.kind === 'user' ? who : null;
}

/** 401 for no valid token, 403 for a valid token of the wrong kind. Only a route a person may call reads the cookie. */
async function guard<K extends Principal['kind']>(hub: Hub, c: Context, kinds: readonly K[]): Promise<Of<K> | Response> {
  const who = await authOf(hub, c, (kinds as readonly string[]).includes('user'));
  if (who instanceof Response) return who;
  if (who === null) return fail(c, 'unauthorized', 'Send a valid bearer token.');
  return kinds.includes(who.kind as K) ? (who as Of<K>) : fail(c, 'forbidden', 'This kind of token may not call this route.');
}

/** The body as an object (400 when it is not one), checked against the request definition `def` in gen/; null skips the check. */
async function checked(c: Context, def: string | null, fromPath: readonly string[] = [], optional: readonly string[] = []): Promise<Body | Response> {
  const body = await parse(c);
  if (body === null) return bad(c, 'The body must be a JSON object.');
  const problem = def === null ? null : check(def, body, fromPath, optional);
  return problem === null ? body : bad(c, problem);
}

/** A route outside any pod: authenticate, check the kind, parse the body, check it against `def`. */
export function setup<K extends Principal['kind']>(hub: Hub, kinds: readonly K[], def: string | null, fn: (c: Context, who: Of<K>, body: Body) => Promise<Response>) {
  return async (c: Context): Promise<Response> => {
    const who = await guard(hub, c, kinds);
    if (who instanceof Response) return who;
    const body = await checked(c, def);
    return body instanceof Response ? body : fn(c, who, body);
  };
}

export interface Call { c: Context; pod: PodRuntime; who: Principal; caller: Caller; body: Body }

/** A route under /v1/pods/:pod: a run token must belong to that pod (403 otherwise, even for a pod that does not exist). */
export function podRoute(hub: Hub, kinds: readonly Principal['kind'][], def: string | null, fromPath: readonly string[], fn: (k: Call) => Promise<Response>, optional: readonly string[] = []) {
  return async (c: Context): Promise<Response> => {
    const g = await guard(hub, c, kinds);
    if (g instanceof Response) return g;
    const who: Principal = g;
    const id = c.req.param('pod') ?? '';
    if (who.kind === 'run' && who.podId !== id) return fail(c, 'forbidden', 'This run token belongs to another pod.');
    const pod = hub.pods.get(id);
    if (pod === undefined) return fail(c, 'not_found', 'No such pod.', id);
    const body = c.req.method === 'GET' ? {} : await checked(c, def, fromPath, optional);
    if (body instanceof Response) return body;
    const caller: Caller = who.kind === 'run' ? { kind: 'run', runId: who.runId } : { kind: 'user', userId: who.kind === 'user' ? who.actor.id : '' };
    return fn({ c, pod, who, caller, body });
  };
}

/** Shorthand for the pod routes that accept a person or a run. */
export const memberRoute = (hub: Hub, def: string | null, fromPath: readonly string[], fn: (k: Call) => Promise<Response>, optional: readonly string[] = []) =>
  podRoute(hub, ['user', 'run'] as const, def, fromPath, fn, optional);
