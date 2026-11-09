# 0006 · The pod-state server is one Bun process, Hono and Postgres

Status: Accepted, by the owner, 2026-11-08. It adds `server/` and two runtime dependencies (`hono`, `@electric-sql/pglite`),
so it needs this note. It chooses between two adapters of the pure pod core (ADR 0003) and leaves two questions open (below).

## Decision

The server Jellymoon runs is `server/`: an adapter around `podcore/` that is one Bun process, Hono for HTTP and the pod
WebSocket, and Postgres for storage. The Cloudflare Worker (`worker/`, Durable Objects and D1) stays in the tree as a second
adapter of the same contract. `make e2e` and CI's `e2e-local` now run the contract suite against the server; `make e2e-a`
still runs it against the Worker. Removing `worker/` is a later, separate unit.

Why this one: a spike built the same server on both stacks under one frozen rule. Both passed every gate. The Worker arm
needed one fix iteration and this one needed none. Self-hosting stays free (ADR 0004): the server runs on a laptop with no
cloud account, and `server/` is core, not `ee/`.

## What it is

An adapter. It does I/O, persistence, the timer, fan-out and the `sinceSeq` log. It decides no rule: every request becomes a
core command, and every answer and event comes out of the core. It meets the five duties of ADR 0003, "What an adapter must do":

| Duty | Where |
|---|---|
| 1. Persist atomically with emitting | `PodRuntime.exec`: apply the core, then write the state and its members-events in one transaction, then fan out, then answer. A command that changed nothing writes nothing |
| 2. Own the single timer | `Hub.rearm`: one `setTimeout` at the earliest `nextDeadline` of any pod, re-armed after every command. A fire applies `tick` to each due pod |
| 3. Own the seq log and replay | the `events` table; `subscribe {sinceSeq}` reads it in batches of 500. Live events that land meanwhile are buffered and deduplicated by seq, so there is no gap |
| 4. Authenticate and mint ids | `jmu_`, `jmc_` and `jmr_` tokens are random and stored only as SHA-256; a run token is bound to its pod. Ids are a timestamp plus 80 fresh random bits, never a counter, and each command gets a new wake seed. The adapter keeps the wake and run indexes the core cannot: `POST /v1/runs {wakeId}` names no pod |
| 5. Boot | `Hub.restore`: load every pod, apply `boot {staleMs: 0}` once, then set every computer the pod held online to offline, because a restarted process has no sockets. Every command is persisted, heartbeats included, and saves are never coalesced |

The server serves `POST /mcp` like the Worker: `server/src/mcp.ts` is a twin of `worker/src/mcp.ts`, adds no behaviour, and sends each tool call through the server's own routes with the caller's token. Like the Worker, it tells the pod about each run's `tools/call` before dispatching it (the core's `tool_call`), and a post is the core's `post` command, so a receipt counts tool calls exactly as on the Worker; a request the adapter dispatched itself is recognised by object identity, not by a header a caller could forge.

With `JM_WEB_DIR` set (in any mode), the server also serves the built web client from that directory (`make e2e-web` points it at `web/dist`): a GET that no route answered gets the file of that path inside the build, or `index.html` so the client's router takes the path. `/v1`, `/__test`, `/mcp` and `/healthz` never fall through to it, and nothing outside the directory is served (a NUL byte is refused and symlinks are resolved before the containment check). A `JM_WEB_DIR` with no `index.html` is refused at startup; if the file goes later, a path with nothing to serve answers the JSON 404. `index.html` is sent `no-cache`, `/assets/*` (hashed by the build) `immutable`, and every file `nosniff`. Unset, there is no client and every unknown path keeps the JSON 404 (`server/src/web.ts`, 2026-11-08, F18 PR 5a).

One writer per pod is a per-pod promise chain; no lock spans pods. Request bodies are checked against `gen/ts/schemas.json`
with the same walker the Worker uses, before anything runs, and a field the core has no place for is refused (400), never
dropped. Invite codes are stored as SHA-256 and the join link is built from `PUBLIC_URL`, else the request's own origin.

## Storage

Production is a real Postgres, reached by `DATABASE_URL` through Bun's built-in `Bun.SQL`, so the choice adds no driver
package. PGlite (in-process Postgres compiled to WASM, Apache-2.0) is allowed only for `make e2e` and a clean shell: without
`DATABASE_URL` the server refuses to start unless `ENV=local`. Both sit behind `server/src/db.ts` (`rows`, `exec`, `tx`), and
`store.ts` holds every statement in plain Postgres SQL. Pod state is stored as `text`, not `jsonb`: jsonb reorders object keys,
and creation order is the order of the state's keys.

CI runs the suite against a Postgres service container. Each server gets its own schema, so every test starts fresh.

## Test routes

`ENV=local` opens `/__test/session`, `/__test/clock` and `/__test/evict/:pod`. Otherwise they exist only when `TEST_BEARER` is
set, each needs `Authorization: Bearer <TEST_BEARER>`, and there is no clock route. A build with neither has no `/__test`
handler at all. The Worker's object-only routes (`discard`, `instance`, `stored-alarm`, `woke`) have no counterpart here,
because there is no object to throw away. The three tests that need them are skipped on the server, each with its reason in
`docs/development/e2e-contract.md`.

## Still open

- **Sharding across processes.** One process owns every pod and its timer. Spreading pods over several processes needs a pod
  to process assignment and a shared timer story, and none is built. One process is the supported shape until a measured load
  says otherwise.
- **Hosting.** Where the process and its Postgres run in production is not decided here, and nothing in `server/` assumes a host.

## Consequences

- `make preflight` runs the contract suite against the server. The Worker's run is `make e2e-a`.
- The server is capped at 1,800 lines including tests (ADR 0001, `entropy.json`).
- The core exports no read model, so the adapter derives `taskView` and `hello` itself (about 30 lines in `views.ts`). The Worker
  does the same.
- Not served by the server: the device flow, evidence, decisions and `ask` posts (stored as plain messages). The Worker does
  not serve them either. Both adapters serve `POST /mcp`.

## Deployed form (2026-11-08)

The Cloudflare preview that CI once planned (F17) tested the Worker, which is no longer what ships. What ships is `server/`, so the
per-PR check runs that: `make e2e-deployed` builds the release bundle (`bun build server/src/main.ts --target=bun`, with `pglite`
left out because it is the local fallback only), and CI's `e2e-deployed` job runs it against a Postgres service container.

- **Boot, by URL.** `make deployed-boot` starts the bundle as its own process with env config only, waits for `GET /healthz` and
  drives it over HTTP. It refuses to start with no database. In the hosted shape (`DATABASE_URL` and `TEST_BEARER` set, `ENV`
  unset) `/__test/*` without the bearer is 401 and there is no clock route. A user's pod is created and read through the real
  routes and survives `kill -9` and a second start. A server that exits at boot (a wrong `DATABASE_URL`) or never answers fails the
  job; nothing in it skips. With `DATABASE_URL` unset (a laptop with no Postgres) the same checks run in the local shape on the
  in-process Postgres.
- **The suite.** It then runs with the bundle as the server under test, each test starting and killing its own bundle process. It
  cannot run against one already-running URL: the tests kill and restart the server, want fresh state each, and set a fake clock,
  and a hosted build must not serve a clock route. So the suite boots the bundle with `ENV=local` and a throwaway schema
  (`JM_PG_SCHEMA`), the one combination `startupRefusal` allows with a database, and the hosted shape is checked by `deployed-boot`
  alone. The two checks together cover the deployed form; neither stands in for the other.
- **Not covered.** A real hosted deploy needs a hosting target and spend, and is undecided (see "Hosting" above).
