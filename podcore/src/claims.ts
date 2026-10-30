// Claim and run state transitions. No replies here: commands.ts decides what a caller is told.
import type { Actor, ClaimReleased, ClaimReleaseCause, RunId, RunOutcome, Task, TaskId } from '../../gen/ts/protocol';
import { emit } from './emit';
import { isoOf } from './time';
import type { Ctx, Run } from './types';

export const LEASE_MS = 60_000;
/** A run token lives at most 2 h (ADR 0002). */
export const RUN_CAP_MS = 7_200_000;

const lostKey = (runId: RunId, taskId: TaskId): string => `${runId}|${taskId}`;

export function taskView(c: Ctx, taskId: TaskId): Task {
  const { claim: _stored, ...task } = c.s.tasks[taskId];
  const held = c.s.claims[taskId];
  if (held === undefined) return task;
  return { ...task, claim: { by: c.s.runs[held.runId].agent, runId: held.runId, leaseUntil: isoOf(held.leaseUntil) } };
}

export function startClaim(c: Ctx, run: Run, taskId: TaskId): void {
  c.s.claims[taskId] = { runId: run.id, leaseUntil: c.now + LEASE_MS };
  delete c.s.lost[lostKey(run.id, taskId)];
  emit(c, 'task.claimed', { taskId, by: run.agent, runId: run.id, leaseUntil: isoOf(c.now + LEASE_MS) });
}

export function lostHow(c: Ctx, runId: RunId, taskId: TaskId): 'expired' | 'released' | undefined {
  return c.s.lost[lostKey(runId, taskId)];
}

/** The one place a claim ends: task.updated with no claim, then exactly one claim.released. */
export function endClaim(c: Ctx, taskId: TaskId, cause: ClaimReleaseCause, by?: Actor, reason?: string): void {
  const { runId } = c.s.claims[taskId];
  delete c.s.claims[taskId];
  if (cause === 'expired' || cause === 'released') c.s.lost[lostKey(runId, taskId)] = cause;
  c.s.tasks[taskId].updatedAt = isoOf(c.now);
  emit(c, 'task.updated', taskView(c, taskId));
  const data: ClaimReleased = { taskId, runId, cause };
  if (by !== undefined) data.by = by;
  if (reason !== undefined) data.reason = reason;
  emit(c, 'claim.released', data);
}

const heldBy = (c: Ctx, runId: RunId): TaskId[] => Object.keys(c.s.claims).filter((t) => c.s.claims[t].runId === runId).sort();

/** A renewal: every claim the run holds moves to now + 60 s. */
export function renewRun(c: Ctx, runId: RunId): void {
  for (const t of heldBy(c, runId)) c.s.claims[t].leaseUntil = c.now + LEASE_MS;
}

export function endRun(c: Ctx, run: Run, outcome: RunOutcome, cause: ClaimReleaseCause, by?: Actor, reason?: string): void {
  run.status = 'ended';
  run.outcome = outcome;
  for (const t of heldBy(c, run.id)) endClaim(c, t, cause, by, reason);
  for (const k of Object.keys(c.s.lost)) if (k.startsWith(`${run.id}|`)) delete c.s.lost[k];
}

const byDeadline = <T>(rows: Array<[number, string, T]>): T[] =>
  rows.sort((a, b) => a[0] - b[0] || (a[1] < b[1] ? -1 : 1)).map((r) => r[2]);

/** Everything due at `now`: lapsed leases first, then run tokens past their cap. Idempotent per instant. */
export function settle(c: Ctx): void {
  const lapsed = Object.keys(c.s.claims).filter((t) => c.s.claims[t].leaseUntil <= c.now);
  for (const t of byDeadline(lapsed.map((t): [number, string, string] => [c.s.claims[t].leaseUntil, t, t]))) {
    endClaim(c, t, 'expired');
  }
  const capped = Object.values(c.s.runs).filter((r) => r.status === 'active' && r.startedAt + RUN_CAP_MS <= c.now);
  for (const r of byDeadline(capped.map((r): [number, string, Run] => [r.startedAt + RUN_CAP_MS, r.id, r]))) {
    endRun(c, r, 'failed', 'run_ended');
  }
}

export function nextDeadlineOf(s: { runs: Record<RunId, Run>; claims: Record<TaskId, { leaseUntil: number }> }): number | null {
  const times = [
    ...Object.values(s.claims).map((k) => k.leaseUntil),
    ...Object.values(s.runs).filter((r) => r.status === 'active').map((r) => r.startedAt + RUN_CAP_MS),
  ];
  return times.length === 0 ? null : Math.min(...times);
}
