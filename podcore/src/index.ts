// The pod core (ADR 0003): apply(state, cmd, now) -> { state, out }. Pure; adapters own I/O and the timer.
import { messagePostCmd, memberAddCmd, podCreateCmd, taskCreateCmd, taskUpdateCmd } from './board';
import { LEASE_MS, nextDeadline, own, renewRun, settle } from './claims';
import { bootCmd, claimCmd, heartbeatCmd, opCmd, releaseCmd, runEndCmd, runStopCmd } from './commands';
import { fail } from './emit';
import { agentRegisterCmd, changeCmd, runClaimCmd, wakeAckCmd } from './wakes';
import type { Cmd, Ctx, Out, PodState } from './types';

export type { Agent, Caller, Cmd, Out, PodState, Result, Run, RunClaimed, Wake } from './types';
export { isoOf } from './time';
export { LEASE_MS, RUN_CAP_MS, nextDeadline } from './claims';
export { WAKE_RESEND_MS } from './wakes';
export { nthWakeId } from './wakeid';

/** A pod nobody has created yet: every command but `pod_create` (and `boot`) gets not_found. */
export const emptyPod = (podId: string): PodState => ({
  podId, seq: 0, members: [], owners: [], tasks: {}, runs: {}, claims: {}, lost: {}, agents: {}, wakes: {},
});

type Handlers = { [K in Cmd['type']]: (c: Ctx, cmd: Extract<Cmd, { type: K }>) => void };
const HANDLERS: Handlers = {
  pod_create: podCreateCmd, member_add: memberAddCmd, agent_register: agentRegisterCmd,
  task_create: taskCreateCmd, task_update: taskUpdateCmd, message_post: messagePostCmd,
  claim: claimCmd, op: opCmd, heartbeat: heartbeatCmd, task_release: releaseCmd, run_end: runEndCmd, run_stop: runStopCmd,
  change: changeCmd, wake_ack: wakeAckCmd, run_claim: runClaimCmd,
  tick: () => undefined,
  boot: () => undefined,
};

/** The last millisecond of year 9999, the most isoOf renders as RFC 3339. */
const MAX_NOW = 253_402_300_799_999;

/** The string fields each command must carry; a missing or mistyped one would otherwise throw deep in a handler. */
const STRINGS: Record<string, string[]> = {
  claim: ['runId', 'taskId'], op: ['runId'], heartbeat: ['computerId'], task_release: ['taskId'], run_end: ['runId', 'computerId'], run_stop: ['runId'],
  pod_create: ['ownerId'], member_add: ['userId'], task_create: ['taskId', 'wakeId'], task_update: ['taskId', 'wakeId'], message_post: ['messageId', 'wakeId'],
  change: ['agentId', 'wakeId'], wake_ack: ['wakeId', 'computerId'], run_claim: ['wakeId', 'computerId', 'runId'], agent_register: ['agentId', 'ownerId', 'computerId'],
};
/** Optional string fields: absent is fine, present must be a string (an array id would otherwise slip past the id checks). */
const OPTIONAL: Record<string, string[]> = { op: ['taskId'], task_create: ['assignee'], task_update: ['assignee'] };
const CALLERS = ['task_release', 'run_stop', 'task_create', 'task_update', 'message_post'];
const isObj = (v: unknown): v is Record<string, unknown> => typeof v === 'object' && v !== null;
const callerOk = (v: unknown): boolean => isObj(v) && ((v.kind === 'user' && typeof v.userId === 'string') || (v.kind === 'run' && typeof v.runId === 'string'));
const shapeBad = (cmd: Cmd): boolean => {
  const o = cmd as Record<string, unknown>;
  return (STRINGS[cmd.type] ?? []).some((k) => typeof o[k] !== 'string') || (OPTIONAL[cmd.type] ?? []).some((k) => o[k] !== undefined && typeof o[k] !== 'string') || (CALLERS.includes(cmd.type) && !callerOk(o.caller)) || (cmd.type === 'change' && !isObj(o.refs));
};

