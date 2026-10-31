# 0001 · Dev tooling and its dependencies

Status: Accepted (maintainers, 2026-10-29). Rule: every new dependency or module gets a note like this one.

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
| gitleaks v8.30.1 (`go run`) | Secret scan of the full history on every preflight, and of a PR's commit range on land with `--ignore-gitleaks-allow`. It can only refuse, never waive: it skips binaries, images, the paths in the pinned version's built-in allowlist, lockfiles and `go.sum`, never reads commit messages in git mode, and reads files in 100,000-byte chunks with no match across a cut, so a scanner cannot prove a secret absent. Every land therefore needs the ledger PASS and the review's pod verdict, and a scan hit or a failed scan refuses (2026-10-30, land review round 16, #10, after four scanner blind spots each became a way to land without the review) | Hand-written regexes miss token formats |
| wrangler 4.151.0 | Runs the Cloudflare adapter (`worker/`) locally under `wrangler dev --local` for `make worker-dev` and `make e2e-a`; loopback only, no Cloudflare account or credentials. Its miniflare 5.20261012.0-beta is patched (`worker/patches/`) to import `undici/index.js` under Bun | Hand-rolled Durable Object and D1 fakes drift from the real runtime |
| @cloudflare/workers-types 5.20261014.1 | Types for the Worker, Durable Object and D1 APIs in `tsc` | Hand-written ambient declarations |
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
| core (`schema/`, `scripts/`) | 3,000 |
| podcore | 2,500 |
| worker | 1,790 |
| daemon | 3,000 |
| e2e | 3,930 |

The caps add up to 14,220 lines. Agent-written comparables grew without bounds: herdr reached
291k lines in seven months, and Tutti 1.6M (owner's figures). Raising a cap takes a note here.

- 10-30: podcore 1,000 → 1,750 (rules plus refusal-path tests, pod core review rounds 1–3, #12). This PR also lowered worker 4,000 → 3,000 and daemon 3,500 → 3,000 in this table and in entropy.json
- 10-30: podcore 1,750 → 1,850, refusal-path tests, pod core review round 4, #12
- 10-30: podcore 1,850 → 1,950, caller-first refusal rules and the tests that pin them, pod core review round 6, #12
- 10-30: podcore 1,950 → 2,050, the differential caller property test and its regression rows (one caller derivation, pod core review round 8, #12)
2026-10-30: core cap 1,500 → 1,650, land security tests, land review round 9, #10.
2026-10-30: core cap 1,650 → 1,750, land security tests, land review round 10, #10.
2026-10-30: core cap 1,750 → 1,800, land security tests, land review round 11, #10.
2026-10-30: core cap 1,800 → 2,000, land I/O seam and end-to-end land cases, land review round 12, #10.
2026-10-30: core cap 2,000 → 2,100, id-keyed verdict, every-range-blob scan and kill/trust/scrub tests, land review round 14, #10.
2026-10-30: core cap 2,100 → 2,200, text-by-land rule, padded range scan, per-magic and end-to-end tests, land review round 15, #10.
2026-10-30: gitleaks row rewritten: scans refuse and never waive; the verdict-free path and its ~320 core lines were deleted, core is 1,787 under the 2,200 cap, land review round 16, #10.
2026-10-30: core cap 2,200 → 2,450, the pre-public deny check (`scripts/check-private.mjs` and its cases) on top of land review rounds 19 and 20; merged, core measures 2,236, unit OS1.
2026-10-30: worker cap 3,000 → 1,250 and e2e cap 1,500 → 1,900, the Cloudflare adapter and the contract suite now in the tree at 1,097 and 1,641 lines (each cap is its count plus about 15%); the total is 10,400. Core stays 2,200 and podcore 2,050. W0.
2026-10-30: worker cap 1,250 → 1,550, input validation against gen/, hashed invite codes, random ids and the test-clock reset now in the Cloudflare adapter at 1,348 lines (the cap is its count plus about 15%); the total is 10,700. e2e stays 1,900 at 1,888. W0 round 2.
2026-10-30: the adapter merged with the core cap raise above (core 2,450): the total is 10,950. W0.
2026-10-30: e2e cap 1,900 → 2,660, the pod feature tests (F03 to F06) and the review fixes to them (computer sockets, refusal-leaves-no-state checks, expired invite, eviction status, poster seqs) take the suite to 2314 lines (the gate printed `LOC e2e 2314 / 2270` before this raise; the cap is that count plus about 15%); the total is 11,710. No other cap changes. W1.
2026-10-30: core cap 2,450 → 2,950, land runs gitleaks itself (`scripts/land-scan.mjs`) instead of through make, refuses paths gitleaks cannot parse, and the cases that prove both (the shape table, the steering fixtures, the env and argv cases), on top of the merge with main; the gate printed `LOC core 2569 / 2450` before this raise and the cap is that count plus about 15%; the total is 12,210. No other cap changes. P08F round 2.
2026-10-30: core cap 2,950 → 3,000, land refuses a tracked symbolic link whose target leaves the checkout or leads into its `.git` or `.tools` (read lexically in every tree land checks, and resolved on disk for the links main tracks) and a `.tools` at any depth in those trees, with the real-scan cases that prove both; the gate printed `LOC core 2973 / 2950` before this raise; the total is 12,260. No other cap changes. P08F round 5.
2026-10-30: e2e cap 2,660 → 3,130, the F08 test (a built daemon run as a black box: registration, presence online and offline, a hung daemon, a crashed server) takes the suite to 2,714 lines (the gate printed `LOC e2e 2700 / 2660` before this raise; the cap is the final count plus about 15%); the total is 12,630. No other cap changes. F08.
2026-10-30: podcore cap 2,050 → 2,500, presence (which computers are online, the 45 s staleness bound, the presence events) and its tests take the core to 2,162 lines (the gate printed `LOC podcore 2162 / 2050` before this raise; the cap is that count plus about 15%); the total is 12,600. No other cap changes. F08.
2026-10-31: the merge with main keeps main's core cap (3,000) and this branch's podcore (2,500) and e2e (3,130) caps, all as they stand in entropy.json; no cap changes; the total is 13,180. F08.
2026-10-30: e2e cap 3,130 → 3,930, the F09 test (a recording proxy between the daemon and the server, the wake claimed over it, a built fake ACP agent that records argv, env and what it was told, a stubborn-agent case) takes the suite to 3,430 lines (the gate measures `LOC e2e 3430 / 3130` on the tree before this raise; the cap is that count plus about 15%); the total is 13,430. F09.
2026-10-30: worker cap 1,550 → 1,790, the MCP adapter at POST /mcp (initialize, tools/list and tools/call as the operation's own HTTP route) and its tests take the Worker to 1,552 lines (the gate printed `LOC worker 1552 / 1550` before this raise; the cap is that count plus about 15%); the total is 13,670. F09.
2026-10-31: the merge with main keeps main's core cap (3,000) and this branch's worker (1,790) and e2e (3,930) caps, all as they stand in entropy.json; no cap changes; the total is 14,220. F09.
