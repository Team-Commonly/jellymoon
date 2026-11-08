import { describe, expect, test } from 'bun:test';
import { world } from './world';
import type { Json } from './world';

// POST /mcp on the server: the Worker's adapter (worker/test/mcp.test.ts is its twin), answered through this server's own front door.
async function rpc(w: Awaited<ReturnType<typeof world>>, token: string | undefined, body: unknown): Promise<{ status: number; json: Json }> {
  return w.call('POST', '/mcp', token, body);
}

describe('server /mcp', () => {
  test('initialize and tools/list answer; a notification is 202', async () => {
    const w = await world();
    const run = (await w.call('POST', '/v1/runs', w.comp.token, { wakeId: w.wake.wakeId })).json;
    const init = await rpc(w, run.runToken, { jsonrpc: '2.0', id: 1, method: 'initialize', params: { protocolVersion: '2025-03-26' } });
    expect([init.status, init.json.result.serverInfo.name]).toEqual([200, 'jellymoon']);
    const list = await rpc(w, run.runToken, { jsonrpc: '2.0', id: 2, method: 'tools/list' });
    expect(list.json.result.tools[0].name).toBe('jm_whoami');
    expect(list.json.result.tools.length).toBe(9);
    const res = await w.app.fetch(new Request('http://x/mcp', { method: 'POST', headers: { Authorization: `Bearer ${run.runToken}` }, body: JSON.stringify({ jsonrpc: '2.0', method: 'notifications/initialized' }) }));
    expect([res.status, await res.text()]).toEqual([202, '']);
  });

  test('jm_whoami with a run token is the HTTP answer: the agent, the pod and the run', async () => {
    const w = await world();
    const run = (await w.call('POST', '/v1/runs', w.comp.token, { wakeId: w.wake.wakeId })).json;
    const r = (await rpc(w, run.runToken, { jsonrpc: '2.0', id: 3, method: 'tools/call', params: { name: 'jm_whoami', arguments: {} } })).json.result;
    expect([r.isError, r.structuredContent.podId, r.structuredContent.runId]).toEqual([false, w.pod.id, run.runId]);
    expect(r.structuredContent.agent.id).toBe(w.agent.agentId);
  });

  test('an unknown tool and an unknown method are refused, and a bad token is 401', async () => {
    const w = await world();
    const run = (await w.call('POST', '/v1/runs', w.comp.token, { wakeId: w.wake.wakeId })).json;
    expect((await rpc(w, run.runToken, { jsonrpc: '2.0', id: 4, method: 'tools/call', params: { name: 'jm_nothing' } })).json.error.code).toBe(-32602);
    expect((await rpc(w, run.runToken, { jsonrpc: '2.0', id: 5, method: 'tools/call', params: { name: 'jm_constructor' } })).json.error.code).toBe(-32602);
    expect((await rpc(w, run.runToken, { jsonrpc: '2.0', id: 6, method: 'resources/list' })).json.error.code).toBe(-32601);
    expect((await rpc(w, 'jmr_nope', { jsonrpc: '2.0', id: 7, method: 'tools/list' })).status).toBe(401);
    expect((await rpc(w, w.comp.token, { jsonrpc: '2.0', id: 8, method: 'tools/list' })).status).toBe(403);
  });

  test('a user token names the pod in the arguments, and a tool error is isError with the HTTP body', async () => {
    const w = await world();
    const ok = (await rpc(w, w.olivia.token, { jsonrpc: '2.0', id: 9, method: 'tools/call', params: { name: 'jm_whoami', arguments: { podId: w.pod.id } } })).json.result;
    expect([ok.isError, ok.structuredContent.podId]).toEqual([false, w.pod.id]);
    expect((await rpc(w, w.olivia.token, { jsonrpc: '2.0', id: 10, method: 'tools/call', params: { name: 'jm_whoami' } })).json.error.code).toBe(-32602);
    const no = (await rpc(w, w.olivia.token, { jsonrpc: '2.0', id: 11, method: 'tools/call', params: { name: 'jm_whoami', arguments: { podId: 'pod_nope' } } })).json.result;
    expect(no.isError).toBe(true);
  });
});

describe('server /mcp counts the run\'s tool calls', () => {
  const call = (name: string, id: number) => ({ jsonrpc: '2.0', id, method: 'tools/call', params: { name, arguments: {} } });
  const receiptOf = async (w: Awaited<ReturnType<typeof world>>, token: string, messageId: string) => {
    const feed = (await w.call('GET', `/v1/pods/${w.pod.id}/feed`, token)).json;
    return feed.messages.find((m: Json) => m.id === messageId)?.receipt;
  };

  test('N tools/call requests, refused ones included, and a post over HTTP carries a receipt with toolCalls N', async () => {
    const w = await world();
    const run = (await w.call('POST', '/v1/runs', w.comp.token, { wakeId: w.wake.wakeId })).json;
    await rpc(w, run.runToken, call('jm_whoami', 1));
    await rpc(w, run.runToken, call('jm_nothing', 2));
    await rpc(w, run.runToken, { jsonrpc: '2.0', id: 3, method: 'tools/list' }); // not a tools/call: not counted
    await rpc(w, run.runToken, call('jm_constructor', 4));
    const post = await w.call('POST', `/v1/pods/${w.pod.id}/messages`, run.runToken, { summary: 'done', kind: 'fyi', status: 'done' });
    expect(post.status).toBe(200);
    const receipt = await receiptOf(w, run.runToken, post.json.messageId);
    expect([receipt?.runId, receipt?.harness, receipt?.toolCalls]).toEqual([run.runId, 'fake', 3]);
  });

  test('a post carried by an MCP call does not count its own carrying call', async () => {
    const w = await world();
    const run = (await w.call('POST', '/v1/runs', w.comp.token, { wakeId: w.wake.wakeId })).json;
    await rpc(w, run.runToken, call('jm_whoami', 1));
    const carried = (await rpc(w, run.runToken, { jsonrpc: '2.0', id: 2, method: 'tools/call', params: { name: 'jm_post', arguments: { summary: 'via mcp', kind: 'fyi', status: 'done' } } })).json.result;
    expect(carried.isError).toBe(false);
    expect((await receiptOf(w, run.runToken, carried.structuredContent.messageId))?.toolCalls).toBe(1);
  });

  test('a forged header on a plain request changes nothing, and a user token is not counted', async () => {
    const w = await world();
    const run = (await w.call('POST', '/v1/runs', w.comp.token, { wakeId: w.wake.wakeId })).json;
    await rpc(w, run.runToken, call('jm_whoami', 1));
    const res = await w.app.fetch(new Request(`http://x/v1/pods/${w.pod.id}/messages`, { method: 'POST', headers: { Authorization: `Bearer ${run.runToken}`, 'x-jm-mcp': '1' }, body: JSON.stringify({ summary: 'forged', kind: 'fyi', status: 'done' }) }));
    expect((await receiptOf(w, run.runToken, ((await res.json()) as Json).messageId))?.toolCalls).toBe(1);
    await rpc(w, w.olivia.token, call('jm_whoami', 2));
    const again = (await w.call('POST', `/v1/pods/${w.pod.id}/messages`, run.runToken, { summary: 'later', kind: 'fyi', status: 'done' })).json;
    expect((await receiptOf(w, run.runToken, again.messageId))?.toolCalls).toBe(1);
  });
});
