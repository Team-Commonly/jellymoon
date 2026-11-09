import { expect, test } from 'bun:test';
import { authRoute } from '../src/auth';
import { createUser } from '../src/directory';
import type { Env } from '../src/env';
import { podNamespace, sqliteD1, workerFile } from './d1';

// POST /v1/auth/device/deny (ADR 0008, step 4) in the Worker, on a real SQLite behind the D1 calls: twin of
// server/test/device-deny.test.ts.
const json = { 'Content-Type': 'application/json' };
const ORIGIN = 'https://w.example';

async function world(limit = '30'): Promise<{ env: Env; tok: string }> {
  const env = { DB: sqliteD1(workerFile('schema.sql')).d1, POD: podNamespace(async () => Response.json({})), AUTH_START_LIMIT: limit, CLOCK_MS: 0 } as unknown as Env;
  return { env, tok: (await createUser(env.DB, 'olivia', 0)).token };
}

/** One request to a sign-in route at time `now`: the status and the raw body, to compare byte for byte. */
async function send(env: Env, path: string, body: unknown, headers: Record<string, string>, now = 0): Promise<{ status: number; body: string }> {
  const res = await authRoute(new Request(`${ORIGIN}${path}`, { method: 'POST', headers, body: body === undefined ? undefined : JSON.stringify(body) }), env, now);
  return { status: res?.status ?? -1, body: (await res?.text()) ?? '' };
}
const by = (token: string): Record<string, string> => ({ ...json, Authorization: `Bearer ${token}` });
const route = (env: Env, r: 'approve' | 'deny', userCode: string, headers: Record<string, string>, now = 0) => send(env, `/v1/auth/device/${r}`, { userCode }, headers, now);
const start = async (env: Env): Promise<{ deviceCode: string; userCode: string }> => JSON.parse((await send(env, '/v1/auth/device', undefined, {})).body);
const poll = async (env: Env, deviceCode: string, now = 5_000): Promise<{ status: number; code?: string; message?: string }> => {
  const r = await send(env, '/v1/auth/device/token', { deviceCode }, json, now);
  return { status: r.status, ...JSON.parse(r.body) };
};

test('deny makes every poll access_denied, and a second deny, in any case and without the hyphen, is still a success', async () => {
  const { env, tok } = await world();
  const d = await start(env);
  expect(await route(env, 'deny', d.userCode, by(tok))).toEqual({ status: 200, body: '{}' });
  for (const now of [5_000, 5_000]) expect(await poll(env, d.deviceCode, now)).toEqual({ status: 400, code: 'access_denied', message: 'The sign-in was refused in the browser.' });
  expect(await route(env, 'deny', d.userCode.toLowerCase().replace('-', ''), by(tok))).toEqual({ status: 200, body: '{}' });
});

test('approve after deny and deny after approve both get approve\'s used-code answer', async () => {
  const { env, tok } = await world();
  const approved = await start(env);
  expect((await route(env, 'approve', approved.userCode, by(tok))).status).toBe(200);
  const used = await route(env, 'approve', approved.userCode, by(tok));
  expect(used.status).toBe(410);
  expect(await route(env, 'deny', approved.userCode, by(tok))).toEqual(used);
  const denied = await start(env);
  expect((await route(env, 'deny', denied.userCode, by(tok))).status).toBe(200);
  expect(await route(env, 'approve', denied.userCode, by(tok))).toEqual(used);
  expect((await poll(env, denied.deviceCode)).code).toBe('access_denied');
});

test('deny of an unknown, exchanged or expired code answers what approve answers for it, byte for byte', async () => {
  const { env, tok } = await world();
  const exchanged = await start(env);
  await route(env, 'approve', exchanged.userCode, by(tok));
  expect((await poll(env, exchanged.deviceCode)).status).toBe(200);
  const expired = await start(env);
  const later = 901_000;
  const same = async (code: string, status: number, now: number) => {
    const a = await route(env, 'approve', code, by(tok), now);
    expect(a.status).toBe(status);
    expect(await route(env, 'deny', code, by(tok), now)).toEqual(a);
  };
  await same('BBBB-BBBB', 404, 0);
  await same(exchanged.userCode, 410, 0);
  await same(expired.userCode, 400, later);
  expect((await poll(env, expired.deviceCode, later)).code).toBe('expired_token');
});

test('deny has approve\'s rate limit, none: with the start limit at 1 it still answers every time', async () => {
  const { env, tok } = await world('1');
  const d = await start(env);
  expect((await send(env, '/v1/auth/device', undefined, {})).status).toBe(429);
  for (let i = 0; i < 40; i++) expect((await route(env, 'deny', d.userCode, by(tok))).status).toBe(200);
});

test('deny needs a person: no token, a bad Bearer, or a cookie write without JSON or the server\'s Origin is refused, and the code stays pending', async () => {
  const { env, tok } = await world();
  const d = await start(env);
  const cases: Array<[Record<string, string>, number]> = [
    [json, 401],
    [by('jmu_notarealtoken'), 401],
    [{ Cookie: `jm_session=${tok}`, Origin: ORIGIN }, 403],
    [{ ...json, Cookie: `jm_session=${tok}`, Origin: 'https://evil.example' }, 403],
    [{ ...json, Cookie: `jm_session=${tok}` }, 403],
  ];
  for (const [headers, status] of cases) expect((await route(env, 'deny', d.userCode, headers)).status).toBe(status);
  expect((await poll(env, d.deviceCode)).code).toBe('authorization_pending');
  expect((await route(env, 'deny', d.userCode, { ...json, Cookie: `jm_session=${tok}`, Origin: ORIGIN }, 1)).status).toBe(200);
});
