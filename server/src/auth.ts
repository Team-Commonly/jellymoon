// Sign-in (ADR 0008). The server mints its own device code; a person approves it from a browser session, and GitHub's OAuth web
// flow only proves who that person is. The GitHub token lives in one function's locals and is dropped there: never stored,
// logged or returned. Twin of the Worker's worker/src/auth.ts.
import type { Context, Hono } from 'hono';
import { getConnInfo } from 'hono/bun';
import { deleteCookie, getCookie, setCookie } from 'hono/cookie';
import type { Actor, ErrorCode } from '../../gen/ts/protocol';
import { isoOf } from '../../podcore/src/index';
import { USER_TOKEN_MS } from './hub';
import type { Hub } from './hub';
import { base62, hashToken, sameSecret } from './ids';
import { authOf, bad, fail, JSON_TYPE, parse, presented, SESSION } from './kit';
import { approveDevice, denyDevice, deviceByHash, deviceByUser, exchangeDevice, newDevice, pollDevice, newState, revokeToken, sweepSignIn, takeState } from './store';
import { check } from './validate';

const DEVICE_S = 900;
const INTERVAL_S = 2;
const STATE_S = 600;
const SESSION_S = USER_TOKEN_MS / 1000;
const USER_CODE = 'BCDFGHJKLMNPQRSTVWXZ';
const STATE = 'jm_oauth_state';
export const GITHUB_TIMEOUT_MS = 10_000;
/** Starts (device and GitHub, each) per client address per minute, unless AUTH_START_LIMIT says otherwise. */
const START_LIMIT = 30;
/** Failed user-code lookups (approve and deny together) per signed-in person per fixed ten minutes (RFC 8628 section 5.1). */
const CODE_LIMIT = 10;
const CODE_WINDOW_MS = 600_000;

export interface GitHub { web: string; api: string; id: string; secret: string }
/** Sign-in settings from the environment (config.ts appOptions). */
export interface SignInOpts { publicUrl?: string; local: boolean; startLimit?: number; trustProxy?: number }

/** Counts per key in a fixed window, in memory (one process, ADR 0006). A window's counts are dropped when it ends. */
class Counts {
  private win = -1;
  private readonly seen = new Map<string, number>();
  constructor(private readonly ms: number) {}
  /** One more for `key` in the window holding `now`: the new count, and the whole seconds left in the window. */
  take(key: string, now: number): { n: number; retry: number } {
    const w = Math.floor(now / this.ms);
    if (w !== this.win) { this.win = w; this.seen.clear(); }
    const n = (this.seen.get(key) ?? 0) + 1;
    this.seen.set(key, n);
    return { n, retry: Math.ceil(((w + 1) * this.ms - now) / 1000) };
  }
  /** Takes back one that `take(key, now)` counted, while its window is still the current one. */
  give(key: string, now: number): void {
    const n = this.seen.get(key);
    if (Math.floor(now / this.ms) === this.win && n !== undefined) this.seen.set(key, n - 1);
  }
}

const limited = (c: Context, retry: number, message: string): Response => { c.header('Retry-After', String(retry)); return fail(c, 'rate_limited', message); };

/**
 * The start limit: a fixed one-minute window per route and client address, in `starts`. Past `limit`, rate_limited with
 * Retry-After, the seconds left in the window. Under it, the expired sign-in rows are swept.
 */
function startGate(hub: Hub, starts: Counts, limit: number, trustProxy: number): (c: Context) => Promise<Response | null> {
  return async (c) => {
    const now = hub.now();
    const got = starts.take(`${c.req.path} ${clientIp(c, trustProxy)}`, now);
    if (got.n <= limit) { await sweepSignIn(hub.db, now); return null; }
    return limited(c, got.retry, 'Too many sign-in starts from this address; wait for Retry-After.');
  };
}

/**
 * The guess limit on approve and deny (ADR 0008 step 4), checked before the lookup, so a caller past it learns nothing:
 * CODE_LIMIT failed lookups per person per fixed ten minutes, and each failed lookup also counts in the address's device
 * start limit, so many accounts from one address share that. A request takes one from both and gives them back when its
 * lookup succeeds (the returned function): a success never counts, and concurrent guesses cannot pass the limit together.
 */
