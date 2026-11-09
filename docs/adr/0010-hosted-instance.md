# 0010 · The hosted instance runs `server/` in a Cloudflare Container, on a Postgres outside Cloudflare

Status: Accepted, by the owner's ruling, 2026-11-09. Postgres provider: open, the owner's spend decision. It adds no code and
no dependency; it decides where a standing instance runs so the deploy units that follow have one shape to build. Acknowledged
unknowns are named under "Still open": three Cloudflare behaviours this ADR read in the docs but has not run (outbound Postgres
from a container, a WebSocket through the container's front, a rollout's overlap), and the Postgres provider.

## Decision

The hosted instance is the same `server/` bundle a self-hoster runs (ADR 0006, "Deployed form"), packaged as an image and run
as one Cloudflare Container instance, reached through a thin front Worker, on `workers.dev` first and a custom domain later.
Its database is a managed Postgres outside Cloudflare, reached by `DATABASE_URL` over TLS. The Cloudflare Worker adapter
(`worker/`, Durable Objects and D1) is not deployed and retires after the hosted instance has been live for a while (below).

The inner loop does not change: `make e2e`, `make e2e-c`, `make e2e-web` and local runs stay on the laptop and the in-process
Postgres, and nothing a contributor runs waits on Cloudflare.

## The two shapes, counted

"One code path" means a partner who self-hosts and the hosted instance run the same server code. Measured against the tree
on 2026-11-09:

| | (a) `worker/` on D1 and Durable Objects | (b) `server/` in a Container, Postgres outside |
|---|---|---|
| What ADR 0006 says ships | A second adapter kept in the tree, not extended for new features (`docs/adr/0006-pod-state-server.md:9-11`) | The adapter Jellymoon runs; the release bundle is what ships (`0006:72-75`, `docs/adr/0006-pod-state-server.md`) |
| What self-hosters run | Not this: it needs a Cloudflare account, which ADR 0004 keeps optional | This: `bun build server/src/main.ts --target=bun` plus any Postgres |
| Contract tests skipped | `TestF19`, the whole connector layer (`Makefile:176`, `docs/development/e2e-contract.md:223`) | Three object-eviction tests that need a Durable Object to throw away (`Makefile:165`, `e2e-contract.md:204-214`); their rules are proved by restart tests instead |
| Features on one adapter only | Connectors: vault, catalogue pin, write gate, receipts, approvers (`docs/adr/0009-connector-layer.md:15`). The built web client: `server/src/web.ts` serves it, the Worker serves no assets | None that ships to users. The Worker's object-only `/__test` routes have no counterpart (`0006:52-55`) |
| State across a restart | Durable Object storage plus D1; an evicted object reloads and its stored alarm fires | Postgres; `Hub.restore` loads every pod and applies `boot` once (`server/src/hub.ts:51`), proved by `TestKillMidLease`, `TestRestartPastDeadline` and `deployed-boot`'s kill -9 |
| Already exercised as deployed | `wrangler.prod.toml` exists, never deployed (placeholder database id, `worker/wrangler.prod.toml:18`) | CI's `e2e-deployed` boots the bundle on a Postgres service container on every PR (`.github/workflows/ci.yml:217-248`) |
| New work to host | Port the connector layer and the web client to the Worker, keep both adapters in step forever | An image, a front Worker of roughly 40 lines that forwards every request to one named container, and a Postgres |

Shape (a) is two code paths by construction: the hosted instance would run code no self-hoster runs and would lack the
connector layer, which is a headline feature (F19). Shape (b) runs the exact bundle CI already boots and the suite already
passes against. Its Cloudflare-specific part is deploy glue (the image and the front), which decides no rule and holds no
state.

**Pick: (b).** The three strongest reasons:

1. It is the shipped form. ADR 0006 made `server/` the adapter and its release bundle the deployed form; the Worker is a
   second adapter that new features skip (`0006:9-11`, `e2e-contract.md:218-219`).
2. The Worker cannot host what users get today. `TestF19` is skipped there because the connector layer exists only in
   `server/` (`Makefile:176`, `0009:15`), and the Worker serves no web client.
