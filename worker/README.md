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
token expiry and the `rate_limits` and `token_pods` tables (ADR 0008, "Hardening"), `0002_device_deny.sql` adds
`device_codes.denied` (ADR 0008, step 4). Apply them before the Worker that needs
them is deployed:

```
bun ../node_modules/wrangler/bin/wrangler.js d1 migrations apply DB -c wrangler.prod.toml --remote
```

A D1 created from the `schema.sql` before the hardening takes 0000 as a no-op and then 0001 and 0002. A D1 created from a
later `schema.sql` already has what some migrations add, and each of those fails there on a duplicate column: mark 0000 and
0001 as applied when it has `tokens.expires_at`, and 0002 too when it has `device_codes.denied`, once (`migrations list` creates wrangler's `d1_migrations` table; `apply` then has nothing to do):

```
bun ../node_modules/wrangler/bin/wrangler.js d1 migrations list DB -c wrangler.prod.toml --remote
bun ../node_modules/wrangler/bin/wrangler.js d1 execute DB -c wrangler.prod.toml --remote --command "insert into d1_migrations (name) values ('0000_initial.sql'), ('0001_auth_hardening.sql')"
```

A new change to the directory is a new numbered
file here plus the same change in `schema.sql`, which the test entry applies to a fresh local state under ENV=local;
`test/migrations.test.ts` fails when the two disagree, on any table, column, index (UNIQUE and primary keys included) or
foreign key. `make e2e-a-migrated` runs the sign-in suites against local states created from 0000 and then migrated
(`JM_D1_SEED` in `dev.sh`). It proves 0001's and 0002's column changes serve sign-in, and not much more: the test entry still runs
`schema.sql`'s `create table if not exists` at boot, which would make any table a migration forgot. The unit shape test is
what covers the tables.
