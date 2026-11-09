import { expect, test } from 'bun:test';
import { apply } from '../../podcore/src/index';
import type { AnyEvent } from '../../gen/ts/protocol';
import type { PodState } from '../../podcore/src/index';
import { podRoute } from '../src/pod-routes';
import type { Reply, PodPort, Who } from '../src/wire';

const id = (p: string, n: number): string => `${p}_${String(n).padStart(26, '0')}`;
const owner = { kind: 'human' as const, id: id('usr', 1), displayName: 'olivia' }, agent = { kind: 'agent' as const, id: id('agt', 1), displayName: 'bee' };
const [RUN, TASK] = [id('run', 1), id('tsk', 1)];
const askCiting = (messageId: string) => {
  const CLAIM = { text: 'The tests passed.', messages: [messageId] };
  return {
  summary: 'Merge or wait?', kind: 'ask', status: 'blocked', needs: { to: { role: 'owner' }, action: 'approve' },
  options: [{ id: 'a', label: 'Merge', consequence: 'It merges.' }, { id: 'b', label: 'Wait', consequence: 'It waits.' }],
  context: { happened: [CLAIM], checked: [], stakes: CLAIM, recommendation: { option: 'a', why: CLAIM } },
  };
};

function pod(): PodPort {
  let state = {
    podId: id('pod', 1), seq: 0, owners: [owner], members: [owner], claims: {}, lost: {}, wakes: {},
    tasks: {}, runs: { [RUN]: { id: RUN, agent, owner, computerId: id('cmp', 1), startedAt: 0, status: 'active', wakeReason: 'task_assigned', wakeTaskId: TASK } },
    agents: { [agent.id]: { actor: agent, owner, computerId: id('cmp', 1) } },
  } as unknown as PodState;
  const log: Array<{ kind: string; event?: unknown }> = [];
  return {
    id: state.podId, meta: { name: 'p', createdAt: '', harness: { [agent.id]: 'fake' } }, now: () => 5000, sql: {} as never,
    get state() { return state; },
    eventsAfter: () => log.flatMap((o) => (o.kind === 'event' ? [o.event] : [])) as AnyEvent[],
    messagesById: (ids) => log.flatMap((o) => (o.kind === 'event' && (o.event as AnyEvent).type === 'message.created' ? [(o.event as AnyEvent).data as { id: string; summary: string }] : []))
      .filter((m) => ids.includes(m.id)),
    exec(cmd): Reply | undefined {
      const r = apply(state, cmd, 5000);
      state = r.state;
      log.push(...r.out);
      return r.out.filter((o) => o.kind === 'reply').pop() as Reply | undefined; // a reply Out has the shape of a Reply
    },
  };
}

const who: Who = { kind: 'run', runId: RUN, podId: id('pod', 1) };
const send = (p: PodPort, body: unknown): Promise<Response | null> =>
  podRoute(p, new Request(`http://x/v1/pods/${p.id}/messages`, { method: 'POST', body: JSON.stringify(body) }), who);
const stored = (p: PodPort, messageId: string) => p.eventsAfter(0, 50).map((e) => e.data as Record<string, unknown>).find((m) => m.id === messageId);

test('the pod post route stores every kind, an ask and a taskId as sent (twin of the server test)', async () => {
  const p = pod();
  for (const kind of ['result', 'progress', 'fyi', 'error']) {
    const sent = { summary: `a ${kind} post`, kind, status: 'done', taskId: TASK };
    const res = await send(p, sent);
    expect(res?.status).toBe(200);
    expect(stored(p, ((await res?.json()) as { messageId: string }).messageId)).toMatchObject(sent);
  }
  let res = await send(p, { summary: 'The tests passed', kind: 'progress', status: 'done' });
  const cited = ((await res?.json()) as { messageId: string }).messageId;
  res = await send(p, askCiting(cited));
  const out = (await res?.json()) as { messageId: string; decisionId: string };
  expect(stored(p, out.messageId)).toMatchObject({ kind: 'ask', decision: { id: out.decisionId, state: 'open' } });
});

test('a task_update carrying a note is refused at the edge', async () => {
  const res = await podRoute(pod(), new Request(`http://x/v1/pods/p/tasks/${TASK}`, { method: 'PATCH', body: JSON.stringify({ status: 'in_progress', note: 'halfway' }) }), who);
  expect([res?.status, ((await res?.json()) as { code: string }).code]).toEqual([400, 'invalid_input']);
});

test('after a ruling the pod feed shows the ask ruled and the wake names the decision, the option and the task (twin of the server test)', async () => {
  const p = pod();
  const cited = ((await (await send(p, { summary: 'The tests passed', kind: 'progress', status: 'done' }))?.json()) as { messageId: string }).messageId;
  const posted = (await (await send(p, { ...askCiting(cited), taskId: TASK }))?.json()) as { messageId: string; decisionId: string };
  const ruled = p.exec({ type: 'rule', caller: { kind: 'user', userId: owner.id }, decisionId: posted.decisionId, option: 'b', wakeId: id('wak', 9) });
  expect(ruled?.ok).toBe(true);
  expect(p.state.wakes[agent.id].refs).toEqual({ decisionId: posted.decisionId, messageId: posted.messageId, optionId: 'b', taskId: TASK });
  const res = await podRoute(p, new Request(`http://x/v1/pods/${p.id}/feed`), who);
  const messages = ((await res?.json()) as { messages: Array<{ id: string; decision?: unknown }> }).messages;
  expect(messages.find((m) => m.id === posted.messageId)?.decision).toEqual({ id: posted.decisionId, state: 'ruled', ruling: { optionId: 'b', by: owner, at: new Date(5000).toISOString() } });
});

test('the Worker keeps no tool-call records, so an ask citing a tool call is ungrounded; one citing a message it holds is checked as on the server', async () => {
  const p = pod();
  const cited = ((await (await send(p, { summary: 'The tests passed', kind: 'progress', status: 'done' }))?.json()) as { messageId: string }).messageId;
  const claim = { text: 'The tests passed.', toolCalls: [id('tc', 1)] };
  const withCall = { ...askCiting(cited), context: { ...askCiting(cited).context, stakes: claim } };
  let res = await send(p, withCall);
  expect([res?.status, ((await res?.json()) as { code: string; message: string }).message]).toEqual([422, 'context.stakes.toolCalls[0] is not a tool call on a run of this agent in this pod.']);
  res = await send(p, { ...askCiting(cited), context: { ...askCiting(cited).context, stakes: { text: 'Waiting costs 3 days.', messages: [cited] } } });
  expect([res?.status, ((await res?.json()) as { code: string }).code]).toEqual([422, 'ungrounded']);
  res = await send(p, askCiting(id('msg', 7)));
  expect([res?.status, ((await res?.json()) as { code: string }).code]).toEqual([422, 'ungrounded']);
});
