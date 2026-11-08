// Sign-in (ADR 0008). The server mints its own device code; a person approves it from a browser session, and GitHub's OAuth web
// flow only proves who that person is. The GitHub token lives in one function's locals and is dropped there: never stored,
// logged or returned. Twin of the Worker's worker/src/auth.ts.
import type { Context, Hono } from 'hono';
import { deleteCookie, getCookie, setCookie } from 'hono/cookie';
import type { Actor, ErrorCode } from '../../gen/ts/protocol';
import { isoOf } from '../../podcore/src/index';
import type { Hub } from './hub';
import { base62, hashToken, sameSecret } from './ids';
import { authOf, bad, fail, JSON_TYPE, parse, presented, SESSION } from './kit';
import { approveDevice, deviceByHash, deviceByUser, exchangeDevice, newDevice, pollDevice, newState, revokeToken, takeState } from './store';
import { check } from './validate';

const DEVICE_S = 900;
const INTERVAL_S = 2;
const STATE_S = 600;
const SESSION_S = 30 * 86400;
const USER_CODE = 'BCDFGHJKLMNPQRSTVWXZ';
const STATE = 'jm_oauth_state';

export interface GitHub { web: string; api: string; id: string; secret: string }

/** The server's own base URL: PUBLIC_URL or the request's origin, upgraded to https except under ENV=local. Its origin is the one a browser session's Origin must equal. */
export function ownBase(c: Context, publicUrl: string | undefined, local: boolean): string {
  const b = (publicUrl ?? new URL(c.req.url).origin).replace(/\/+$/, '');
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

/** GitHub's numeric id and login for an OAuth code, or the error code to answer with. */
async function fromGitHub(gh: GitHub, code: string, redirect: string): Promise<{ id: number; login: string } | ErrorCode> {
  try {
    const body = new URLSearchParams({ client_id: gh.id, client_secret: gh.secret, code, redirect_uri: redirect });
    const t = await fetch(`${gh.web}/login/oauth/access_token`, { method: 'POST', body, headers: { Accept: 'application/json' }, redirect: 'manual' });
    const token: unknown = t.ok ? ((await t.json()) as { access_token?: unknown }).access_token : undefined;
    if (typeof token !== 'string' || token === '') return 'upstream';
    const u = await fetch(`${gh.api}/user`, { headers: { Authorization: `Bearer ${token}`, Accept: 'application/vnd.github+json', 'User-Agent': 'jellymoon' }, redirect: 'manual' });
    if (u.status === 401) return 'unauthorized';
    const j = (u.ok ? await u.json() : null) as { id?: unknown; login?: unknown } | null;
    return Number.isSafeInteger(j?.id) && typeof j?.login === 'string' && j.login.length > 0 && j.login.length <= 80 ? { id: j.id as number, login: j.login } : 'upstream';
  } catch { return 'upstream'; }
}

/** The device flow (the CLI's side and the approval) and GET /v1/me. */
function deviceRoutes(app: Hono, hub: Hub, base: (c: Context) => string, person: (c: Context) => Promise<Actor | null>): void {
  app.post('/v1/auth/device', async (c) => {
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
    const polled = await pollDevice(hub.db, hash, now, now - INTERVAL_S * 1000);
    if (polled === undefined) return fail(c, 'slow_down', `Poll at most every ${INTERVAL_S} seconds.`);
    if (polled.userId === null) return fail(c, 'authorization_pending', 'Not approved yet.');
    const userId = await exchangeDevice(hub.db, hash);
    const user = userId === undefined ? undefined : hub.users.get(userId);
    if (user === undefined) return fail(c, 'invalid_grant', 'This device code was already exchanged.');
    return c.json({ token: await hub.userToken(user.id), user: { kind: 'human', id: user.id, displayName: user.name }, expiresAt: isoOf(now + SESSION_S * 1000) });
  });

  app.post('/v1/auth/device/approve', async (c) => {
    const origin = c.req.header('Origin');
    if (!JSON_TYPE.test(c.req.header('Content-Type') ?? '') || (origin !== undefined && origin !== new URL(base(c)).origin)) {
      return fail(c, 'forbidden', 'Approve from this server\'s own page, with a JSON body.');
    }
    const me = await person(c);
    if (me === null) return fail(c, 'unauthorized', 'Sign in first.');
    const body = await parse(c);
    const problem = body === null ? 'The body must be a JSON object.' : check('DeviceApproveRequest', body);
    if (problem !== null || body === null) return bad(c, problem ?? '');
    const hash = hashToken(body.userCode as string);
    const now = hub.now();
    if (await approveDevice(hub.db, hash, me.id, now)) return c.json({});
    const row = await deviceByUser(hub.db, hash);
    if (row === undefined) return fail(c, 'not_found', 'No such code.');
    return row.expiresAt <= now ? fail(c, 'expired_token', 'This code expired.') : fail(c, 'gone', 'This code was already approved.');
  });

  app.get('/v1/me', async (c) => {
    const me = await person(c);
    return me === null ? fail(c, 'unauthorized', 'Sign in first.') : c.json(me);
  });
}

export function authRoutes(app: Hono, hub: Hub, gh: GitHub | null, publicUrl: string | undefined, local: boolean): void {
  const base = (c: Context): string => ownBase(c, publicUrl, local);
  const cookie = { httpOnly: true, secure: !local, sameSite: 'Lax' } as const;
  /** The person behind a Bearer jmu_ or the session cookie. Approve keeps its own F15 check (item 4); /v1/me is a read. */
  const person = async (c: Context): Promise<Actor | null> => {
    const p = presented(c, true);
    const who = p === null ? null : await hub.authenticate(`Bearer ${p.token}`);
    return who?.kind === 'user' ? who.actor : null;
  };

  deviceRoutes(app, hub, base, person);

  app.post('/v1/auth/logout', async (c) => {
    const who = await authOf(hub, c, true);
    if (who instanceof Response) return who;
    const p = presented(c, true);
    if (who === null || p === null) return fail(c, 'unauthorized', 'Sign in first.');
    if (who.kind !== 'user') return fail(c, 'forbidden', 'Only a person signs out.');
    await revokeToken(hub.db, hashToken(p.token), hub.now());
    deleteCookie(c, SESSION, { ...cookie, path: '/' });
    return c.json({});
  });

  app.get('/v1/auth/github/start', async (c) => {
    if (gh === null) return fail(c, 'not_found', 'GitHub sign-in is not configured on this server.');
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
    setCookie(c, SESSION, await hub.userToken(user.id), { ...cookie, path: '/', maxAge: SESSION_S });
    return c.redirect(next, 302);
  });
}
