import { expect, test } from 'bun:test';
import { hashToken } from '../src/ids';
import { handle } from '../src/front';
import { internalRoute } from '../src/internal';
import type { Starter } from '../src/internal';
import type { Env } from '../src/env';

// A join code is a credential. These run the Worker's invite routes and the pod's invite handlers over small fakes of D1 and the
// pod object that record every value they are asked to store, so a stored code would show.
const TOKEN = 'jmu_' + 'a'.repeat(43);
const OWNER = 'usr_01ARZ3NDEKTSV4RRFFQ69G5FAV';

function harness(publicUrl?: string) {
  const d1 = new Map<string, string>(); // invite key -> pod
  const podRows = new Map<string, { uses_left: number; expires_at: number }>();
  const stored: unknown[] = [];
  const members = [{ id: OWNER }];
  const pod = {
    id: 'pod_1', state: { members }, meta: { name: 'p' }, now: () => 1000,
    sql: {
      exec(sql: string, ...args: unknown[]) {
        stored.push(...args);
        if (sql.startsWith('insert into invites')) podRows.set(args[0] as string, { uses_left: args[1] as number, expires_at: args[2] as number });
        const row = sql.startsWith('select') ? podRows.get(args[0] as string) : undefined;
        return { toArray: () => (row === undefined ? [] : [row]) };
      },
    },
    exec: (_cmd: unknown, o?: { pre?: (s: unknown) => unknown; extra?: () => void }) => {
      const early = o?.pre?.({ members: [] });
      if (early) return early;
      o?.extra?.();
      return { ok: true, result: null };
    },
  };
  const db = {
    prepare: (sql: string) => ({
      bind: (...args: unknown[]) => {
        stored.push(...args);
        return {
          run: async () => { if (sql.startsWith('insert into invites')) d1.set(args[0] as string, args[1] as string); },
          first: async () => {
            if (sql.includes('from tokens')) return { id: OWNER, name: 'olivia' };
            if (sql.includes('from pods')) return { one: 1 };
            return d1.has(args[0] as string) ? { pod: d1.get(args[0] as string) } : null;
          },
        };
      },
    }),
  };
  const POD = { idFromName: (n: string) => n, get: () => ({ fetch: (url: string, init: RequestInit) => internalRoute(pod as unknown as Starter, new Request(url, init)) }) };
  const env = { DB: db, POD, PUBLIC_URL: publicUrl } as unknown as Env;
  return { env, stored, d1, podRows };
}

const send = (env: Env, path: string, body: object): Promise<Response> =>
  handle(new Request(`http://127.0.0.1:8787${path}`, { method: 'POST', headers: { Authorization: `Bearer ${TOKEN}` }, body: JSON.stringify(body) }), env);

test('an invite code is stored only as its SHA-256, in D1 and in the pod, and accepting it still works', async () => {
  const h = harness('https://jellymoon.example');
  const made = await send(h.env, '/v1/pods/pod_1/invites', {});
  expect(made.status).toBe(200);
  const invite = (await made.json()) as { code: string; url: string };
  expect(/^inv_[0-9A-Za-z]{22}$/.test(invite.code)).toBe(true);
  expect(invite.url).toBe(`https://jellymoon.example/join/${invite.code}`);

  const hash = await hashToken(invite.code);
  expect([...h.d1.keys()]).toEqual([hash]);
  expect([...h.podRows.keys()]).toEqual([hash]);
  expect(h.stored.includes(invite.code)).toBe(false);

  const accepted = await send(h.env, `/v1/invites/${invite.code}/accept`, {});
  expect(accepted.status).toBe(200);
  expect((await send(h.env, `/v1/invites/${hash}/accept`, {})).status).toBe(404);
});

test('without PUBLIC_URL the join link uses the request origin, not a fixed host', async () => {
  const made = await send(harness().env, '/v1/pods/pod_1/invites', {});
  expect(((await made.json()) as { url: string }).url.startsWith('http://127.0.0.1:8787/join/inv_')).toBe(true);
});