3. It is already tested as deployed. `make e2e-deployed` boots the same bundle on a real Postgres with env config only and
   runs the contract suite against it on every PR; shape (a) has never had a production deploy.

## How the Container is run

- **One instance, never asleep.** One process owns every pod and its single timer (`0006:58-60`, `server/src/hub.ts:38`). The
  front Worker routes every request to one container by a fixed name, `max_instances` is 1, and the inactivity timeout is
  long enough that the container does not sleep: a sleeping container stops the timer, and its disk is ephemeral anyway. A
  cron trigger on the front Worker calls `/healthz` every minute, so a stopped container is started again within a minute.
- **Size.** `basic` (1/4 vCPU, 1 GiB) to start; Containers bill memory and disk while the instance runs and CPU on use, so an
  always-on `basic` is a few dollars a month. A measured load moves it up.
- **No disk state.** Container disk is ephemeral. All state is in Postgres; PGlite is not in the bundle (`0006:74-75`) and
  `startupRefusal` refuses to start without `DATABASE_URL` outside ENV=local (`server/src/config.ts:7`).
- **Proxy and address.** The bundle serves plain http behind Cloudflare's TLS, so `PUBLIC_URL` is the https workers.dev URL
  and `TRUST_PROXY=1`. The front deletes any incoming `X-Forwarded-For` and writes `CF-Connecting-IP` as the only entry, so the
  start limit keys on the real client (ADR 0008, "Hardening" item 2).
- **Hosted env.** `ENV` unset, no `TEST_BEARER` (so no `/__test` handler at all), `GITHUB_CLIENT_ID` and `GITHUB_CLIENT_SECRET`,
  `JM_VAULT_KEY`, `JM_WEB_DIR` pointing at the client built into the image.

## Where user data lives, and what it is

A user's data is everything in the instance's Postgres (`server/src/store.ts:19-36`): their account (`users`, `github_users`:
the GitHub id and login, no GitHub token), tokens (`tokens`, `device_codes`, `oauth_states`, stored only as SHA-256), the pods
they belong to with every message, task and run event (`pods`, `events`), computers and invites, and connectors
(`connections` holding the upstream credential as an AES-256-GCM envelope under `JM_VAULT_KEY`, `grants`, `connector_calls`
receipts, `approvers`; ADR 0009). The container holds none of it between restarts.

- **Region.** The Postgres is in one region, chosen when the provider is chosen and written in the deploy runbook; the
  container is placed by Cloudflare and may run elsewhere. A user-facing statement of where data lives names the Postgres
  region.
- **Encryption at rest.** The provider's disk encryption is a requirement for picking it. On top of that, connector
  credentials are encrypted by the server itself, so a database dump alone opens no upstream account. Connections use TLS
  (`sslmode=require` or stricter).
- **Who can read it.** Only the deploy job (through `DATABASE_URL`) and the named maintainers who hold the database
  console's access, listed in the runbook. `JM_VAULT_KEY` is held only as a Cloudflare secret and a GitHub Actions secret, never
  by a person's laptop.
