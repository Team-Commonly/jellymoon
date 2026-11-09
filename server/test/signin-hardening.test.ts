import { expect, test } from 'bun:test';
import { forwardedFor } from '../src/auth';
import { appOptions, startupRefusal } from '../src/config';
import { openDb } from '../src/db';
import { Hub } from '../src/hub';
import { hashToken, newToken } from '../src/ids';
import { saveToken } from '../src/store';

// The sign-in hardening follow-ups (ADR 0008, "Hardening"): TRUST_PROXY behind TLS, user tokens with no expiry, hub.sockets.
const hosted = { DATABASE_URL: 'postgres://db.example/jm', PUBLIC_URL: 'https://jm.example' };

test('behind TLS (an https PUBLIC_URL outside ENV=local) the server will not start until TRUST_PROXY says how many proxies', () => {
  for (const v of [undefined, '', 'yes', '-1', '1.5']) expect(startupRefusal({ ...hosted, TRUST_PROXY: v })?.includes('TRUST_PROXY')).toBe(true);
  for (const v of ['0', '1', '2']) expect(startupRefusal({ ...hosted, TRUST_PROXY: v })).toBe(undefined);
  expect(startupRefusal({ DATABASE_URL: 'postgres://db.example/jm', PUBLIC_URL: 'http://10.0.0.5:8787' })).toBe(undefined);
  expect(startupRefusal({ ENV: 'local', PUBLIC_URL: 'https://jm.example' })).toBe(undefined);
  expect([{}, { TRUST_PROXY: '0' }, { TRUST_PROXY: '1' }, { TRUST_PROXY: '2' }].map((e) => appOptions(e).trustProxy)).toEqual([0, 0, 1, 2]);
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

test('a token hash leaves hub.sockets when its last socket closes', async () => {
  const hub = await Hub.restore(await openDb({}));
  const a = (): void => undefined;
  const b = (): void => undefined;
  hub.addSocket('h', a);
  hub.addSocket('h', b);
  hub.dropSocket('h', a);
  expect(hub.sockets.get('h')?.size).toBe(1);
  hub.dropSocket('h', b);
  expect(hub.sockets.has('h')).toBe(false);
});
