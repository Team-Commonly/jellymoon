import { describe, expect, test } from 'bun:test';
import { buildApp } from '../src/app';
import { openDb } from '../src/db';
import { Hub } from '../src/hub';
import type { Conn } from '../src/pod';
import { eventsAfter } from '../src/store';

type Json = Record<string, any>;

async function setup(opts: { local: boolean; testBearer?: string } = { local: true }) {
  const db = await openDb({});
  const hub = await Hub.restore(db);
  const app = buildApp(hub, opts);
  const call = async (method: string, path: string, token?: string, body?: unknown): Promise<{ status: number; json: Json }> => {
    const headers: Record<string, string> = token ? { Authorization: `Bearer ${token}` } : {};
    const res = await app.fetch(new Request(`http://x${path}`, { method, headers, body: body === undefined ? undefined : JSON.stringify(body) }));
    return { status: res.status, json: (await res.json()) as Json };
  };
  return { db, hub, call };
}

const conn = (who: Conn['who'], frames: string[]): Conn => ({ who, send: (t) => { frames.push(t); }, subscribed: true, replaying: false, buf: [], sent: 0 });

async function pod() {
  const s = await setup();
  const olivia = (await s.call('POST', '/__test/session', undefined, { name: 'olivia' })).json;
  const p = (await s.call('POST', '/v1/pods', olivia.token, { name: 'p' })).json;
  const comp = (await s.call('POST', '/v1/computers', olivia.token, { name: 'mac', os: 'darwin', arch: 'arm64' })).json;
  const agent = (await s.call('POST', `/v1/pods/${p.id}/agents`, comp.token, { harness: 'fake', displayName: 'bee' })).json;
  return { ...s, olivia, p, comp, agent, rt: s.hub.pods.get(p.id)! };
}

describe('server behaviour', () => {
  test('a computer socket marks its agents online, and the last close marks them offline', async () => {
    const w = await pod();
    const watcher: string[] = [];
    w.rt.conns.add(conn({ kind: 'user', actor: w.olivia.user }, watcher));
    const socket = conn({ kind: 'computer', computerId: w.comp.computerId, owner: w.olivia.user }, []);
    await w.rt.attach(socket);
    expect(Object.keys(w.rt.state.online ?? {}).includes(w.comp.computerId)).toBe(true);
    await w.rt.detach(socket);
    expect(Object.keys(w.rt.state.online ?? {}).includes(w.comp.computerId)).toBe(false);
    expect(watcher.map((f) => JSON.parse(f).data.state)).toEqual(['online', 'offline']);
  });

  test('a restart sets every computer the pod held online to offline', async () => {
    const w = await pod();
    await w.rt.attach(conn({ kind: 'computer', computerId: w.comp.computerId, owner: w.olivia.user }, []));
    const again = await Hub.restore(w.db);
    expect(Object.keys(again.pods.get(w.p.id)!.state.online ?? {})).toEqual([]);
  });

  test('evict reloads the pod from storage and keeps the sequence', async () => {
    const w = await pod();
    const seq = w.rt.state.seq;
    await w.rt.evict();
    expect(w.rt.state.seq).toBe(seq);
    const log = await eventsAfter(w.db, w.p.id, 0, 100);
    expect(log.length).toBe(seq);
  });

  test('a field the core cannot store is refused, not dropped', async () => {
    const w = await pod();
    expect((await w.call('POST', '/v1/pods', w.olivia.token, { name: 'x', surprise: 1 })).status).toBe(400);
    expect((await w.call('POST', '/v1/pods', w.olivia.token, {})).status).toBe(400);
  });

  test('an invite code is stored only as a hash', async () => {
    const w = await pod();
    const inv = (await w.call('POST', `/v1/pods/${w.p.id}/invites`, w.olivia.token, {})).json;
    const rows = await w.db.rows<{ hash: string }>('select hash from invites');
    expect(rows.length).toBe(1);
    expect(rows[0].hash === inv.code).toBe(false);
    expect(JSON.stringify(rows).includes(inv.code)).toBe(false);
  });

  test('test routes: open in local, bearer-gated with a bearer, absent with neither', async () => {
    const open = await setup({ local: true });
    expect((await open.call('POST', '/__test/session', undefined, { name: 'a' })).status).toBe(200);
    const gated = await setup({ local: false, testBearer: 'sekret' });
    expect((await gated.call('POST', '/__test/session', undefined, { name: 'a' })).status === 200).toBe(false);
    expect((await gated.call('POST', '/__test/session', 'sekret', { name: 'a' })).status).toBe(200);
    expect((await gated.call('POST', '/__test/clock', 'sekret', { now: 1 })).status).toBe(404);
    const none = await setup({ local: false });
    expect((await none.call('POST', '/__test/session', 'sekret', { name: 'a' })).status).toBe(404);
  });
});
