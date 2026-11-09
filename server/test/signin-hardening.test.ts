import { expect, test } from 'bun:test';
import { forwardedFor } from '../src/auth';
import { appOptions, startupRefusal } from '../src/config';
import { openDb } from '../src/db';
import { Hub } from '../src/hub';
import { hashToken, newToken } from '../src/ids';
import { saveToken } from '../src/store';
import { websocket } from 'hono/bun';
import { setup } from './world';

// The sign-in hardening follow-ups (ADR 0008, "Hardening"): TRUST_PROXY behind TLS, user tokens with no expiry, hub.sockets.
const hosted = { DATABASE_URL: 'postgres://db.example/jm', PUBLIC_URL: 'https://jm.example' };

test('behind TLS (an https PUBLIC_URL outside ENV=local) the server will not start until TRUST_PROXY says how many proxies', () => {
  for (const v of [undefined, '', 'yes', '-1', '1.5']) expect(startupRefusal({ ...hosted, TRUST_PROXY: v })?.includes('TRUST_PROXY')).toBe(true);
  for (const v of ['0', '1', '2']) expect(startupRefusal({ ...hosted, TRUST_PROXY: v })).toBe(undefined);
  expect(startupRefusal({ ENV: 'local', PUBLIC_URL: 'https://jm.example' })).toBe(undefined);
  expect(startupRefusal({ ENV: 'local', PUBLIC_URL: 'http://127.0.0.1:8787' })).toBe(undefined);
  expect([{}, { TRUST_PROXY: '0' }, { TRUST_PROXY: '1' }, { TRUST_PROXY: '2' }].map((e) => appOptions(e).trustProxy)).toEqual([0, 0, 1, 2]);
});

test('PUBLIC_URL\'s scheme is read in any case, and outside ENV=local it must be https, the origin the server holds a browser to', () => {
  for (const u of ['HTTPS://jm.example', 'Https://jm.example/']) expect(startupRefusal({ ...hosted, PUBLIC_URL: u })?.includes('TRUST_PROXY')).toBe(true);
  expect(startupRefusal({ ...hosted, PUBLIC_URL: 'HTTPS://jm.example', TRUST_PROXY: '1' })).toBe(undefined);
  for (const u of ['http://10.0.0.5:8787', 'HTTP://jm.example', 'jm.example', 'ftp://jm.example']) {
    expect(startupRefusal({ ...hosted, PUBLIC_URL: u, TRUST_PROXY: '1' })?.startsWith('PUBLIC_URL must be an https URL')).toBe(true);
  }
});

test('TRUST_PROXY=<n> reads the n-th X-Forwarded-For entry from the right, the one the outermost trusted proxy wrote', () => {
  expect(forwardedFor('203.0.113.9', 1)).toBe('203.0.113.9');
  expect(forwardedFor('6.6.6.6, 203.0.113.9', 1)).toBe('203.0.113.9');
  expect(forwardedFor('6.6.6.6, 203.0.113.9, 10.0.0.2', 2)).toBe('203.0.113.9');
  for (const [h, n] of [['203.0.113.9', 2], ['6.6.6.6', 0], [undefined, 1], ['', 1]] as const) expect(forwardedFor(h, n)).toBe(undefined);
});

test('a user token with no expiry (minted before expiry existed) is no caller; computer tokens keep none', async () => {
  const hub = await Hub.restore(await openDb({}));
  const { user } = await hub.createUser('legacy');
  const { token: jmc } = await hub.createComputer(user.id, 'mac', 'darwin', 'arm64');
  const legacy = newToken('jmu_');
  await hub.db.tx((t) => saveToken(t, hashToken(legacy), 'user', user.id));
  expect(await hub.authenticate(`Bearer ${legacy}`)).toBe(null);
  expect((await hub.authenticate(`Bearer ${jmc}`))?.kind).toBe('computer');
});

/**
 * Opens two real pod sockets with one token. `between` runs after the upgrade authenticated the token and before the socket
 * opened (a logout landing there). Resolves with how each socket ended: its first frame's type, or its close code.
 */
async function sockets(between?: (s: Awaited<ReturnType<typeof setup>>, token: string) => Promise<unknown>) {
  const s = await setup();
  const token = (await s.call('POST', '/__test/session', undefined, { name: 'olivia' })).json.token as string;
  const podId = (await s.call('POST', '/v1/pods', token, { name: 'p' })).json.id as string;
  const srv = Bun.serve({ port: 0, fetch: s.app.fetch, websocket });
  const auth = s.hub.authenticate.bind(s.hub);
  let armed = between !== undefined;
  s.hub.authenticate = async (h) => { const who = await auth(h); if (armed) { armed = false; await between?.(s, token); } return who; };
  const dial = (): { ws: WebSocket; first: Promise<string> } => {
    const ws = new WebSocket(`ws://127.0.0.1:${srv.port}/v1/pods/${podId}/ws`, { headers: { Authorization: `Bearer ${token}` } } as unknown as string[]);
    const first = new Promise<string>((done) => {
      ws.onmessage = (e) => done(JSON.parse(String(e.data)).type);
      ws.onclose = (e) => done(`close ${e.code}`);
    });
    return { ws, first };
  };
  return { s, srv, hash: hashToken(token), dial };
}

const sleep = (ms: number): Promise<void> => new Promise((done) => setTimeout(done, ms));
const settle = async (until: () => boolean): Promise<void> => { for (let i = 0; i < 100 && !until(); i++) await sleep(10); };

test('a token hash leaves hub.sockets when its last real socket closes', async () => {
  const { s, srv, hash, dial } = await sockets();
  const [a, b] = [dial(), dial()];
  expect([await a.first, await b.first]).toEqual(['hello', 'hello']);
  expect(s.hub.sockets.get(hash)?.size).toBe(2);
  a.ws.close();
  await settle(() => s.hub.sockets.get(hash)?.size === 1);
  expect(s.hub.sockets.get(hash)?.size).toBe(1);
  b.ws.close();
  await settle(() => !s.hub.sockets.has(hash));
  expect(s.hub.sockets.has(hash)).toBe(false);
  srv.stop(true);
});

test('a logout between the upgrade\'s authentication and the socket opening leaves no socket open', async () => {
  const logout = (x: Awaited<ReturnType<typeof setup>>, token: string) => x.call('POST', '/v1/auth/logout', token);
  const { s, srv, hash, dial } = await sockets(logout);
  const a = dial();
  expect(await Promise.race([a.first, sleep(1000).then(() => 'still open')])).toBe('close 1008');
  await settle(() => !s.hub.sockets.has(hash));
  expect(s.hub.sockets.has(hash)).toBe(false);
  srv.stop(true);
});
