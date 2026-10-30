# 0001 · Dev tooling and its dependencies

Status: Accepted (owner ruling,2026-10-29). Rule: every new dependency or module gets a note like this one.

Each dependency below is a dev-only dependency, pinned to an exact version at least two weeks old.
None of them ships in a runtime artifact.

| Dependency | Why | Rejected alternative |
|---|---|---|
| Bun 1.4.4 | Package manager and script runner for the TS workspace (owner ruling) | npm: slower installs, no TS script runner |
| Go 1.27.3 | The daemon's language; ACP's Go SDK | — |
| ajv 8.17.1 | Strict JSON Schema 2020-12 validation of fixtures | Hand-written checks drift from the schema |
| typescript 5.9.3 | Type-checks generated contracts; its AST powers the function-size check | A regex line counter misreads nested functions |
| knip 6.41.0 | Unused files, exports and dependencies on the TS side | ts-prune (unmaintained) |
| dependency-cruiser 18.6.0 | TS module boundaries and cycles | Hand-written import greps |
| staticcheck v0.8.2 (`go run`) | Go unused code and correctness lints | golangci-lint: one more binary to pin, more config |
| deadcode, x/tools v0.51.0 (`go run`) | Unreachable Go functions from `main`; starts enforcing when a main package exists | — |

Go tools run through `go run pkg@version`. They resolve from the module cache, with no install. Go and
Bun themselves are pinned in `.tool-versions` and installed into `.tools/` by `scripts/tools.sh`
(checksum-verified, repo-local), so nothing is installed outside the repo.

The budgets they enforce live in `entropy.json`: a file is at most 400 lines and a function at most 60;
LOC per area is capped; exceptions only shrink unless the exception cites a note here.

The LOC budget for the slice is a deliberate choice to stay small:

| Area | Cap |
|---|---|
| core (`schema/`, `scripts/`) | 1,500 |
| podcore | 1,950 |
| worker | 3,000 |
| daemon | 3,000 |
| e2e | 1,500 |

That is under 11k hand-written lines. Agent-written comparables grew without bounds: herdr reached
291k lines in seven months, and Tutti 1.6M (owner's figures). Raising a cap takes a note here.

- 10-30: podcore 1,000 → 1,750 (rules plus refusal-path tests, PC rounds 1–3). This PR also lowered worker 4,000 → 3,000 and daemon 3,500 → 3,000 in this table and in entropy.json
- 10-30: podcore 1,750 → 1,850, refusal-path tests, PC round 4
- 10-30: podcore 1,850 → 1,950, caller-first refusal rules and the tests that pin them, PC round 6
