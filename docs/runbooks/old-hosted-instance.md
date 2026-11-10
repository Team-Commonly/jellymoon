# The old hosted instance

The product this repo held before the restart ([ADR 0012](../adr/0012-restart-as-the-journal-app-in-rust.md)) had one
hosted instance. Removing its code from `main` did not stop it. Treat it as still running, and still billed, until
someone has done "How to stop it" below. Whether and when to stop it is the owner's decision.

## What runs where

- A Cloudflare Worker named `jellymoon`, the front. It forwards every request to one Cloudflare Container and a cron
  wakes that container every minute, so a container that is only stopped starts again within a minute.
- One Cloudflare Container (one instance, size `basic`) running the old server bundle and web client.
- A managed Postgres outside Cloudflare (Google Cloud SQL, public IP, TLS only) holding all of its data.
- A GitHub OAuth app used for its sign-in.
- Address: `https://jellymoon.<account subdomain>.workers.dev`. No custom domain was set up.

The last commit deployed is `df20848`, the tag `harness-final`. The config that built it is at that tag: `hosted/`,
`.github/workflows/hosted.yml` and `docs/runbooks/hosted-instance.md`.

## What this repo no longer does

Nothing on `main` deploys, rolls back or checks the instance: the deploy workflow and its config were removed in the
same commit as the server code. No push here can change what is running. The instance keeps the old commit until it is
stopped by hand.

## How to stop it

1. Turn the deploy switch off: `gh variable delete JM_HOSTED_ENABLED -R Team-Commonly/jellymoon`. With the workflow
   gone from `main` nothing reads it; deleting it makes the settings say what is true.
2. In the Cloudflare dashboard, under Workers & Pages: delete the container application, then the `jellymoon` Worker.
   Delete the Worker too, or its cron starts the container again.
3. Decide about the data before touching Postgres. Deleting the Worker and the container loses nothing; the data stays
   in the database. Take a dump first if anything in it is wanted, then delete the database instance.
4. Delete the GitHub OAuth app, or at least revoke its client secret.
5. Revoke the Cloudflare API token the deploys used.
6. Remove what is now unused in the repository's settings (next section).

## Unused after the restart

Repository variable: `JM_HOSTED_ENABLED`.

Repository secrets: `JM_CF_PREVIEW_TOKEN`, `JM_CF_ACCOUNT_ID`, `JM_HOSTED_DATABASE_URL`, `JM_HOSTED_DB_CLIENT_CERT`,
`JM_HOSTED_DB_CLIENT_KEY`, `JM_HOSTED_DB_SERVER_CA`, `JM_HOSTED_DB_TLS_SERVERNAME`, `JM_HOSTED_GITHUB_CLIENT_ID`,
`JM_HOSTED_GITHUB_CLIENT_SECRET`, `JM_HOSTED_VAULT_KEY`, `JM_HOSTED_PUBLIC_URL`, and `JM_PRIVATE_NAMES` if it was ever set
(a CI job that no longer exists read it).

The GitHub environment `hosted`, which holds the record of past deploys.

`JM_HOSTED_VAULT_KEY` decrypts the connections stored in the database. Keep a copy for as long as a dump of that
database is kept; without it the dump's stored connections cannot be read.

## To bring it back

Check out `harness-final` in a separate clone and follow `docs/runbooks/hosted-instance.md` there. It cannot be deployed
from `main`.
