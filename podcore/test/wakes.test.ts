import { describe, expect, test } from 'bun:test';
import type { Cmd } from '../src';
import { AG1, AG3, AG_UNREG, CMP_1, CMP_2, RUN_A, TASKS, U_AGENT_OWNER, U_OWNER, errCode, events, lastReply, play, runN, toComputer, wak, world } from './helpers';

const [T1, T2] = TASKS;
const change = (n: number, refs = {}, agentId = AG1, kind: 'task_assigned' | 'mention' | 'ruling' = 'task_assigned'): Cmd => ({ type: 'change', kind, agentId, refs, wakeId: wak(n) });
const hb = (computerId: string): Cmd => ({ type: 'heartbeat', computerId });
const claimWake = (n: number, computerId = CMP_1, r = 1): Cmd => ({ type: 'run_claim', wakeId: wak(n), computerId, runId: runN(r) });
const wakes = (out: ReturnType<typeof play>['out']) => toComputer(out).filter((o) => o.event.type === 'run.wake');
const stops = (out: ReturnType<typeof play>['out']) => toComputer(out).filter((o) => o.event.type === 'run.stop');

describe('a change writes a wake only for the agent it addresses', () => {
  test('a task assigned to the agent writes one wake and sends run.wake to its computer only, with no members event', () => {
    const r = play([[change(1, { taskId: T1 }), 1000]]);
    expect(r.out.filter((o) => o.kind !== 'reply')).toEqual([
      { kind: 'to-computer', computerId: CMP_1, event: { v: 1, type: 'run.wake', podId: r.state.podId, ts: '1970-01-01T00:00:01.000Z', data: { wakeId: wak(1), agentId: AG1, reason: 'task_assigned', refs: { taskId: T1 } } } },
    ]);
    expect(r.state.seq).toBe(0);
    expect(errCode(r.out)).toBeNull();
  });

  test('kinds map to reasons: mention is mention, ruling is decision_ruled', () => {
    const reasons = (['mention', 'ruling'] as const).map((k) => {
      const e = wakes(play([[change(1, {}, AG1, k), 0]]).out)[0].event;
      return e.type === 'run.wake' ? e.data.reason : null;
    });
    expect(reasons).toEqual(['mention', 'decision_ruled']);
  });

  test('an unaddressed kind (an ambient message) writes nothing', () => {
    const r = play([[{ type: 'change', kind: 'message' as never, agentId: AG1, refs: {}, wakeId: wak(1) }, 0]]);
    expect(r.state.wakes).toEqual({});
    expect(r.out.filter((o) => o.kind !== 'reply')).toEqual([]);
  });

  test('a change for an unregistered agent is dropped: no wake is held for it', () => {
    const r = play([[change(1, {}, AG_UNREG), 0], [{ type: 'agent_register', agentId: AG_UNREG, ownerId: U_AGENT_OWNER.id, computerId: CMP_1 }, 1]]);
    expect(r.state.wakes).toEqual({});
    expect(r.out.filter((o) => o.kind === 'to-computer')).toEqual([]);
  });
});

describe('coalescing', () => {
  test('three changes in a row leave one pending wake, delivered once, under the first wakeId', () => {
    const r = play([[change(1, { taskId: T1 }), 0], [change(2, { taskId: T2 }), 1], [change(3, { messageId: 'msg_00000000000000000000000001' }), 2]]);
    expect(wakes(r.out).length).toBe(1);
    expect(Object.values(r.state.wakes).length).toBe(1);
    expect(r.state.wakes[AG1]).toMatchObject({ id: wak(1), refs: { taskId: T1, messageId: 'msg_00000000000000000000000001' } });
  });

  test('a repeated ref is deduped, and the first ref wins where the schema holds only one', () => {
    const r = play([[change(1, { taskId: T1 }), 0], [change(2, { taskId: T1 }), 1], [change(3, { taskId: T2 }), 2]]);
    expect(r.state.wakes[AG1].refs).toEqual({ taskId: T1 });
  });

  test('wakes for different agents do not coalesce', () => {
    const r = play([[change(1), 0], [change(2, {}, AG3), 1]]);
    expect(Object.keys(r.state.wakes).sort()).toEqual([AG1, AG3]);
    expect(toComputer(r.out).map((o) => o.computerId)).toEqual([CMP_1, CMP_2]);
  });

  test('once a run_claim consumes the wake, the next change starts a new one', () => {
    const r = play([[change(1), 0], [claimWake(1), 1], [change(2, { taskId: T2 }), 2]]);
    expect(r.state.wakes[AG1].id).toBe(wak(2));
    expect(wakes(r.out).length).toBe(2);
  });
});

