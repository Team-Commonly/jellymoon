import { describe, expect, test } from 'bun:test';
import { canonical, driftOf, pinOf, scrub } from '../src/catalogue';
import { startupRefusal } from '../src/config';
import { openDb } from '../src/db';
import { claimParked, migrate, saveCall } from '../src/store';
import type { CallRow } from '../src/store';
import { keyRing, makeVault } from '../src/vault';
import { world } from './world';

// The connector layer's pieces (ADR 0009). The end-to-end behaviour is TestF19; these pin what the suite cannot see.
const key = (ch: string): string => btoa(ch.repeat(32));
const ring = (env: Record<string, string>) => keyRing(env) as Uint8Array<ArrayBuffer>[];

describe('the vault', () => {
  test('seals with a fresh data key per row, opens with the same additional data only, and never stores the plaintext', async () => {
    const v = await makeVault(ring({ JM_VAULT_KEY: key('a') }));
    const one = await v.seal('the-credential', 'pod_1/con_1');
    const two = await v.seal('the-credential', 'pod_1/con_1');
    expect(one.wrapped === two.wrapped).toBe(false);
    expect(JSON.stringify(one)).not.toContain('the-credential');
    expect(await v.open(one, 'pod_1/con_1')).toBe('the-credential');
    let moved = 'opened';
    await v.open(one, 'pod_1/con_2').catch(() => { moved = 'refused'; });
    expect(moved).toBe('refused');
  });

  test('a rotated ring seals under the new key and still opens a row sealed under a retired one', async () => {
    const old = await makeVault(ring({ JM_VAULT_KEY: key('a') }));
    const sealed = await old.seal('t', 'x');
    const next = await makeVault(ring({ JM_VAULT_KEY: key('b'), JM_VAULT_OLD_KEYS: key('a') }));
    expect(await next.open(sealed, 'x')).toBe('t');
    expect((await next.seal('t', 'x')).kid === sealed.kid).toBe(false);
    let gone = 'opened';
    await (await makeVault(ring({ JM_VAULT_KEY: key('b') }))).open(sealed, 'x').catch(() => { gone = 'refused'; });
    expect(gone).toBe('refused');
  });

  test('each seal draws a fresh IV for both layers, and a tampered tag does not open', async () => {
    const v = await makeVault(ring({ JM_VAULT_KEY: key('a') }));
    const one = await v.seal('same', 'x');
    const two = await v.seal('same', 'x');
    const iv = (s: string) => atob(s).slice(0, 12);
    expect([iv(one.sealed) === iv(two.sealed), iv(one.wrapped) === iv(two.wrapped), one.sealed === two.sealed]).toEqual([false, false, false]);
    const raw = atob(one.sealed);
    const tampered = btoa(raw.slice(0, -1) + String.fromCharCode(raw.charCodeAt(raw.length - 1) ^ 1));
    let opened = 'opened';
    await v.open({ ...one, sealed: tampered }, 'x').catch(() => { opened = 'refused'; });
    expect(opened).toBe('refused');
  });

  test('a malformed key refuses the start without echoing it; no key at all is allowed', () => {
    const why = startupRefusal({ ENV: 'local', JM_VAULT_KEY: 'not-a-key-value' });
    expect(why?.includes('JM_VAULT_KEY')).toBe(true);
    expect(why).not.toContain('not-a-key-value');
    expect(startupRefusal({ ENV: 'local' })).toBe(undefined);
    expect(startupRefusal({ ENV: 'local', JM_VAULT_KEY: key('a') })).toBe(undefined);
  });
});

