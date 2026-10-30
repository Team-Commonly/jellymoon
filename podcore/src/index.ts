// The pod core (ADR 0003): apply(state, cmd, now) -> { state, out }. Pure; adapters own I/O and the timer.
import { messagePostCmd, memberAddCmd, podCreateCmd, taskCreateCmd, taskUpdateCmd } from './board';
import { LEASE_MS, nextDeadlineOf, settle } from './claims';
import { bootCmd, claimCmd, heartbeatCmd, opCmd, releaseCmd, runEndCmd, runStopCmd } from './commands';
import { fail } from './emit';
import { agentRegisterCmd, changeCmd, runClaimCmd, wakeAckCmd } from './wakes';
import type { Cmd, Ctx, Out, PodState } from './types';

export type { Agent, Caller, Cmd, Out, PodState, Result, Run, RunClaimed, Wake } from './types';
export { isoOf } from './time';
export { LEASE_MS, RUN_CAP_MS } from './claims';
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

/** Why a call cannot run at all: input the types promise and an adapter might still get wrong. Never a throw. */
function refusal(cmd: Cmd, now: number): string | null {
  if (!Number.isInteger(now) || now < 0) return 'now must be a non-negative integer number of milliseconds.';
  if (!Object.prototype.hasOwnProperty.call(HANDLERS, cmd.type)) return 'Unknown command type.';
  if (cmd.type === 'boot' && !(Number.isInteger(cmd.staleMs) && cmd.staleMs >= 0 && cmd.staleMs <= LEASE_MS)) return 'staleMs must be an integer from 0 to LEASE_MS.';
  return null;
}

/** Settles whatever is due at `now`, then runs the command. The input state is never touched. */
export function apply(state: PodState, cmd: Cmd, now: number): { state: PodState; out: Out[] } {
  const c: Ctx = { s: structuredClone(state), now, out: [], opened: 0 };
  const bad = refusal(cmd, now);
  if (bad !== null) return fail(c, 'invalid_input', bad), { state: c.s, out: c.out };
  // boot stretches stale leases BEFORE the settle, or a lease that only looks lapsed would be expired early.
  if (cmd.type === 'boot') bootCmd(c, cmd);
  settle(c);
  if (cmd.type !== 'pod_create' && c.s.members.length === 0) {
    if (cmd.type !== 'boot') fail(c, 'not_found', 'No such pod.', c.s.podId);
  } else (HANDLERS[cmd.type] as (cc: Ctx, m: Cmd) => void)(c, cmd);
  return { state: c.s, out: c.out };
}

/** The earliest moment `apply(state, {type:'tick'}, t)` has work to do, or null. */
export function nextDeadline(state: PodState): number | null {
  return nextDeadlineOf(state);
}
