// One function per command. Authorization is read from state, never from the input's own claims.
import type { Actor } from '../../gen/ts/protocol';
import { LEASE_MS, acquire, endClaim, endRun, liveRun, lostHow, renewRun, taskView, who } from './claims';
import { fail, ok, push } from './emit';
import { isoOf } from './time';
import { resendWakes } from './wakes';
import type { Cmd, Ctx, Run } from './types';

type Of<T extends Cmd['type']> = Extract<Cmd, { type: T }>;

const noTask = (c: Ctx, taskId: string): void => fail(c, 'not_found', 'No such task.', taskId);

export function claimCmd(c: Ctx, cmd: Of<'claim'>): void {
  const run = liveRun(c, cmd.runId);
  if (run === null) return;
  if (c.s.tasks[cmd.taskId] === undefined) return noTask(c, cmd.taskId);
  if (acquire(c, run, cmd.taskId, true)) return;
  ok(c, { taskId: cmd.taskId, leaseUntil: isoOf(c.s.claims[cmd.taskId].leaseUntil) });
}

/** Any other operation a run makes: it renews, and may restore an expired claim on `taskId`. */
export function opCmd(c: Ctx, cmd: Of<'op'>): void {
  const run = liveRun(c, cmd.runId);
  if (run === null) return;
  if (cmd.taskId === undefined) return renewRun(c, run.id), ok(c);
  if (c.s.tasks[cmd.taskId] === undefined) return noTask(c, cmd.taskId);
  if (acquire(c, run, cmd.taskId, false)) return;
  const mine = c.s.claims[cmd.taskId]?.runId === run.id;
  ok(c, mine ? { taskId: cmd.taskId, leaseUntil: isoOf(c.s.claims[cmd.taskId].leaseUntil) } : null);
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
  const w = who(c, cmd.caller);
  if (w === null) return;
  if (c.s.tasks[cmd.taskId] === undefined) return noTask(c, cmd.taskId);
  const { run } = w;
  if (run !== undefined && lostHow(c, run.id, cmd.taskId) === 'released') return fail(c, 'forbidden', 'A person released this claim; it cannot be taken back.', cmd.taskId);
  const held = c.s.claims[cmd.taskId];
  if (held === undefined) {
    if (run !== undefined) renewRun(c, run.id);
    return ok(c, taskView(c, cmd.taskId));
  }
  const holder = c.s.runs[held.runId];
  let by: Actor | undefined;
  if (run !== undefined) by = run.id === held.runId ? run.agent : undefined;
  else by = holder.owner.id === w.actor.id ? holder.owner : c.s.owners.find((o) => o.id === w.actor.id);
  if (by === undefined) return fail(c, 'forbidden', 'Only the holding run, its agent\'s owner or a pod owner may release.', cmd.taskId);
  if (run !== undefined) renewRun(c, run.id);
  endClaim(c, cmd.taskId, 'released', by, cmd.reason);
  ok(c, taskView(c, cmd.taskId));
}

export function runEndCmd(c: Ctx, cmd: Of<'run_end'>): void {
  const run = c.s.runs[cmd.runId];
  if (run === undefined) return fail(c, 'not_found', 'No such run.', cmd.runId);
  if (run.computerId !== cmd.computerId) return fail(c, 'forbidden', 'This computer does not host that run.', run.id);
  if (run.status === 'active') endRun(c, run, cmd.outcome, 'run_ended');
  delete run.stop;
  ok(c, { runId: run.id, outcome: run.outcome ?? cmd.outcome });
}

function stopper(c: Ctx, actor: Actor, run: Run, isUser: boolean): Actor | undefined {
  if (!isUser) return undefined;
  return run.owner.id === actor.id ? run.owner : c.s.owners.find((o) => o.id === actor.id);
}

export function runStopCmd(c: Ctx, cmd: Of<'run_stop'>): void {
  const w = who(c, cmd.caller);
  if (w === null) return;
  const run = c.s.runs[cmd.runId];
  if (run === undefined) return fail(c, 'not_found', 'No such run.', cmd.runId);
  const by = stopper(c, w.actor, run, w.run === undefined);
  if (by === undefined) return fail(c, 'forbidden', 'Only the agent\'s owner or a pod owner may stop a run.', run.id);
  if (run.status === 'ended') return ok(c, { runId: run.id, outcome: run.outcome ?? 'cancelled' });
  endRun(c, run, 'cancelled', 'stopped', by, cmd.reason);
  const data = cmd.reason === undefined ? { runId: run.id } : { runId: run.id, reason: cmd.reason };
  run.stop = cmd.reason === undefined ? {} : { reason: cmd.reason };
  push(c, run.computerId, 'run.stop', data);
  ok(c, { runId: run.id, outcome: 'cancelled' });
}

/** Restart: stretch each live lease by up to `staleMs` (never past now + 60 s, never shorter), then settle at `now`. */
export function bootCmd(c: Ctx, cmd: Of<'boot'>): void {
  if (!Number.isInteger(cmd.staleMs) || cmd.staleMs < 0 || cmd.staleMs > LEASE_MS) throw new Error('boot: staleMs must be an integer from 0 to LEASE_MS');
  for (const k of Object.values(c.s.claims)) k.leaseUntil = Math.max(k.leaseUntil, Math.min(k.leaseUntil + cmd.staleMs, c.now + LEASE_MS));
}
