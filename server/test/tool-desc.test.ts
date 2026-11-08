import { expect, test } from 'bun:test';
import { world } from './world';
import type { Json } from './world';

// What a post tool description allows (every kind, an ask's parts, a taskId) the server stores, over HTTP and over jm_post.
const TASK = 'tsk_00000000000000000000000001';
const claim = { text: 'The tests passed.', toolCalls: [] as string[] };
const ask = {
  summary: 'Merge or wait?', kind: 'ask', status: 'blocked', needs: { to: { role: 'owner' }, action: 'approve' },
  options: [{ id: 'a', label: 'Merge', consequence: 'It merges.' }, { id: 'b', label: 'Wait', consequence: 'It waits.' }],
  context: { happened: [claim], checked: [], stakes: claim, recommendation: { option: 'a', why: claim } },
};

test('a post of every kind, an ask and a taskId are stored as sent, over HTTP and MCP', async () => {
  const w = await world();
  const run = (await w.call('POST', '/v1/runs', w.comp.token, { wakeId: w.wake.wakeId })).json;
  const feed = async (id: string): Promise<Json> => ((await w.call('GET', `/v1/pods/${w.pod.id}/feed`, run.runToken)).json.messages as Json[]).find((m) => m.id === id) as Json;
  const mcp = async (body: Json) => (await w.call('POST', '/mcp', run.runToken, { jsonrpc: '2.0', id: 1, method: 'tools/call', params: { name: 'jm_post', arguments: body } })).json.result;
  for (const kind of ['result', 'progress', 'fyi', 'error']) {
    const sent = { summary: `a ${kind} post`, kind, status: 'done', taskId: TASK };
    const http = await w.call('POST', `/v1/pods/${w.pod.id}/messages`, run.runToken, sent);
    expect(http.status).toBe(200);
    expect(await feed(http.json.messageId)).toMatchObject(sent);
    const viaMcp = await mcp(sent);
    expect(viaMcp.isError).toBe(false);
    expect(await feed(viaMcp.structuredContent.messageId)).toMatchObject(sent);
  }
  for (const out of [(await w.call('POST', `/v1/pods/${w.pod.id}/messages`, run.runToken, ask)).json, (await mcp(ask)).structuredContent]) {
    expect(await feed(out.messageId)).toMatchObject({ kind: 'ask', decision: { id: out.decisionId, state: 'open' } });
  }
});
