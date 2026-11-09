import { expect, test } from 'bun:test';
import { fromGitHub, GITHUB_TIMEOUT_MS, githubOf, safeNext } from '../src/auth';
import { startupRefusal } from '../src/config';
import { openDb } from '../src/db';
import { Hub } from '../src/hub';
import { newDevice, newState } from '../src/store';

/** A fetch that answers the token exchange (when `exchange`) and otherwise never answers, until its signal aborts. */
function hangingFetch(exchange: boolean): typeof fetch {
  return ((url: string, init?: RequestInit) => (exchange && String(url).endsWith('/access_token') ? Promise.resolve(Response.json({ access_token: 'gho_x' }))
    : new Promise((_, reject) => init?.signal?.addEventListener('abort', () => reject(init.signal?.reason))))) as typeof fetch;
}

test('next is a same-origin path or /', () => {
  for (const ok of ['/', '/device?code=BBBB-BBBB', '/a/b#c']) expect(safeNext(ok)).toBe(ok);
  for (const no of [undefined, '', 'https://evil.example/', '//evil.example', '/\\evil.example', 'javascript:x', 'evil', '/a\\b', '/a\r\nSet-Cookie: x', `/${'a'.repeat(512)}`]) {
    expect(safeNext(no)).toBe('/');
  }
});

test('GitHub endpoints: the defaults, an override in the Enterprise layout, none without a client', () => {
  expect(githubOf({ GITHUB_CLIENT_ID: 'i', GITHUB_CLIENT_SECRET: 's' })).toEqual({ web: 'https://github.com', api: 'https://api.github.com', id: 'i', secret: 's' });
  expect(githubOf({ GITHUB_CLIENT_ID: 'i', GITHUB_CLIENT_SECRET: 's', GITHUB_URL: 'https://ghe.example/' })?.api).toBe('https://ghe.example/api/v3');
  expect(githubOf({ GITHUB_CLIENT_ID: 'i' })).toBeNull();
});

test('an http GitHub override is refused outside ENV=local', () => {
  expect(startupRefusal({ DATABASE_URL: 'postgres://x', GITHUB_URL: 'http://ghe.example' })?.includes('GITHUB_URL')).toBe(true);
  expect(startupRefusal({ DATABASE_URL: 'postgres://x', GITHUB_URL: 'https://ghe.example' })).toBe(undefined);
  expect(startupRefusal({ ENV: 'local', GITHUB_URL: 'http://127.0.0.1:1' })).toBe(undefined);
});

test('a GitHub call that never answers gets upstream once it times out, on the exchange and on GET /user', async () => {
  const real = globalThis.fetch;
  const gh = { web: 'https://gh.example', api: 'https://gh.example/api/v3', id: 'i', secret: 's' };
  try {
    for (const exchange of [false, true]) {
      globalThis.fetch = hangingFetch(exchange);
      const t0 = Date.now();
      expect(await fromGitHub(gh, 'code', 'https://x/cb', 100)).toBe('upstream');
      expect(Date.now() - t0 < 2000).toBe(true);
    }
  } finally { globalThis.fetch = real; }
  expect(GITHUB_TIMEOUT_MS).toBe(10_000);
});

test('a GitHub client outside ENV=local needs PUBLIC_URL', () => {
  const gh = { DATABASE_URL: 'postgres://x', GITHUB_CLIENT_ID: 'i', GITHUB_CLIENT_SECRET: 's' };
  expect(startupRefusal(gh)?.includes('PUBLIC_URL')).toBe(true);
  expect(startupRefusal({ ...gh, PUBLIC_URL: ' ' })?.includes('PUBLIC_URL')).toBe(true);
  expect(startupRefusal({ ...gh, PUBLIC_URL: 'https://jm.example', TRUST_PROXY: '1' })).toBe(undefined);
  expect(startupRefusal({ ENV: 'local', GITHUB_CLIENT_ID: 'i', GITHUB_CLIENT_SECRET: 's' })).toBe(undefined);
});

test('boot sweeps device codes and OAuth states an hour past their expiry and keeps the rest', async () => {
  const db = await openDb({});
  await Hub.restore(db);
  const now = Date.now();
  await newDevice(db, 'd-old', 'u-old', now - 3_600_000 - 1000);
  await newDevice(db, 'd-expired', 'u-expired', now - 1000);
  await newState(db, 's-old', '/', now - 3_600_000 - 1000);
  await newState(db, 's-live', '/', now + 600_000);
  await Hub.restore(db);
  expect((await db.rows<{ hash: string }>('select hash from device_codes order by hash')).map((r) => r.hash)).toEqual(['d-expired']);
  expect((await db.rows<{ hash: string }>('select hash from oauth_states')).map((r) => r.hash)).toEqual(['s-live']);
});

test('six first sign-ins of one GitHub account at once make one user, and none fails', async () => {
  const hub = await Hub.restore(await openDb({}));
  const got = await Promise.all(Array.from({ length: 6 }, () => hub.githubUser('4300009', 'octo-race')));
  expect(new Set(got.map((a) => a.id)).size).toBe(1);
});
