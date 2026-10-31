import { expect, test } from 'bun:test';
import { mcp } from '../src/mcp';
import type { Who } from '../src/wire';

const run: Who = { kind: 'run', runId: 'run_1', podId: 'pod_1' };
const user: Who = { kind: 'user', actor: { kind: 'human', id: 'usr_1', displayName: 'olivia' } };
const rpc = (body: unknown): Request => new Request('https://x/mcp', { method: 'POST', headers: { Authorization: 'Bearer jmr_t' }, body: JSON.stringify(body) });
const seen: Request[] = [];
const dispatch = async (r: Request): Promise<Response> => { seen.push(r); return Response.json({ podId: 'pod_1' }); };
const ask = async (who: Who, body: unknown) => (await mcp(rpc(body), 'https://x', who, dispatch)).json() as Promise<Record<string, any>>;

test('initialize answers, and a notification gets 202 with no body', async () => {
  expect((await ask(run, { jsonrpc: '2.0', id: 1, method: 'initialize', params: { protocolVersion: '2025-03-26' } })).result.serverInfo.name).toBe('jellymoon');
  const note = await mcp(rpc({ jsonrpc: '2.0', method: 'notifications/initialized' }), 'https://x', run, dispatch);
  expect([note.status, await note.text()]).toEqual([202, '']);
});

test('tools/list is the generated list, one tool per operation', async () => {
  const names = (await ask(run, { jsonrpc: '2.0', id: 1, method: 'tools/list' })).result.tools.map((t: { name: string }) => t.name);
  expect(names.length).toBe(9);
  expect(names[0]).toBe('jm_whoami');
});

test('a call is its operation over HTTP: the run token names the pod, the caller token is forwarded, the answer is the HTTP body', async () => {
  seen.length = 0;
  const r = (await ask(run, { jsonrpc: '2.0', id: 2, method: 'tools/call', params: { name: 'jm_whoami', arguments: {} } })).result;
  expect([seen[0].method, seen[0].url, seen[0].headers.get('Authorization')]).toEqual(['GET', 'https://x/v1/pods/pod_1/me', 'Bearer jmr_t']);
  expect([r.isError, r.structuredContent]).toEqual([false, { podId: 'pod_1' }]);
});

test('path fields come from the arguments, the rest is the body, and a run token cannot name another pod', async () => {
  seen.length = 0;
  await ask(run, { jsonrpc: '2.0', id: 3, method: 'tools/call', params: { name: 'jm_task_update', arguments: { podId: 'pod_2', taskId: 'tsk_9', status: 'done' } } });
  expect([seen[0].method, seen[0].url, await seen[0].json()]).toEqual(['PATCH', 'https://x/v1/pods/pod_1/tasks/tsk_9', { status: 'done' }]);
});

test('an unknown tool, an unknown method and a user token with no pod are refused', async () => {
  expect((await ask(run, { jsonrpc: '2.0', id: 4, method: 'tools/call', params: { name: 'jm_nothing' } })).error.code).toBe(-32602);
  expect((await ask(run, { jsonrpc: '2.0', id: 5, method: 'resources/list' })).error.code).toBe(-32601);
  expect((await ask(user, { jsonrpc: '2.0', id: 6, method: 'tools/call', params: { name: 'jm_whoami' } })).error.code).toBe(-32602);
  expect((await ask(run, [1])).error.code).toBe(-32600);
});

test('a tool name that is an inherited object property is no such tool, not a server error', async () => {
  for (const name of ['jm_constructor', 'jm___proto__', 'jm_toString', 'jm_hasOwnProperty']) {
    const r = await ask(run, { jsonrpc: '2.0', id: 4, method: 'tools/call', params: { name, arguments: {} } });
    expect([name, r.error]).toEqual([name, { code: -32602, message: `No such tool: ${name}` }]);
  }
});
