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

test('a token check that fails when the socket opens closes the socket rather than leaving it live', async () => {
  const fail = async (x: Awaited<ReturnType<typeof setup>>) => {
    const rows = x.hub.db.rows.bind(x.hub.db);
    let once = true;
    x.hub.db.rows = (async (sql: string, args?: unknown[]) => {
      if (once && sql.includes('from tokens where hash')) { once = false; throw new Error('db down'); }
      return rows(sql, args);
    }) as typeof x.hub.db.rows;
  };
  const { srv, dial } = await sockets(fail);
  const a = dial();
  const seen: string[] = [];
  a.ws.onmessage = (e) => seen.push(JSON.parse(String(e.data)).type);
  const closed = new Promise<number>((done) => { a.ws.onclose = (e) => done(e.code); });
  expect(await Promise.race([closed, sleep(1000).then(() => -1)])).toBe(1011);
  expect(seen).not.toContain('hello');
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

test('a socket that throws after the pod attached it is closed 1011 and leaves nothing attached in the pod', async () => {
  const boom = async (x: Awaited<ReturnType<typeof setup>>) => {
    for (const pod of x.hub.pods.values()) {
      const attach = pod.attach.bind(pod);
      let once = true;
      pod.attach = async (c) => { const r = await attach(c); if (once) { once = false; throw new Error('hello failed'); } return r; };
    }
  };
  const { s, srv, dial } = await sockets(boom);
  const a = dial();
  const closed = new Promise<number>((done) => { a.ws.onclose = (e) => done(e.code); });
  expect(await Promise.race([closed, sleep(1000).then(() => -1)])).toBe(1011);
  const pod = [...s.hub.pods.values()][0];
  await settle(() => pod.conns.size === 0);
  expect(pod.conns.size).toBe(0);
  srv.stop(true);
});

test('logout with a dead session cookie clears it and revokes nothing; a dead Bearer alone stays 401', async () => {
  const s = await setup();
  const live = (await s.call('POST', '/__test/session', undefined, { name: 'olivia' })).json.token as string;
  const dead = (await s.call('POST', '/__test/session', undefined, { name: 'ben' })).json.token as string;
  expect((await s.call('POST', '/v1/auth/logout', dead)).status).toBe(200);
  const out = (headers: Record<string, string>) => s.app.fetch(new Request('http://x/v1/auth/logout', { method: 'POST', headers: { Origin: 'http://x', 'Content-Type': 'application/json', ...headers } }));
  const res = await out({ Cookie: `jm_session=${dead}` });
  expect([res.status, res.headers.get('Set-Cookie')?.includes('Max-Age=0')]).toEqual([200, true]);
  expect((await out({ Authorization: `Bearer ${dead}` })).status).toBe(401);
  expect((await s.call('GET', '/v1/me', live)).status).toBe(200);
});
