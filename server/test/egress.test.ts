import { describe, expect, test } from 'bun:test';
import { rulingOpts } from '../src/connector-call';
import { destOf, internalAddress } from '../src/egress';
import type { Dest, Resolve } from '../src/egress';
import { keyRing, makeVault } from '../src/vault';
import type { Json } from './world';
import { podIn, setup } from './world';

// Where a connector may send the pod's credential (ADR 0009, "Egress"). The classifier, the resolve step with an injected
// resolver (no real DNS), and the re-check before every upstream call, with fetch replaced so nothing leaves the process.
const noDns: Resolve = async (h) => { throw new Error(`a literal or refused host was resolved: ${h}`); };
const refused = async (raw: string, local = false, resolve = noDns) => typeof (await destOf(raw, local, resolve)) === 'string';

describe('the address classifier', () => {
  test('every internal range is refused, IPv4 and IPv6, v4-mapped, NAT64 and v4-compatible included', () => {
    const inside = ['0.0.0.0', '0.1.2.3', '10.0.0.1', '10.255.255.255', '100.64.0.1', '100.127.255.255', '127.0.0.1', '127.255.255.254',
      '169.254.0.1', '169.254.169.254', '172.16.0.1', '172.31.255.255', '192.0.0.8', '192.168.0.1', '192.168.255.255', '198.18.0.1',
      '198.19.255.255', '224.0.0.1', '239.255.255.255', '240.0.0.1', '255.255.255.254', '255.255.255.255',
      '::', '::1', '0:0:0:0:0:0:0:1', 'fc00::1', 'fdff:ffff::1', 'fe80::1', 'febf::1', 'ff02::1', 'ff00::',
      '::ffff:127.0.0.1', '::ffff:7f00:1', '::ffff:10.0.0.1', '::ffff:a9fe:a9fe', '64:ff9b::a9fe:a9fe', '64:ff9b::10.0.0.1', '64:ff9b::7f00:1',
      '::127.0.0.1', '::7f00:1', '::8.8.8.8', '192.0.2.1', '198.51.100.7', '203.0.113.9', '192.88.99.1', '64:ff9b:1::808:808', '64:ff9b:1:ffff::1',
      '2002:7f00:1::', '2002:a9fe:a9fe::1', '2002:c0a8:101::', '2001::1', '2001:0:4136:e378:8000:63bf:3fff:fdd2', '::ffff:0:7f00:1', '::ffff:0:a00:1', 'not-an-address', '1.2.3', '256.1.1.1', '1.2.3.4.5', '01.2.3.4', ':::1', '1::2::3', 'fe80::1%en0'];
    expect(inside.filter((a) => !internalAddress(a))).toEqual([]);
  });
  test('public addresses pass, including the edges of each range', () => {
    const outside = ['8.8.8.8', '1.1.1.1', '93.184.216.34', '9.255.255.255', '11.0.0.0', '100.63.255.255', '100.128.0.0', '126.255.255.255', '128.0.0.0',
      '169.253.255.255', '169.255.0.0', '172.15.255.255', '172.32.0.0', '192.0.1.0', '191.255.255.255', '192.167.255.255', '192.169.0.0',
      '198.17.255.255', '198.20.0.0', '223.255.255.255', '2606:4700::1111', '2001:4860:4860::8888', '::ffff:8.8.8.8', '64:ff9b::808:808', 'fbff::1', 'fec0::1',
      '192.0.3.1', '198.51.99.1', '198.51.101.1', '203.0.112.1', '203.0.114.1', '192.88.98.1', '192.88.100.1', '2002:808:808::1', '::ffff:0:808:808', '2001:1::1', '64:ff9b:2::1'];
    expect(outside.filter((a) => internalAddress(a))).toEqual([]);
  });
});

