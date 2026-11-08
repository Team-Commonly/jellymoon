import { describe, expect, jest, test } from 'bun:test';
import type { Actor } from '../../gen/ts/protocol';
import { buildApp } from '../src/app';
import type { Db } from '../src/db';
import { openDb } from '../src/db';
import { Hub } from '../src/hub';
import { PodRuntime } from '../src/pod';
import { eventsAfter } from '../src/store';
import { caller, fakeConn, flaky, setup, world } from './world';

const TASK = 'tsk_00000000000000000000000001';

async function until(check: () => Promise<boolean>, ms = 3000): Promise<boolean> {
  for (const end = Date.now() + ms; Date.now() < end; await new Promise((r) => setTimeout(r, 25))) if (await check()) return true;
  return false;
}

async function claimed() {
  const f = flaky(await openDb({}));
  const w = await world(f.db);
  const run = (await w.call('POST', '/v1/runs', w.comp.token, { wakeId: w.wake.wakeId })).json;
  const claim = await w.call('POST', `/v1/pods/${w.pod.id}/tasks/${TASK}/claim`, run.runToken, {});
  return { ...w, f, run, lease: Date.parse(claim.json.leaseUntil) };
}

describe('a database that fails', () => {
  test('a tick that keeps failing is retried on a back-off, not in a loop, and releases the claim once the database recovers', async () => {
    const w = await claimed();
    const t0 = Date.now();
    w.hub.now = () => w.lease - 150 + (Date.now() - t0);
    w.f.ctl.down = true;
    w.hub.rearm();
    await new Promise((r) => setTimeout(r, 1600));
    expect(w.f.ctl.refused >= 2 && w.f.ctl.refused <= 6).toBe(true);
    w.f.ctl.down = false;
    const released = await until(async () => (await eventsAfter(w.db, w.pod.id, 0, 100)).some((e) => e.type === 'claim.released'));
    expect(released).toBe(true);
  });

  test('a write that fails means no fan-out and the pod in memory is unchanged', async () => {
    const f = flaky(await openDb({}));
    const w = await world(f.db);
    const frames: string[] = [];
    w.rt.conns.add(fakeConn({ kind: 'user', actor: w.olivia.user }, frames));
    const before = w.rt.state;
    frames.length = 0;
    const cmd = { type: 'task_create' as const, caller: { kind: 'user' as const, userId: w.olivia.user.id }, taskId: 'tsk_00000000000000000000000002', title: 'u', kind: 'chore' as const, wakeId: 'wak_00000000000000000000000002' };
    f.ctl.fail = true;
    let err: unknown;
    try { await w.rt.exec(cmd); } catch (e) { err = e; }
    expect(err instanceof Error).toBe(true);
    expect(frames.length).toBe(0);
    expect(w.rt.state === before).toBe(true);
    expect((await w.rt.exec(cmd))?.ok).toBe(true);
    expect(frames.length > 0).toBe(true);
  });

  test('a run read persists its renewal: after a restart the lease is the renewal plus 60 s', async () => {
    const w = await claimed();
    const later = Date.now() + 30000;
    w.hub.now = () => later;
    expect((await w.call('GET', `/v1/pods/${w.pod.id}/me`, w.run.runToken)).status).toBe(200);
    const again = caller(buildApp(await Hub.restore(w.db), { local: true }));
    const tasks = (await again('GET', `/v1/pods/${w.pod.id}/tasks`, w.olivia.token)).json.tasks;
    expect(Date.parse(tasks[0].claim.leaseUntil)).toBe(later + 60000);
  });

  test('a restart over a lease past its deadline releases the claim with no command', async () => {
    const w = await claimed();
    const [row] = await w.db.rows<{ state: string }>('select state from pods where id = $1', [w.pod.id]);
    const state = JSON.parse(row.state);
    state.claims[TASK].leaseUntil = 1000;
    await w.db.rows('update pods set state = $1 where id = $2', [JSON.stringify(state), w.pod.id]);
    const again = await Hub.restore(w.db);
    const log = await eventsAfter(w.db, w.pod.id, 0, 100);
    expect(log.filter((e) => e.type === 'claim.released').length).toBe(1);
    expect(again.pods.get(w.pod.id)?.state.claims[TASK] === undefined).toBe(true);
  });

});

