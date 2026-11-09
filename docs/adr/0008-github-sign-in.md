# 0008 · Sign-in: Jellymoon's own device code, approved in a browser that GitHub signs in

Status: Accepted, by the owner's ruling, 2026-11-08 (the contract wins over F15's first wording). It adds no dependency. Open
below: the `/device` page itself. The browser session on every user route was added on 2026-11-08, and the hardening (token
expiry, the start limit, the sweep, timeouts, logout with both and its sockets) on 2026-11-09 (both below), with its follow-ups
the same day (TRUST_PROXY behind TLS, user tokens with no expiry, the D1 migrations and three fixes around logout's sockets:
Hardening items 2, 1, 7 and 6).

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
   `unauthorized`; any other GitHub failure answers `upstream` (502) and echoes nothing GitHub sent. Each of the two calls to
   GitHub gives up after 10 seconds (`AbortSignal.timeout`), which is `upstream` too. On success it mints a
   `jmu_` session, sets it as an httpOnly, SameSite=Lax cookie (Secure except under ENV=local), and redirects to `next`.
4. `POST /v1/auth/device/approve {userCode}` is the only approver; no page load approves. It takes the session cookie or a
   Bearer `jmu_` and needs `Content-Type: application/json`; by cookie it needs the server's own Origin, a missing one
   included, like every cookie write (Browser session, item 3). The user code matches in either case, with or without its
   hyphen: the server upper-cases it and puts the hyphen back before hashing. It binds the code to the caller: unknown
   `not_found`, expired `expired_token`, already approved `gone`.
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

## Browser session

A browser that signed in through step 3 uses the pod with its session cookie alone, so the web client's pod and invite
screens work against the real server. One authentication function per adapter (`authOf` in `server/src/kit.ts` and
`worker/src/auth.ts`) decides the caller of every route:

1. A request with an `Authorization` header uses it as a Bearer and ignores any cookie; Bearer requests have no Origin or
   content-type rule. Else, on a route whose auth includes `user`, the `jm_session` cookie is the token.
2. The cookie must hold a `jmu_`. Any other token in it (`jmr_`, `jmc_`) is no caller: `unauthorized`. A cookie session is the
   same `jmu_` as a Bearer, with the same membership checks and the same errors.
