import { expect, test } from 'bun:test';
import { authRoute } from '../src/auth';
import type { Env } from '../src/env';
import { handle } from '../src/front';
import { hashToken } from '../src/ids';
import { internalRoute } from '../src/internal';
import type { Starter } from '../src/internal';
import { hello } from '../src/views';

// The pod's name in hello, the signed-out invite read and GET /v1/session, on the Worker. D1 and the pod object are small
// in-memory fakes; the routes and the pod's handlers are the real ones.
const TOKEN = 'jmu_' + 'a'.repeat(43);
const OWNER = { kind: 'human', id: 'usr_01ARZ3NDEKTSV4RRFFQ69G5FAV', displayName: 'olivia' } as const;

function harness() {
  const d1 = new Map<string, string>();
  const rows = new Map<string, { uses_left: number; expires_at: number; inviter: string | null }>();
  const clock = { now: 1000 };
  const pod = {
    id: 'pod_1', state: { members: [OWNER] }, meta: { name: 'Launch room' }, now: () => clock.now,
    sql: {
      exec(sql: string, ...a: unknown[]) {
        if (sql.startsWith('insert into invites')) rows.set(a[0] as string, { uses_left: a[1] as number, expires_at: a[2] as number, inviter: a[3] as string });
        if (sql.startsWith('update invites')) rows.get(a[0] as string)!.uses_left -= 1;
        const row = sql.startsWith('select') ? rows.get(a[0] as string) : undefined;
        return { toArray: () => (row === undefined ? [] : [{ ...row }]) };
      },
    },
    exec: (_cmd: unknown, o?: { pre?: (s: unknown) => unknown; extra?: () => void }) => o?.pre?.({ members: [] }) ?? (o?.extra?.(), { ok: true, result: null }),
  };
  const db = {
    prepare: (sql: string) => ({
      bind: (...a: unknown[]) => ({
        run: async () => { if (sql.startsWith('insert into invites')) d1.set(a[0] as string, a[1] as string); },
        first: async () => {
          if (sql.includes('from tokens')) return { id: OWNER.id, name: OWNER.displayName };
          if (sql.includes('from pods')) return { one: 1 };
          return d1.has(a[0] as string) ? { pod: d1.get(a[0] as string) } : null;
        },
      }),
    }),
  };
  const POD = { idFromName: (n: string) => n, get: () => ({ fetch: (url: string, init: RequestInit) => internalRoute(pod as unknown as Starter, new Request(url, init)) }) };
  return { env: { DB: db, POD, PUBLIC_URL: 'https://j.example' } as unknown as Env, rows, clock, pod };
}

const call = async (env: Env, method: string, path: string, body?: object) => {
  const res = await handle(new Request(`https://j.example${path}`, { method, headers: method === 'GET' ? {} : { Authorization: `Bearer ${TOKEN}` }, body: body && JSON.stringify(body) }), env);
  return { status: res.status, json: (await res.json()) as Record<string, unknown> };
};

test('hello carries the pod\'s name', () => {
  const h = harness();
  expect(hello(h.pod.state as never, { name: 'Launch room', createdAt: '', harness: {} }, OWNER).podName).toBe('Launch room');
});

test('an invite read needs no token and returns exactly the pod name, the inviter\'s name and the expiry, spending nothing', async () => {
  const h = harness();
  const inv = (await call(h.env, 'POST', '/v1/pods/pod_1/invites', { maxUses: 1 })).json;
  const before = JSON.stringify([...h.rows.values()]);
  for (let i = 0; i < 3; i++) {
    expect(await call(h.env, 'GET', `/v1/invites/${inv.code}`)).toEqual({ status: 200, json: { podName: 'Launch room', inviterName: 'olivia', expiresAt: inv.expiresAt } });
  }
  expect(JSON.stringify([...h.rows.values()])).toBe(before);
});

test('unknown, used, expired and inviter-less codes all get the same 404', async () => {
  const h = harness();
  const make = async (body: object) => (await call(h.env, 'POST', '/v1/pods/pod_1/invites', body)).json.code as string;
  const used = await make({ maxUses: 1 });
  expect((await call(h.env, 'POST', `/v1/invites/${used}/accept`, {})).status).toBe(200);
  const expiring = await make({ ttlSeconds: 60 });
  const old = await make({});
  h.rows.get(await hashToken(old))!.inviter = null; // an invite from before inviters were kept
  h.clock.now += 61_000;
  for (const c of [`inv_${'A'.repeat(22)}`, used, expiring, old]) {
    expect(await call(h.env, 'GET', `/v1/invites/${c}`)).toEqual({ status: 404, json: { code: 'not_found', message: 'No such invite.' } });
  }
});

test('GET /v1/session is 200 with the user signed in and 200 with none signed out', async () => {
  const h = harness();
  const session = async (tok?: string) => {
    const res = await authRoute(new Request('https://j.example/v1/session', { headers: tok ? { Authorization: `Bearer ${tok}` } : {} }), h.env, 0);
    return { status: res?.status, json: await res?.json() };
  };
  expect(await session()).toEqual({ status: 200, json: {} });
  expect(await session(TOKEN)).toEqual({ status: 200, json: { user: OWNER } });
  expect(await session('nope')).toEqual({ status: 200, json: {} });
});
