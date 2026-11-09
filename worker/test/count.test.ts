import { expect, test } from 'bun:test';
import { handle } from '../src/front';
import type { Env } from '../src/env';

// The front door hands the MCP adapter its count: a run's tool call is told to the pod before the operation is dispatched, and a pod that
// does not answer the count with a success is a call that was not counted, so the operation never runs.
const TOKEN = 'jmr_' + 'r'.repeat(43);

function world(countStatus: number) {
  const fetched: string[] = [];
  const db = { prepare: () => ({ bind: () => ({ first: async () => ({ subject: 'run_1', pod: 'pod_1', one: 1 }) }) }) };
  const POD = { idFromName: (n: string) => n, get: () => ({ fetch: async (req: string | Request) => {
    const url = typeof req === 'string' ? req : req.url;
    fetched.push(new URL(url).pathname);
    return url.endsWith('/_in/tool-call') ? Response.json({}, { status: countStatus }) : Response.json({ podId: 'pod_1' });
  } }) };
  return { env: { DB: db, POD } as unknown as Env, fetched };
}

const call = (env: Env): Promise<Response> => handle(new Request('http://127.0.0.1:8787/mcp', {
  method: 'POST', headers: { Authorization: `Bearer ${TOKEN}` }, body: JSON.stringify({ jsonrpc: '2.0', id: 1, method: 'tools/call', params: { name: 'jm_whoami', arguments: {} } }),
}), env);

test('a count the pod answers with a success lets the call through, after the count', async () => {
  const w = world(200);
  const r = (await (await call(w.env)).json()) as Record<string, any>;
  expect([r.error, r.result?.isError, w.fetched]).toEqual([undefined, false, ['/_in/tool-call', '/v1/pods/pod_1/me']]);
});

test('a count the pod answers with a failure refuses the call, and the operation is never dispatched', async () => {
  const w = world(500);
  const r = (await (await call(w.env)).json()) as Record<string, any>;
  expect([r.error?.code, r.result, w.fetched]).toEqual([-32603, undefined, ['/_in/tool-call']]);
});
