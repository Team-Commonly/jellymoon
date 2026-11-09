import { expect, test } from 'bun:test';
import { setup } from './world';

// POST /v1/auth/device/deny (ADR 0008, step 4) on the pod-state server: a person refuses a code they did not start.
type S = Awaited<ReturnType<typeof setup>>;
const json = { 'Content-Type': 'application/json' };

async function signedIn(s: S): Promise<string> {
  return (await s.call('POST', '/__test/session', undefined, { name: 'olivia' })).json.token as string;
}

/** One device route by Bearer (or the headers given): the status and the raw body, to compare byte for byte. */
async function send(s: S, route: 'approve' | 'deny', userCode: string, headers: Record<string, string>): Promise<{ status: number; body: string }> {
  const res = await s.app.fetch(new Request(`http://x/v1/auth/device/${route}`, { method: 'POST', headers, body: JSON.stringify({ userCode }) }));
  return { status: res.status, body: await res.text() };
}
const by = (token: string): Record<string, string> => ({ ...json, Authorization: `Bearer ${token}` });

async function start(s: S): Promise<{ deviceCode: string; userCode: string }> {
  return (await s.call('POST', '/v1/auth/device')).json as { deviceCode: string; userCode: string };
}
const poll = (s: S, deviceCode: string) => s.call('POST', '/v1/auth/device/token', undefined, { deviceCode });

test('deny makes every poll access_denied, and a second deny, in any case and without the hyphen, is still a success', async () => {
  const s = await setup();
  const tok = await signedIn(s);
  const d = await start(s);
  expect(await send(s, 'deny', d.userCode, by(tok))).toEqual({ status: 200, body: '{}' });
  for (let i = 0; i < 2; i++) expect(await poll(s, d.deviceCode)).toEqual({ status: 400, json: { code: 'access_denied', message: 'The sign-in was refused in the browser.' } });
  expect(await send(s, 'deny', d.userCode.toLowerCase().replace('-', ''), by(tok))).toEqual({ status: 200, body: '{}' });
});

test('approve after deny and deny after approve both get approve\'s used-code answer', async () => {
  const s = await setup();
  const tok = await signedIn(s);
  const approved = await start(s);
  expect((await send(s, 'approve', approved.userCode, by(tok))).status).toBe(200);
  const used = await send(s, 'approve', approved.userCode, by(tok));
  expect(used.status).toBe(410);
  expect(await send(s, 'deny', approved.userCode, by(tok))).toEqual(used);
  const denied = await start(s);
  expect((await send(s, 'deny', denied.userCode, by(tok))).status).toBe(200);
  expect(await send(s, 'approve', denied.userCode, by(tok))).toEqual(used);
  expect((await poll(s, denied.deviceCode)).json.code).toBe('access_denied');
});

test('deny of an unknown, exchanged or expired code answers what approve answers for it, byte for byte', async () => {
  const s = await setup();
  const tok = await signedIn(s);
  const exchanged = await start(s);
  await send(s, 'approve', exchanged.userCode, by(tok));
  expect((await poll(s, exchanged.deviceCode)).status).toBe(200);
  const expired = await start(s);
  const same = async (code: string, status: number) => {
    const a = await send(s, 'approve', code, by(tok));
    expect(a.status).toBe(status);
    expect(await send(s, 'deny', code, by(tok))).toEqual(a);
  };
  await same('BBBB-BBBB', 404);
  await same(exchanged.userCode, 410);
  await s.hub.setClock(s.hub.now() + 901_000);
  await same(expired.userCode, 400);
  expect((await poll(s, expired.deviceCode)).json.code).toBe('expired_token');
});

// A clock at the start of a ten-minute window, so a test's requests share one window and one minute.
const T0 = 1_800_000_000_000;

/** A signed-in person by name, on a server whose clock is fixed at T0 (sessions are minted after the clock, so they are live). */
async function person(s: S, name: string): Promise<string> {
  return (await s.call('POST', '/__test/session', undefined, { name })).json.token as string;
}
const limited = (r: { status: number; body: string }) => r.status === 429 && JSON.parse(r.body).code === 'rate_limited';

