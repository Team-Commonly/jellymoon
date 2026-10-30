import { describe, expect, test } from 'bun:test';
import { apply, isoOf, nextDeadline } from '../src';
import type { Cmd } from '../src';
import {
  CMP_1, CMP_2, RUN_A, RUN_B, RUN_C, TASKS, U_AGENT_OWNER, U_OWNER, U_STRANGER, errCode, events, id, lastReply, play, released, types, world,
} from './helpers';

const [T1, T2] = TASKS;
const claimA = (t = T1): Cmd => ({ type: 'claim', runId: RUN_A, taskId: t });
const releaseBy = (userId: string, t = T1): Cmd => ({ type: 'task_release', caller: { kind: 'user', userId }, taskId: t });
const stopBy = (userId: string, runId = RUN_A, reason?: string): Cmd => ({ type: 'run_stop', caller: { kind: 'user', userId }, runId, ...(reason ? { reason } : {}) });

describe('claims', () => {
  test('one claim per task: the loser gets already_claimed naming the holder', () => {
    const r = play([[claimA(), 1000], [{ type: 'claim', runId: RUN_B, taskId: T1 }, 2000]]);
    const reply = lastReply(r.out);
    expect(errCode(r.out)).toBe('already_claimed');
    expect(reply?.kind === 'reply' && !reply.ok && reply.error.ref).toBe(r.state.runs[RUN_A].agent.id);
    expect(r.state.claims[T1].runId).toBe(RUN_A);
  });

  test('a claim announces itself and sets leaseUntil to the claim time plus 60000 ms', () => {
    const r = play([[claimA(), 5000]]);
    const e = events(r.out)[0];
    expect(e.type).toBe('task.claimed');
    expect(e.data).toEqual({ taskId: T1, by: r.state.runs[RUN_A].agent, runId: RUN_A, leaseUntil: isoOf(65_000) });
  });

  test('an operation by the holding run renews to that moment plus 60000 ms', () => {
    const r = play([[claimA(), 0], [{ type: 'op', runId: RUN_A }, 30_000]]);
    expect(r.state.claims[T1].leaseUntil).toBe(90_000);
  });

  test('a heartbeat from the computer hosting the run renews', () => {
    const r = play([[claimA(), 0], [{ type: 'heartbeat', computerId: CMP_1 }, 45_000]]);
    expect(r.state.claims[T1].leaseUntil).toBe(105_000);
  });

  test('a heartbeat from a DIFFERENT computer does not renew', () => {
    const r = play([[claimA(), 0], [{ type: 'heartbeat', computerId: CMP_2 }, 45_000]]);
    expect(r.state.claims[T1].leaseUntil).toBe(60_000);
  });

  test('a heartbeat does not renew a run that has ended', () => {
    const r = play([[claimA(), 0], [{ type: 'run_end', runId: RUN_A, computerId: CMP_1, outcome: 'completed' }, 10_000], [{ type: 'heartbeat', computerId: CMP_1 }, 20_000]]);
    expect(r.state.claims).toEqual({});
  });
});

