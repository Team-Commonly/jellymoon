import { expect, test } from 'bun:test';
import { world } from './world';
import type { Json } from './world';

// After a ruling the agent has to be able to act on it over HTTP and MCP alone: the wake names the decision, the option that won and
// the task, and the feed shows the ask's decision ruled (the socket already gets this as message.updated).
const TASK = 'tsk_00000000000000000000000001';
const claim = { text: 'The tests passed.', toolCalls: [] as string[] };
const ask = {
  summary: 'Merge or wait?', kind: 'ask', status: 'blocked', needs: { to: { role: 'owner' }, action: 'approve' }, taskId: TASK,
  options: [{ id: 'a', label: 'Merge', consequence: 'It merges.' }, { id: 'b', label: 'Wait', consequence: 'It waits.' }],
  context: { happened: [claim], checked: [], stakes: claim, recommendation: { option: 'a', why: claim } },
};

test('a ruling wakes the asker with the decision, the chosen option and the task, and the feed (HTTP and MCP) shows it ruled', async () => {
  const w = await world();
  const run = (await w.call('POST', '/v1/runs', w.comp.token, { wakeId: w.wake.wakeId })).json;
  const posted = (await w.call('POST', `/v1/pods/${w.pod.id}/messages`, run.runToken, ask)).json;
  const ruled = await w.rt.exec({ type: 'rule', caller: { kind: 'user', userId: w.olivia.user.id }, decisionId: posted.decisionId, option: 'b', wakeId: 'wak_00000000000000000000000009' });
  expect(ruled?.ok).toBe(true);
  expect(w.rt.state.wakes[w.agent.agentId].refs).toEqual({ decisionId: posted.decisionId, messageId: posted.messageId, optionId: 'b', taskId: TASK });
  const decision = { id: posted.decisionId, state: 'ruled', ruling: { optionId: 'b', by: w.olivia.user } };
  const http = (await w.call('GET', `/v1/pods/${w.pod.id}/feed`, run.runToken)).json.messages as Json[];
  expect(http.find((m) => m.id === posted.messageId)?.decision).toMatchObject(decision);
  const mcp = (await w.call('POST', '/mcp', run.runToken, { jsonrpc: '2.0', id: 1, method: 'tools/call', params: { name: 'jm_read', arguments: {} } })).json.result;
  expect((mcp.structuredContent.messages as Json[]).find((m) => m.id === posted.messageId)?.decision).toMatchObject(decision);
});