describe('the catalogue pin', () => {
  const ops = () => pinOf('linear').operations.map((o) => ({ name: o.name, args: [...o.args].reverse(), required: o.required }));
  test('an upstream listing every pinned operation as pinned (in any order, plus others) is no drift', () => {
    expect(driftOf(pinOf('linear'), [...ops(), { name: 'delete_issue', args: ['id'], required: ['id'] }])).toBeNull();
  });
  test('an empty list, a missing operation and changed arguments are drift, each naming what differs', () => {
    expect(driftOf(pinOf('linear'), [])?.includes('lists no operations')).toBe(true);
    expect(driftOf(pinOf('linear'), ops().filter((o) => o.name !== 'get_issue'))?.includes('get_issue')).toBe(true);
    const added = ops().map((o) => (o.name === 'create_issue' ? { ...o, args: [...o.args, 'priority'] } : o));
    expect(driftOf(pinOf('linear'), added)?.includes('create_issue changed upstream')).toBe(true);
  });
  test('the args hash is over sorted keys, so the agent\'s key order does not change it', () => {
    expect(canonical({ b: 1, a: { d: [2, { z: 1, y: 2 }], c: null } })).toBe('{"a":{"c":null,"d":[2,{"y":2,"z":1}]},"b":1}');
  });
  test('scrub removes a credential in raw and in JSON-escaped form', () => {
    expect(scrub({ e: 'Bearer s3cr"et and s3cr"et' }, 's3cr"et')).toEqual({ e: 'Bearer [credential removed] and [credential removed]' });
    expect(scrub('plain', 'other')).toBe('plain');
  });
  test('scrub also removes the token \\u-escaped, URL-encoded and base64-encoded at any alignment, by dropping the text', () => {
    const t = 'tok/en+value-0123';
    const u = [...t].map((c) => `\\u${c.charCodeAt(0).toString(16).padStart(4, '0')}`).join('');
    const url64 = (s: string) => btoa(s).replace(/\+/g, '-').replace(/\//g, '_');
    for (const s of [`x ${u} y`, `x ${encodeURIComponent(t)} y`, `x=${btoa(t)} y`, `x ${btoa(`Bearer ${t}`)} y`, `x ${url64(`ab${t}`)}`, `zz${btoa(`a${t}`)}`]) {
      expect([s, scrub({ e: s }, t)]).toEqual([s, { e: '[credential removed]' }]);
    }
    expect(scrub({ e: `HTTP 401 ${btoa('unrelated words here')}` }, t)).toEqual({ e: `HTTP 401 ${btoa('unrelated words here')}` });
  });
});

describe('the write gate', () => {
  const parked = (id: string, expiresAt: number): CallRow => ({
    id, pod: 'pod_1', connection: 'con_1', operation: 'create_issue', args: '{}', argsHash: '0'.repeat(64), runId: 'run_1', agentId: 'agt_1', outcome: 'parked',
    upstreamStatus: null, decisionId: `dec_${id}`, approver: null, result: null, createdAt: 1, settledAt: null, expiresAt,
  });

  test('a parked write is claimed once: a second claim, a retry or a race finds nothing to release', async () => {
    const db = await openDb({});
    await migrate(db);
    await saveCall(db, parked('a', 1000));
    const both = await Promise.all([db.tx((t) => claimParked(t, 'dec_a', 'released', null, 10)), db.tx((t) => claimParked(t, 'dec_a', 'released', null, 10))]);
    expect(both.filter((r) => r !== undefined).length).toBe(1);
    expect(await db.tx((t) => claimParked(t, 'dec_a', 'rejected', null, 11))).toBe(undefined);
  });

  test('an expired parked write is never claimed', async () => {
    const db = await openDb({});
    await migrate(db);
    await saveCall(db, parked('b', 1000));
    expect(await db.tx((t) => claimParked(t, 'dec_b', 'released', null, 1000))).toBe(undefined);
  });

  test('without JM_VAULT_KEY a connect is refused with vault_unconfigured', async () => {
    const w = await world();
    const r = await w.call('POST', `/v1/pods/${w.pod.id}/connections`, w.olivia.token, { vendor: 'linear', url: 'http://127.0.0.1:1/mcp', token: 't' });
    expect([r.status, r.json.code]).toEqual([501, 'vault_unconfigured']);
  });
});
