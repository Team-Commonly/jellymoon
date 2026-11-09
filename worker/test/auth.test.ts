import { expect, test } from 'bun:test';
import { authRoute, fromGitHub, GITHUB_TIMEOUT_MS } from '../src/auth';
import type { Env } from '../src/env';

/** A fetch that answers the token exchange (when `exchange`) and otherwise never answers, until its signal aborts. */
function hangingFetch(exchange: boolean): typeof fetch {
  return ((url: string, init?: RequestInit) => (exchange && String(url).endsWith('/access_token') ? Promise.resolve(Response.json({ access_token: 'gho_x' }))
    : new Promise((_, reject) => init?.signal?.addEventListener('abort', () => reject(init.signal?.reason))))) as typeof fetch;
}

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

test('outside ENV=local, GitHub sign-in without PUBLIC_URL is refused, naming it; the Worker has no startup hook to refuse at', async () => {
  for (const extra of [{}, { PUBLIC_URL: ' ' }]) {
    const env = { GITHUB_CLIENT_ID: 'i', GITHUB_CLIENT_SECRET: 's', ...extra } as unknown as Env;
    for (const path of ['/v1/auth/github/start?next=/', '/v1/auth/github/callback?state=x&code=y']) {
      const res = await authRoute(new Request(`https://w.example${path}`), env, 0);
      const body = (await res?.json()) as { code: string; message: string };
      expect([res?.status, body.code, body.message.includes('PUBLIC_URL')]).toEqual([500, 'internal', true]);
    }
  }
});