describe('expiry', () => {
  test('boundary: leaseUntil - 1 keeps the claim, leaseUntil drops it', () => {
    const base = play([[claimA(), 0]]);
    expect(apply(base.state, { type: 'tick' }, 59_999).out).toEqual([]);
    const at = apply(base.state, { type: 'tick' }, 60_000);
    expect(types(at.out)).toEqual(['task.updated', 'claim.released']);
    expect(at.state.claims).toEqual({});
  });

  test('expiry drops the claim, keeps status and assignee, and does not stop the run', () => {
    const r = play([[claimA(), 0], [{ type: 'tick' }, 60_000]]);
    const upd = events(r.out)[1];
    expect(upd.type).toBe('task.updated');
    expect(upd.data).toEqual(r.state.tasks[T1]);
    expect('claim' in r.state.tasks[T1]).toBe(false);
    expect(r.state.tasks[T1].status).toBe('open');
    expect(r.state.tasks[T1].assignee).toBe(world().tasks[T1].assignee);
    expect(released(r.out)).toEqual([{ taskId: T1, runId: RUN_A, cause: 'expired' }]);
    expect(r.state.runs[RUN_A].status).toBe('active');
    expect(r.out.some((o) => o.kind === 'to-computer')).toBe(false);
  });

  test('restore after expiry when still unclaimed', () => {
    const r = play([[claimA(), 0], [{ type: 'tick' }, 60_000], [{ type: 'op', runId: RUN_A, taskId: T1 }, 70_000]]);
    expect(r.state.claims[T1]).toEqual({ runId: RUN_A, leaseUntil: 130_000 });
    expect(types(r.out).slice(-1)).toEqual(['task.claimed']);
  });

  test('restore after expiry is already_claimed when someone else holds the task', () => {
    const r = play([[claimA(), 0], [{ type: 'tick' }, 60_000], [{ type: 'claim', runId: RUN_B, taskId: T1 }, 61_000], [{ type: 'op', runId: RUN_A, taskId: T1 }, 62_000]]);
    expect(errCode(r.out)).toBe('already_claimed');
    expect(r.state.claims[T1].runId).toBe(RUN_B);
  });

  test('an operation that arrives at the lease boundary finds it already expired, not renewed', () => {
    const r = play([[claimA(), 0], [{ type: 'op', runId: RUN_A }, 60_000]]);
    expect(released(r.out).map((x) => x.cause)).toEqual(['expired']);
  });

  test('a duplicate tick at the same now emits nothing', () => {
    const first = play([[claimA(), 0], [{ type: 'tick' }, 60_000]]);
    const again = apply(first.state, { type: 'tick' }, 60_000);
    expect(again.out).toEqual([]);
    expect(again.state).toEqual(first.state);
  });

  test('nextDeadline is the earliest lease or run cap, and null with nothing to do', () => {
    expect(nextDeadline(world())).toBe(7_200_000);
    expect(nextDeadline(play([[claimA(), 1000]]).state)).toBe(61_000);
    const ended = play([[{ type: 'run_end', runId: RUN_A, computerId: CMP_1, outcome: 'completed' }, 1], [{ type: 'run_end', runId: RUN_B, computerId: CMP_1, outcome: 'completed' }, 1], [{ type: 'run_end', runId: RUN_C, computerId: CMP_2, outcome: 'completed' }, 1]]);
    expect(nextDeadline(ended.state)).toBeNull();
  });
});

describe('run end and token cap', () => {
  test('run_end releases the run claims with cause run_ended', () => {
    const r = play([[claimA(), 0], [claimA(T2), 0], [{ type: 'run_end', runId: RUN_A, computerId: CMP_1, outcome: 'completed' }, 5000]]);
    expect(released(r.out)).toEqual([{ taskId: T1, runId: RUN_A, cause: 'run_ended' }, { taskId: T2, runId: RUN_A, cause: 'run_ended' }]);
    expect(r.state.runs[RUN_A].outcome).toBe('completed');
  });

  test('the 2 h run-token cap is enforced by tick and releases with run_ended', () => {
    const r = play([[claimA(), 7_130_000], [{ type: 'heartbeat', computerId: CMP_1 }, 7_189_000], [{ type: 'tick' }, 7_200_000]]);
    expect(released(r.out)).toEqual([{ taskId: T1, runId: RUN_A, cause: 'run_ended' }]);
    expect(r.state.runs[RUN_A]).toMatchObject({ status: 'ended', outcome: 'failed' });
    expect(errCode(play([[{ type: 'op', runId: RUN_A }, 7_200_001]], r.state).out)).toBe('unauthorized');
  });
});

