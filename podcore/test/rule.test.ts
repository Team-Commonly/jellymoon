import { describe, expect, test } from 'bun:test';
import type { AskContext, Message, PostInput } from '../../gen/ts/protocol';
import { apply, isoOf } from '../src';
import type { Cmd, PodState } from '../src';
import { AG1, CMP_1, RUN_A, RUN_B, U_AGENT_OWNER, U_OWNER, U_STRANGER, errCode, events, id, play, runN, toComputer, wak, world } from './helpers';

const DEC = id('dec', 1);
const MSG = id('msg', 1);
const claim = { text: 'Tests passed.', toolCalls: [id('tc', 1)] };
const ctx: AskContext = { happened: [claim], checked: [], stakes: claim, recommendation: { option: 'a', why: claim } };
const ask: PostInput = {
  summary: 'Merge or wait?', kind: 'ask', status: 'blocked', needs: { to: { role: 'owner' }, action: 'approve' }, context: ctx,
  options: [{ id: 'a', label: 'Merge', consequence: 'It merges.' }, { id: 'b', label: 'Wait', consequence: 'It waits.' }],
};
type Caller = { kind: 'run'; runId: string } | { kind: 'user'; userId: string };
const owner: Caller = { kind: 'user', userId: U_OWNER.id };
const post = (input: PostInput, caller: Caller = { kind: 'run', runId: RUN_A }, decisionId = DEC, messageId = MSG): Cmd => (
  { type: 'post', caller, messageId, decisionId, wakeId: wak(1), harness: 'fake', input } as unknown as Cmd);
const rule = (option: unknown = 'a', caller: Caller = owner, extra: Record<string, unknown> = {}): Cmd => (
  { type: 'rule', caller, decisionId: DEC, option, wakeId: wak(2), ...extra } as unknown as Cmd);

/** A pod where run A (agent 1, on computer 1) has posted an ask. */
const asked = (): { state: PodState; now: number } => {
  const s = world();
  Object.assign(s.runs[RUN_A], { startedAt: 1000, wakeReason: 'task_assigned' });
  return { state: play([[post(ask), 2000]], s).state, now: 3000 };
};
const ruledOut = () => { const { state, now } = asked(); return play([[rule('b'), now]], state); };
const updates = (out: ReturnType<typeof play>['out']) => events(out).flatMap((e) => (e.type === 'message.updated' ? [e.data] : []));
const wakes = (out: ReturnType<typeof play>['out']) => toComputer(out).filter((o) => o.event.type === 'run.wake');

describe('a member rules on an ask', () => {
  test('the decision is ruled and frozen, members get message.updated, and the asking agent gets decision_ruled naming the decision and its message', () => {
    const r = ruledOut();
    expect(errCode(r.out)).toBeNull();
    expect(updates(r.out)).toEqual([{ id: MSG, decision: { id: DEC, state: 'ruled', ruling: { optionId: 'b', by: U_OWNER, at: isoOf(3000) } } }]);
    expect(events(r.out).map((e) => e.type)).toEqual(['message.updated']);
    expect(wakes(r.out).map((o) => [o.computerId, o.event.type === 'run.wake' && o.event.data])).toEqual([
      [CMP_1, { wakeId: wak(2), agentId: AG1, reason: 'decision_ruled', refs: { decisionId: DEC, messageId: MSG } }]]);
  });

  test('any member may rule, the agent\'s owner included', () => {
    const { state, now } = asked();
    const by = play([[rule('a', { kind: 'user', userId: U_AGENT_OWNER.id }), now]], state);
    expect(updates(by.out)[0].decision.ruling?.by).toEqual(U_AGENT_OWNER);
  });

  test('a second ruling gets already_ruled and changes nothing: not the option, not the state, no event, no wake', () => {
    const r = ruledOut();
    const other: Caller = { kind: 'user', userId: U_AGENT_OWNER.id };
    for (const [option, caller] of [['b', owner], ['a', owner], ['b', other], ['a', other], ['zz', other]] as Array<[unknown, Caller]>) {
      const again = apply(r.state, rule(option, caller, { wakeId: wak(3) }), 4000);
      expect([option, errCode(again.out), events(again.out).length, toComputer(again.out).length]).toEqual([option, 'already_ruled', 0, 0]);
      expect(again.state).toEqual(r.state);
    }
  });

  test('already_ruled wins over a bad seed, and the ruled decision stays after a JSON round trip', () => {
    const r = ruledOut();
    const reloaded = JSON.parse(JSON.stringify(r.state)) as PodState;
    expect(errCode(apply(reloaded, rule('a', owner, { wakeId: 'nonsense' }), 4000).out)).toBe('already_ruled');
  });

});

