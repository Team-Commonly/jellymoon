import { describe, expect, test } from 'bun:test';
import { apply, isoOf } from '../src';
import type { Cmd } from '../src';
import { CMP_1, RUN_A, RUN_B, TASKS, U_OWNER, errCode, events, id, lastReply, play, released, types, world } from './helpers';

const [T1, T2] = TASKS;
const CAP = 7_200_000;
const claim = (t = T1, runId = RUN_A): Cmd => ({ type: 'claim', runId, taskId: t });
const noReply = (out: ReturnType<typeof play>['out']) => out.filter((o) => o.kind !== 'reply');

describe('settle walks one timeline', () => {
  test('cap before leaseUntil, late tick: run_ended, stamped with the cap', () => {
    const r = play([[claim(), CAP - 50_000], [{ type: 'tick' }, CAP + 100_000]]);
    expect(released(r.out)).toEqual([{ taskId: T1, runId: RUN_A, cause: 'run_ended' }]);
    expect(events(r.out).slice(1).map((e) => e.ts)).toEqual([isoOf(CAP), isoOf(CAP)]);
  });

  test('leaseUntil before the cap, late tick: expired, stamped with leaseUntil, and not ended again by the cap', () => {
    const r = play([[claim(), 0], [{ type: 'tick' }, CAP + 5]]);
    expect(released(r.out)).toEqual([{ taskId: T1, runId: RUN_A, cause: 'expired' }]);
    expect(r.state.tasks[T1].updatedAt).toBe(isoOf(60_000));
    expect(events(r.out).slice(1).map((e) => e.ts)).toEqual([isoOf(60_000), isoOf(60_000)]);
    expect(r.state.runs[RUN_A].status).toBe('ended');
  });

  test('a tie goes to the run cap', () => {
    const r = play([[claim(), CAP - 60_000], [{ type: 'tick' }, CAP]]);
    expect(released(r.out)).toEqual([{ taskId: T1, runId: RUN_A, cause: 'run_ended' }]);
  });

  test('a late tick and an on-time tick emit identical out; so do an early request and a late tick', () => {
    const base = play([[claim(), 0], [claim(T2, RUN_B), 10_000]]).state;
    const at = (cmd: Cmd, now: number) => noReply(apply(base, cmd, now).out);
    expect(at({ type: 'tick' }, 100_000)).toEqual(at({ type: 'tick' }, 70_000));
    expect(at({ type: 'op', runId: RUN_B }, 100_000)).toEqual(at({ type: 'tick' }, 70_000));
    expect(apply(base, { type: 'tick' }, 100_000).state).toEqual(apply(base, { type: 'tick' }, 70_000).state);
  });
});

describe('lost=released is recorded for a person only', () => {
  const selfRelease: Cmd = { type: 'task_release', caller: { kind: 'run', runId: RUN_A }, taskId: T1 };
  test('a run that released its own claim may claim again', () => {
    const r = play([[claim(), 0], [selfRelease, 1], [claim(), 2]]);
    expect(errCode(r.out)).toBeNull();
    expect(r.state.claims[T1].runId).toBe(RUN_A);
    expect(r.state.lost).toEqual({});
  });
  test('after a self-release an op behaves as on any free task', () => {
    const a = play([[claim(), 0], [selfRelease, 1], [{ type: 'op', runId: RUN_A, taskId: T1 }, 2]]);
    const b = play([[{ type: 'op', runId: RUN_A, taskId: T1 }, 2]]);
    expect(lastReply(a.out)).toEqual(lastReply(b.out));
    expect(a.state.claims).toEqual({});
  });
  test('a person\'s release is recorded by run, then task; the run\'s own task_release is forbidden too', () => {
    const r = play([[claim(), 0], [{ type: 'task_release', caller: { kind: 'user', userId: U_OWNER.id }, taskId: T1 }, 1]]);
    expect(r.state.lost).toEqual({ [RUN_A]: { [T1]: 'released' } });
    expect(errCode(play([[selfRelease, 2]], r.state).out)).toBe('forbidden');
  });
});