function guessGate(hub: Hub, starts: Counts, limit: number, trustProxy: number): (c: Context, userId: string) => Response | (() => void) {
  const codes = new Counts(CODE_WINDOW_MS);
  return (c, userId) => {
    const now = hub.now();
    const mine = codes.take(userId, now);
    if (mine.n > CODE_LIMIT) return limited(c, mine.retry, 'Too many unknown, expired or used codes from this account; wait for Retry-After.');
    const addr = `/v1/auth/device ${clientIp(c, trustProxy)}`;
    const here = starts.take(addr, now);
    if (here.n > limit) { codes.give(userId, now); return limited(c, here.retry, 'Too many sign-in attempts from this address; wait for Retry-After.'); }
    return () => { codes.give(userId, now); starts.give(addr, now); };
  };
}

/**
 * Behind `hops` trusted proxies, each appending the address it saw, the client is the hops-th X-Forwarded-For entry from the
 * right: entries left of it are the caller's own writing. Undefined for 0 hops or fewer entries than hops.
 */
export function forwardedFor(header: string | undefined, hops: number): string | undefined {
  const all = hops > 0 ? (header ?? '').split(',').map((s) => s.trim()) : [];
  return all.length >= hops ? all[all.length - hops] || undefined : undefined;
}

/** The client's address: the socket's, or behind `hops` trusted proxies the X-Forwarded-For entry the outermost one wrote. */
function clientIp(c: Context, hops: number): string {
  const fwd = forwardedFor(c.req.header('X-Forwarded-For'), hops);
  if (fwd) return fwd;
  try { return getConnInfo(c).remote.address ?? 'unknown'; } catch { return 'unknown'; }
}

/** WDJB-MJHT however it was typed: either case, with or without the hyphen (the schema has already checked the letters). */
const canonCode = (s: string): string => `${s.toUpperCase().replace('-', '').slice(0, 4)}-${s.toUpperCase().replace('-', '').slice(4)}`;

/** The server's own base URL: PUBLIC_URL or the request's origin, upgraded to https except under ENV=local. Its origin is the one a browser session's Origin must equal. */
export function ownBase(c: Context, publicUrl: string | undefined, local: boolean): string {
  const b = (publicUrl?.trim() || new URL(c.req.url).origin).replace(/\/+$/, '');
  return local ? b : b.replace(/^http:/, 'https:');
}

/** GitHub's endpoints and the OAuth client, from the environment; null when no client is configured. An override base uses the GitHub Enterprise layout. */
export function githubOf(env: Record<string, string | undefined>): GitHub | null {
  const { GITHUB_CLIENT_ID: id, GITHUB_CLIENT_SECRET: secret } = env;
  if (!id || !secret) return null;
  const base = env.GITHUB_URL?.replace(/\/+$/, '');
  return base ? { web: base, api: `${base}/api/v3`, id, secret } : { web: 'https://github.com', api: 'https://api.github.com', id, secret };
}

/** A same-origin path: one leading `/`, not `//` or `/\`, no backslash or control character anywhere. Anything else is `/`. */
export function safeNext(next: string | undefined): string {
  return next !== undefined && next.length <= 512 && /^\/(?![/\\])/.test(next) && !/[\\\x00-\x1f\x7f]/.test(next) ? next : '/';
}

function userCode(): string {
  const pick = (): string => Array.from(crypto.getRandomValues(new Uint8Array(4)), (b) => USER_CODE[b % 20]).join('');
  return `${pick()}-${pick()}`; // 256 % 20 != 0 leaves a slight bias; 20^8 codes, valid 15 minutes, each needs a signed-in approver
}

/** GitHub's numeric id and login for an OAuth code, or the error code to answer with. Each call to GitHub gives up after `timeoutMs`. */
export async function fromGitHub(gh: GitHub, code: string, redirect: string, timeoutMs = GITHUB_TIMEOUT_MS): Promise<{ id: number; login: string } | ErrorCode> {
  try {
    const body = new URLSearchParams({ client_id: gh.id, client_secret: gh.secret, code, redirect_uri: redirect });
    const t = await fetch(`${gh.web}/login/oauth/access_token`, { method: 'POST', body, headers: { Accept: 'application/json' }, redirect: 'manual', signal: AbortSignal.timeout(timeoutMs) });
    const token: unknown = t.ok ? ((await t.json()) as { access_token?: unknown }).access_token : undefined;
    if (typeof token !== 'string' || token === '') return 'upstream';
    const u = await fetch(`${gh.api}/user`, { headers: { Authorization: `Bearer ${token}`, Accept: 'application/vnd.github+json', 'User-Agent': 'jellymoon' }, redirect: 'manual', signal: AbortSignal.timeout(timeoutMs) });
    if (u.status === 401) return 'unauthorized';
    const j = (u.ok ? await u.json() : null) as { id?: unknown; login?: unknown } | null;
    return Number.isSafeInteger(j?.id) && typeof j?.login === 'string' && j.login.length > 0 && j.login.length <= 80 ? { id: j.id as number, login: j.login } : 'upstream';
  } catch { return 'upstream'; }
}