- **Deletion on request.** No route deletes an account today. Until one is built, a maintainer deletes on request by a
  written procedure (the user's rows, their tokens, their computers, the connections they made, and their membership; pods
  they shared keep other members' messages), within 30 days, and notes the request. Backups age out on their own schedule
  (below), which the privacy note states.

## Backups and restore

- **What.** The whole Postgres database. Nothing else holds state; the image is rebuilt from the tree.
- **How often and where.** The provider's continuous backup with point-in-time recovery of at least 7 days, plus a nightly
  `pg_dump` taken by a scheduled CI job, encrypted, and kept 30 days in an R2 bucket that only that job can write.
- **Restore is tested.** A weekly CI job restores the latest dump into a throwaway Postgres, boots the bundle on it with
  `deployed-boot`'s hosted-shape checks, and reads back a known pod. A restore nobody has run is not a backup.
- **The vault key is backed up separately**, by the maintainers, outside both Cloudflare and the database provider: a dump
  without its key cannot open connector credentials, and a key lost is every connection lost.

## The sign-in OAuth app

The hosted instance has its own GitHub OAuth app, separate from any developer's local one: callback
`<PUBLIC_URL>/v1/auth/github/callback`, no scopes requested (ADR 0008 reads only `/user`). It is owned by the Team-Commonly
GitHub organization, not a person. The client id and secret are GitHub Actions secrets; the deploy job sets them as
Cloudflare secrets on the container's environment. When the custom domain arrives the callback changes, so the app gets a
second callback or a new app then.

## CI deploy (plan only)

1. A `deploy` job runs on pushes to `main` after `e2e-local`, `e2e-deployed` and `e2e-web` pass. It builds the image from
   that commit (tagged with its SHA), runs `wrangler deploy` for the front Worker and the container with an API token scoped
   to this one job, and sets secrets from Actions secrets.
2. **Deployed-boot, read-only, against the live URL.** The existing `deployed-boot` kills and restarts its own process, which a
   live instance cannot allow, so it gains a URL mode: `/healthz` is 200, `/__test/*` is never 200 (404 with no handler,
   401 when a test bearer is set), an unauthenticated user route is 401, and the web client's `index.html` is served. It
   creates nothing, so it leaves out `/v1/auth/github/start` and `POST /v1/auth/device`, which each write a sign-in
   state row and use a start-limit slot; the hosted client id is checked by the walk instead.
3. **Rollback.** A failed live check redeploys the previous good SHA's image, which the job records. Schema changes are
   additive only (`create table if not exists`, new nullable columns), so the previous bundle still runs on a newer schema.
   A change that cannot be additive needs its own runbook before it merges.
4. A walk (a person signs in, makes a pod, a fake agent completes a task) proves the deploy end to end once, before the
   instance is announced.

## Secrets and identifiers

Account ids, API tokens, database names, connection strings, bucket names and the vault key are never committed. CI
supplies them from GitHub Actions secrets; the repo carries only the shape (the variable names). The `wrangler` config for
the container keeps the all-zero placeholder style of `worker/wrangler.toml` where an id is needed.

## Whether `worker/` stays

It goes, later, in order, each step its own change:

1. Now: nothing deleted. `make e2e-a` and CI's `e2e-worker` keep running while the hosted instance is new.
2. After the hosted instance has served real users for a month with no rollback to a Worker: drop `e2e-worker` from required
   checks, then from CI.
3. Then delete `worker/`, `make e2e-a`, `make e2e-a-migrated`, `make worker-dev`, `wrangler.prod.toml` and the `worker`
   paths in the entropy budgets; remove the eviction tests' Worker-only routes from the suite, and update ADR 0004's note on
   `worker/src/entitlements.ts` to name `server/`.

The new front Worker is not `worker/`: it forwards requests and decides nothing.

## Still open

- **Outbound Postgres from a container.** Cloudflare documents internet access as on by default and filtering only ports 80
  and 443, which implies a container can open TLS to Postgres on 5432; Hyperdrive is a Worker binding and is not used by
  `Bun.SQL`. The first deploy unit proves a connection before anything else.
- **A WebSocket through the front.** The pod WebSocket (`/v1/pods/:pod/ws`) must pass the front Worker and the container's
  Durable Object. Proved in the same first unit; if it cannot, this ADR is revisited before any other work.
- **A rollout's overlap.** If a rollout starts the new instance before the old one stops, two processes would own every pod
  at once. Until that is proved not to happen, the server takes a Postgres advisory lock at boot and refuses to serve without
  it (a small server change that self-hosters also get, one code path). An owner whose host vanishes without closing its
  connection would hold that lock until the database notices, so the lock's session sets `idle_session_timeout`, and the
  provider must run Postgres 14 or later.
- **The Postgres provider and region**, and the spend, are chosen in the deploy unit.
- **Sharding.** Unchanged from ADR 0006: one process until a measured load says otherwise.

## Consequences

- One code path: the hosted instance, CI's `e2e-deployed` and every self-hoster run the same bundle.
- The Cloudflare-specific code is an image definition and a small front Worker, neither of which holds state or a rule.
- The hosted instance depends on two vendors (Cloudflare for compute and the edge, a Postgres provider for data). A move of
  either changes deploy glue, not `server/`.