/** True when any id-like field (a name ending in Id, or assignee), at any depth, is an Object.prototype key such as `__proto__`. */
const protoKeyed = (v: unknown): boolean => typeof v === 'object' && v !== null && Object.entries(v).some(([k, x]) => (
  typeof x === 'string' ? (/Id$|^assignee$/.test(k) && (x === '__proto__' || x in {})) : protoKeyed(x)));

/** The run id a command's token names, if it names one as a string: a run caller, or `claim`/`op`. */
const runOf = (cmd: Cmd): string | null => {
  const o = cmd as Record<string, unknown>;
  if (isObj(o.caller)) return o.caller.kind === 'run' && typeof o.caller.runId === 'string' ? o.caller.runId : null;
  return (cmd.type === 'claim' || cmd.type === 'op') && typeof o.runId === 'string' ? o.runId : null;
};

/** Why a call cannot run at all: input the types promise and an adapter might still get wrong. Never a throw. */
function refusal(cmd: Cmd, now: number): { code: 'invalid_input' | 'not_found'; why: string } | null {
  if (!isObj(cmd)) return { code: 'invalid_input', why: 'A command must be an object.' };
  if (!Number.isInteger(now) || now < 0 || now > MAX_NOW) return { code: 'invalid_input', why: 'now must be an integer from 0 to the end of year 9999, in milliseconds.' };
  if (!Object.hasOwn(HANDLERS, cmd.type)) return { code: 'invalid_input', why: 'Unknown command type.' };
  if (cmd.type === 'boot' && !(Number.isInteger(cmd.staleMs) && cmd.staleMs >= 0 && cmd.staleMs <= LEASE_MS)) return { code: 'invalid_input', why: 'staleMs must be an integer from 0 to LEASE_MS.' };
  if (shapeBad(cmd)) return { code: 'invalid_input', why: 'A required field is missing or has the wrong type.' };
  return protoKeyed(cmd) ? { code: 'not_found', why: 'No such id.' } : null;
}

/**
 * A refused call is still a call. With a usable clock: settle, then renew a live run's claims (ADR 0002), then answer who
 * the caller is before what they asked: a dead or unknown run is unauthorized, a non-member is forbidden, else the refusal.
 */
function refuse(c: Ctx, cmd: Cmd, bad: { code: 'invalid_input' | 'not_found'; why: string }): void {
  const o = cmd as Record<string, unknown>;
  if (!isObj(cmd) || !Number.isInteger(c.now) || c.now < 0 || c.now > MAX_NOW) return fail(c, bad.code, bad.why);
  settle(c);
  const id = runOf(cmd);
  const run = id === null ? undefined : own(c.s.runs, id);
  if (run?.status === 'active') renewRun(c, run.id);
  else if (id !== null && c.s.members.length > 0) return fail(c, 'unauthorized', 'This run has ended; its token no longer works.', id);
  const u = isObj(o.caller) && o.caller.kind === 'user' && typeof o.caller.userId === 'string' ? o.caller.userId : null;
  if (u !== null && c.s.members.length > 0 && !c.s.members.some((m) => m.id === u)) return fail(c, 'forbidden', 'Only members of this pod may do this.');
  fail(c, bad.code, bad.why);
}

/** Settles whatever is due at `now`, then runs the command. The input state is never touched. */
export function apply(state: PodState, cmd: Cmd, now: number): { state: PodState; out: Out[] } {
  const c: Ctx = { s: structuredClone(state), now, out: [], opened: 0 };
  const bad = refusal(cmd, now);
  if (bad !== null) return refuse(c, cmd, bad), { state: c.s, out: c.out };
  // boot stretches stale leases BEFORE the settle, or a lease that only looks lapsed would be expired early.
  if (cmd.type === 'boot') bootCmd(c, cmd);
  settle(c);
  const noPod = cmd.type !== 'pod_create' && c.s.members.length === 0;
  if (noPod && cmd.type !== 'boot') fail(c, 'not_found', 'No such pod.', c.s.podId);
  else if (!noPod) (HANDLERS[cmd.type] as (cc: Ctx, m: Cmd) => void)(c, cmd);
  return { state: c.s, out: c.out };
}
