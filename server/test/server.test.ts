import { describe, expect, test } from 'bun:test';
import type { AnyEvent } from '../../gen/ts/protocol';
import { buildApp } from '../src/app';
import { openDb } from '../src/db';
import { Hub } from '../src/hub';
import type { Conn } from '../src/pod';
import { eventsAfter } from '../src/store';

type Json = Record<string, any>;

async function setup() {
  const db = await openDb({});
  const hub = await Hub.restore(db);
  const app = buildApp(hub, { local: true });
  const call = async (method: string, path: string, token?: string, body?: unknown): Promise<{ status: number; json: Json }> => {
    const headers: Record<string, string> = token ? { Authorization: `Bearer ${token}` } : {};
    const res = await app.fetch(new Request(`http://x${path}`, { method, headers, body: body === undefined ? undefined : JSON.stringify(body) }));
    return { status: res.status, json: (await res.json()) as Json };
  };
  return { db, hub, call };
}

const fakeConn = (who: Conn['who'], frames: string[]): Conn => ({ who, send: (t) => { frames.push(t); }, subscribed: true, replaying: false, buf: [], sent: 0 });

/** olivia owns a pod; ben (a member via invite) owns a computer with one agent; a task assigned to the agent has woken it. */
async function world() {
  const s = await setup();
  const olivia = (await s.call('POST', '/__test/session', undefined, { name: 'olivia' })).json;
  const ben = (await s.call('POST', '/__test/session', undefined, { name: 'ben' })).json;
  const pod = (await s.call('POST', '/v1/pods', olivia.token, { name: 'p' })).json;
  const inv = (await s.call('POST', `/v1/pods/${pod.id}/invites`, olivia.token, {})).json;
  await s.call('POST', `/v1/invites/${inv.code}/accept`, ben.token);
  const comp = (await s.call('POST', '/v1/computers', ben.token, { name: 'mac', os: 'darwin', arch: 'arm64' })).json;
  const agent = (await s.call('POST', `/v1/pods/${pod.id}/agents`, comp.token, { harness: 'fake', displayName: 'bee' })).json;
  const frames: string[] = [];
  const rt = s.hub.pods.get(pod.id)!;
  rt.conns.add(fakeConn({ kind: 'computer', computerId: comp.computerId, owner: ben.user }, frames));
  const made = await rt.exec({ type: 'task_create', caller: { kind: 'user', userId: olivia.user.id }, taskId: 'tsk_00000000000000000000000001', title: 't', kind: 'chore', assignee: agent.agentId, wakeId: 'wak_00000000000000000000000001' });
  expect(made?.ok).toBe(true);
  const wake = JSON.parse(frames.find((f) => f.includes('"run.wake"')) as string).data;
  return { ...s, olivia, ben, pod, comp, agent, wake, rt };
}

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
