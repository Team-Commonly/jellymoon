// What the process must refuse at startup, kept apart from main.ts so a test can pin it.
import { keyRing, makeVault } from './vault';
import type { Vault } from './vault';

export function startupRefusal(env: Record<string, string | undefined>): string | undefined {
  const local = env.ENV === 'local';
  // JM_VAULT_KEY is optional (without it the connector routes answer vault_unconfigured), but a malformed one is refused.
  const ring = keyRing(env);
  if (typeof ring === 'string') return ring;
  if (env.DATABASE_URL === undefined || env.DATABASE_URL === '') {
    return local ? undefined : 'DATABASE_URL is required. The in-process Postgres is for local runs only (ENV=local).';
  }
  // ENV=local opens /__test/session and /__test/clock with no bearer. Against a real database that is an open door, so it is
  // refused; only a throwaway schema (the e2e suite sets JM_PG_SCHEMA per server) may combine the two.
  return local && (env.JM_PG_SCHEMA === undefined || env.JM_PG_SCHEMA === '')
    ? 'ENV=local is refused with DATABASE_URL: it opens the test routes with no bearer. Unset ENV, or set TEST_BEARER.'
    : undefined;
}

/** What buildApp is given, from the environment. Test routes open only for ENV=local or a configured TEST_BEARER; kept apart from main.ts so a test can start the app as production does. */
export function appOptions(env: Record<string, string | undefined>): { publicUrl: string | undefined; local: boolean; testBearer: string | undefined } {
  return { publicUrl: env.PUBLIC_URL, local: env.ENV === 'local', testBearer: env.TEST_BEARER };
}

/** The connector vault from JM_VAULT_KEY (and JM_VAULT_OLD_KEYS), or undefined when no key is set. Call after startupRefusal. */
export async function vaultOf(env: Record<string, string | undefined>): Promise<Vault | undefined> {
  const ring = keyRing(env);
  return typeof ring === 'string' || ring.length === 0 ? undefined : makeVault(ring);
}
