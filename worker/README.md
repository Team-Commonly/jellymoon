# worker/

The Cloudflare adapter: a Worker, one Durable Object per pod, and D1 for the directory (users, computers, run tokens, which pods exist). It does I/O, persistence, the alarm, fan-out and the `sinceSeq` log, and nothing else. Every rule lives in `podcore/` (`docs/adr/0003-pure-pod-core.md`); the adapter imports the core through `podcore/src/index.ts` and the core never imports the adapter.

- `src/index.ts` is the production entry. It has no `/__test/*` handler, and `test/bundle.test.ts` asserts that.
- `src/test/index.ts` is the test entry: the same public routes plus `/__test/*` (session, clock, stats), used by local runs and the contract suite.
- `wrangler.toml` points at the test entry, `wrangler.prod.toml` at the production one. The database id in both is an all-zero placeholder; there is no account id, route or zone.

Run it locally (loopback only, no Cloudflare credentials):

```
make worker-dev   # the test entry on http://127.0.0.1:8787, state in .wrangler-data/
make e2e-a        # the black-box contract suite against it (docs/development/e2e-contract.md)
```

`make test-ts` runs the unit and bundle tests in `worker/test/`.

## D1 migrations

A real D1 is created and changed only by wrangler's migrations, from `migrations/` (`migrations_dir` in both configs), in
number order: `0000_initial.sql` is the directory before the sign-in hardening, `0001_auth_hardening.sql` adds the user
token expiry and the `rate_limits` and `token_pods` tables (ADR 0008, "Hardening"). Apply them before the Worker that needs
them is deployed:

```
bun ../node_modules/wrangler/bin/wrangler.js d1 migrations apply DB -c wrangler.prod.toml --remote
```

A D1 created earlier from `schema.sql` takes 0000 as a no-op and then 0001. A new change to the directory is a new numbered
file here plus the same change in `schema.sql`, which the test entry applies to a fresh local state under ENV=local;
`test/migrations.test.ts` fails when the two disagree. `make e2e-a-migrated` runs the sign-in suites against local states
created from 0000 and then migrated (`JM_D1_SEED` in `dev.sh`).
