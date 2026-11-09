import { describe, expect, test } from 'bun:test';
import type { AnyEvent } from '../../gen/ts/protocol';
import { buildApp } from '../src/app';
import { openDb } from '../src/db';
import { Hub } from '../src/hub';
import { eventsAfter } from '../src/store';
import { caller, world } from './world';

describe('server', () => {
  test('a restart restores users, tokens, pods and the event log, with no seq gap', async () => {
    const w = await world();
    const before = w.rt.state.seq;
    const again = await Hub.restore(w.db);
    expect(again.pods.get(w.pod.id)?.state.seq).toBe(before);
    expect((await again.authenticate(`Bearer ${w.olivia.token}`))?.kind).toBe('user');
    const log = await eventsAfter(w.db, w.pod.id, 0, 100);
    expect(log.map((e: AnyEvent) => e.seq)).toEqual(Array.from({ length: before }, (_, i) => i + 1));
  });

  test('a run token is bound to its pod, and an ended run is unauthorized', async () => {
    const w = await world();
    const run = (await w.call('POST', '/v1/runs', w.comp.token, { wakeId: w.wake.wakeId })).json;
    expect(/^jmr_[0-9A-Za-z]{43}$/.test(run.runToken)).toBe(true);
    expect((await w.call('GET', `/v1/pods/${w.pod.id}/me`, run.runToken)).status).toBe(200);
    const other = (await w.call('POST', '/v1/pods', w.olivia.token, { name: 'other' })).json;
    expect((await w.call('GET', `/v1/pods/${other.id}/me`, run.runToken)).status).toBe(403);
    expect((await w.call('POST', `/v1/runs/${run.runId}/end`, w.comp.token, { outcome: 'completed' })).status).toBe(200);
    expect((await w.call('GET', `/v1/pods/${w.pod.id}/me`, run.runToken)).json.code).toBe('unauthorized');
  });

  test('an invite works once and then is gone; a stranger and a bad token are refused', async () => {
    const w = await world();
    const inv = (await w.call('POST', `/v1/pods/${w.pod.id}/invites`, w.olivia.token, { maxUses: 1 })).json;
    const cara = (await w.call('POST', '/__test/session', undefined, { name: 'cara' })).json;
    const dan = (await w.call('POST', '/__test/session', undefined, { name: 'dan' })).json;
    expect((await w.call('POST', `/v1/invites/${inv.code}/accept`, cara.token)).status).toBe(200);
    expect((await w.call('POST', `/v1/invites/${inv.code}/accept`, dan.token)).status).toBe(410);
    expect((await w.call('GET', `/v1/pods/${w.pod.id}/tasks`, dan.token)).status).toBe(403);
    expect((await w.call('GET', `/v1/pods/${w.pod.id}/tasks`, 'jmu_nope')).status).toBe(401);
    expect((await w.call('POST', `/v1/pods/${w.pod.id}/invites`, dan.token, {})).status).toBe(403);
  });

  test('moving the test clock past a lease expires it at once, with one claim.released', async () => {
    const w = await world();
    const run = (await w.call('POST', '/v1/runs', w.comp.token, { wakeId: w.wake.wakeId })).json;
    const claimed = await w.call('POST', `/v1/pods/${w.pod.id}/tasks/tsk_00000000000000000000000001/claim`, run.runToken, {});
    expect(claimed.status).toBe(200);
    await w.hub.setClock(Date.parse(claimed.json.leaseUntil));
    const released = (await eventsAfter(w.db, w.pod.id, 0, 100)).filter((e) => e.type === 'claim.released');
    expect(released.length).toBe(1);
    const tasks = (await w.call('GET', `/v1/pods/${w.pod.id}/tasks`, w.olivia.token)).json.tasks;
    expect(tasks[0].claim === undefined).toBe(true);
  });

  test('test routes are absent unless ENV is local', async () => {
    const db = await openDb({});
    const app = buildApp(await Hub.restore(db), { local: false });
    expect((await app.fetch(new Request('http://x/__test/session', { method: 'POST', body: '{"name":"a"}' }))).status).toBe(404);
    expect((await app.fetch(new Request('http://x/healthz'))).status).toBe(200);
  });
});

describe('the public base URL', () => {
  const links = async (publicUrl: string | undefined) => {
    const w = await world();
    const call = caller(buildApp(w.hub, { local: true, publicUrl }));
    const invite = (await call('POST', `/v1/pods/${w.pod.id}/invites`, w.olivia.token, {})).json.url as string;
    const mcp = (await call('POST', '/v1/runs', w.comp.token, { wakeId: w.wake.wakeId })).json.mcpUrl as string;
    return { invite, code: invite.split('/join/')[1], mcp };
  };

  test('an empty PUBLIC_URL counts as unset: the request origin stands in', async () => {
    const l = await links('');
    expect([l.invite, l.mcp]).toEqual([`http://x/join/${l.code}`, 'http://x/mcp']);
  });

  test('a trailing slash is dropped once, for the invite link and the mcpUrl alike', async () => {
    const l = await links('https://x/');
    expect([l.invite, l.mcp]).toEqual([`https://x/join/${l.code}`, 'https://x/mcp']);
  });
});
