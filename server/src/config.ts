import { webDirProblem } from './web';

// What the process must refuse at startup, kept apart from main.ts so a test can pin it.
import { keyRing, makeVault } from './vault';
import type { Vault } from './vault';
import type { DbTls } from './db';

const DB_TLS = ['JM_DB_CLIENT_CERT', 'JM_DB_CLIENT_KEY', 'JM_DB_SERVER_CA'] as const;

/**
 * The database client certificate: JM_DB_CLIENT_CERT, JM_DB_CLIENT_KEY and JM_DB_SERVER_CA (PEM), all three or none, and
 * JM_DB_TLS_SERVERNAME when the server's certificate names a host other than DATABASE_URL's. None set is a plain connection,
 * whose TLS comes from the URL's sslmode. A partial or non-PEM set is a string, which startupRefusal returns.
 */
export function dbTlsOf(env: Record<string, string | undefined>): DbTls | string | undefined {
  const set = DB_TLS.filter((k) => env[k]);
  if (set.length === 0) return undefined;
  if (set.length < DB_TLS.length) return `The database client certificate needs all of ${DB_TLS.join(', ')}; missing ${DB_TLS.filter((k) => !env[k]).join(', ')}.`;
  const bad = DB_TLS.filter((k) => !/^-----BEGIN [A-Z ]+-----/.test(env[k]!.trim()));
  if (bad.length) return `${bad.join(', ')} must be PEM (starting -----BEGIN ...-----).`;
  const [cert, key, ca] = DB_TLS.map((k) => env[k]!);
  const servername = env.JM_DB_TLS_SERVERNAME?.trim();
  return { cert: cert!, key: key!, ca: ca!, ...(servername ? { servername } : {}) };
}

export function startupRefusal(env: Record<string, string | undefined>): string | undefined {
  const local = env.ENV === 'local';
  if (env.JM_WEB_DIR) {
    const web = webDirProblem(env.JM_WEB_DIR);
    if (web !== undefined) return web;
  }
  if (env.GITHUB_URL && !local && !env.GITHUB_URL.startsWith('https://')) return 'GITHUB_URL must be https; an http override is for ENV=local only.';
  // The OAuth redirect and the browser session's own Origin come from PUBLIC_URL; a request's Host must not stand in for it.
  if (env.GITHUB_CLIENT_ID && !local && !env.PUBLIC_URL?.trim()) return 'PUBLIC_URL is required when GITHUB_CLIENT_ID is set outside ENV=local: set it to the public base URL.';
  // Bun serves no TLS, so an https PUBLIC_URL means a proxy in front, and every client would share its address in the start limit.
  const hops = env.TRUST_PROXY;
  if (hops && !/^\d+$/.test(hops)) return 'TRUST_PROXY must be a hop count: the number of proxies in front, each appending one X-Forwarded-For entry (1 for one TLS proxy), or 0.';
  // Outside ENV=local ownBase serves https whatever PUBLIC_URL says (auth.ts), so an http one names an origin no browser session
  // could match, and the cookie is Secure. The scheme is the parsed one: `HTTPS://` is https too.
  const pub = env.PUBLIC_URL?.trim();
  const scheme = ((): string | undefined => { try { return pub ? new URL(pub).protocol : undefined; } catch { return 'invalid'; } })();
  if (!local && pub && scheme !== 'https:') return 'PUBLIC_URL must be an https URL outside ENV=local: the server holds a browser session to its https origin. Put the TLS proxy\'s address here.';
  if (!local && scheme === 'https:' && !hops) {
    return 'TRUST_PROXY is required behind TLS (PUBLIC_URL is https; this server serves http): set it to the number of proxies in front, each appending one X-Forwarded-For entry (1 for one TLS proxy), or 0 to key the start limit on the socket address.';
  }
  // JM_VAULT_KEY is optional (without it the connector routes answer vault_unconfigured), but a malformed one is refused.
  const ring = keyRing(env);
  if (typeof ring === 'string') return ring;
  const tls = dbTlsOf(env);
  if (typeof tls === 'string') return tls;
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
 * per-minute start limit; TRUST_PROXY=<n> reads the client address from X-Forwarded-For, n entries from the right (n proxies
 * in front, each appending one); unset or 0 is the socket's address. JM_BUILD is the commit the process runs, reported on
 * /healthz; anything but 7 to 40 lowercase hex digits is ignored.
 */
export function appOptions(env: Record<string, string | undefined>): { publicUrl: string | undefined; local: boolean; testBearer: string | undefined; startLimit: number | undefined; trustProxy: number; webDir: string | undefined; build: string | undefined } {
  const limit = Number(env.AUTH_START_LIMIT);
  const hops = /^\d+$/.test(env.TRUST_PROXY ?? '') ? Number(env.TRUST_PROXY) : 0;
  return { publicUrl: env.PUBLIC_URL, local: env.ENV === 'local', testBearer: env.TEST_BEARER, startLimit: limit > 0 ? limit : undefined, trustProxy: hops, webDir: env.JM_WEB_DIR || undefined, build: /^[0-9a-f]{7,40}$/.test(env.JM_BUILD ?? '') ? env.JM_BUILD : undefined };
}

/** The connector vault from JM_VAULT_KEY (and JM_VAULT_OLD_KEYS), or undefined when no key is set. Call after startupRefusal. */
export async function vaultOf(env: Record<string, string | undefined>): Promise<Vault | undefined> {
  const ring = keyRing(env);
  return typeof ring === 'string' || ring.length === 0 ? undefined : makeVault(ring);
}