describe('a ruling the pod refuses', () => {
  test('an option the ask does not offer is invalid_input and leaves the decision open; prototype keys are options like any other', () => {
    const { state, now } = asked();
    for (const option of ['c', 'e', '', '__proto__', 'constructor', 'toString', 'A', 7, null, ['a'], { id: 'a' }]) {
      const r = apply(state, rule(option), now);
      expect([String(option), errCode(r.out), events(r.out).length, wakes(r.out).length]).toEqual([String(option), 'invalid_input', 0, 0]);
      expect(r.state).toEqual(state);
    }
    const missing = apply(state, { type: 'rule', caller: owner, decisionId: DEC, wakeId: wak(2) } as unknown as Cmd, now);
    expect(errCode(missing.out)).toBe('invalid_input');
  });

  test('an option id that is a prototype key can be ruled when the ask offers it', () => {
    const { state, now } = asked();
    const odd = play([[post({ ...ask, options: [{ id: 'constructor', label: 'C', consequence: 'x' }, { id: 'b', label: 'B', consequence: 'y' }], context: { ...ctx, recommendation: { option: 'b', why: claim } } } as unknown as PostInput, { kind: 'run', runId: RUN_A }, id('dec', 2), id('msg', 2)), now]], state);
    const cmd = { ...rule('constructor'), decisionId: id('dec', 2) } as Cmd;
    expect(updates(apply(odd.state, cmd, now + 1).out)[0].decision.ruling?.optionId).toBe('constructor');
  });

  test('a decision that does not exist is not_found: unknown, an ask nobody recorded, a fyi, and prototype keys', () => {
    const { state, now } = asked();
    const fyi = play([[post({ summary: 'Done', kind: 'fyi', status: 'done' }, { kind: 'run', runId: RUN_A }, id('dec', 5), id('msg', 5)), now]], state);
    for (const decisionId of [id('dec', 9), id('dec', 5), '__proto__', 'constructor', 'toString', 'hasOwnProperty']) {
      const r = apply(fyi.state, rule('a', owner, { decisionId }), now + 1);
      expect([decisionId, errCode(r.out), events(r.out).length, wakes(r.out).length]).toEqual([decisionId, 'not_found', 0, 0]);
    }
    expect(errCode(apply(fyi.state, { ...rule('a'), decisionId: 7 } as unknown as Cmd, now).out)).toBe('invalid_input');
  });

  test('who is calling is answered first: a stranger is forbidden, a run is forbidden before any lookup, a dead run is unauthorized', () => {
    const { state, now } = asked();
    const stranger: Caller = { kind: 'user', userId: U_STRANGER.id };
    for (const decisionId of [DEC, id('dec', 9), '__proto__']) expect([decisionId, errCode(apply(state, rule('a', stranger, { decisionId }), now).out)]).toEqual([decisionId, 'forbidden']);
    for (const decisionId of [DEC, id('dec', 9), '__proto__']) {
      const r = apply(state, rule('a', { kind: 'run', runId: RUN_A }, { decisionId }), now);
      expect([decisionId, errCode(r.out), events(r.out).length]).toEqual([decisionId, 'forbidden', 0]);
    }
    expect(errCode(apply(state, rule('a', { kind: 'run', runId: id('run', 77) }), now).out)).toBe('unauthorized');
    const ended = play([[{ type: 'run_end', runId: RUN_B, computerId: CMP_1, outcome: 'completed' }, now]], state).state;
    expect(errCode(apply(ended, rule('a', { kind: 'run', runId: RUN_B }, { decisionId: '__proto__' }), now + 1).out)).toBe('unauthorized');
    expect(errCode(apply(state, { type: 'rule', decisionId: DEC, option: 'a', wakeId: wak(2) } as unknown as Cmd, now).out)).toBe('invalid_input');
  });

});

