import { expect, test } from 'bun:test';
import { authRoute } from '../src/auth';
import { addPod, authenticate, createComputer, createUser, mint } from '../src/directory';
import type { Env } from '../src/env';
import { handle } from '../src/front';
import { hashToken } from '../src/ids';
import { podNamespace, sqliteD1, workerFile } from './d1';

// The sign-in hardening follow-ups (ADR 0008, "Hardening"), on a real SQLite behind the D1 calls and a fake pod namespace.
const fresh = (): D1Database => sqliteD1(workerFile('schema.sql')).d1;
const noPods = podNamespace(async () => Response.json({}));

test('the start limit keys on CF-Connecting-IP and ignores X-Forwarded-For, which a caller writes', async () => {
  const env = { DB: fresh(), POD: noPods, AUTH_START_LIMIT: '2' } as unknown as Env;
  const start = async (ip: string, fwd: string): Promise<number | undefined> =>
    (await authRoute(new Request('https://w.example/v1/auth/device', { method: 'POST', headers: { 'CF-Connecting-IP': ip, 'X-Forwarded-For': fwd } }), env, 0))?.status;
  const got: Array<number | undefined> = [];
  for (let i = 1; i <= 3; i++) got.push(await start('198.51.100.7', `203.0.113.${i}`));
  expect(got).toEqual([200, 200, 429]);
  expect(await start('198.51.100.8', '203.0.113.1')).toBe(200);
});

test('a user token with no expiry (minted before expiry existed) is no caller; a computer token keeps none', async () => {
  const db = fresh();
  const { user } = await createUser(db, 'legacy', 0);
  const legacy = await mint(db, 'jmu_', 'user', user.id, null);
  const { token: jmc } = await createComputer(db, user.id, 'mac', 'darwin', 'arm64');
  expect(await authenticate(db, `Bearer ${legacy}`, 0)).toBe(null);
  expect((await authenticate(db, `Bearer ${jmc}`, 0))?.kind).toBe('computer');
});

test('logout revokes and clears the cookie even when a pod fails to close its sockets', async () => {
  const db = fresh();
  const { token } = await createUser(db, 'ana', 0);
  for (const pod of ['pod_down', 'pod_up']) await db.prepare('insert into token_pods (hash, pod_id) values (?, ?)').bind(await hashToken(token), pod).run();
  const closed: string[] = [];
  const POD = podNamespace(async (pod) => { if (pod === 'pod_down') throw new Error('the pod is down'); closed.push(pod); return Response.json({}); });
  const res = await authRoute(new Request('https://w.example/v1/auth/logout', { method: 'POST', headers: { Authorization: `Bearer ${token}` } }), { DB: db, POD, CLOCK_MS: 1 } as unknown as Env, 1);
  expect([res?.status, res?.headers.get('Set-Cookie')?.includes('Max-Age=0'), closed]).toEqual([200, true, ['pod_up']]);
  expect(await authenticate(db, `Bearer ${token}`, 1)).toBe(null);
});

/** Opens the pod socket through the Worker with a pod that answers `answer`; what token_pods held while the pod ran, and after. */
async function dial(answer: 'open' | 'refuse' | 'throw', before = false): Promise<{ during: number; after: number; status?: number }> {
  const db = fresh();
  const { token } = await createUser(db, 'ana', 0);
  await addPod(db, 'pod_1');
  const rows = async (): Promise<number> => (await db.prepare('select count(*) as n from token_pods').first<{ n: number }>())?.n ?? -1;
  if (before) await db.prepare('insert into token_pods (hash, pod_id) values (?, ?)').bind(await hashToken(token), 'pod_1').run();
  let during = -1;
  const POD = podNamespace(async () => {
    during = await rows();
    if (answer === 'throw') throw new Error('the pod is down');
    return answer === 'open' ? ({ status: 101 } as Response) : Response.json({ code: 'forbidden' }, { status: 403 });
  });
  const req = new Request('https://w.example/v1/pods/pod_1/ws', { headers: { Authorization: `Bearer ${token}`, Upgrade: 'websocket' } });
  const status = await handle(req, { DB: db, POD, CLOCK_MS: 0 } as unknown as Env).then((r) => r.status, () => undefined);
  return { during, after: await rows(), status };
}

test('the socket\'s token_pods row is written before the pod opens it, and dropped when the pod does not', async () => {
  expect(await dial('open')).toEqual({ during: 1, after: 1, status: 101 });
  expect(await dial('refuse')).toEqual({ during: 1, after: 0, status: 403 });
  expect(await dial('throw')).toEqual({ during: 1, after: 0, status: undefined });
  // A row another socket of the same token already needs stays when this one fails.
  expect(await dial('refuse', true)).toEqual({ during: 1, after: 1, status: 403 });
});
