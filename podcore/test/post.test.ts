import { describe, expect, test } from 'bun:test';
import type { AskContext, Message, PostInput } from '../../gen/ts/protocol';
import { isoOf } from '../src';
import type { Cmd } from '../src';
import { RUN_A, RUN_B, U_OWNER, U_STRANGER, errCode, events, id, play, world } from './helpers';

/** The test world's runs are hand-built, so give run A the facts a woken run carries. */
const woken = () => {
  const s = world();
  Object.assign(s.runs[RUN_A], { startedAt: 1000, wakeReason: 'task_assigned' });
  return s;
};
const DEC = id('dec', 1);
const OPTS = [{ id: 'a', label: 'Merge', consequence: 'It merges.' }, { id: 'b', label: 'Wait', consequence: 'It waits.' }] as const;
const claim = { text: 'Tests passed.', toolCalls: [id('tc', 1)] };
const ctx: AskContext = { happened: [claim], checked: [], stakes: claim, recommendation: { option: 'a', why: claim } };
const fyi: PostInput = { summary: 'Done', kind: 'fyi', status: 'done' };
const ask: PostInput = { summary: 'Merge or wait?', kind: 'ask', status: 'blocked', needs: { to: { role: 'owner' }, action: 'approve' }, options: [...OPTS], context: ctx };
const post = (input: unknown, caller: { kind: 'run'; runId: string } | { kind: 'user'; userId: string } = { kind: 'run', runId: RUN_A }, harness = 'fake'): Cmd => (
  { type: 'post', caller, messageId: id('msg', 1), decisionId: DEC, wakeId: id('wak', 1), harness, input } as unknown as Cmd);
const stored = (out: ReturnType<typeof play>['out']) => events(out).map((e) => e.data as Message)[0];

describe('the headline contract', () => {
  test('a post without a summary, or with only blank space, is summary_required; the other refusals are invalid_input', () => {
    for (const bad of [{ kind: 'fyi', status: 'done' }, { ...fyi, summary: '   ' }, { ...fyi, summary: 7 }]) expect(errCode(play([[post(bad), 2000]], woken()).out)).toBe('summary_required');
    for (const bad of [{ ...ask, options: undefined }, { ...ask, needs: undefined }, { ...ask, context: undefined }, { ...fyi, needs: ask.needs },
      { ...fyi, options: ask.options }, { ...fyi, context: ask.context }, { ...ask, options: [OPTS[0], OPTS[0]] },
      { ...ask, context: { ...ctx, recommendation: { option: 'e', why: claim } } }]) expect(errCode(play([[post(bad), 2000]], woken()).out)).toBe('invalid_input');
  });

  test('a post that names any field the pod sets is refused, and nothing is stored', () => {
    for (const k of ['receipt', 'decision', 'runId', 'author', 'id', 'seq', 'createdAt', 'podId', 'threadOf']) {
      const r = play([[post({ ...fyi, [k]: { x: 1 } }), 2000]], woken());
      expect([k, errCode(r.out), events(r.out).length]).toEqual([k, 'invalid_input', 0]);
    }
  });
});

describe('the receipt and the decision', () => {
  test('a run\'s post carries a receipt from the run record, the harness the adapter gave, and the calls the pod counted', () => {
    const r = play([[{ type: 'tool_call', runId: RUN_A }, 1500], [{ type: 'tool_call', runId: RUN_A }, 1600], [post(ask), 3000]], woken());
    const m = stored(r.out);
    expect(m.receipt).toEqual({ runId: RUN_A, agentId: r.state.runs[RUN_A].agent.id, computerId: r.state.runs[RUN_A].computerId, harness: 'fake',
      startedAt: isoOf(1000), postedAt: isoOf(3000), toolCalls: 2, wakeReason: 'task_assigned' });
    expect(m.runId).toBe(RUN_A);
    expect(m.decision).toEqual({ id: DEC, state: 'open' });
  });

  test('a task id in the post is echoed into the receipt; a post that is not an ask has no decision', () => {
    const m = stored(play([[post({ ...fyi, taskId: id('tsk', 1) }), 3000]], woken()).out);
    expect([m.receipt?.taskId, m.decision]).toEqual([id('tsk', 1), undefined]);
  });

  test('a person\'s post has no receipt and no run; an ask by a person still carries a decision', () => {
    const m = stored(play([[post(ask, { kind: 'user', userId: U_OWNER.id }), 3000]], woken()).out);
    expect([m.receipt, m.runId, m.author.id, m.decision?.state]).toEqual([undefined, undefined, U_OWNER.id, 'open']);
  });

  test('only a live run\'s calls are counted; an ended or unknown run counts nothing', () => {
    const ended = play([[{ type: 'run_end', runId: RUN_A, computerId: world().runs[RUN_A].computerId, outcome: 'completed' }, 2000], [{ type: 'tool_call', runId: RUN_A }, 2100],
      [{ type: 'tool_call', runId: id('run', 77) }, 2200]], woken());
    expect(ended.state.runs[RUN_A].toolCalls ?? 0).toBe(0);
    expect(errCode(ended.out)).toBeNull();
  });

  test('the harness is the adapter\'s: a run whose harness is unknown, or whose wake reason was never recorded, gets internal, not a guess', () => {
    expect(errCode(play([[post(fyi, { kind: 'run', runId: RUN_A } as never, 'nonsense'), 2000]], woken()).out)).toBe('internal');
    expect(errCode(play([[post(fyi, { kind: 'run', runId: RUN_B } as never), 2000]], woken()).out)).toBe('internal');
  });

  test('who is calling is answered before what they asked: a stranger is forbidden, a dead run unauthorized', () => {
    expect(errCode(play([[post({ nonsense: 1 }, { kind: 'user', userId: U_STRANGER.id }), 2000]], woken()).out)).toBe('forbidden');
    expect(errCode(play([[post(fyi, { kind: 'run', runId: id('run', 77) } as never), 2000]], woken()).out)).toBe('unauthorized');
  });
});