describe('resend', () => {
  const sent = play([[change(1), 0]]).state;
  const at = (cmd: Cmd, now: number) => play([[cmd, now]], sent).out;

  test('boundary: 59 999 ms resends nothing, 60 000 ms resends to the next heartbeat', () => {
    expect(wakes(at(hb(CMP_1), 59_999))).toEqual([]);
    expect(wakes(at(hb(CMP_1), 60_000)).length).toBe(1);
  });

  test('a heartbeat from another computer resends nothing, and neither does a tick', () => {
    expect(wakes(at(hb(CMP_2), 90_000))).toEqual([]);
    expect(wakes(at({ type: 'tick' }, 90_000))).toEqual([]);
  });

  test('an acked wake is never resent', () => {
    const r = play([[{ type: 'wake_ack', wakeId: wak(1), computerId: CMP_1 }, 10], [hb(CMP_1), 500_000]], sent);
    expect(wakes(r.out)).toEqual([]);
  });

  test('a resend restarts the 60 s wait', () => {
    const r = play([[hb(CMP_1), 60_000], [hb(CMP_1), 100_000], [hb(CMP_1), 120_000]], sent);
    expect(wakes(r.out).length).toBe(2);
  });

  test('wake_ack from the wrong computer is ignored', () => {
    const r = play([[{ type: 'wake_ack', wakeId: wak(1), computerId: CMP_2 }, 10], [hb(CMP_1), 60_000]], sent);
    expect(wakes(r.out).length).toBe(1);
  });

  test('a claimed wake is gone, so nothing is resent', () => {
    expect(wakes(play([[claimWake(1), 5], [hb(CMP_1), 70_000]], sent).out)).toEqual([]);
  });
});

describe('run.stop resend', () => {
  const stopped = play([[{ type: 'run_stop', caller: { kind: 'user', userId: U_OWNER.id }, runId: RUN_A, reason: 'hung' }, 1000]]).state;

  test('re-sent on every heartbeat from the hosting computer, with no wait, until run_end', () => {
    const r = play([[hb(CMP_1), 1001], [hb(CMP_1), 1002], [{ type: 'run_end', runId: RUN_A, outcome: 'cancelled' }, 1003], [hb(CMP_1), 1004]], stopped);
    expect(stops(r.out).length).toBe(2);
    expect(stops(r.out)[0]).toMatchObject({ computerId: CMP_1, event: { type: 'run.stop', data: { runId: RUN_A, reason: 'hung' } } });
  });

  test('a heartbeat from another computer re-sends nothing', () => {
    expect(stops(play([[hb(CMP_2), 1001]], stopped).out)).toEqual([]);
  });
});

describe('run_claim', () => {
  test('creates the run for the agent on the hosting computer, consumes the wake and caps the token at 2 h', () => {
    const r = play([[change(1, { taskId: T1 }), 0], [claimWake(1), 5000]]);
    expect(r.state.runs[runN(1)]).toMatchObject({ agent: { id: AG1 }, owner: U_AGENT_OWNER, computerId: CMP_1, startedAt: 5000, status: 'active' });
    expect(r.state.wakes).toEqual({});
    const reply = lastReply(r.out);
    expect(reply?.kind === 'reply' && reply.ok && reply.result).toMatchObject({ runId: runN(1), expiresAt: '1970-01-01T02:00:05.000Z', reason: 'task_assigned' });
    expect(events(r.out)).toEqual([]);
  });

  test('the new run is live: it can claim a task', () => {
    const r = play([[change(1), 0], [claimWake(1), 1], [{ type: 'claim', runId: runN(1), taskId: T1 }, 2]]);
    expect(r.state.claims[T1].runId).toBe(runN(1));
  });

  test('an unknown or already consumed wake gets not_found', () => {
    expect(errCode(play([[claimWake(7), 0]]).out)).toBe('not_found');
    expect(errCode(play([[change(1), 0], [claimWake(1), 1], [claimWake(1, CMP_1, 2), 2]]).out)).toBe('not_found');
  });

  test('a computer that does not host the agent gets forbidden and the wake stays', () => {
    const r = play([[change(1), 0], [claimWake(1, CMP_2), 1]]);
    expect(errCode(r.out)).toBe('forbidden');
    expect(r.state.wakes[AG1].id).toBe(wak(1));
    expect(Object.keys(r.state.runs).includes(runN(1))).toBe(false);
  });
});

describe('agent_register', () => {
  test('moves hosting: a later wake goes to the new computer', () => {
    const r = play([[{ type: 'agent_register', agentId: AG1, ownerId: U_AGENT_OWNER.id, computerId: CMP_2 }, 0], [change(1), 1]]);
    expect(wakes(r.out).map((o) => o.computerId)).toEqual([CMP_2]);
    expect(world().agents[AG1].computerId).toBe(CMP_1);
  });
});
