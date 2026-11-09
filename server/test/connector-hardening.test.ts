import { describe, expect, test } from 'bun:test';
import { pinOf } from '../src/catalogue';
import { rulingOpts } from '../src/connector-call';
import type { Db } from '../src/db';
import { openDb } from '../src/db';
import type { Resolve } from '../src/egress';
import { Hub } from '../src/hub';
import { migrate, saveCall } from '../src/store';
import type { CallRow } from '../src/store';
import type { Adapter } from '../src/upstream';
import { keyRing, makeVault } from '../src/vault';
import type { Vault } from '../src/vault';
import type { Json } from './world';
import { podIn, setup } from './world';

// The connector layer's hardening (ADR 0009): the crash window between a release's claim and its answer, a receipt for a
// read that throws, and an upstream call that runs outside the pod's command order. The approver set is TestF19's.
const CRED = 'pod-held-credential';
const noDns: Resolve = async (h) => { throw new Error(`resolved ${h}`); };
const vaultOf = async (ch: string): Promise<Vault> => makeVault(keyRing({ JM_VAULT_KEY: btoa(ch.repeat(32)) }) as Uint8Array<ArrayBuffer>[]);
const listed = pinOf('linear').operations.map((o) => ({ name: o.name, args: [...o.args], required: [...o.required] }));
/** An adapter whose listing matches the pin and whose call is `call`: no request leaves the process. */
const adapterWith = (call: Adapter['call']): Adapter => ({ list: async () => ({ ok: true, ops: listed }), call });

/** A pod with a connection the agent holds write-with-confirm on, and the agent's run. */
async function connected(call: Adapter['call'], db?: Db, vault?: Vault) {
  const v = vault ?? (await vaultOf('k'));
  const adapter = adapterWith(call);
  const s = await setup(db, { local: true, vault: v, resolve: noDns, adapter });
  const A = await podIn(s);
  const con = (await s.call('POST', `/v1/pods/${A.pod.id}/connections`, A.olivia.token, { vendor: 'linear', url: 'http://127.0.0.1:1/mcp', token: CRED })).json;
  expect(String(con.id).startsWith('con_')).toBe(true);
  await s.call('POST', `/v1/pods/${A.pod.id}/grants`, A.olivia.token, { connectionId: con.id, agentId: A.agent.agentId, access: 'write-with-confirm' });
  const claim = (await s.call('POST', '/v1/runs', A.comp.token, { wakeId: A.wake.wakeId })).json;
  const op = (operation: string, args: Json, as: typeof s = s) => as.call('POST', `/v1/pods/${A.pod.id}/connectors/call`, claim.runToken, { connectionId: con.id, operation, args });
  const receipts = async (q = '', token = A.olivia.token as string, as: typeof s = s) => (await as.call('GET', `/v1/pods/${A.pod.id}/connectors/receipts${q}`, token)).json.receipts as Json[];
  let n = 10;
  /** A pod owner's ruling, through the same opts the socket passes. */
  const rule = (decisionId: string, option: 'a' | 'b') => {
    const cmd = { type: 'rule' as const, caller: { kind: 'user' as const, userId: A.olivia.user.id }, decisionId, option, wakeId: `wak_00000000000000000000000${n++}0` };
    return A.rt.exec(cmd, rulingOpts({ hub: s.hub, vault: v, local: true, resolve: noDns, adapter }, A.rt, cmd));
  };
  return { ...A, s, con, claim, op, receipts, rule, adapter };
}

const released = (id: string, pod: string, connection: string, settledAt: number, upstreamStatus: number | null = null): CallRow => ({
  id, pod, connection, operation: 'create_issue', args: '{"title":"t","teamId":"T"}', argsHash: '0'.repeat(64), runId: 'run_1', agentId: 'agt_1', outcome: 'released',
  upstreamStatus, decisionId: `dec_${id.slice(4)}`, approver: null, result: null, createdAt: settledAt - 1000, settledAt, expiresAt: settledAt + 86_400_000,
});
const ULID = (c: string): string => c.repeat(26);

describe('the crash window between a release and its answer', () => {
  test('on start, a released row with no upstream answer older than the request budget settles as unknown; a fresh or answered one does not', async () => {
    const db = await openDb({});
    await migrate(db);
    const now = Date.now();
    await saveCall(db, released(`crc_${ULID('A')}`, 'pod_1', 'con_1', now - 120_000));
    await saveCall(db, released(`crc_${ULID('B')}`, 'pod_1', 'con_1', now - 1000));
    await saveCall(db, released(`crc_${ULID('C')}`, 'pod_1', 'con_1', now - 120_000, 200));
    await Hub.restore(db);
    const rows = await db.rows<{ id: string; outcome: string; reason: string | null }>('select id, outcome, reason from connector_calls order by id');
    expect(rows.map((r) => r.outcome)).toEqual(['unknown', 'released', 'released']);
    expect(String(rows[0].reason)).toContain('outcome not recorded');
    expect([rows[1].reason, rows[2].reason]).toEqual([null, null]);
  });

  test('lazily on read, the receipts (a person\'s and the agent\'s) show unknown with its reason, and nothing is sent again', async () => {
    let calls = 0;
    const w = await connected(async () => { calls += 1; return { ok: true, status: 200, result: {} }; });
    const now = w.s.hub.now();
    const stale = released(`crc_${ULID('D')}`, w.pod.id, w.con.id, now - 61_000);
    const fresh = released(`crc_${ULID('E')}`, w.pod.id, w.con.id, now - 1000);
    await saveCall(w.s.db, stale);
    await saveCall(w.s.db, fresh);
    for (const token of [w.olivia.token, w.claim.runToken]) {
      const [r] = await w.receipts(`?decisionId=${stale.decisionId}`, token);
      expect([r.outcome, String(r.reason).includes('sent at most once'), r.upstreamStatus]).toEqual(['unknown', true, undefined]);
    }
    expect((await w.receipts(`?decisionId=${fresh.decisionId}`))[0].outcome).toBe('released');
    expect(calls).toBe(0);
  });
});