/** The device flow (the CLI's side, the approval and the refusal), GET /v1/me and GET /v1/session. */
function deviceRoutes(app: Hono, hub: Hub, base: (c: Context) => string, person: (c: Context) => Promise<Actor | null>, gate: (c: Context) => Promise<Response | null>, guess: ReturnType<typeof guessGate>): void {
  app.post('/v1/auth/device', async (c) => {
    const refused = await gate(c);
    if (refused !== null) return refused;
    const deviceCode = base62(43);
    const code = userCode();
    await newDevice(hub.db, hashToken(deviceCode), hashToken(code), hub.now() + DEVICE_S * 1000);
    return c.json({ deviceCode, userCode: code, verificationUri: `${base(c)}/device?code=${code}`, interval: INTERVAL_S, expiresIn: DEVICE_S });
  });

  app.post('/v1/auth/device/token', async (c) => {
    const body = await parse(c);
    const problem = body === null ? 'The body must be a JSON object.' : check('DeviceTokenRequest', body);
    if (problem !== null || body === null) return bad(c, problem ?? '');
    const hash = hashToken(body.deviceCode as string);
    const now = hub.now();
    const row = await deviceByHash(hub.db, hash);
    if (row === undefined || row.exchanged) return fail(c, 'invalid_grant', 'This device code is unknown or was already exchanged.');
    if (row.expiresAt <= now) return fail(c, 'expired_token', 'This device code expired.');
    if (row.denied) return fail(c, 'access_denied', 'The sign-in was refused in the browser.');
    const polled = await pollDevice(hub.db, hash, now, now - INTERVAL_S * 1000);
    if (polled === undefined) return fail(c, 'slow_down', `Poll at most every ${INTERVAL_S} seconds.`);
    if (polled.userId === null) return fail(c, 'authorization_pending', 'Not approved yet.');
    const userId = await exchangeDevice(hub.db, hash);
    const user = userId === undefined ? undefined : hub.users.get(userId);
    if (user === undefined) return fail(c, 'invalid_grant', 'This device code was already exchanged.');
    const minted = await hub.userToken(user.id);
    return c.json({ token: minted.token, user: { kind: 'human', id: user.id, displayName: user.name }, expiresAt: isoOf(minted.expiresAt) });
  });

  // Approve binds a pending code to the caller; deny refuses it (idempotent). Each refusal is the same answer from both routes,
  // and both share one guess limit.
  for (const route of ['approve', 'deny'] as const) {
    app.post(`/v1/auth/device/${route}`, async (c) => {
      if (!JSON_TYPE.test(c.req.header('Content-Type') ?? '')) return fail(c, 'forbidden', `${route === 'approve' ? 'Approve' : 'Deny'} with a JSON body.`);
      const who = await authOf(hub, c, true); // by cookie, the server's own Origin, like every cookie write
      if (who instanceof Response) return who;
      if (who?.kind !== 'user') return fail(c, 'unauthorized', 'Sign in first.');
      const body = await parse(c);
      const problem = body === null ? 'The body must be a JSON object.' : check(route === 'approve' ? 'DeviceApproveRequest' : 'DeviceDenyRequest', body);
      if (problem !== null || body === null) return bad(c, problem ?? '');
      const held = guess(c, who.actor.id);
      if (held instanceof Response) return held;
      const hash = hashToken(canonCode(body.userCode as string));
      const now = hub.now();
      if (route === 'approve' ? await approveDevice(hub.db, hash, who.actor.id, now) : await denyDevice(hub.db, hash, now)) { held(); return c.json({}); }
      const row = await deviceByUser(hub.db, hash);
      if (row === undefined) return fail(c, 'not_found', 'No such code.');
      return row.expiresAt <= now ? fail(c, 'expired_token', 'This code expired.') : fail(c, 'gone', 'This code was already used.');
    });
  }

  app.get('/v1/me', async (c) => {
    const me = await person(c);
    return me === null ? fail(c, 'unauthorized', 'Sign in first.') : c.json(me);
  });
  // /v1/me that never refuses, so a signed-out page load logs no failed request.
  app.get('/v1/session', async (c) => {
    const me = await person(c);
    return c.json(me === null ? {} : { user: me });
  });
}

