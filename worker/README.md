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
