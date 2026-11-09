# 0008 · Sign-in: Jellymoon's own device code, approved in a browser that GitHub signs in

Status: Accepted, by the owner's ruling, 2026-11-08 (the contract wins over F15's first wording). It adds no dependency. Open
below: the session cookie on the other user routes, token expiry, and the `/device` page itself.

## Decision

`jellymoon login` runs a device flow that the server owns. GitHub only proves who the person is, through its OAuth web flow in
a browser. The CLI never talks to GitHub and never holds a GitHub token; the server holds one for the length of one function
and drops it.

1. `POST /v1/auth/device` mints a device code and a user code. The device code is 43 random base62 characters (about 256
   bits), with no prefix: it is not a token kind `authenticate` accepts, and a `jm?_` prefix would say it is. The user code is
   eight letters from the schema's alphabet. Both are stored only as SHA-256, with the expiry (15 minutes) and the last poll.
   `verificationUri` is `<PUBLIC_URL or the request's origin>/device?code=<userCode>`, upgraded to https except under ENV=local.
   The server never calls GitHub's device endpoints.
2. In a browser, `GET /v1/auth/github/start?next=<path>` stores a random state (hashed) with `next` and a 10-minute expiry,
   sets it in an httpOnly cookie scoped to `/v1/auth/github`, and redirects to GitHub's authorize page. `next` must be a
   same-origin path (one leading `/`, not `//` or `/\`, no backslash or control character); anything else becomes `/`.
3. `GET /v1/auth/github/callback?code&state` refuses (forbidden) a missing state, a state that is not the cookie's, and a
   state already used: the stored row is deleted as it is read, and its `next` is the one bound at start. It exchanges the
   code for a GitHub token server-side, reads `GET /user`, and keys the Jellymoon user by GitHub's numeric id (a login can be
   renamed; the first login becomes the display name). Then the GitHub token goes out of scope. A 401 from `/user` answers
   `unauthorized`; any other GitHub failure answers `upstream` (502) and echoes nothing GitHub sent. On success it mints a
   `jmu_` session, sets it as an httpOnly, SameSite=Lax cookie (Secure except under ENV=local), and redirects to `next`.
4. `POST /v1/auth/device/approve {userCode}` is the only approver; no page load approves. It takes the session cookie or a
   Bearer `jmu_`, needs `Content-Type: application/json`, and, when an Origin header is sent, the server's own origin
   (forbidden otherwise: a cross-site form cannot send JSON, and a cross-site fetch sends a foreign Origin). It binds the code
   to the caller: unknown `not_found`, expired `expired_token`, already approved `gone`.
5. `POST /v1/auth/device/token` keeps the contract: `invalid_grant` for an unknown or exchanged code, `expired_token` after
   the expiry, `slow_down` when polled sooner than `interval` (2 s) after the last poll, `authorization_pending` before
   approval, then exactly one exchange (a conditional update) returns a new `jmu_` for the approver, never the browser's
   session token.
6. `jellymoon login` prints the code and the page, polls at `interval` (+5 s on `slow_down`, RFC 8628), stops with a plain
   message on `expired_token`, `invalid_grant` or `unauthorized`, and gives up at `expiresIn` on its own. It writes the token
   to the config dir's `user-token` (what registering a computer reads) through a 0600 temp file renamed into place, and prints
   only `Signed in as <login>`. The token never reaches argv, stdout, stderr or a log.

GitHub's endpoints come from one base: `https://github.com` and `https://api.github.com`, or `GITHUB_URL` with the GitHub
Enterprise layout (`<base>/login/oauth/*`, `<base>/api/v3/user`). An http override is refused outside ENV=local (the server
exits at startup; the Worker treats sign-in as not configured). `GITHUB_CLIENT_ID` and `GITHUB_CLIENT_SECRET` are server
config, never in the repo; without them the GitHub routes answer `not_found`. Both adapters implement all of it
(`server/src/auth.ts`, `worker/src/auth.ts`).

## Why this shape

GitHub's own device flow would put a GitHub token on the person's machine and make the CLI a GitHub client. Here the
approval is Jellymoon's, so the web client (F18) and the CLI share one sign-in, and the only GitHub credential is a
short-lived one inside the server.

## Not in this unit

- The session cookie on `/v1/pods/:pod/ws` (with an Origin check) and on the other user routes (BSESS). Only `/v1/me` and
  `/device/approve` accept it here.
- Expiry of user tokens. `DeviceTokenResponse.expiresAt` says 30 days, as `tokens.json` does, but no user token expires yet.
- The `/device` page the verification URL names belongs to the web client (F18).
- A limit on how fast device codes can be started.

## Tested by

`e2e/f15_test.go` against a stub GitHub (`e2e/f15_stub_test.go`), both adapters; `daemon/internal/cli/login_test.go` (the poll
loop) and `daemon/internal/config/save_test.go` (the 0600 write).
