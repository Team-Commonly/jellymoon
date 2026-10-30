# Standing orders

Paste verbatim into every worker and verifier spawn.

1. Program: Jellymoon, repository Team-Commonly/jellymoon, base branch `main`. Units come from `feature_list.json`; that file,
   `progress/` (with `progress.md` for older entries) and `init.sh` are the in-repo source of truth.
2. One long-running coordinator session builds Jellymoon with a defined engineering loop. Workflow comes before building.
3. Connectors are open-source-first: check for MIT or Apache projects that already do the job before building. F19 waits
   for the reviewed options doc.
4. The client is web-first, verification is an API-level e2e, and the Go daemon is headless with a minimal CLI. F18 (client) and
   F19 (connectors) stay blocked until the maintainers review their options docs.
5. Rulings in force: Bun is the package manager and runner for TS; Worker and Durable Object tests run on vitest-pool-workers;
   the runtime is workerd; ACP adapters run under Node. Never cut the connector design to hit the engine slice's date.
6. The coordinator owns the program, never the code. Workers are background subagents, each in its own git worktree and
   branch named for the unit, with `make hooks fence=1` so they cannot push. Verifiers are separate subagents on a different model
   than the worker.
7. Verification bar: a feature passes only on its `test` command from `feature_list.json`, run against the real Worker and daemon.
   CI green is an input, not a verdict. Ledger rows key on PR and head SHA.
8. Merge: a PR lands only through `node scripts/land.mjs <pr>` with the trust gate (the PR author and every push-capable login
   in `JM_TRUSTED_LOGINS`). Every land needs both the ledger PASS at the exact head and the review's pod verdict at the head.
   Scans only refuse, they never waive either. A failing check run blocks; no check runs is fine.
9. Workers commit on their own branch and stop. They never push, merge, rebase, reset, force-push or retarget PRs. The
   coordinator pushes, opens PRs and merges.
10. Commits use the maintainers' GitHub noreply identity, set in each worktree, with a DCO sign-off. The repo's pre-commit runs
    via `make hooks`. End commits with the trailers the brief gives.
11. Host: no global installs. Go and Bun come from the repo-local `.tools/`; npm and bun dependencies stay repo-local. Never
    print tokens.
12. At most 3 background workers at once, for host memory.
13. Never put infrastructure identifiers (cloud project, registry hosts, account ids) in the repo. `scripts/check-private.mjs` enforces it.
14. Escalate to the owner only for irreversible actions, product calls no experiment settles, and quota near its weekly limit.
    Everything else: act and log.
15. No product code until the loop passes its pilot and the three designs (protocol, connectors, client) are reviewed.
16. Maintainability and entropy control are part of the loop. `make preflight` includes `make entropy` and `make secrets`;
    budgets live in `entropy.json` and only ratchet down. Every unit report states lines added and removed and what it deleted
    (subtract before you add). A new dependency or module needs a `docs/adr` note. A budget grows only by a new feature's own
    contract test in that feature's unit, or by a dated note in ADR 0001's table; never for refactors and never by deleting tests.
17. The product is Jellymoon, by Commonly; the company stays Commonly.
18. The repository goes public on the owner's call, with a clean full-history secret scan first.
19. Review: one fresh-context review subagent per PR head (`docs/development/verifier.md`), on a different model from the worker;
    its PASS lands the unit, and the operator posts it as the pod verdict. The brief names the unit brief and FAILs any rule that
    contradicts an Accepted ADR (`.loop/gates.md`, B-ADR). The coordinator names each verifier's scratch dir (`<unit>-<sha8>`);
    verifiers never pick a path (two once shared one and wiped each other's clone).
20. Disk: after a unit passes and its branch matches the remote, the coordinator removes its worktree and its verifier scratch
    dir. Each unit worktree holds about 517 MB of `.tools`; leftovers cost gigabytes on a host that was already losing disk.
    Keep the shared Go cache.
21. The client's visual direction is dark-first. The v4 web client (F18) starts from that shell.