describe('recourse: task_release', () => {
  for (const [label, who] of [['the agent owner', U_AGENT_OWNER], ['a pod owner', U_OWNER]] as const) {
    test(`${label} releases with cause released and by`, () => {
      const r = play([[claimA(), 0], [releaseBy(who.id), 1000]]);
      expect(released(r.out)).toEqual([{ taskId: T1, runId: RUN_A, cause: 'released', by: who }]);
      expect(types(r.out)).toEqual(['task.claimed', 'task.updated', 'claim.released']);
    });
  }

  test('the holding run releases its own claim, with a reason', () => {
    const cmd: Cmd = { type: 'task_release', caller: { kind: 'run', runId: RUN_A }, taskId: T1, reason: 'switching' };
    const r = play([[claimA(), 0], [cmd, 1000]]);
    expect(released(r.out)).toEqual([{ taskId: T1, runId: RUN_A, cause: 'released', by: r.state.runs[RUN_A].agent, reason: 'switching' }]);
  });

  test('a plain member (neither pod owner nor agent owner) cannot release or stop', () => {
    const member: Cmd = { type: 'member_add', userId: U_STRANGER.id, name: U_STRANGER.displayName };
    const r = play([[claimA(), 0], [member, 1], [releaseBy(U_STRANGER.id), 2], [stopBy(U_STRANGER.id), 3]]);
    expect(r.out.filter((o) => o.kind === 'reply' && !o.ok).map((o) => o.kind === 'reply' && !o.ok && o.error.code)).toEqual(['forbidden', 'forbidden']);
    expect(r.state.claims[T1].runId).toBe(RUN_A);
    expect(r.state.runs[RUN_A].status).toBe('active');
  });

  test('anyone else gets forbidden: a stranger, or a different run', () => {
    const base = play([[claimA(), 0]]);
    expect(errCode(play([[releaseBy(U_STRANGER.id), 1000]], base.state).out)).toBe('forbidden');
    const other: Cmd = { type: 'task_release', caller: { kind: 'run', runId: RUN_B }, taskId: T1 };
    expect(errCode(play([[other, 1000]], base.state).out)).toBe('forbidden');
  });

  test('releasing an unclaimed task is a no-op that returns the task', () => {
    const r = play([[releaseBy(U_OWNER.id), 1000]]);
    expect(events(r.out)).toEqual([]);
    const reply = lastReply(r.out);
    expect(reply?.kind === 'reply' && reply.ok && reply.result).toEqual(world().tasks[T1]);
  });

  test('a run whose claim a person released can never restore it (forbidden), even if the task is free', () => {
    const r = play([[claimA(), 0], [releaseBy(U_OWNER.id), 1000], [{ type: 'op', runId: RUN_A, taskId: T1 }, 2000]]);
    expect(errCode(r.out)).toBe('forbidden');
    expect(r.state.claims).toEqual({});
    expect(errCode(play([[claimA(), 3000]], r.state).out)).toBe('forbidden');
  });
});

describe('recourse: run_stop', () => {
  test('stops: revokes the token, releases with stopped and by, cancels, and tells the computer', () => {
    const r = play([[claimA(), 0], [stopBy(U_OWNER.id, RUN_A, 'hung'), 1000]]);
    expect(released(r.out)).toEqual([{ taskId: T1, runId: RUN_A, cause: 'stopped', by: U_OWNER, reason: 'hung' }]);
    expect(r.state.runs[RUN_A]).toMatchObject({ status: 'ended', outcome: 'cancelled' });
    const stop = r.out.find((o) => o.kind === 'to-computer');
    expect(stop).toMatchObject({ computerId: CMP_1, event: { type: 'run.stop', data: { runId: RUN_A, reason: 'hung' } } });
    expect(errCode(play([[{ type: 'op', runId: RUN_A }, 1001]], r.state).out)).toBe('unauthorized');
  });

  test('the agent owner may stop too; a stranger and a run token may not', () => {
    expect(errCode(play([[stopBy(U_AGENT_OWNER.id), 1]]).out)).toBe(null);
    expect(errCode(play([[stopBy(U_STRANGER.id), 1]]).out)).toBe('forbidden');
    const byRun: Cmd = { type: 'run_stop', caller: { kind: 'run', runId: RUN_B }, runId: RUN_A };
    expect(errCode(play([[byRun, 1]]).out)).toBe('forbidden');
  });

  test('stopping an ended run returns its outcome unchanged and emits nothing', () => {
    const base = play([[{ type: 'run_end', runId: RUN_A, computerId: CMP_1, outcome: 'completed' }, 1]]);
    const r = play([[stopBy(U_OWNER.id), 2]], base.state);
    const reply = lastReply(r.out);
    expect(reply?.kind === 'reply' && reply.ok && reply.result).toEqual({ runId: RUN_A, outcome: 'completed' });
    expect(r.out.length).toBe(1);
    expect(r.state.runs[RUN_A].outcome).toBe('completed');
  });
});

