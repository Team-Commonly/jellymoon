// The pod core (ADR 0003): apply(state, cmd, now) -> { state, out }. Pure; adapters own I/O and the timer.
import { nextDeadlineOf, settle } from './claims';
import { claimCmd, heartbeatCmd, opCmd, releaseCmd, runEndCmd, runStopCmd } from './commands';
import type { Cmd, Ctx, Out, PodState } from './types';

export type { Caller, Cmd, Out, PodState, Result, Run } from './types';
export { isoOf } from './time';
export { LEASE_MS, RUN_CAP_MS } from './claims';

/** Settles whatever is due at `now`, then runs the command. The input state is never touched. */
export function apply(state: PodState, cmd: Cmd, now: number): { state: PodState; out: Out[] } {
  const c: Ctx = { s: structuredClone(state), now, out: [] };
  settle(c);
  switch (cmd.type) {
    case 'claim': claimCmd(c, cmd); break;
    case 'op': opCmd(c, cmd); break;
    case 'heartbeat': heartbeatCmd(c, cmd); break;
    case 'task_release': releaseCmd(c, cmd); break;
    case 'run_end': runEndCmd(c, cmd); break;
    case 'run_stop': runStopCmd(c, cmd); break;
    case 'tick': break;
  }
  return { state: c.s, out: c.out };
}

/** The earliest moment `apply(state, {type:'tick'}, t)` has work to do, or null. */
export function nextDeadline(state: PodState): number | null {
  return nextDeadlineOf(state);
}
