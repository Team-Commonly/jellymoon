# The hosted instance

The hosted instance is the `server/` release bundle in one Cloudflare Container, behind a small front Worker, on a Postgres
outside Cloudflare. The decision and its reasons are in [ADR 0010](../adr/0010-hosted-instance.md).

What is in the tree:

- `hosted/Dockerfile`: the image. It builds the bundle with the same command as `make e2e-deployed`, builds the web client, and
  keeps only those two. PGlite is not in it.
- `hosted/src/index.ts`: the front Worker. It forwards every request, WebSocket upgrades included, to one container by a fixed
  name. A cron calls `/healthz` every minute, so a stopped container starts again within a minute. It decides nothing.
- `hosted/wrangler.jsonc`: one container, `max_instances` 1, size `basic`. Nothing account-specific.
- `hosted/ship.sh`: one deploy, called only by the workflow.
- `.github/workflows/hosted.yml`: the `dry-run`, `gate` and `deploy` jobs.

**Never run `wrangler deploy` by hand, from a laptop or anywhere else** (the owner's ruling). Every deploy goes through the
workflow, so every live version is a commit whose CI passed and whose live check ran.

## What CI does

- `dry-run` runs on every pull request that touches `hosted/`, `server/`, `web/`, `podcore/`, `gen/`, the lockfile or the
  workflow. It type-checks the front, runs `wrangler deploy --dry-run` (which builds the image and uploads nothing), boots that
  image on a Postgres service container in the hosted shape, and runs `make deployed-check` against it. It has no Cloudflare
  credential.
- `gate` and `deploy` run after CI passes on a push to `main`, or on a manual run on `main`. They do nothing until the variable
  below is `true`. `gate` checks that CI passed on the commit and that it is still `main`'s head; an older commit is skipped,
  because the newer one deploys on its own run. `deploy` runs `wrangler deploy` with the secrets, waits up to 10 minutes for
  `/healthz`, then runs `make deployed-check DEPLOYED_URL=<the workers.dev URL>`. That check is read-only: it signs nobody in and
  writes nothing.

## Secrets and the variable

Set these as GitHub Actions secrets on the repository, by name. Values never go in the tree, a chat or a log.

| Secret | What it holds |
|---|---|
| `JM_CF_PREVIEW_TOKEN` | The repository's existing Cloudflare API token, scoped to the one account. It was made for the retired preview job and nothing else reads it now. It needs Workers Scripts edit, Containers edit and Account Settings read. Editing a token's permissions in Cloudflare keeps its value, so the secret isn't pasted again. If a permission is missing, `wrangler deploy` names it in the run log |
| `JM_CF_ACCOUNT_ID` | The Cloudflare account id |
| `JM_HOSTED_DATABASE_URL` | The Postgres connection string (below) |
| `JM_HOSTED_GITHUB_CLIENT_ID` | The hosted sign-in OAuth app's client id (ADR 0010, "The sign-in OAuth app") |
| `JM_HOSTED_GITHUB_CLIENT_SECRET` | That app's client secret |
| `JM_HOSTED_VAULT_KEY` | The connector vault key. Back it up outside Cloudflare and the database provider; a lost key loses every connection |
| `JM_HOSTED_PUBLIC_URL` | The https base URL people use: `https://jellymoon.<account subdomain>.workers.dev` until a custom domain |

The one variable is `JM_HOSTED_ENABLED`. Unset, or anything but `true`, and nothing deploys.

`deploy` runs in the GitHub environment `hosted`. GitHub records each run there with its commit and outcome; the rollback reads
that record. The secrets can move to that environment later, with a required reviewer, without changing the workflow.

## The database

Any managed Postgres works (Neon, Aiven or another): Postgres 14 or later, disk encryption at rest, point-in-time recovery of at
least 7 days. The server needs one `DATABASE_URL` with `sslmode=require`, or `sslmode=verify-full` when the provider gives a CA.
Create the database empty. The server creates its own tables at boot (`create table if not exists`); there is no migration step.
Write the provider and region here when they are chosen.

## How to ship

1. Set the seven secrets above.
2. Turn it on: `gh variable set JM_HOSTED_ENABLED --body true -R Team-Commonly/jellymoon`.
3. The next push to `main` deploys once CI passes. To deploy `main` now: `gh workflow run hosted.yml --ref main -R Team-Commonly/jellymoon`.
4. Watch the run. The first deploy is not announced until a person has walked it: signed in, made a pod, and had a fake agent
   complete a task (ADR 0010, "CI deploy" item 4).

## Rollback

When the live check fails, `deploy` deploys the last commit whose deploy succeeded again, image included, runs the same check on
it, and fails the run so the bad commit is visible. `wrangler rollback` is not used: it moves the Worker back a version and
leaves the container image as it is.

The first deploy has nothing to roll back to. If it fails, the run fails and the instance stays as the failed deploy left it.

To go back by hand, revert the commit on `main`; the revert deploys like any other commit. Schema changes are additive only, so
an older bundle runs on a newer schema (ADR 0010, "CI deploy" item 3).

## How to stop it

- Stop deploying: `gh variable delete JM_HOSTED_ENABLED -R Team-Commonly/jellymoon`. The running instance keeps running.
- Stop the instance: delete the container application and the `jellymoon` Worker in the Cloudflare dashboard (Workers &
  Pages, then Containers). The data stays in Postgres. Turning the variable back on deploys it again.

## Not built yet

- The nightly `pg_dump` to R2, and the weekly restore test (ADR 0010, "Backups and restore").
- The custom domain. Its OAuth callback changes with it.
- A run of a rollout's overlap. The boot lock exists (`server/src/db.ts`): a second server on the same database refuses to start
  while the first holds it. Whether a Cloudflare rollout stops the old instance before the new one starts is still open (ADR
  0010, "Still open"). The first deploy shows it: a new instance that keeps refusing on the boot lock means the old one wasn't
  stopped.
- A way for the live check to tell the new image from the old one. A deploy starts the container rollout and does not wait for
  it, so `/healthz` can answer from the old instance for a while.
