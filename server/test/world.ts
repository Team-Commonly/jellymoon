import { expect } from 'bun:test';
import { buildApp } from '../src/app';
import type { Db } from '../src/db';
import { openDb } from '../src/db';
import { Hub } from '../src/hub';
import type { Conn } from '../src/pod';

export type Json = Record<string, any>;

/** A Db whose next transaction rejects once when `ctl.fail` is set: a Postgres blip. */
export function flaky(db: Db): { db: Db; ctl: { fail: boolean } } {
  const ctl = { fail: false };
  const tx: Db['tx'] = (fn) => {
    if (!ctl.fail) return db.tx(fn);
    ctl.fail = false;
    return Promise.reject(new Error('db down'));
  };
  return { db: { ...db, tx }, ctl };
}

export function caller(app: ReturnType<typeof buildApp>) {
  return async (method: string, path: string, token?: string, body?: unknown): Promise<{ status: number; json: Json }> => {
    const headers: Record<string, string> = token ? { Authorization: `Bearer ${token}` } : {};
    const res = await app.fetch(new Request(`http://x${path}`, { method, headers, body: body === undefined ? undefined : JSON.stringify(body) }));
    return { status: res.status, json: (await res.json()) as Json };
  };
}

export async function setup(db?: Db, opts: { local: boolean; testBearer?: string } = { local: true }) {
  const use = db ?? (await openDb({}));
  const hub = await Hub.restore(use);
  const app = buildApp(hub, opts);
  return { db: use, hub, app, call: caller(app) };
}

export const fakeConn = (who: Conn['who'], frames: string[]): Conn => ({ who, send: (t) => { frames.push(t); }, subscribed: true, replaying: false, buf: [], sent: 0 });


/** olivia owns a pod; ben (a member via invite) owns a computer with one agent; a task assigned to the agent has woken it. */
export async function world(db?: Db) {
  const s = await setup(db);
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

