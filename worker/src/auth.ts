// Sign-in (ADR 0008), twin of server/src/auth.ts: the server's own device code, approved from a browser session that GitHub's
// OAuth web flow proves. The GitHub token lives in fromGitHub's locals only: never stored, logged or returned.
// D1 tables (schema.sql): device_codes and oauth_states (hashes only), github_users, rate_limits (the start limit) and
// token_pods (which pods a token has sockets in, for logout).
import type { Actor, ErrorCode } from '../../gen/ts/protocol';
import { authenticate, mint, USER_TOKEN_MS } from './directory';
import type { Env } from './env';
import { base62, hashToken, newId } from './ids';
import { check } from './validate';
import { bad, fail, parse } from './wire';
import type { Who } from './wire';

const DEVICE_S = 900;
const INTERVAL_S = 2;
const STATE_S = 600;
const SESSION_S = USER_TOKEN_MS / 1000;
const SESSION = 'jm_session';
const STATE = 'jm_oauth_state';
const JSON_TYPE = /^application\/json\s*(;|$)/i;
export const GITHUB_TIMEOUT_MS = 10_000;
const START_LIMIT = 30;
/** Expired device codes and states stay an hour, so a late poll still reads expired_token; then a start deletes them. */
const SWEEP_GRACE_MS = 3_600_000;

interface GitHub { web: string; api: string; id: string; secret: string }
interface Device { expires_at: number; user_id: string | null; exchanged: number }

/** The OAuth client and GitHub's endpoints; null when no client is set, or an http override outside ENV=local. */
function githubOf(env: Env): GitHub | null {
  const { GITHUB_CLIENT_ID: id, GITHUB_CLIENT_SECRET: secret } = env;
  const base = env.GITHUB_URL?.replace(/\/+$/, '');
  if (!id || !secret || (base && env.ENV !== 'local' && !base.startsWith('https://'))) return null;
  return base ? { web: base, api: `${base}/api/v3`, id, secret } : { web: 'https://github.com', api: 'https://api.github.com', id, secret };
}

/** A same-origin path: one leading `/`, not `//` or `/\`, no backslash or control character anywhere. Anything else is `/`. */
const safeNext = (n: string | null): string => (n !== null && n.length <= 512 && /^\/(?![/\\])/.test(n) && !/[\\\x00-\x1f\x7f]/.test(n) ? n : '/');

function userCode(): string {
  const pick = (): string => Array.from(crypto.getRandomValues(new Uint8Array(4)), (b) => 'BCDFGHJKLMNPQRSTVWXZ'[b % 20]).join('');
  return `${pick()}-${pick()}`;
}

/** WDJB-MJHT however it was typed: either case, with or without the hyphen (the schema has already checked the letters). */
const canonCode = (s: string): string => `${s.toUpperCase().replace('-', '').slice(0, 4)}-${s.toUpperCase().replace('-', '').slice(4)}`;

const cookieOf = (req: Request, name: string): string | undefined =>
  (req.headers.get('Cookie') ?? '').split(';').map((p) => p.trim().split('=')).find(([k]) => k === name)?.[1];
const setCookie = (env: Env, name: string, value: string, path: string, maxAge: number): string =>
  `${name}=${value}; Path=${path}; Max-Age=${maxAge}; HttpOnly; SameSite=Lax${env.ENV === 'local' ? '' : '; Secure'}`;
const redirect = (to: string, cookie: string): Response => new Response(null, { status: 302, headers: { Location: to, 'Set-Cookie': cookie } });
/** The clock: the test entry's under ENV=local, else the real one. */
const nowOf = (env: Env): number => env.CLOCK_MS ?? Date.now();