describe('a ruling and its wake', () => {
  test('a ruling renews a live run\'s claims even when it is refused', () => {
    const { state, now } = asked();
    const held = play([[{ type: 'claim', runId: RUN_B, taskId: Object.keys(state.tasks)[0] }, now]], state);
    const later = apply(held.state, rule('a', { kind: 'run', runId: RUN_B }), now + 30_000);
    expect(errCode(later.out)).toBe('forbidden');
    expect(Object.values(later.state.claims)[0].leaseUntil).toBe(now + 30_000 + 60_000);
  });

  test('a ruling folds into the agent\'s pending wake: nothing is sent, the wakeId and reason stay, the refs are added', () => {
    const s = world();
    Object.assign(s.runs[RUN_A], { startedAt: 1000, wakeReason: 'task_assigned' });
    const pending = play([[post(ask), 2000], [{ type: 'change', kind: 'ruling', agentId: AG1, refs: { taskId: id('tsk', 1) }, wakeId: wak(7) }, 2500]], s);
    expect(pending.state.wakes[AG1].reason).toBe('decision_ruled');
    const r = apply(pending.state, rule('a'), 3000);
    expect([errCode(r.out), wakes(r.out).length, r.state.wakes[AG1].id]).toEqual([null, 0, wak(7)]);
    expect(r.state.wakes[AG1].refs).toEqual({ taskId: id('tsk', 1), decisionId: DEC, messageId: MSG });
  });

  test('a bad wakeId seed is invalid_input and the decision stays open', () => {
    const { state, now } = asked();
    for (const wakeId of ['nonsense', '', wak(2).replace('wak', 'run')]) {
      const r = apply(state, rule('a', owner, { wakeId }), now);
      expect([wakeId, errCode(r.out), r.state.decisions?.[DEC]?.decision.state]).toEqual([wakeId, 'invalid_input', 'open']);
    }
  });

  test('an ask by a person is ruled too, and wakes nobody', () => {
    const s = world();
    const r = play([[post(ask, { kind: 'user', userId: U_AGENT_OWNER.id }), 2000], [rule('a'), 3000]], s);
    expect([errCode(r.out), updates(r.out).length, wakes(r.out).length]).toEqual([null, 1, 0]);
  });

  test('the run the decision_ruled wake starts stamps its posts with wakeReason decision_ruled', () => {
    const r = ruledOut();
    const started = play([[{ type: 'run_claim', wakeId: wak(2), computerId: CMP_1, runId: runN(1) }, 5000]], r.state);
    const fyi = { type: 'post', caller: { kind: 'run', runId: runN(1) }, messageId: id('msg', 2), decisionId: id('dec', 2), wakeId: wak(3), harness: 'fake', input: { summary: 'Merged', kind: 'result', status: 'done' } } as unknown as Cmd;
    const m = events(play([[fyi, 6000]], started.state).out).map((e) => e.data as Message)[0];
    expect([m.receipt?.wakeReason, m.receipt !== undefined && 'taskId' in m.receipt]).toEqual(['decision_ruled', false]);
  });
});

describe('what a ruling is made of, and what bounds it', () => {
  test('an ask records its decision once; reusing a decision id is invalid_input and the first stays', () => {
    const { state, now } = asked();
    const dup = apply(state, post(ask, { kind: 'run', runId: RUN_A }, DEC, id('msg', 2)), now);
    expect([errCode(dup.out), events(dup.out).length]).toEqual(['invalid_input', 0]);
    expect(dup.state.decisions).toEqual(state.decisions);
    expect(Object.keys(state.decisions ?? {})).toEqual([DEC]);
  });

  test('only asks are recorded', () => {
    const s = world();
    const r = play([[post({ summary: 'Done', kind: 'fyi', status: 'done' }), 2000]], s);
    expect(Object.keys(r.state.decisions ?? {})).toEqual([]);
  });

  test('a pod saved before decisions were kept still loads: it posts an ask and rules it, and an old ask cannot be ruled', () => {
    const old = JSON.parse(JSON.stringify(world())) as PodState;
    expect('decisions' in old).toBe(false);
    expect(errCode(apply(old, rule('a'), 1000).out)).toBe('not_found');
    Object.assign(old.runs[RUN_A], { startedAt: 1000, wakeReason: 'task_assigned' });
    const r = play([[post(ask), 2000], [rule('a'), 3000]], old);
    expect(errCode(r.out)).toBeNull();
    expect(r.state.decisions?.[DEC]?.decision.state).toBe('ruled');
  });

  test('a ruled record holds no option list and does not grow when ruled again; the state round trips as JSON', () => {
    const open = asked().state.decisions?.[DEC];
    expect(open?.options).toEqual(['a', 'b']);
    const r = ruledOut();
    const rec = r.state.decisions?.[DEC];
    expect(rec?.options).toEqual([]);
    const size = JSON.stringify(r.state).length;
    const again = apply(r.state, rule('a'), 4000);
    expect(JSON.stringify(again.state).length).toBe(size);
    expect(JSON.parse(JSON.stringify(r.state))).toEqual(r.state);
  });

  test('the ruling does not touch the message log: seq moves by one event and no other state changes', () => {
    const { state, now } = asked();
    const r = apply(state, rule('a'), now);
    expect(r.state.seq).toBe(state.seq + 1);
    expect({ ...r.state, seq: 0, decisions: undefined, wakes: undefined }).toEqual({ ...state, seq: 0, decisions: undefined, wakes: undefined });
  });
});
