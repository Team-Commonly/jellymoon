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

test('deny has approve\'s rate limit, none: with the start limit at 1 it still answers every time', async () => {
  const s = await setup(undefined, { local: true, startLimit: 1 });
  const tok = await signedIn(s);
  const d = await start(s);
  expect((await s.call('POST', '/v1/auth/device')).status).toBe(429);
  for (let i = 0; i < 40; i++) expect((await send(s, 'deny', d.userCode, by(tok))).status).toBe(200);
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