/** GitHub's numeric id and login for an OAuth code, or the error code to answer with. Each call to GitHub gives up after `timeoutMs`. */
export async function fromGitHub(gh: GitHub, code: string, redirectUri: string, timeoutMs = GITHUB_TIMEOUT_MS): Promise<{ id: number; login: string } | ErrorCode> {
  try {
    const body = new URLSearchParams({ client_id: gh.id, client_secret: gh.secret, code, redirect_uri: redirectUri });
    const t = await fetch(`${gh.web}/login/oauth/access_token`, { method: 'POST', body, headers: { Accept: 'application/json' }, redirect: 'manual', signal: AbortSignal.timeout(timeoutMs) });
    const token: unknown = t.ok ? ((await t.json()) as { access_token?: unknown }).access_token : undefined;
    if (typeof token !== 'string' || token === '') return 'upstream';
    const u = await fetch(`${gh.api}/user`, { headers: { Authorization: `Bearer ${token}`, Accept: 'application/vnd.github+json', 'User-Agent': 'jellymoon' }, redirect: 'manual', signal: AbortSignal.timeout(timeoutMs) });
    if (u.status === 401) return 'unauthorized';
    const j = (u.ok ? await u.json() : null) as { id?: unknown; login?: unknown } | null;
    return Number.isSafeInteger(j?.id) && typeof j?.login === 'string' && j.login.length > 0 && j.login.length <= 80 ? { id: j.id as number, login: j.login } : 'upstream';
  } catch { return 'upstream'; }
}

/**
 * The user a GitHub account signs in as, keyed by GitHub's numeric id, created on first sign-in; then a new jmu_ for them. One
 * batch is one transaction: the user row is written only while no row holds the GitHub id, so two first sign-ins at once make
 * one user, and both read it.
 */
async function githubUser(db: D1Database, githubId: string, login: string, now: number): Promise<string> {
  const id = newId('usr_');
  const [, , got] = await db.batch([
    db.prepare('insert into users (id, name) select ?, ? where not exists (select 1 from github_users where github_id = ?)').bind(id, login, githubId),
    db.prepare('insert or ignore into github_users (github_id, user_id) values (?, ?)').bind(githubId, id),
    db.prepare('select user_id from github_users where github_id = ?').bind(githubId),
  ]);
  return mint(db, 'jmu_', 'user', (got.results[0] as { user_id: string }).user_id, null, now + USER_TOKEN_MS);
}

/** The server's own base URL: PUBLIC_URL or the request's origin, upgraded to https except under ENV=local. A browser session's Origin must be its origin. */
function ownBase(env: Env, req: Request): string {
  const b = (env.PUBLIC_URL?.trim() || new URL(req.url).origin).replace(/\/+$/, '');
  return env.ENV === 'local' ? b : b.replace(/^http:/, 'https:');
}

/** The token a request presents: its Bearer, else, where a person may call, the session cookie. */
function presented(req: Request, cookie: boolean): { token: string; byCookie: boolean } | null {
  const h = req.headers.get('Authorization');
  if (h !== null) return { token: h.startsWith('Bearer ') ? h.slice(7).trim() : '', byCookie: false };
  const s = cookie ? cookieOf(req, SESSION) : undefined;
  return s ? { token: s, byCookie: true } : null;
}

/** The SHA-256 of the token a socket upgrade presents, the tag logout closes it by. */
export async function presentedHash(req: Request): Promise<string | null> {
  const p = presented(req, true);
  return p === null ? null : hashToken(p.token);
}

/**
 * The one authentication function (ADR 0008, "Browser session"), twin of the server's kit.ts authOf. Bearer first; else, when
 * `cookie`, the session cookie, which must hold a jmu_. A cookie request that changes state, or opens the socket, must carry this
 * server's own Origin, and a state change must be JSON (forbidden otherwise). Null is no valid caller.
 */