test('a person\'s 11th failed code lookup in ten minutes is rate_limited, on approve and on deny alike, and a success does not count', async () => {
  const s = await setup();
  await s.hub.setClock(T0);
  const tok = await person(s, 'guesser');
  const d = await start(s);
  for (let i = 0; i < 12; i++) expect((await send(s, 'deny', d.userCode, by(tok))).status).toBe(200);
  for (let i = 0; i < 10; i++) expect((await send(s, i % 2 === 0 ? 'approve' : 'deny', 'BBBB-BBBB', by(tok))).status).toBe(404);
  for (const route of ['approve', 'deny'] as const) {
    const res = await s.app.fetch(new Request(`http://x/v1/auth/device/${route}`, { method: 'POST', headers: by(tok), body: JSON.stringify({ userCode: 'BBBB-BBBB' }) }));
    expect(res.status).toBe(429);
    expect((await res.json()).code).toBe('rate_limited');
    expect(res.headers.get('Retry-After')).toBe('600');
  }
});

test('over the limit even a valid code is refused, and the code stays pending; another person approves it; the window resets', async () => {
  const s = await setup();
  await s.hub.setClock(T0);
  const tok = await person(s, 'guesser');
  const other = await person(s, 'owner');
  const d = await start(s);
  for (let i = 0; i < 10; i++) await send(s, 'approve', 'BBBB-BBBB', by(tok));
  expect(limited(await send(s, 'approve', d.userCode, by(tok)))).toBe(true);
  expect(limited(await send(s, 'deny', d.userCode, by(tok)))).toBe(true);
  expect((await poll(s, d.deviceCode)).json.code).toBe('authorization_pending');
  expect(await send(s, 'approve', d.userCode, by(other))).toEqual({ status: 200, body: '{}' });
  await s.hub.setClock(T0 + 599_000);
  expect(limited(await send(s, 'deny', 'BBBB-BBBB', by(tok)))).toBe(true);
  await s.hub.setClock(T0 + 600_000);
  expect((await send(s, 'deny', 'BBBB-BBBB', by(tok))).status).toBe(404);
});

test('a lookup whose clock reads an older window than the counter holds does not restart the newer count', async () => {
  const s = await setup();
  await s.hub.setClock(T0 + 600_000);
  const tok = await person(s, 'guesser');
  for (let i = 0; i < 10; i++) expect((await send(s, 'deny', 'BBBB-BBBB', by(tok))).status).toBe(404);
  await s.hub.setClock(T0 + 599_000);
  expect(limited(await send(s, 'deny', 'BBBB-BBBB', by(tok)))).toBe(true);
  await s.hub.setClock(T0 + 600_000);
  expect(limited(await send(s, 'deny', 'BBBB-BBBB', by(tok)))).toBe(true);
});

test('failed lookups count in the address\'s start limit, so many accounts share it, and a start stops with it', async () => {
  const s = await setup(undefined, { local: true, startLimit: 5 });
  await s.hub.setClock(T0);
  const [a, b] = [await person(s, 'a'), await person(s, 'b')];
  const d = await start(s);
  for (let i = 0; i < 4; i++) expect((await send(s, 'deny', 'BBBB-BBBB', by(a))).status).toBe(404);
  const res = await s.app.fetch(new Request('http://x/v1/auth/device/approve', { method: 'POST', headers: by(b), body: JSON.stringify({ userCode: 'BBBB-BBBB' }) }));
  expect(res.status).toBe(429);
  expect(res.headers.get('Retry-After')).toBe('60');
  expect((await s.call('POST', '/v1/auth/device')).status).toBe(429);
  expect(limited(await send(s, 'approve', d.userCode, by(b)))).toBe(true);
  await s.hub.setClock(T0 + 60_000);
  expect(await send(s, 'approve', d.userCode, by(b))).toEqual({ status: 200, body: '{}' });
});

test('deny needs a person: no token, a bad Bearer, or a cookie write without JSON or the server\'s Origin is refused, and the code stays pending', async () => {
  const s = await setup();
  const tok = await signedIn(s);
  const d = await start(s);
  const cases: Array<[Record<string, string>, number]> = [
    [json, 401],
    [by('jmu_notarealtoken'), 401],
    [{ Cookie: `jm_session=${tok}`, Origin: 'http://x' }, 403],
    [{ ...json, Cookie: `jm_session=${tok}`, Origin: 'https://evil.example' }, 403],
    [{ ...json, Cookie: `jm_session=${tok}` }, 403],
  ];
  for (const [headers, status] of cases) expect((await send(s, 'deny', d.userCode, headers)).status).toBe(status);
  expect((await poll(s, d.deviceCode)).json.code).toBe('authorization_pending');
  expect((await send(s, 'deny', d.userCode, { ...json, Cookie: `jm_session=${tok}`, Origin: 'http://x' })).status).toBe(200);
});
