import { webDirProblem } from './web';

// What the process must refuse at startup, kept apart from main.ts so a test can pin it.
import { keyRing, makeVault } from './vault';
import type { Vault } from './vault';

export function startupRefusal(env: Record<string, string | undefined>): string | undefined {
  const local = env.ENV === 'local';
  if (env.JM_WEB_DIR) {
    const web = webDirProblem(env.JM_WEB_DIR);
    if (web !== undefined) return web;
  }
  if (env.GITHUB_URL && !local && !env.GITHUB_URL.startsWith('https://')) return 'GITHUB_URL must be https; an http override is for ENV=local only.';
  // The OAuth redirect and the browser session's own Origin come from PUBLIC_URL; a request's Host must not stand in for it.
  if (env.GITHUB_CLIENT_ID && !local && !env.PUBLIC_URL?.trim()) return 'PUBLIC_URL is required when GITHUB_CLIENT_ID is set outside ENV=local: set it to the public base URL.';
  // JM_VAULT_KEY is optional (without it the connector routes answer vault_unconfigured), but a malformed one is refused.
  const ring = keyRing(env);
  if (typeof ring === 'string') return ring;
  if (env.DATABASE_URL === undefined || env.DATABASE_URL === '') {
    return local ? undefined : 'DATABASE_URL is required. The in-process Postgres is for local runs only (ENV=local).';
  }
  // ENV=local opens /__test/session and /__test/clock with no bearer. Against a real database that is an open door, so it is
  // refused; only a throwaway schema (the e2e suite sets JM_PG_SCHEMA per server) may combine the two.
  return local && (env.JM_PG_SCHEMA === undefined || env.JM_PG_SCHEMA === '')
    ? 'ENV=local is refused with DATABASE_URL: it opens the test routes with no bearer. Unset ENV, or confine the server to a throwaway schema with JM_PG_SCHEMA.'
    : undefined;
}

/**
 * What buildApp is given, from the environment. Test routes open only for ENV=local or a configured TEST_BEARER; JM_WEB_DIR is
 * the built web client to serve. Kept apart from main.ts so a test can start the app as production does. AUTH_START_LIMIT is the
 * per-minute start limit; TRUST_PROXY=1 reads the client address from X-Forwarded-For (only behind a proxy that sets it).
 */
export function appOptions(env: Record<string, string | undefined>): { publicUrl: string | undefined; local: boolean; testBearer: string | undefined; startLimit: number | undefined; trustProxy: boolean; webDir: string | undefined } {
  const limit = Number(env.AUTH_START_LIMIT);
  return { publicUrl: env.PUBLIC_URL, local: env.ENV === 'local', testBearer: env.TEST_BEARER, startLimit: limit > 0 ? limit : undefined, trustProxy: env.TRUST_PROXY === '1', webDir: env.JM_WEB_DIR || undefined };
}

/** The connector vault from JM_VAULT_KEY (and JM_VAULT_OLD_KEYS), or undefined when no key is set. Call after startupRefusal. */
export async function vaultOf(env: Record<string, string | undefined>): Promise<Vault | undefined> {
  const ring = keyRing(env);
  return typeof ring === 'string' || ring.length === 0 ? undefined : makeVault(ring);
}