3. CSRF. A cookie request that changes state (any method but GET, HEAD and OPTIONS) needs `Content-Type: application/json`
   and an `Origin` equal to the server's own origin: the origin of the base the Decision's item 1 computes (`ownBase` in each
   adapter's `auth.ts`, the one function approve and the verification URL also use). A missing Origin is refused too: browsers send it on
   every cross-origin request and on same-origin POST, PATCH and DELETE. Both refusals are `forbidden`.
4. The pod WebSocket (`GET /v1/pods/:pod/ws`) takes the cookie only when the upgrade's Origin equals the server's own origin,
   against cross-site WebSocket hijacking: SameSite=Lax keeps the cookie off a socket opened from another site, but not off
   one opened from a sibling origin of the same site, and a socket has no CORS. A token never rides in the query string. A computer's `jmc_` Bearer is unchanged.
5. Routes the cookie reaches: `POST /v1/pods`, `POST /v1/pods/:pod/invites`, `POST /v1/invites/:code/accept`,
   `POST /v1/computers`, the pod socket, `GET /v1/me`, `POST /v1/auth/logout`, and every operation whose auth includes `user`
   (`whoami`, `read`, `task_list`, `task_update`, `task_release`, `run_stop`, `post`; `task_claim` is run-only, and `evidence`
   is not served). Bearer-only: `POST /v1/pods/:pod/agents`, `POST /v1/runs` and `/v1/runs/:run/end` (computer), and
   `POST /mcp`. MCP is for agents and their harnesses, not a page: no browser screen calls it, and it re-sends the caller's
   `Authorization` on every tool call, so a cookie there would add a CSRF surface and nothing a person needs.
6. `POST /v1/auth/device/approve` is a cookie write like any other (until 2026-11-09 it allowed a missing Origin; that
   exception is gone, and TestF15 approves with the Origin a browser sends).
7. `POST /v1/auth/logout` (cookie or Bearer `jmu_`) sets `revoked_at` on the token the request presented, so it stops
   authenticating at once as cookie and as Bearer, and clears the cookie. A request that carries a Bearer and the cookie
   revokes both tokens. A pod socket open with a revoked token is closed (code 1008), and one that was opening when the
   token was revoked is closed or refused once it opens (Hardening, item 6, which names the one race left). Other tokens of
   the same person, and their sockets, stay live.

## Hardening

Added 2026-11-09 (AUTHH), in both adapters.

1. **User token expiry.** A `jmu_` is stored with `expires_at`, 30 days after it was minted (the `expiresAt` the device
   exchange returns and the cookie's Max-Age). Past it the token is no caller on every route and on the socket upgrade, as a
   Bearer and as the cookie: `unauthorized`. Computer and run tokens keep a null expiry, which never expires. A user token
   with a null expiry is expired: every `jmu_` is minted with one, so a null one was minted before this, and its holder signs
   in again. That was chosen over a backfill (now + 30 days at migration) because it fails closed for any row that ever lacks
   one, needs no clock in a migration, and costs nothing real: no instance had been deployed, so only dev databases held such
   tokens. A socket already open when its token expires stays open until it closes.
2. **The start limit.** `POST /v1/auth/device` and `GET /v1/auth/github/start`, the two routes anyone can call, each take 30
   requests per client address per fixed minute (`AUTH_START_LIMIT` changes it). Past it they answer `rate_limited` (429)
   with `Retry-After`, the whole seconds left in the minute. The server keys on the socket's address by default; a client can
   write `X-Forwarded-For`, so it is read only with `TRUST_PROXY=<n>`, the number of proxies in front, each appending the
   address it saw: the client is then the n-th entry from the right (the last one for `1`, one TLS proxy), and anything left
   of it is the caller's own writing. Fewer entries than n, or `0`, is the socket's address. Bun serves no TLS, so an https
   `PUBLIC_URL` outside ENV=local means a proxy in front, and with the default every client would share its address: one
   client's 31 starts would block every sign-in. There the server refuses to start until `TRUST_PROXY` is set (a hop count,
   or `0` to keep the socket's address on purpose). A refusal, not a boot warning: a warning scrolls past in a deploy log,
   and the failure it warns of is an outage nobody connects to it. It counts in memory, which fits one process (ADR 0006).
   The Worker keys on `CF-Connecting-IP`, which Cloudflare's edge sets, never on `X-Forwarded-For`, and counts in the D1
   table `rate_limits`, one row per route and address, dropping older minutes as it goes; an isolate's memory would not be
   shared.
3. **The sweep.** Device codes and OAuth states are deleted an hour after they expire, so a poll just after expiry still
   reads `expired_token`; a swept code is `invalid_grant`. The server sweeps at boot, on its timer's tick and on each start;
   the Worker on each start, in the start limit's batch.
4. **PUBLIC_URL.** With `GITHUB_CLIENT_ID` set outside ENV=local, the server refuses to start without `PUBLIC_URL`: the
   OAuth redirect URI and the session's own Origin must not come from a request's Host. The Worker has no startup hook, so
   there both GitHub routes answer `internal` (500) with a message naming `PUBLIC_URL`. Outside ENV=local the server also
   refuses a `PUBLIC_URL` that is not an https URL, reading the scheme as `new URL` parses it (`HTTPS://` is https, and so
   needs `TRUST_PROXY`, item 2): `ownBase` upgrades an http base to https there, so an http `PUBLIC_URL` names an origin
   no browser session could match, and the session cookie is Secure.
5. **A concurrent first sign-in.** Two callbacks for one new GitHub account make one user. The server inserts the
   `github_users` row with `on conflict do nothing` and writes the user only if it won, else reads the winner's; the Worker
   writes the user only while no row holds the GitHub id, in one D1 batch (one transaction).
6. **Sockets of a revoked token.** Logout revokes first and closes second, so a socket is safe if it is registered for
   the close before its token is checked one last time: either that check sees the revoke, or the close comes after it and
   finds the socket. The server keeps a close for each open socket by its token's hash, and drops a hash when its last
   socket closes, so the map holds only tokens with an open socket. The upgrade authenticates, but the socket registers
   later (`onOpen`), so `onOpen` reads the token again after registering, and a revoked or expired one is closed (1008)
   before hello and before any frame. The Worker tags each socket in the pod object with `t:<token hash>` and records the
   pod in the D1 table `token_pods` before it hands the upgrade to the pod. A pod that does not open the socket leaves the
   row, since a failing dial deleting it could take the row of a socket of the same token that opened meanwhile, and the
   open socket would outlive a logout; a stale row costs logout one close to a pod with no sockets for that tag. Rows go
   with their token: logout deletes them, and the start-limit sweep deletes those of an expired or revoked token. A logout can still slip in twice: its revoke
   and its `token_pods` read between the Worker's authentication and the row, or its close reaching the pod before the pod
   accepts the socket. So once the pod answers 101 the Worker authenticates the token again; if it is now revoked or
   expired it asks the pod to close the token's sockets, deletes the row and answers `unauthorized` instead of the 101.
   The pod may have sent hello on that socket in between, to the person who just signed out. Logout reads `token_pods` and
   asks each pod to close the tagged sockets, with `Promise.allSettled`: the revoke is what signs out, so a pod that fails
   to close answers nothing to the person, and the cookie is cleared all the same. On the server a socket that throws after
   the pod attached it (hello failing to send, say) is closed 1011 and still detached when it closes: `attached` is tracked
   apart from `live`. Logout whose only credential is a dead (expired or revoked) session cookie answers 200 `{}` and clears
   the cookie: the sign-out already happened, so it reads as one, and the browser stops sending the dead cookie. It revokes
   nothing, having authenticated nothing; a dead Bearer, with or without a cookie, stays 401.
7. **D1 migrations.** A real D1 is created and changed only by `wrangler d1 migrations apply` on `worker/migrations`
   (`migrations_dir` in both wrangler configs): `0000_initial.sql` is the schema before this hardening, `0001_auth_hardening.sql`
   adds `tokens.expires_at` (no backfill, item 1) and the tables `rate_limits` and `token_pods`. A D1 created from the old
   `schema.sql` takes 0000 as a no-op and then 0001. A D1 created from today's `schema.sql` already has
   `tokens.expires_at`, so 0001 fails there on a duplicate column: mark both as applied instead (worker/README.md, "D1
   migrations", gives the step). `schema.sql` stays the test entry's fresh shape under ENV=local, and a
   unit test holds it equal to the migrations applied in order. The server migrates its own database at boot.

## Not in this unit

- The `/device` page the verification URL names belongs to the web client (F18).
- A refresh of a user token: after 30 days a person signs in again.
- A socket already open stays open after its token expires; `jmc_`
  tokens never expire and logout does not revoke them (this predates the hardening).
- Logout with a `jmc_` bearer, or with a dead Bearer, answers 403 or 401 (this predates the hardening).

## Tested by

`e2e/authh_test.go` (the hardening, both adapters; its start-limit step varies `X-Forwarded-For` on every request),
`server/test/auth.test.ts` and `worker/test/auth.test.ts` (the timeouts, PUBLIC_URL, the boot sweep, six concurrent first
sign-ins); `server/test/signin-hardening.test.ts` and `worker/test/signin.test.ts` (TRUST_PROXY, the PUBLIC_URL scheme,
null expiry, hub.sockets through real sockets, the limiter key, the token_pods order, logout with a failing pod, and a
logout racing a socket's open: on the server between the upgrade and `onOpen`, on the Worker before the row and before the
pod accepts); `worker/test/migrations.test.ts` (columns, indexes with UNIQUE and primary keys, foreign keys) and
`make e2e-a-migrated` (a D1 from the old schema, migrated, serves TestF15, TestBSESS and TestAUTHH); `e2e/bsess_test.go` (the browser session, both adapters); `e2e/f15_test.go` against a stub GitHub (`e2e/f15_stub_test.go`), both adapters; `daemon/internal/cli/login_test.go` (the poll
loop) and `daemon/internal/config/save_test.go` (the 0600 write).