describe('members events', () => {
  test('every claim that ends emits exactly one claim.released, whatever the cause', () => {
    const ends: Cmd[] = [{ type: 'tick' }, releaseBy(U_OWNER.id), { type: 'run_end', runId: RUN_A, computerId: CMP_1, outcome: 'failed' }, stopBy(U_OWNER.id)];
    ends.forEach((end) => {
      const r = play([[claimA(), 0], [end, end.type === 'tick' ? 60_000 : 1000]]);
      expect(released(r.out).length).toBe(1);
      expect(Object.keys(r.state.claims).length).toBe(0);
    });
  });

  test('seq is per pod, starts at 1 and rises by one', () => {
    const r = play([[claimA(), 0], [claimA(T2), 0], [{ type: 'tick' }, 60_000]]);
    expect(events(r.out).map((e) => e.seq)).toEqual([1, 2, 3, 4, 5, 6]);
  });

  test('isoOf agrees with the platform clock on awkward instants', () => {
    for (const ms of [0, 951_782_400_000, 1_782_431_999_999, 4_102_444_800_123]) expect(isoOf(ms)).toBe(new Date(ms).toISOString());
  });
});

describe('one claim per task', () => {
  const upd = (caller: { kind: 'run'; runId: string } | { kind: 'user'; userId: string }): Cmd => ({ type: 'task_update', caller, taskId: T1, status: 'in_progress', wakeId: id('wak', 1) });
  test('a run\'s task_update on a task another live run holds is already_claimed naming the holder; a person may update', () => {
    const held = play([[claimA(), 0]]).state;
    const r = play([[upd({ kind: 'run', runId: RUN_B }), 1]], held);
    const reply = lastReply(r.out);
    expect([errCode(r.out), reply?.kind === 'reply' && !reply.ok && reply.error.ref]).toEqual(['already_claimed', held.runs[RUN_A].agent.id]);
    expect(r.state.tasks[T1].status).toBe('open');
    expect(errCode(play([[upd({ kind: 'user', userId: U_OWNER.id }), 1]], held).out)).toBeNull();
    expect(errCode(play([[upd({ kind: 'run', runId: RUN_A }), 1]], held).out)).toBeNull();
  });
  test('now past the end of year 9999 is refused as invalid_input and changes nothing', () => {
    const r = apply(world(), { type: 'tick' }, 253_402_300_800_000);
    expect([errCode(r.out), r.state]).toEqual(['invalid_input', world()]);
    expect(errCode(apply(world(), { type: 'tick' }, 253_402_300_799_999).out)).toBeNull();
  });
  test('run_end with an outcome outside the schema enum is invalid_input and the run stays active', () => {
    const r = apply(world(), { type: 'run_end', runId: RUN_A, computerId: CMP_1, outcome: 'done' as never }, 5);
    expect([errCode(r.out), r.state.runs[RUN_A].status]).toEqual(['invalid_input', 'active']);
  });
  test('a malformed call is invalid_input and leaves the state alone: missing or null caller, bad caller kind, null refs', () => {
    const base = { taskId: T1, wakeId: id('wak', 1) };
    const bad: unknown[] = [
      { type: 'task_release', taskId: T1 }, { type: 'task_release', caller: null, taskId: T1 }, { type: 'run_stop', runId: RUN_A }, { type: 'run_stop', caller: null, runId: RUN_A },
      { type: 'task_create', ...base, title: 't', kind: 'chore' }, { type: 'task_update', ...base, caller: null },
      { type: 'message_post', messageId: id('msg', 1), text: 'x', wakeId: id('wak', 1) }, { type: 'message_post', caller: null, messageId: id('msg', 1), text: 'x', wakeId: id('wak', 1) },
      { type: 'task_release', caller: { kind: 'other', userId: 'u' }, taskId: T1 }, { type: 'task_release', caller: { kind: 'run' }, taskId: T1 },
      { type: 'change', kind: 'ruling', agentId: id('agt', 1), refs: null, wakeId: id('wak', 1) }, { type: 'claim', runId: 5, taskId: T1 },
    ];
    for (const cmd of bad) {
      const r = apply(world(), cmd as Cmd, 5);
      expect([JSON.stringify(cmd), errCode(r.out), r.state]).toEqual([JSON.stringify(cmd), 'invalid_input', world()]);
    }
  });
  test('a run\'s message_post without a summary is summary_required; a person\'s is derived; a given summary is used', () => {
    const post = (caller: { kind: 'run'; runId: string } | { kind: 'user'; userId: string }, summary?: string): Cmd => ({ type: 'message_post', caller, messageId: id('msg', 1), text: 'body', summary, wakeId: id('wak', 1) });
    const run = { kind: 'run' as const, runId: RUN_A };
    expect(errCode(apply(world(), post(run), 5).out)).toBe('summary_required');
    expect(errCode(apply(world(), post(run, '  '), 5).out)).toBe('summary_required');
    expect((events(apply(world(), post(run, 'Done'), 5).out)[0].data as { summary: string }).summary).toBe('Done');
    expect((events(apply(world(), post({ kind: 'user', userId: U_OWNER.id }), 5).out)[0].data as { summary: string }).summary).toBe('body');
  });
  test('a live run\'s refused command still renews its claims (ADR 0002): T1 is held to 110 000 in all four sequences', () => {
    const wid = id('wak', 1);
    const refused: Cmd[] = [
      { type: 'op', runId: RUN_A, taskId: 'constructor' }, { type: 'op', runId: RUN_A, taskId: 'nonexistent' },
      { type: 'task_create', caller: { kind: 'run', runId: RUN_A }, taskId: id('tsk', 30), title: 't', kind: 'chore' } as unknown as Cmd,
      { type: 'task_update', caller: { kind: 'run', runId: RUN_A }, taskId: T1, assignee: '__proto__', wakeId: wid },
    ];
    for (const cmd of refused) {
      const r = play([[claimA(), 0], [cmd, 50_000], [{ type: 'tick' }, 109_999]]);
      expect([JSON.stringify(cmd), r.state.claims[T1]?.runId]).toEqual([JSON.stringify(cmd), RUN_A]);
      expect(play([[{ type: 'tick' }, 110_000]], r.state).state.claims[T1]).toBe(undefined);
    }
  });
});

