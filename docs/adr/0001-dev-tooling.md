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
| gitleaks v8.30.1 (`go run`) | Secret scan of the full history on every preflight, and of a PR's commit range on land with `--ignore-gitleaks-allow`. It can only refuse, never waive: it skips binaries, images, its own allowlisted paths (`node_modules`, `bower_components`, `gitleaks.toml`), lockfiles and `go.sum`, never reads commit messages in git mode, and reads files in 100,000-byte chunks with no match across a cut, so a scanner cannot prove a secret absent. Every land therefore needs the ledger PASS and the review's pod verdict, and a scan hit or a failed scan refuses (2026-10-30, P08 r16, after four scanner blind spots each became a way to land without the review) | Hand-written regexes miss token formats |
| Node 20 or newer (land's runtime, not pinned) | `scripts/land.mjs` runs under `node`, never bun, so bun does not auto-load a `.env` from the checkout; land refuses to start on an older Node | bun: loads `.env` and `bunfig.toml` from the cwd into the environment |
| deadcode, x/tools v0.51.0 (`go run`) | Unreachable Go functions from `main`; starts enforcing when a main package exists | — |

Go tools run through `go run pkg@version`. They resolve from the module cache, with no install. Go and
Bun themselves are pinned in `.tool-versions` and installed into `.tools/` by `scripts/tools.sh`
(checksum-verified, repo-local). The tools themselves are repo-local, but Go and Bun keep their caches (module cache, build cache, bun install cache) under HOME, so a first run writes there; the preflight points HOME at its throwaway dir.

The budgets they enforce live in `entropy.json`: a file is at most 400 lines and a function at most 60;
LOC per area is capped; exceptions only shrink unless the exception cites a note here.

The LOC budget for the slice is a deliberate choice to stay small:

| Area | Cap |
|---|---|
| core (`schema/`, `scripts/`) | 2,200 |
| worker | 4,000 |
| daemon | 3,500 |
| e2e | 1,500 |

The caps add up to 11,200 lines. Agent-written comparables grew without bounds: herdr reached
291k lines in seven months, and Tutti 1.6M (owner's figures). Raising a cap takes a note here.

2026-10-30: core cap 1,500 → 1,650, land security tests, P08 r9.
2026-10-30: core cap 1,650 → 1,750, land security tests, P08 r10.
2026-10-30: core cap 1,750 → 1,800, land security tests, P08 r11.
2026-10-30: core cap 1,800 → 2,000, land I/O seam and end-to-end land cases, P08 r12.
2026-10-30: core cap 2,000 → 2,100, id-keyed verdict, every-range-blob scan and kill/trust/scrub tests, P08 r14.
2026-10-30: core cap 2,100 → 2,200, text-by-land rule, padded range scan, per-magic and end-to-end tests, P08 r15.
2026-10-30: gitleaks row rewritten: scans refuse and never waive; the verdict-free path and its ~320 core lines were deleted, core is 1,787 under the 2,200 cap, P08 r16.