describe('the url and its host', () => {
  test('WHATWG URL normalises odd IPv4 spellings and bracketed IPv6; each normalised form is refused without a lookup', async () => {
    expect(['https://2130706433/', 'https://0x7f.1/', 'https://127.1/', 'https://0177.0.0.1/'].map((u) => new URL(u).hostname)).toEqual(['127.0.0.1', '127.0.0.1', '127.0.0.1', '127.0.0.1']);
    expect(new URL('https://[::127.0.0.1]/').hostname).toBe('[::7f00:1]');
    const odd = ['https://2130706433/', 'https://0x7f.1/', 'https://127.1/', 'https://[::127.0.0.1]/', 'https://[::ffff:169.254.169.254]/', 'https://[64:ff9b::a9fe:a9fe]/',
      'https://169.254.169.254/', 'https://[fd00::1]/', 'https://localhost/', 'https://localhost./', 'https://LOCALHOST../', 'https://a.localhost./'];
    expect((await Promise.all(odd.map((u) => refused(u)))).every(Boolean)).toBe(true);
  });
  test('https only, no credentials in the url, and a name is lowercased with its trailing dots stripped before the lookup', async () => {
    expect(await refused('http://mcp.example.com/')).toBe(true);
    expect(await refused('https://u:p@example.com/', false, async () => ['93.184.216.34'])).toBe(true);
    expect(await refused('nope')).toBe(true);
    for (const u of ['https://localhost./', 'https://LocalHost../', 'https://a.localhost./']) expect([u, await refused(u, false, async () => ['93.184.216.34'])]).toEqual([u, true]);
    const asked: string[] = [];
    const d = (await destOf('https://MCP.Example.COM../mcp', false, async (h) => { asked.push(h); return ['93.184.216.34']; })) as Dest;
    expect([asked, d.url, d.host, d.serverName]).toEqual([['mcp.example.com'], 'https://93.184.216.34/mcp', 'mcp.example.com', 'mcp.example.com']);
  });
});

describe('the resolve step', () => {
  const dns: Record<string, string[]> = { 'private.test': ['10.0.0.5'], 'mixed.test': ['93.184.216.34', '169.254.169.254'], 'v6.test': ['2606:4700::1111', 'fe80::1'],
    'public.test': ['2606:2800:220:1::1', '93.184.216.34'], 'v6only.test': ['2606:2800:220:1::1'], 'empty.test': [] };
  const resolve: Resolve = async (h) => {
    if (h === 'slow.test') return new Promise<string[]>(() => {});
    const a = dns[h];
    if (a === undefined) throw new Error('ENOTFOUND');
    return a;
  };
  test('a name resolving to an internal address, to public plus internal, to nothing, or failing to resolve, is refused', async () => {
    for (const h of ['private.test', 'mixed.test', 'v6.test', 'empty.test', 'fails.test']) expect([h, await refused(`https://${h}/mcp`, false, resolve)]).toEqual([h, true]);
    expect(String(await destOf('https://fails.test/mcp', false, resolve)).includes('could not be resolved')).toBe(true);
  });
  test('a name resolving only to public addresses passes, pinned to its first IPv4 (else its first address); Host and TLS name stay the name', async () => {
    expect(await destOf('https://public.test:8443/mcp?x=1', false, resolve)).toEqual({ url: 'https://93.184.216.34:8443/mcp?x=1', host: 'public.test:8443', serverName: 'public.test' });
    expect(await destOf('https://v6only.test/mcp', false, resolve)).toEqual({ url: 'https://[2606:2800:220:1::1]/mcp', host: 'v6only.test', serverName: 'v6only.test' });
  });
  test('a lookup that does not answer within its 2 s budget is refused', async () => {
    const t0 = Date.now();
    expect(String(await destOf('https://slow.test/mcp', false, resolve)).includes('could not be resolved')).toBe(true);
    expect(Date.now() - t0 < 4000).toBe(true);
  });
  test('ENV=local allows literal loopback over http or https, and nothing else that the rules refuse', async () => {
    expect(await destOf('http://127.0.0.1:4000/mcp', true, noDns)).toEqual({ url: 'http://127.0.0.1:4000/mcp' });
    expect(await refused('http://u:p@127.0.0.1:4000/mcp', true)).toBe(true);
    expect(await destOf('https://[::1]:4000/mcp', true, noDns)).toEqual({ url: 'https://[::1]:4000/mcp' });
    for (const u of ['http://127.0.0.1:4000/mcp', 'http://public.test/', 'https://private.test/', 'https://10.0.0.1/', 'https://localhost./']) {
      expect([u, await refused(u, !u.startsWith('http://127'), resolve)]).toEqual([u, true]);
    }
  });
});

