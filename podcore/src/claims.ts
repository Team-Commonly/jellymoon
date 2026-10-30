// Claim and run state transitions, plus who may call. Commands in commands.ts decide what a caller is told.
import type { Actor, ClaimReleased, ClaimReleaseCause, RunId, RunOutcome, Task, TaskId } from '../../gen/ts/protocol';
import { emit, fail } from './emit';
import { isoOf } from './time';
import type { Caller, Ctx, Run } from './types';

export const LEASE_MS = 60_000;

/** The one record lookup: an own key only, so an id like `constructor` finds nothing instead of Object.prototype's member. */
export const own = <T>(rec: Record<string, T>, key: string): T | undefined => (Object.hasOwn(rec, key) ? rec[key] : undefined);
/** A run token lives at most 2 h (ADR 0002). */
export const RUN_CAP_MS = 7_200_000;

export function taskView(c: Ctx, taskId: TaskId): Task {
  const { claim: _stored, ...task } = c.s.tasks[taskId];
  const held = c.s.claims[taskId];
  if (held === undefined) return task;
  return { ...task, claim: { by: c.s.runs[held.runId].agent, runId: held.runId, leaseUntil: isoOf(held.leaseUntil) } };
}

function startClaim(c: Ctx, run: Run, taskId: TaskId): void {
  c.s.claims[taskId] = { runId: run.id, leaseUntil: c.now + LEASE_MS };
  delete c.s.lost[run.id]?.[taskId];
  emit(c, 'task.claimed', { taskId, by: run.agent, runId: run.id, leaseUntil: isoOf(c.now + LEASE_MS) });
}

export function lostHow(c: Ctx, runId: RunId, taskId: TaskId): 'expired' | 'released' | undefined {
  return own(own(c.s.lost, runId) ?? {}, taskId);
}

/** The one place a claim ends: task.updated with no claim, then exactly one claim.released. */
export function endClaim(c: Ctx, taskId: TaskId, cause: ClaimReleaseCause, by?: Actor, reason?: string): void {
  const { runId } = c.s.claims[taskId];
  delete c.s.claims[taskId];
  // Only a person's release bars the run from taking the task back; a run letting go of its own claim may reclaim it.
  if (cause === 'expired' || (cause === 'released' && by?.kind === 'human')) (c.s.lost[runId] ??= {})[taskId] = cause;
  c.s.tasks[taskId].updatedAt = isoOf(c.now);
  emit(c, 'task.updated', taskView(c, taskId));
  const data: ClaimReleased = { taskId, runId, cause };
  if (by !== undefined) data.by = by;
  if (reason !== undefined) data.reason = reason;
  emit(c, 'claim.released', data);
}

const heldBy = (c: Ctx, runId: RunId): TaskId[] => Object.keys(c.s.claims).filter((t) => c.s.claims[t].runId === runId).sort();

/** A renewal: every claim the run holds moves to now + 60 s (ADR 0002: any operation the run's token makes). */
export function renewRun(c: Ctx, runId: RunId): void {
  for (const t of heldBy(c, runId)) c.s.claims[t].leaseUntil = c.now + LEASE_MS;
}

export function endRun(c: Ctx, run: Run, outcome: RunOutcome, cause: ClaimReleaseCause, by?: Actor, reason?: string): void {
  run.status = 'ended';
  run.outcome = outcome;
  for (const t of heldBy(c, run.id)) endClaim(c, t, cause, by, reason);
  delete c.s.lost[run.id];
}

/**
 * Everything due at c.now, on ONE timeline in deadline order: lease expiries (AT leaseUntil) and run caps. A tie goes
 * to the cap. Each effect is stamped with its own deadline, so a late tick and an on-time one emit the same output.
 */
export function settle(c: Ctx): void {
  const real = c.now;
  const due: Array<[number, number, string]> = []; // [deadline, 0 cap | 1 lease, run or task id]
  for (const [t, k] of Object.entries(c.s.claims)) if (k.leaseUntil <= real) due.push([k.leaseUntil, 1, t]);
  for (const r of Object.values(c.s.runs)) if (r.status === 'active' && r.startedAt + RUN_CAP_MS <= real) due.push([r.startedAt + RUN_CAP_MS, 0, r.id]);
  due.sort((a, b) => a[0] - b[0] || a[1] - b[1] || (a[2] < b[2] ? -1 : 1));
  for (const [at, rank, id] of due) {
    c.now = at;
    if (rank === 0) endRun(c, c.s.runs[id], 'failed', 'run_ended');
    else if (own(c.s.claims, id) !== undefined) endClaim(c, id, 'expired');
  }
  c.now = real;
}

/** The earliest moment `apply(state, {type:'tick'}, t)` has work to do, or null. */
export function nextDeadline(s: { runs: Record<RunId, Run>; claims: Record<TaskId, { leaseUntil: number }> }): number | null {
  const times = [
    ...Object.values(s.claims).map((k) => k.leaseUntil),
    ...Object.values(s.runs).filter((r) => r.status === 'active').map((r) => r.startedAt + RUN_CAP_MS),
  ];
  return times.length === 0 ? null : Math.min(...times);
}

/** The run behind a run token, or a refusal: an ended or unknown run's token is dead. A live run renews first, so no refusal below it can skip the renewal. */
export function liveRun(c: Ctx, runId: RunId): Run | null {
  const run = own(c.s.runs, runId);
  if (run !== undefined && run.status === 'active') return renewRun(c, runId), run;
  fail(c, 'unauthorized', 'This run has ended; its token no longer works.', runId);
  return null;
}

/** Who is calling: a live run, or a member. Checked before any lookup, so a stranger cannot probe ids. */
export function who(c: Ctx, caller: Caller): { actor: Actor; run?: Run } | null {
  if (caller.kind === 'run') {
    const run = liveRun(c, caller.runId);
    return run === null ? null : { actor: run.agent, run };
  }
  const actor = c.s.members.find((m) => m.id === caller.userId);
  if (actor === undefined) fail(c, 'forbidden', 'Only members of this pod may do this.');
  return actor === undefined ? null : { actor };
}

/**
 * Take, keep or restore a claim for `run` on a task. Returns true when it refused (the reply is written; the caller
 * already renewed in liveRun). With create false, only a restore after expiry may add a claim.
 */
export function acquire(c: Ctx, run: Run, taskId: TaskId, create: boolean): boolean {
  const how = lostHow(c, run.id, taskId);
  if (how === 'released') return fail(c, 'forbidden', 'A person released this claim; it cannot be taken back.', taskId), true;
  const held = own(c.s.claims, taskId);
  if (held !== undefined && held.runId !== run.id && (create || how === 'expired')) {
    const holder = c.s.runs[held.runId].agent;
    return fail(c, 'already_claimed', `${holder.displayName} holds this task.`, holder.id), true;
  }
  if (held === undefined && (create || how === 'expired')) startClaim(c, run, taskId);
  return false;
}
