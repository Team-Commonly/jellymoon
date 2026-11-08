import { expect, test } from 'bun:test';
import { apply } from '../../podcore/src/index';
import type { AnyEvent } from '../../gen/ts/protocol';
import type { PodState } from '../../podcore/src/index';
import { podRoute } from '../src/pod-routes';
import type { Reply, PodPort, Who } from '../src/wire';

const id = (p: string, n: number): string => `${p}_${String(n).padStart(26, '0')}`;
const owner = { kind: 'human' as const, id: id('usr', 1), displayName: 'olivia' }, agent = { kind: 'agent' as const, id: id('agt', 1), displayName: 'bee' };
const [RUN, TASK] = [id('run', 1), id('tsk', 1)];
const CLAIM = { text: 'The tests passed.', toolCalls: [] as string[] };
const ASK = {
  summary: 'Merge or wait?', kind: 'ask', status: 'blocked', needs: { to: { role: 'owner' }, action: 'approve' },
  options: [{ id: 'a', label: 'Merge', consequence: 'It merges.' }, { id: 'b', label: 'Wait', consequence: 'It waits.' }],
  context: { happened: [CLAIM], checked: [], stakes: CLAIM, recommendation: { option: 'a', why: CLAIM } },
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
  const res = await send(p, ASK);
  const out = (await res?.json()) as { messageId: string; decisionId: string };
  expect(stored(p, out.messageId)).toMatchObject({ kind: 'ask', decision: { id: out.decisionId, state: 'open' } });
});