describe('a read whose adapter throws', () => {
  const throwers: Array<[string, () => never]> = [
    ['a non-HTTP error', () => { throw new TypeError(`socket hang up near ${CRED}`); }],
    ['a parse failure', () => { JSON.parse('{ not json'); throw new Error('unreachable'); }],
    ['a thrown string', () => { throw CRED; }],
  ];
  for (const [what, boom] of throwers) {
    test(`${what}: the agent gets upstream_failed and a failed receipt with a safe reason is written`, async () => {
      const w = await connected(async () => boom());
      const r = await w.op('get_issue', { id: 'ISS-1' });
      expect([r.status, r.json.code]).toEqual([502, 'upstream_failed']);
      expect(JSON.stringify(r.json)).not.toContain(CRED);
      const all = await w.receipts();
      expect(all.map((x) => [x.operation, x.outcome, x.id === r.json.ref])).toEqual([['get_issue', 'failed', true]]);
      expect(typeof all[0].reason === 'string' && !String(all[0].reason).includes(CRED)).toBe(true);
    });
  }

  test('a vault that cannot open the credential: the read and a release each leave a failed receipt; nothing is sent', async () => {
    let calls = 0;
    const db = await openDb({});
    const w = await connected(async () => { calls += 1; return { ok: true, status: 200, result: {} }; }, db, await vaultOf('a'));
    const parked = (await w.op('create_issue', { title: 't', teamId: 'T' })).json;
    expect(parked.state).toBe('parked');
    const other = await setup(db, { local: true, vault: await vaultOf('b'), resolve: noDns, adapter: w.adapter });
    const r = await w.op('list_issues', {}, other);
    expect([r.status, r.json.code]).toEqual([502, 'upstream_failed']);
    const kit = { hub: other.hub, vault: await vaultOf('b'), local: true, resolve: noDns, adapter: w.adapter };
    const cmd = { type: 'rule' as const, caller: { kind: 'user' as const, userId: w.olivia.user.id }, decisionId: parked.decisionId, option: 'a', wakeId: `wak_${ULID('9')}` };
    const rt = other.hub.pods.get(w.pod.id)!;
    expect((await rt.exec(cmd, rulingOpts(kit, rt, cmd)))?.ok).toBe(true);
    const settled = async () => (await w.receipts('', w.olivia.token, other)).map((x) => [x.operation, x.outcome, typeof x.reason]);
    for (let i = 0; i < 40 && (await settled()).some(([, o]) => o === 'released'); i += 1) await Bun.sleep(25);
    expect(await settled()).toEqual([['list_issues', 'failed', 'string'], ['create_issue', 'failed', 'string']]);
    expect(calls).toBe(0);
  });
});

describe('a slow upstream', () => {
  test('a released write and a read in flight for 2 s hold neither the pod\'s command order nor the next ruling', async () => {
    let started = 0;
    const w = await connected(async () => { started += 1; await Bun.sleep(2000); return { ok: true, status: 200, result: { done: true } }; });
    const one = (await w.op('create_issue', { title: 'one', teamId: 'T' })).json;
    const two = (await w.op('create_issue', { title: 'two', teamId: 'T' })).json;
    const t0 = Date.now();
    expect((await w.rule(one.decisionId, 'a'))?.ok).toBe(true);
    expect((await w.rule(two.decisionId, 'b'))?.ok).toBe(true);
    expect(Date.now() - t0 < 1000).toBe(true);
    const inFlight = (await w.receipts(`?decisionId=${one.decisionId}`))[0];
    expect([inFlight.outcome, inFlight.upstreamStatus]).toEqual(['released', undefined]);
    const read = w.op('list_issues', {});
    for (let i = 0; i < 40 && started < 2; i += 1) await Bun.sleep(25);
    const t1 = Date.now();
    const three = (await w.op('create_issue', { title: 'three', teamId: 'T' })).json;
    expect((await w.rule(three.decisionId, 'b'))?.ok).toBe(true);
    expect([started, Date.now() - t1 < 1000]).toEqual([2, true]);
    expect((await read).json.state).toBe('done');
    for (let i = 0; i < 80 && (await w.receipts(`?decisionId=${one.decisionId}`))[0].upstreamStatus === undefined; i += 1) await Bun.sleep(50);
    const done = (await w.receipts(`?decisionId=${one.decisionId}`))[0];
    expect([done.outcome, done.upstreamStatus, done.result]).toEqual(['released', 200, { done: true }]);
  }, 15_000);
});
