// One function per command. Authorization is read from state, never from the input's own claims.
import type { Actor, RunId, TaskId } from '../../gen/ts/protocol';
import { endClaim, endRun, lostHow, renewRun, startClaim, taskView } from './claims';
import { fail, ok, push } from './emit';
import { isoOf } from './time';
import { resendWakes } from './wakes';
import type { Caller, Cmd, Ctx, Run } from './types';

type Of<T extends Cmd['type']> = Extract<Cmd, { type: T }>;

/** The run behind a run token, or a refusal: an ended or unknown run's token is dead. */
function liveRun(c: Ctx, runId: RunId): Run | null {
  const run = c.s.runs[runId];
  if (run !== undefined && run.status === 'active') return run;
  fail(c, 'unauthorized', 'This run has ended; its token no longer works.', runId);
  return null;
}

const isPodOwner = (c: Ctx, userId: string): boolean => c.s.owners.some((o) => o.id === userId);

/** Take, keep or restore a claim for `run`. With create false, only a restore after expiry may add one. */
function acquire(c: Ctx, run: Run, taskId: TaskId, create: boolean): void {
  const how = lostHow(c, run.id, taskId);
  if (how === 'released') return fail(c, 'forbidden', 'A person released this claim; it cannot be taken back.', taskId);
  const held = c.s.claims[taskId];
  if (held !== undefined && held.runId !== run.id) {
    if (!create && how !== 'expired') return ok(c);
    const holder = c.s.runs[held.runId].agent;
    return fail(c, 'already_claimed', `${holder.displayName} holds this task.`, holder.id);
  }
  if (held === undefined && (create || how === 'expired')) startClaim(c, run, taskId);
  const now = c.s.claims[taskId];
  return ok(c, now === undefined ? null : { taskId, leaseUntil: isoOf(now.leaseUntil) });
}

export function claimCmd(c: Ctx, cmd: Of<'claim'>): void {
  const run = liveRun(c, cmd.runId);
  if (run === null) return;
  renewRun(c, run.id);
  if (c.s.tasks[cmd.taskId] === undefined) return fail(c, 'not_found', 'No such task.', cmd.taskId);
  acquire(c, run, cmd.taskId, true);
}

/** Any other operation a run makes: it renews, and may restore an expired claim on `taskId`. */
export function opCmd(c: Ctx, cmd: Of<'op'>): void {
  const run = liveRun(c, cmd.runId);
  if (run === null) return;
  renewRun(c, run.id);
  if (cmd.taskId === undefined) return ok(c);
  if (c.s.tasks[cmd.taskId] === undefined) return fail(c, 'not_found', 'No such task.', cmd.taskId);
  acquire(c, run, cmd.taskId, false);
}

export function heartbeatCmd(c: Ctx, cmd: Of<'heartbeat'>): void {
  for (const run of Object.values(c.s.runs)) {
    if (run.status === 'active' && run.computerId === cmd.computerId) renewRun(c, run.id);
    if (run.stop !== undefined && run.computerId === cmd.computerId) push(c, run.computerId, 'run.stop', { runId: run.id, ...run.stop });
  }
  resendWakes(c, cmd.computerId);
  ok(c);
}

export function releaseCmd(c: Ctx, cmd: Of<'task_release'>): void {
  let by: Actor | undefined;
  if (cmd.caller.kind === 'run') {
    const run = liveRun(c, cmd.caller.runId);
    if (run === null) return;
    renewRun(c, run.id);
    by = run.agent;
  }
  if (c.s.tasks[cmd.taskId] === undefined) return fail(c, 'not_found', 'No such task.', cmd.taskId);
  const held = c.s.claims[cmd.taskId];
  if (held === undefined) return ok(c, taskView(c, cmd.taskId));
  const holder = c.s.runs[held.runId];
  if (cmd.caller.kind === 'user') {
    const uid = cmd.caller.userId;
    by = holder.owner.id === uid ? holder.owner : c.s.owners.find((o) => o.id === uid);
  } else if (cmd.caller.runId !== held.runId) by = undefined;
  if (by === undefined) return fail(c, 'forbidden', 'Only the holding run, its agent\'s owner or a pod owner may release.', cmd.taskId);
  endClaim(c, cmd.taskId, 'released', by, cmd.reason);
  ok(c, taskView(c, cmd.taskId));
}

export function runEndCmd(c: Ctx, cmd: Of<'run_end'>): void {
  const run = c.s.runs[cmd.runId];
  if (run === undefined) return fail(c, 'not_found', 'No such run.', cmd.runId);
  if (run.status === 'active') endRun(c, run, cmd.outcome, 'run_ended');
  delete run.stop;
  ok(c, { runId: run.id, outcome: run.outcome ?? cmd.outcome });
}

function stopper(c: Ctx, caller: Caller, run: Run): Actor | undefined {
  if (caller.kind !== 'user') return undefined;
  return run.owner.id === caller.userId ? run.owner : c.s.owners.find((o) => o.id === caller.userId);
}

export function runStopCmd(c: Ctx, cmd: Of<'run_stop'>): void {
  const run = c.s.runs[cmd.runId];
  if (run === undefined) return fail(c, 'not_found', 'No such run.', cmd.runId);
  const by = stopper(c, cmd.caller, run);
  if (by === undefined) return fail(c, 'forbidden', 'Only the agent\'s owner or a pod owner may stop a run.', run.id);
  if (run.status === 'ended') return ok(c, { runId: run.id, outcome: run.outcome ?? 'cancelled' });
  endRun(c, run, 'cancelled', 'stopped', by, cmd.reason);
  const data = cmd.reason === undefined ? { runId: run.id } : { runId: run.id, reason: cmd.reason };
  run.stop = cmd.reason === undefined ? {} : { reason: cmd.reason };
  push(c, run.computerId, 'run.stop', data);
  ok(c, { runId: run.id, outcome: 'cancelled' });
}
