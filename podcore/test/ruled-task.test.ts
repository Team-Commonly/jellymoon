import { describe, expect, test } from 'bun:test';
import type { PostInput } from '../../gen/ts/protocol';
import { apply } from '../src';
import type { Cmd } from '../src';
import { AG1, CITED, CMP_1, EVIDENCE, RUN_A, U_OWNER, id, play, toComputer, wak, world } from './helpers';

// A receipt's taskId is the pod's own record of the task that woke the run. An ask's own taskId is the agent's word and never reaches it.
const claim = { text: 'x', messages: [CITED] };
const BOGUS = id('tsk', 99);
const REAL = id('tsk', 1);
const ask = (taskId?: string): PostInput => ({
  summary: 'q', kind: 'ask', status: 'blocked', needs: { to: { role: 'owner' }, action: 'approve' }, ...(taskId === undefined ? {} : { taskId }),
  context: { happened: [claim], checked: [], stakes: claim, recommendation: { option: 'a', why: claim } },
  options: [{ id: 'a', label: 'A', consequence: 'c' }, { id: 'b', label: 'B', consequence: 'c' }],
} as unknown as PostInput);
const post = (input: PostInput, n: number, runId: string = RUN_A): Cmd => (
  { type: 'post', caller: { kind: 'run', runId }, messageId: id('msg', n), decisionId: id('dec', n), wakeId: wak(n), harness: 'fake', input, evidence: EVIDENCE } as unknown as Cmd);
const rule = { type: 'rule', caller: { kind: 'user', userId: U_OWNER.id }, decisionId: id('dec', 1), option: 'a', wakeId: wak(2) } as unknown as Cmd;
const refsOf = (out: ReturnType<typeof play>['out']) => toComputer(out).flatMap((o) => (o.event.type === 'run.wake' ? [o.event.data.refs] : []));

/** Run A, woken for `wakeTaskId` (or by a mention, for none), asks with `askTaskId`; the owner rules; the asker's next run posts. */
const laundered = (wakeReason: string, wakeTaskId: string | undefined, askTaskId: string | undefined) => {
  const s = world();
  Object.assign(s.runs[RUN_A], { startedAt: 1000, wakeReason, ...(wakeTaskId === undefined ? {} : { wakeTaskId }) });
  const r = play([[post(ask(askTaskId), 1), 2000], [rule, 3000]], s);
  const run2 = id('run', 9);
  const c = apply(r.state, { type: 'run_claim', wakeId: r.state.wakes[AG1].id, computerId: CMP_1, runId: run2 } as unknown as Cmd, 4000);
  const p = apply(c.state, post({ summary: 'done', kind: 'result', status: 'done' } as PostInput, 2, run2), 5000);
  const text = JSON.stringify(p.out);
  return { wakeRefs: refsOf(r.out), text, receipt: JSON.parse(text.match(/"receipt":(\{[^}]*\})/)?.[1] ?? 'null') as Record<string, unknown> | null };
};

describe('an ask never writes the taskId the pod stamps', () => {
  test('a bogus taskId on an ask from a run woken by a mention reaches neither the wake refs nor the next receipt', () => {
    const r = laundered('mention', undefined, BOGUS);
    expect(JSON.stringify(r.wakeRefs)).not.toContain(BOGUS);
    expect(r.text).not.toContain(BOGUS);
    expect(r.receipt?.wakeReason).toBe('decision_ruled');
    expect('taskId' in (r.receipt ?? {})).toBe(false);
  });

  test('an ask naming a different task than the one that woke the run is ruled with the run\'s own task', () => {
    const r = laundered('task_assigned', REAL, BOGUS);
    expect(r.wakeRefs).toEqual([{ decisionId: id('dec', 1), messageId: id('msg', 1), optionId: 'a', taskId: REAL }]);
    expect(r.receipt?.taskId).toBe(REAL);
  });

  test('the legitimate path: a run woken by task_assigned asks, is ruled, and its next run carries that task', () => {
    const r = laundered('task_assigned', REAL, undefined);
    expect(r.wakeRefs).toEqual([{ decisionId: id('dec', 1), messageId: id('msg', 1), optionId: 'a', taskId: REAL }]);
    expect(r.receipt?.taskId).toBe(REAL);
  });
});