export async function authOf(env: Env, req: Request, cookie: boolean): Promise<Who | null | Response> {
  const p = presented(req, cookie);
  if (p === null || !p.byCookie) return authenticate(env.DB, p === null ? null : `Bearer ${p.token}`, nowOf(env));
  const write = !['GET', 'HEAD', 'OPTIONS'].includes(req.method);
  if ((write || req.headers.has('upgrade')) && req.headers.get('Origin') !== new URL(ownBase(env, req)).origin) return fail('forbidden', 'A browser session works only from this server\'s own pages.');
  if (write && !JSON_TYPE.test(req.headers.get('Content-Type') ?? '')) return fail('forbidden', 'A browser session sends JSON.');
  const who = await authenticate(env.DB, `Bearer ${p.token}`, nowOf(env));
  return who?.kind === 'user' ? who : null;
}

/** The person behind a Bearer jmu_ or the session cookie, for GET /v1/me and GET /v1/session (reads). */
async function person(env: Env, req: Request): Promise<Actor | null> {
  const p = presented(req, true);
  const who = p === null ? null : await authenticate(env.DB, `Bearer ${p.token}`, nowOf(env));
  return who?.kind === 'user' ? who.actor : null;
}

/**
 * POST /v1/auth/logout: revokes the presented token, and the cookie's too when a Bearer came beside it; closes their pod sockets;
 * clears the cookie. The revoke is what signs out, so a pod that fails to close its sockets does not keep the cookie (allSettled).
 */
async function logout(env: Env, req: Request, now: number): Promise<Response> {
  const who = await authOf(env, req, true);
  if (who instanceof Response) return who;
  const p = presented(req, true);
  // A dead session cookie with no Bearer is a sign-out that already happened: nothing to revoke, the dead cookie is cleared.
  if (who === null && p?.byCookie === true) return Response.json({}, { headers: { 'Set-Cookie': setCookie(env, SESSION, '', '/', 0) } });
  if (who === null || p === null) return fail('unauthorized', 'Sign in first.');
  if (who.kind !== 'user') return fail('forbidden', 'Only a person signs out.');
  for (const hash of new Set(await Promise.all([p.token, cookieOf(req, SESSION) ?? p.token].map(hashToken)))) {
    await env.DB.prepare("update tokens set revoked_at = ? where hash = ? and kind = 'user' and revoked_at is null").bind(now, hash).run();
    const pods = (await env.DB.prepare('delete from token_pods where hash = ? returning pod_id').bind(hash).all<{ pod_id: string }>()).results;
    const closed = await Promise.allSettled(pods.map((r) => env.POD.get(env.POD.idFromName(r.pod_id)).fetch('https://pod/_in/close-token', { method: 'POST', body: JSON.stringify({ hash }) })));
    for (const c of closed) if (c.status === 'rejected') console.error('logout: a pod failed to close its sockets', c.reason);
  }
  return Response.json({}, { headers: { 'Set-Cookie': setCookie(env, SESSION, '', '/', 0) } });
}

/**
 * The start limit: a fixed one-minute window per route and client address (CF-Connecting-IP), counted in D1; null is under it.
 * The same batch drops the rate rows of older minutes and the sign-in rows an hour past their expiry.
 */
async function startGate(env: Env, req: Request, route: string, now: number): Promise<Response | null> {
  const win = Math.floor(now / 60_000);
  const key = `${route} ${req.headers.get('CF-Connecting-IP') ?? 'unknown'}`;
  const [hit] = await env.DB.batch([
    env.DB.prepare('insert into rate_limits (k, win, n) values (?, ?, 1) on conflict (k) do update set n = case when win = excluded.win then n + 1 else 1 end, win = excluded.win returning n').bind(key, win),
    env.DB.prepare('delete from rate_limits where win < ?').bind(win),
    env.DB.prepare('delete from device_codes where expires_at <= ?').bind(now - SWEEP_GRACE_MS),
    env.DB.prepare('delete from oauth_states where expires_at <= ?').bind(now - SWEEP_GRACE_MS),
    env.DB.prepare('delete from token_pods where not exists (select 1 from tokens where tokens.hash = token_pods.hash and revoked_at is null and (expires_at is null or expires_at > ?))').bind(now),
  ]);
  if (((hit.results[0] as { n: number } | undefined)?.n ?? 0) <= (Number(env.AUTH_START_LIMIT) || START_LIMIT)) return null;
  const res = fail('rate_limited', 'Too many sign-in starts from this address; wait for Retry-After.');
  res.headers.set('Retry-After', String(Math.ceil(((win + 1) * 60_000 - now) / 1000)));
  return res;
}