describe('a refused call is still a call', () => {
  test('a run\'s task_update on a free task does not create a claim', () => {
    const upd: Cmd = { type: 'task_update', caller: { kind: 'run', runId: RUN_A }, taskId: T1, status: 'in_progress', wakeId: id('wak', 1) };
    const r = play([[upd, 0]]);
    expect([errCode(r.out), r.state.claims[T1]]).toEqual([null, undefined]);
  });
  test('a refused call settles first, then renews: an expired lease is released, not revived (run caller and user caller)', () => {
    const refusedAt61: Cmd[] = [{ type: 'op', runId: RUN_A, taskId: 'constructor' }, { type: 'task_release', caller: { kind: 'user', userId: U_OWNER.id }, taskId: '__proto__' }];
    for (const cmd of refusedAt61) {
      const r = play([[claimA(), 0], [cmd, 61_000]]);
      expect([JSON.stringify(cmd), released(r.out), r.state.claims[T1]]).toEqual([JSON.stringify(cmd), [{ taskId: T1, runId: RUN_A, cause: 'expired' }], undefined]);
    }
  });
  test('a plain member\'s run_stop on an ENDED run stays forbidden', () => {
    const add: Cmd = { type: 'member_add', userId: U_STRANGER.id, name: 'x' };
    const r = play([[add, 0], [{ type: 'run_end', runId: RUN_A, computerId: CMP_1, outcome: 'completed' }, 1], [{ type: 'run_stop', caller: { kind: 'user', userId: U_STRANGER.id }, runId: RUN_A }, 2]]);
    expect(errCode(r.out)).toBe('forbidden');
  });
  test('a run\'s no-op release of an expired claim keeps its right to restore it', () => {
    const rel: Cmd = { type: 'task_release', caller: { kind: 'run', runId: RUN_A }, taskId: T1 };
    const r = play([[claimA(), 0], [rel, 61_000], [{ type: 'op', runId: RUN_A, taskId: T1 }, 62_000]]);
    expect(r.state.claims[T1]?.runId).toBe(RUN_A);
  });
  test('a heartbeat renews every live run on its computer, not only the first', () => {
    const r = play([[claimA(), 0], [{ type: 'claim', runId: RUN_B, taskId: T2 }, 0], [{ type: 'heartbeat', computerId: CMP_1 }, 50_000], [{ type: 'tick' }, 109_999]]);
    expect(Object.keys(r.state.claims).sort()).toEqual([T1, T2]);
  });
  test('a null or non-object command, and array-valued ids, are invalid_input and change nothing', () => {
    for (const cmd of [null, 7, 'tick', [], { type: 'op', runId: RUN_A, taskId: [T1] }, { type: 'task_update', caller: { kind: 'user', userId: U_OWNER.id }, taskId: T1, assignee: [], wakeId: id('wak', 1) }]) {
      const r = apply(world(), cmd as unknown as Cmd, 5);
      expect([JSON.stringify(cmd), errCode(r.out), r.state]).toEqual([JSON.stringify(cmd), 'invalid_input', world()]);
    }
  });
});