describe('a tick that fails', () => {
  test('a failed tick leaves the timer armed, so the next deadline still fires and releases the claim', async () => {
    const w = await claimed();
    // The clock and the timers are the test's, so host load cannot reorder anything: it waits on the hub's next rearm, never on the time.
    jest.useFakeTimers();
    try {
      let clock = w.lease - 150;
      w.hub.now = () => clock;
      const retries: number[] = [];
      let seen = 0;
      let wake = () => {};
      const arm = w.hub.rearm.bind(w.hub);
      w.hub.rearm = (least = 0) => { arm(least); if (least > 0) retries.push(least); seen += 1; wake(); };
      const released = async () => (await eventsAfter(w.db, w.pod.id, 0, 100)).some((e) => e.type === 'claim.released');
      const settle = async (done: () => boolean | Promise<boolean>) => {
        for (let n = seen; !(await done()); n = seen) if (seen === n) await new Promise<void>((r) => { wake = r; });
      };
      w.f.ctl.fail = true;
      w.hub.rearm();
      clock = w.lease;
      jest.advanceTimersByTime(150);
      await settle(() => retries.length > 0);
      expect(await released()).toBe(false);
      jest.advanceTimersByTime(1000);
      await settle(released);
      expect(await released()).toBe(true);
    } finally {
      jest.useRealTimers();
    }
  });
});

describe('a replay that fails', () => {
  test('the subscription ends: a live event that landed meanwhile is not sent, and a later subscribe replays from its own sinceSeq', async () => {
    const base = await openDb({});
    const gate = { read: null as null | (() => void), failReads: true };
    const db: Db = { ...base, rows: async (sql, params) => {
      if (!gate.failReads || !sql.includes('from events')) return base.rows(sql, params);
      await new Promise<void>((r) => { gate.read = r; });
      throw new Error('db down');
    } };
    const w = await world(db);
    const frames: string[] = [];
    const conn = fakeConn({ kind: 'user', actor: w.olivia.user }, frames);
    conn.subscribed = false;
    w.rt.conns.add(conn);
    const pending = w.rt.subscribe(conn, 0).then(() => 'ok', (e: unknown) => String(e));
    await until(async () => gate.read !== null);
    const made = await w.rt.exec({ type: 'task_create', caller: { kind: 'user', userId: w.olivia.user.id }, taskId: 'tsk_00000000000000000000000002', title: 'live', kind: 'chore', wakeId: 'wak_00000000000000000000000002' });
    expect(made?.ok).toBe(true);
    gate.read?.();
    expect((await pending).includes('db down')).toBe(true);
    expect([frames.length, conn.subscribed, conn.buf.length]).toEqual([0, false, 0]);
    gate.failReads = false;
    await w.rt.subscribe(conn, 0);
    expect(frames.some((f) => f.includes('"live"'))).toBe(true);
  });
});

describe('a database that fails mid-write', () => {
  test('a failed pod_create writes no row', async () => {
    const s = await setup();
    const owner = (await s.call('POST', '/__test/session', undefined, { name: 'o' })).json.user as Actor;
    const { pod } = await s.hub.createPod(owner, 'p');
    const dup = new PodRuntime('pod_dup', pod.state, pod.meta, { db: s.db, now: s.hub.now, rearm: () => undefined }, false);
    expect((await dup.exec({ type: 'pod_create', ownerId: owner.id, ownerName: owner.displayName }))?.ok).toBe(false);
    expect((await s.db.rows('select id from pods where id = $1', ['pod_dup'])).length).toBe(0);
  });

  test('a user and its token are written together, and so are a computer and its token', async () => {
    const f = flaky(await openDb({}));
    const s = await setup(f.db);
    f.ctl.fail = true;
    let err: unknown;
    try { await s.hub.createUser('ghost'); } catch (e) { err = e; }
    expect(err instanceof Error).toBe(true);
    expect((await s.db.rows('select id from users')).length).toBe(0);
    const u = await s.hub.createUser('real');
    f.ctl.fail = true;
    err = undefined;
    try { await s.hub.createComputer(u.user.id, 'mac', 'darwin', 'arm64'); } catch (e) { err = e; }
    expect(err instanceof Error).toBe(true);
    expect((await s.db.rows('select id from computers')).length).toBe(0);
  });
});