async function deviceToken(env: Env, req: Request, now: number): Promise<Response> {
  const body = await parse(req);
  const problem = body === null ? 'The body must be a JSON object.' : check('DeviceTokenRequest', body);
  if (problem !== null || body === null) return bad(problem ?? '');
  const hash = await hashToken(body.deviceCode as string);
  const row = await env.DB.prepare('select expires_at, user_id, exchanged from device_codes where hash = ?').bind(hash).first<Device>();
  if (row === null || row.exchanged) return fail('invalid_grant', 'This device code is unknown or was already exchanged.');
  if (row.expires_at <= now) return fail('expired_token', 'This device code expired.');
  const polled = await env.DB.prepare('update device_codes set last_poll = ? where hash = ? and last_poll <= ? returning user_id').bind(now, hash, now - INTERVAL_S * 1000).first<Device>();
  if (polled === null) return fail('slow_down', `Poll at most every ${INTERVAL_S} seconds.`);
  if (polled.user_id === null) return fail('authorization_pending', 'Not approved yet.');
  const won = await env.DB.prepare('update device_codes set exchanged = 1 where hash = ? and exchanged = 0 and user_id is not null returning user_id').bind(hash).first<Device>();
  const user = won?.user_id ? await env.DB.prepare('select id, name from users where id = ?').bind(won.user_id).first<{ id: string; name: string }>() : null;
  if (user === null) return fail('invalid_grant', 'This device code was already exchanged.');
  const token = await mint(env.DB, 'jmu_', 'user', user.id, null, now + USER_TOKEN_MS);
  return Response.json({ token, user: { kind: 'human', id: user.id, displayName: user.name }, expiresAt: new Date(now + USER_TOKEN_MS).toISOString() });
}

/** The approval: JSON, and by cookie the server's own Origin (authOf), like every cookie write. */
async function approve(env: Env, req: Request, now: number): Promise<Response> {
  if (!JSON_TYPE.test(req.headers.get('Content-Type') ?? '')) return fail('forbidden', 'Approve with a JSON body.');
  const who = await authOf(env, req, true);
  if (who instanceof Response) return who;
  if (who?.kind !== 'user') return fail('unauthorized', 'Sign in first.');
  const body = await parse(req);
  const problem = body === null ? 'The body must be a JSON object.' : check('DeviceApproveRequest', body);
  if (problem !== null || body === null) return bad(problem ?? '');
  const hash = await hashToken(canonCode(body.userCode as string));
  const won = await env.DB.prepare('update device_codes set user_id = ? where user_hash = ? and user_id is null and expires_at > ? returning hash').bind(who.actor.id, hash, now).first();
  if (won !== null) return Response.json({});
  const row = await env.DB.prepare('select expires_at from device_codes where user_hash = ?').bind(hash).first<Device>();
  if (row === null) return fail('not_found', 'No such code.');
  return row.expires_at <= now ? fail('expired_token', 'This code expired.') : fail('gone', 'This code was already approved.');
}

