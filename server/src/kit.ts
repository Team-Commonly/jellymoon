// Shared HTTP plumbing: error bodies, body parsing, tiny validators, and route wrappers. Every wrapper authenticates before
// anything else, then checks the token kind, then (for a pod route) the pod, then the body against the schema.
import type { Context } from 'hono';
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

/** 401 for no valid token, 403 for a valid token of the wrong kind. */
async function guard<K extends Principal['kind']>(hub: Hub, c: Context, kinds: readonly K[]): Promise<Of<K> | Response> {
  const who = await hub.authenticate(c.req.header('Authorization'));
  if (who === null) return fail(c, 'unauthorized', 'Send a valid bearer token.');
  return kinds.includes(who.kind as K) ? (who as Of<K>) : fail(c, 'forbidden', 'This kind of token may not call this route.');
}

/** The body as an object (400 when it is not one), checked against the request definition `def` in gen/; null skips the check. */
async function checked(c: Context, def: string | null, fromPath: readonly string[] = []): Promise<Body | Response> {
  const body = await parse(c);
  if (body === null) return bad(c, 'The body must be a JSON object.');
  const problem = def === null ? null : check(def, body, fromPath);
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
export function podRoute(hub: Hub, kinds: readonly Principal['kind'][], def: string | null, fromPath: readonly string[], fn: (k: Call) => Promise<Response>) {
  return async (c: Context): Promise<Response> => {
    const g = await guard(hub, c, kinds);
    if (g instanceof Response) return g;
    const who: Principal = g;
    const id = c.req.param('pod') ?? '';
    if (who.kind === 'run' && who.podId !== id) return fail(c, 'forbidden', 'This run token belongs to another pod.');
    const pod = hub.pods.get(id);
    if (pod === undefined) return fail(c, 'not_found', 'No such pod.', id);
    const body = c.req.method === 'GET' ? {} : await checked(c, def, fromPath);
    if (body instanceof Response) return body;
    const caller: Caller = who.kind === 'run' ? { kind: 'run', runId: who.runId } : { kind: 'user', userId: who.kind === 'user' ? who.actor.id : '' };
    return fn({ c, pod, who, caller, body });
  };
}

/** Shorthand for the pod routes that accept a person or a run. */
export const memberRoute = (hub: Hub, def: string | null, fromPath: readonly string[], fn: (k: Call) => Promise<Response>) =>
  podRoute(hub, ['user', 'run'] as const, def, fromPath, fn);
