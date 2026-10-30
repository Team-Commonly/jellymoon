// What the Worker and the pod object share: the caller identity header, error bodies, body parsing, tiny validators,
// and the narrow view of a pod (`PodPort`) that route handlers get instead of the object itself.
import { ERROR_STATUS } from '../../gen/ts/protocol';
import type { Actor, AnyEvent, ErrorBody, ErrorCode } from '../../gen/ts/protocol';
import type { Cmd, PodState, Result } from '../../podcore/src/index';

export type Reply = { ok: true; result: Result } | { ok: false; error: ErrorBody };
export type Who =
  | { kind: 'user'; actor: Actor }
  | { kind: 'computer'; computerId: string; owner: Actor }
  | { kind: 'run'; runId: string; podId: string };
export interface PodMeta { name: string; createdAt: string; harness: Record<string, string> }
export type Body = Record<string, unknown>;

/** Set by the Worker after it has authenticated; any incoming copy is deleted first. */
export const WHO_HEADER = 'x-jm-who';

export const encodeWho = (who: Who): string => encodeURIComponent(JSON.stringify(who));
export function decodeWho(v: string | null): Who | null {
  try { return v === null ? null : (JSON.parse(decodeURIComponent(v)) as Who); } catch { return null; }
}

export function fail(code: ErrorCode, message: string, ref?: string): Response {
  return Response.json(ref === undefined ? { code, message } : { code, message, ref }, { status: ERROR_STATUS[code] });
}
export const bad = (what: string): Response => fail('invalid_input', what);

/** An empty body is `{}`; anything that is not a JSON object is null. */
export async function parse(req: Request): Promise<Body | null> {
  const text = await req.text();
  if (text === '') return {};
  try {
    const v: unknown = JSON.parse(text);
    return v !== null && typeof v === 'object' && !Array.isArray(v) ? (v as Body) : null;
  } catch { return null; }
}

export const str = (v: unknown, min: number, max: number): string | undefined => (typeof v === 'string' && v.length >= min && v.length <= max ? v : undefined);
export const pick = <T extends string>(v: unknown, list: readonly T[]): T | undefined => list.find((x) => x === v);
export const intIn = (v: unknown, dflt: number, min: number, max: number): number | null => {
  const n = v === undefined ? dflt : v;
  return typeof n === 'number' && Number.isInteger(n) && n >= min && n <= max ? n : null;
};

export function respond(reply: Reply | undefined, shape?: (r: unknown) => unknown): Response {
  if (reply === undefined) return fail('internal', 'The pod gave no answer.');
  if (!reply.ok) return fail(reply.error.code, reply.error.message, reply.error.ref);
  return Response.json(shape ? shape(reply.result) : (reply.result ?? {}));
}

/** `pre` may answer instead of the core; `meta` and `extra` run only when the command succeeded, inside its transaction. */
export interface Opts {
  pre?: (s: PodState) => Reply | null;
  meta?: (m: PodMeta) => void;
  extra?: () => void;
}

export interface PodPort {
  readonly id: string;
  readonly state: PodState;
  readonly meta: PodMeta;
  exec(cmd: Cmd, o?: Opts): Reply | undefined;
  eventsAfter(since: number, limit: number): AnyEvent[];
  now(): number;
  sql: SqlStorage;
}