describe('the re-check before every upstream call', () => {
  test('a connection that resolved public at connect and resolves inside later is refused on a read and on its release; nothing is sent', async () => {
    let inside = false;
    const resolve: Resolve = async (h) => (h === 'up.test' ? [inside ? '169.254.169.254' : '93.184.216.34'] : []);
    const sent: Array<{ url: string; host: string | null; serverName?: string; method: string }> = [];
    const real = globalThis.fetch;
    globalThis.fetch = (async (input: string, init: RequestInit & { tls?: { serverName?: string } }) => {
      const m = JSON.parse(String(init.body)) as Json;
      sent.push({ url: String(input), host: new Headers(init.headers).get('host'), serverName: init.tls?.serverName, method: String(m.method) });
      if (m.id === undefined) return new Response(null, { status: 202 });
      const tools = [['list_issues', ['query', 'teamId', 'limit'], []], ['get_issue', ['id'], ['id']], ['create_issue', ['title', 'teamId', 'description'], ['title', 'teamId']]]
        .map(([name, args, required]) => ({ name, inputSchema: { type: 'object', properties: Object.fromEntries((args as string[]).map((a) => [a, { type: 'string' }])), required } }));
      const result = m.method === 'initialize' ? { protocolVersion: '2025-03-26', capabilities: {} } : m.method === 'tools/list' ? { tools } : { structuredContent: { ok: true } };
      return Response.json({ jsonrpc: '2.0', id: m.id, result });
    }) as unknown as typeof fetch;
    try {
      const vault = await makeVault(keyRing({ JM_VAULT_KEY: btoa('k'.repeat(32)) }) as Uint8Array<ArrayBuffer>[]);
      const s = await setup(undefined, { local: true, vault, resolve });
      const A = await podIn(s);
      const con = (await s.call('POST', `/v1/pods/${A.pod.id}/connections`, A.olivia.token, { vendor: 'linear', url: 'https://up.test/mcp', token: 'pod-held-credential' })).json;
      await s.call('POST', `/v1/pods/${A.pod.id}/grants`, A.olivia.token, { connectionId: con.id, agentId: A.agent.agentId, access: 'write-with-confirm' });
      const run = (await s.call('POST', '/v1/runs', A.comp.token, { wakeId: A.wake.wakeId })).json.runToken as string;
      const op = (operation: string, args: Json) => s.call('POST', `/v1/pods/${A.pod.id}/connectors/call`, run, { connectionId: con.id, operation, args });
      const kit = { hub: s.hub, vault, local: true, resolve };
      const approve = async (decisionId: string, n: number) => {
        const cmd = { type: 'rule' as const, caller: { kind: 'user' as const, userId: A.olivia.user.id }, decisionId, option: 'a', wakeId: `wak_0000000000000000000000000${n}` };
        expect((await A.rt.exec(cmd, rulingOpts(kit, A.rt, cmd)))?.ok).toBe(true);
        return (await s.call('GET', `/v1/pods/${A.pod.id}/connectors/receipts?decisionId=${decisionId}`, A.olivia.token)).json.receipts[0].outcome as string;
      };
      expect((await op('list_issues', {})).json.state).toBe('done');
      const [early, late] = [(await op('create_issue', { title: 't', teamId: 'T' })).json, (await op('create_issue', { title: 'u', teamId: 'T' })).json];
      expect([early.state, late.state]).toEqual(['parked', 'parked']);
      expect(await approve(early.decisionId, 8)).toBe('released');
      // While the name resolves public: connect's three requests, the read's, and the release's, each pinned to the checked IP.
      expect(sent.map((x) => x.method)).toEqual(['initialize', 'notifications/initialized', 'tools/list', ...Array(2).fill(['initialize', 'notifications/initialized', 'tools/call']).flat()]);
      expect(sent.filter((x) => x.url !== 'https://93.184.216.34/mcp' || x.host !== 'up.test' || x.serverName !== 'up.test')).toEqual([]);
      inside = true;
      const before = sent.length;
      const read = await op('list_issues', {});
      expect([read.status, read.json.code, String(read.json.message).includes('internal address')]).toEqual([502, 'upstream_failed', true]);
      expect([await approve(late.decisionId, 9), sent.length - before]).toEqual(['failed', 0]);
    } finally { globalThis.fetch = real; }
  });
});