async function callback(env: Env, req: Request, gh: GitHub, base: string, now: number): Promise<Response> {
  const url = new URL(req.url);
  const state = url.searchParams.get('state') ?? '';
  const held = cookieOf(req, STATE) ?? '';
  if (state === '' || held === '' || (await hashToken(state)) !== (await hashToken(held))) return fail('forbidden', 'The sign-in state is missing or not this browser\'s.');
  const taken = await env.DB.prepare('delete from oauth_states where hash = ? and expires_at > ? returning next').bind(await hashToken(state), now).first<{ next: string }>();
  if (taken === null) return fail('forbidden', 'This sign-in state was used or expired. Start again.');
  const got = await fromGitHub(gh, url.searchParams.get('code') ?? '', `${base}/v1/auth/github/callback`);
  if (typeof got === 'string') return fail(got, got === 'unauthorized' ? 'GitHub refused the sign-in.' : 'GitHub failed; try again.');
  const res = redirect(taken.next, setCookie(env, SESSION, await githubUser(env.DB, String(got.id), got.login, now), '/', SESSION_S));
  res.headers.append('Set-Cookie', setCookie(env, STATE, '', '/v1/auth/github', 0));
  return res;
}

/** POST /v1/auth/device: a new device code and user code, past the start limit. */
async function deviceStart(env: Env, req: Request, base: string, now: number): Promise<Response> {
  const refused = await startGate(env, req, 'device', now);
  if (refused !== null) return refused;
  const deviceCode = base62(43);
  const code = userCode();
  await env.DB.prepare('insert into device_codes (hash, user_hash, expires_at) values (?, ?, ?)').bind(await hashToken(deviceCode), await hashToken(code), now + DEVICE_S * 1000).run();
  return Response.json({ deviceCode, userCode: code, verificationUri: `${base}/device?code=${code}`, interval: INTERVAL_S, expiresIn: DEVICE_S });
}

/**
 * GET /v1/auth/github/*. The Worker has no startup hook to refuse a missing PUBLIC_URL at (the server refuses to start), so
 * outside ENV=local both routes answer internal, naming it: the redirect URI and the session's Origin must not come from a Host.
 */
async function githubRoute(env: Env, req: Request, route: string, base: string, now: number): Promise<Response> {
  const gh = githubOf(env);
  if (gh === null) return fail('not_found', 'GitHub sign-in is not configured on this server.');
  if (env.ENV !== 'local' && !env.PUBLIC_URL?.trim()) return fail('internal', 'GitHub sign-in needs PUBLIC_URL on this server; set it to the public base URL.');
  if (route === 'GET /v1/auth/github/callback') return callback(env, req, gh, base, now);
  const refused = await startGate(env, req, 'github', now);
  if (refused !== null) return refused;
  const state = base62(32);
  await env.DB.prepare('insert into oauth_states (hash, next, expires_at) values (?, ?, ?)').bind(await hashToken(state), safeNext(new URL(req.url).searchParams.get('next')), now + STATE_S * 1000).run();
  const q = new URLSearchParams({ client_id: gh.id, redirect_uri: `${base}/v1/auth/github/callback`, state });
  return redirect(`${gh.web}/login/oauth/authorize?${q}`, setCookie(env, STATE, state, '/v1/auth/github', STATE_S));
}

/** The sign-in routes; null for any other path. `now` is the server clock (the test entry's, under ENV=local). */
export async function authRoute(req: Request, env: Env, now: number): Promise<Response | null> {
  const base = ownBase(env, req);
  const route = `${req.method} ${new URL(req.url).pathname}`;
  if (route === 'POST /v1/auth/logout') return logout(env, req, now);
  if (route === 'POST /v1/auth/device') return deviceStart(env, req, base, now);
  if (route === 'POST /v1/auth/device/token') return deviceToken(env, req, now);
  if (route === 'POST /v1/auth/device/approve') return approve(env, req, now);
  if (route === 'GET /v1/me' || route === 'GET /v1/session') {
    // /v1/session is /v1/me that never refuses, so a signed-out page load logs no failed request.
    const me = await person(env, req);
    if (route === 'GET /v1/session') return Response.json(me === null ? {} : { user: me });
    return me === null ? fail('unauthorized', 'Sign in first.') : Response.json(me);
  }
  if (route === 'GET /v1/auth/github/start' || route === 'GET /v1/auth/github/callback') return githubRoute(env, req, route, base, now);
  return null;
}
