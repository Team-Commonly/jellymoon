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
