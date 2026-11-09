import { describe, expect, test } from 'bun:test';
import { pinOf } from '../src/catalogue';
import { call } from '../src/upstream-mcp';
import { keyRing, makeVault } from '../src/vault';
import type { Json } from './world';
import { podIn, setup } from './world';

// What keys a connector call (ADR 0009): a connection is the pod's own, a grant is the connection's own, and an upstream
// answer is bounded. A small MCP stub stands in for the vendor and records every tools/call it gets.

function stubMcp(pad = 0) {
  const calls: string[] = [];
  const tools = pinOf('linear').operations.map((o) => ({ name: o.name, inputSchema: { type: 'object', properties: Object.fromEntries(o.args.map((a) => [a, { type: 'string' }])), required: o.required } }));
  const srv = Bun.serve({
    port: 0, websocket: undefined,
    async fetch(req) {
      const m = (await req.json()) as Json;
      if (m.id === undefined) return new Response(null, { status: 202 });
      if (m.method === 'tools/call') calls.push(String(m.params.name));
      const result = m.method === 'initialize' ? { protocolVersion: '2025-03-26', capabilities: {} } : m.method === 'tools/list' ? { tools } : { structuredContent: { pad: 'x'.repeat(pad) } };
      return Response.json({ jsonrpc: '2.0', id: m.id, result });
    },
  });
  return { url: `http://127.0.0.1:${srv.port}/mcp`, calls, stop: () => srv.stop(true) };
}

describe('a connector call is keyed on its pod and its connection', () => {
  test('another pod\'s connection is not found and a grant on one connection does not open another; the stub sees nothing', async () => {
    const s = await setup(undefined, { local: true, vault: await makeVault(keyRing({ JM_VAULT_KEY: btoa('k'.repeat(32)) }) as Uint8Array<ArrayBuffer>[]) });
    const A = await podIn(s);
    const B = await podIn(s, 'b');
    const up = stubMcp();
    type W = typeof A;
    const connect = async (w: W) => (await s.call('POST', `/v1/pods/${w.pod.id}/connections`, w.olivia.token, { vendor: 'linear', url: up.url, token: 'pod-held-credential' })).json.id as string;
    const grant = (w: W, conn: string) => s.call('POST', `/v1/pods/${w.pod.id}/grants`, w.olivia.token, { connectionId: conn, agentId: w.agent.agentId, access: 'read' });
    const runOf = async (w: W) => (await s.call('POST', '/v1/runs', w.comp.token, { wakeId: w.wake.wakeId })).json.runToken as string;
    const read = (w: W, run: string, conn: string) => s.call('POST', `/v1/pods/${w.pod.id}/connectors/call`, run, { connectionId: conn, operation: 'list_issues', args: {} });
    const one = await connect(A);
    const two = await connect(A);
    expect((await grant(A, one)).status).toBe(200);
    const [ra, rb] = [await runOf(A), await runOf(B)];
    expect([(await grant(B, one)).json.code, (await read(B, rb, one)).json.code, (await read(A, ra, two)).json.code]).toEqual(['not_found', 'not_found', 'forbidden']);
    expect(up.calls).toEqual([]);
    expect((await read(A, ra, one)).status).toBe(200);
    expect(up.calls).toEqual(['list_issues']);
    up.stop();
  });
});

describe('an upstream answer is bounded', () => {
  test('a body over 1 MiB is refused with upstream_too_large and none of it is kept; one under it is read', async () => {
    const [over, under] = [stubMcp(1100 * 1024), stubMcp(900 * 1024)];
    const big = await call(over.url, 't', 'list_issues', {});
    expect([big.ok, big.code, big.result, JSON.stringify(big).length < 400]).toEqual([false, 'upstream_too_large', undefined, true]);
    const fine = await call(under.url, 't', 'list_issues', {});
    expect([fine.ok, String(fine.result?.pad).length]).toEqual([true, 900 * 1024]);
    over.stop();
    under.stop();
  });
});