describe('a refused command renews nothing; a successful one renews the others', () => {
  const lease = (r: ReturnType<typeof play>, t: string) => r.state.claims[t].leaseUntil;
  test('task_claim renews the run\'s other claims', () => {
    expect(lease(play([[claim(), 0], [claim(T2), 30_000]]), T1)).toBe(90_000);
  });
  test('a run\'s own task_release renews its other claims', () => {
    const release: Cmd = { type: 'task_release', caller: { kind: 'run', runId: RUN_A }, taskId: T1 };
    expect(lease(play([[claim(), 0], [claim(T2), 0], [release, 30_000]]), T2)).toBe(90_000);
  });
  test('already_claimed, not_found and forbidden renew nothing', () => {
    const base = play([[claim(), 0], [claim(T2, RUN_B), 0]]);
    const refused = (cmd: Cmd) => lease(play([[cmd, 30_000]], base.state), T2);
    expect(refused(claim(T1, RUN_B))).toBe(60_000);
    expect(refused({ type: 'claim', runId: RUN_B, taskId: id('tsk', 99) })).toBe(60_000);
    expect(refused({ type: 'task_release', caller: { kind: 'run', runId: RUN_B }, taskId: T1 })).toBe(60_000);
  });
});

describe('run callers and run_end', () => {
  const ended = play([[{ type: 'run_end', runId: RUN_A, computerId: CMP_1, outcome: 'completed' }, 1]]).state;
  test('an ended run gets unauthorized on every command, before any other check', () => {
    const caller = { kind: 'run' as const, runId: RUN_A };
    const cmds: Cmd[] = [
      claim(), { type: 'op', runId: RUN_A, taskId: id('tsk', 99) }, { type: 'task_release', caller, taskId: id('tsk', 99) }, { type: 'run_stop', caller, runId: id('run', 99) },
      { type: 'task_create', caller, taskId: T1, title: 'x', kind: 'chore' }, { type: 'task_update', caller, taskId: id('tsk', 99), status: 'done' },
      { type: 'message_post', caller, messageId: id('msg', 1), text: 'hi' },
    ];
    for (const cmd of cmds) expect(errCode(play([[cmd, 2]], ended).out)).toBe('unauthorized');
  });
  test('a live run calling run_stop gets forbidden', () => {
    expect(errCode(play([[{ type: 'run_stop', caller: { kind: 'run', runId: RUN_B }, runId: RUN_B }, 2]]).out)).toBe('forbidden');
  });
  test('run_end from a computer that does not host the run is forbidden and ends nothing', () => {
    const r = play([[{ type: 'run_end', runId: RUN_A, computerId: id('cmp', 2), outcome: 'completed' }, 2]]);
    expect(errCode(r.out)).toBe('forbidden');
    expect(r.state.runs[RUN_A].status).toBe('active');
  });
  test('run_end after run_stop keeps outcome cancelled', () => {
    const stop: Cmd = { type: 'run_stop', caller: { kind: 'user', userId: U_OWNER.id }, runId: RUN_A };
    const r = play([[stop, 1], [{ type: 'run_end', runId: RUN_A, computerId: CMP_1, outcome: 'completed' }, 2]]);
    const reply = lastReply(r.out);
    expect(reply?.kind === 'reply' && reply.ok && reply.result).toEqual({ runId: RUN_A, outcome: 'cancelled' });
    expect(r.state.runs[RUN_A].outcome).toBe('cancelled');
  });
  test('a task_update by a run follows the restore path and is forbidden after a person\'s release', () => {
    const upd: Cmd = { type: 'task_update', caller: { kind: 'run', runId: RUN_A }, taskId: T1, status: 'in_progress' };
    const r = play([[claim(), 0], [{ type: 'tick' }, 60_000], [upd, 61_000]]);
    expect(r.state.claims[T1].runId).toBe(RUN_A);
    expect(types(r.out).slice(-2)).toEqual(['task.claimed', 'task.updated']);
    const gone = play([[claim(), 0], [{ type: 'task_release', caller: { kind: 'user', userId: U_OWNER.id }, taskId: T1 }, 1], [upd, 2]]);
    expect(errCode(gone.out)).toBe('forbidden');
    expect(world().tasks[T1].status).toBe('open');
  });
});