export function authRoutes(app: Hono, hub: Hub, gh: GitHub | null, opts: SignInOpts): void {
  const { publicUrl, local } = opts;
  const base = (c: Context): string => ownBase(c, publicUrl, local);
  const cookie = { httpOnly: true, secure: !local, sameSite: 'Lax' } as const;
  const starts = new Counts(60_000);
  const limit = opts.startLimit ?? START_LIMIT;
  const gate = startGate(hub, starts, limit, opts.trustProxy ?? 0);
  /** The person behind a Bearer jmu_ or the session cookie, for GET /v1/me (a read). */
  const person = async (c: Context): Promise<Actor | null> => {
    const p = presented(c, true);
    const who = p === null ? null : await hub.authenticate(`Bearer ${p.token}`);
    return who?.kind === 'user' ? who.actor : null;
  };

  deviceRoutes(app, hub, base, person, gate, guessGate(hub, starts, limit, opts.trustProxy ?? 0));

  /** Revokes the presented token, and the session cookie's too when a Bearer came beside it; closes their pod sockets. A dead session cookie alone is cleared and answers 200. */
  app.post('/v1/auth/logout', async (c) => {
    const who = await authOf(hub, c, true);
    if (who instanceof Response) return who;
    const p = presented(c, true);
    // A dead session cookie with no Bearer is a sign-out that already happened: nothing to revoke, the dead cookie is cleared.
    if (who === null && p?.byCookie === true) { deleteCookie(c, SESSION, { ...cookie, path: '/' }); return c.json({}); }
    if (who === null || p === null) return fail(c, 'unauthorized', 'Sign in first.');
    if (who.kind !== 'user') return fail(c, 'forbidden', 'Only a person signs out.');
    for (const hash of new Set([p.token, getCookie(c, SESSION) ?? p.token].map(hashToken))) {
      await revokeToken(hub.db, hash, hub.now());
      hub.closeSockets(hash);
    }
    deleteCookie(c, SESSION, { ...cookie, path: '/' });
    return c.json({});
  });

  app.get('/v1/auth/github/start', async (c) => {
    if (gh === null) return fail(c, 'not_found', 'GitHub sign-in is not configured on this server.');
    const refused = await gate(c);
    if (refused !== null) return refused;
    const state = base62(32);
    await newState(hub.db, hashToken(state), safeNext(c.req.query('next')), hub.now() + STATE_S * 1000);
    setCookie(c, STATE, state, { ...cookie, path: '/v1/auth/github', maxAge: STATE_S });
    const q = new URLSearchParams({ client_id: gh.id, redirect_uri: `${base(c)}/v1/auth/github/callback`, state });
    return c.redirect(`${gh.web}/login/oauth/authorize?${q}`, 302);
  });

  app.get('/v1/auth/github/callback', async (c) => {
    if (gh === null) return fail(c, 'not_found', 'GitHub sign-in is not configured on this server.');
    const state = c.req.query('state') ?? '';
    const held = getCookie(c, STATE) ?? '';
    if (state === '' || held === '' || !sameSecret(state, held)) return fail(c, 'forbidden', 'The sign-in state is missing or not this browser\'s.');
    const next = await takeState(hub.db, hashToken(state), hub.now());
    if (next === undefined) return fail(c, 'forbidden', 'This sign-in state was used or expired. Start again.');
    deleteCookie(c, STATE, { path: '/v1/auth/github' });
    const got = await fromGitHub(gh, c.req.query('code') ?? '', `${base(c)}/v1/auth/github/callback`);
    if (typeof got === 'string') return fail(c, got, got === 'unauthorized' ? 'GitHub refused the sign-in.' : 'GitHub failed; try again.');
    const user = await hub.githubUser(String(got.id), got.login);
    setCookie(c, SESSION, (await hub.userToken(user.id)).token, { ...cookie, path: '/', maxAge: SESSION_S });
    return c.redirect(next, 302);
  });
}
