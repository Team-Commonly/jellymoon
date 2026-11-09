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
| hono 4.13.14 | HTTP routing and the pod WebSocket of the pod-state server (`server/`, ADR 0006); one small dependency with no transitive packages | Hand-rolled routing and upgrade handling |
| @electric-sql/pglite 0.5.13 | In-process Postgres for `make e2e` and a clean shell, when `DATABASE_URL` is unset and `ENV=local`; production uses a real Postgres through Bun's built-in `Bun.SQL`, so there is no driver package (ADR 0006) | Requiring a Postgres install for every checkout and every preflight |
| wrangler 4.151.0 | Runs the Cloudflare adapter (`worker/`) locally under `wrangler dev --local` for `make worker-dev` and `make e2e-a`; loopback only, no Cloudflare account or credentials. Its miniflare 5.20261012.0-beta is patched (`worker/patches/`) to import `undici/index.js` under Bun | Hand-rolled Durable Object and D1 fakes drift from the real runtime |
| @cloudflare/workers-types 5.20261014.1 | Types for the Worker, Durable Object and D1 APIs in `tsc` | Hand-written ambient declarations |
| react 19.3.1, react-dom 19.3.1, react-router-dom 7.18.5 | The web client (`web/`): the shell, its router and the screens that follow. Bundled by Vite for the browser; none runs in the server or the daemon | Preact or a hand-rolled DOM layer: fewer testing and routing tools to lean on |
| vite 8.3.5, @vitejs/plugin-react 6.1.2 | Dev server and bundle of `web/`; the plugin only turns on the JSX transform and fast refresh | A custom esbuild script |
| vitest 5.0.5, jsdom 30.1.3, @testing-library/react 16.3.3, @testing-library/dom 10.4.2 | Unit tests of the web store, card model, key map and shell with no browser (`bun run test:web`). Testing Library's dom package is its peer | bun:test with a hand-made DOM: the React render tests need a document |
| @types/react 19.3.0, @types/react-dom 19.3.0 | Types for `tsc -p web/tsconfig.json` | Untyped JSX |
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
| core (`schema/`, `scripts/`) | 4,200 |
| podcore | 3,000 |
| worker | 2,000 |
| server | 3,030 |
| daemon | 5,000 |
| e2e | 7,160 |
| web | 1,580 |

The caps add up to 25,970 lines (restated 2026-11-09 against the merged entropy.json: core 4,200, podcore 3,000, worker 2,000, server 3,030, daemon 5,000, web 1,580, e2e 7,160). Agent-written comparables grew without bounds: herdr reached
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
2026-10-30: core cap 3,000 → 3,720, land admits a head CodeRabbit reviewed clean after the PR's one local review (`scripts/land-coderabbit.mjs`, its wiring in `land.mjs` and `land-decide.mjs`, and the cases that prove each condition and refusal, `scripts/check-land-coderabbit.mjs` plus new end-to-end cases); the gate printed `LOC core 3235 / 3000` before this raise and the cap is that count plus about 15%; the total is 12,980. No other cap changes. LCR.
2026-10-30: e2e cap 2,660 → 3,130, the F08 test (a built daemon run as a black box: registration, presence online and offline, a hung daemon, a crashed server) takes the suite to 2,714 lines (the gate measures `LOC e2e 2714 / 2660` on the tree before this raise, 2,700 being an earlier print from before the argv check was added; the cap is that count plus about 15%); the total is 12,630. No other cap changes. F08.
2026-10-30: podcore cap 2,050 → 2,500, presence (which computers are online, the 45 s staleness bound, the presence events) and its tests take the core to 2,162 lines (the gate printed `LOC podcore 2162 / 2050` before this raise; the cap is that count plus about 15%); the total is 12,600. No other cap changes. F08.
2026-10-31: the merge with main keeps main's core cap (3,000) and this branch's podcore (2,500) and e2e (3,130) caps, all as they stand in entropy.json; no cap changes; the total is 13,180. F08.
2026-10-30: e2e cap 3,130 → 3,930, the F09 test (a recording proxy between the daemon and the server, the wake claimed over it, a built fake ACP agent that records argv, env and what it was told, a stubborn-agent case) takes the suite to 3,430 lines (the gate measures `LOC e2e 3430 / 3130` on the tree before this raise; the cap is that count plus about 15%); the total is 13,430. F09.
2026-10-30: worker cap 1,550 → 1,790, the MCP adapter at POST /mcp (initialize, tools/list and tools/call as the operation's own HTTP route) and its tests take the Worker to 1,552 lines (the gate printed `LOC worker 1552 / 1550` before this raise; the cap is that count plus about 15%); the total is 13,670. F09.
2026-10-31: the merge with main keeps main's core cap (3,000) and this branch's worker (1,790) and e2e (3,930) caps, all as they stand in entropy.json; no cap changes; the total is 14,220. F09.
2026-11-08: e2e cap 3,930 → 4,650, the F11 fix pass (every stored post read back field by field, the receipt task checked from outside, a forged identity header, and the count of MCP calls shown unreachable by any public path) takes the suite to 4,051 lines (the gate printed `LOC e2e 4051 / 3930` before this raise; the cap is that count plus about 15%); the total is 14,940. No other cap changes. F11.
2026-11-08: the merge with main keeps this branch's core cap (3,720) and main's podcore (2,500) and e2e (3,130) caps, all as they stand in entropy.json; no cap changes; the total is 13,900. LCR.
2026-11-08: the merge with main keeps main's core cap (3,720) and this branch's podcore (2,500), worker (1,790) and e2e (3,930) caps, all as they stand in entropy.json; no cap changes; the total is 14,940. F09.
2026-11-08: server cap (new) 1,800, the pod-state server and its tests (ADR 0006) are 1,267 lines at the first green contract run; the cap is that count plus room for the tests the port still needs (the brief allows at most 3,000); the total is 15,700. No other cap changes. CPORT.
2026-11-08: the merge of main (F09) into the pod-state server branch keeps main's worker (1,790) and e2e (3,930) caps and this branch's server (1,800) cap, all as they stand in entropy.json; no cap changes; the total is 16,740. CPORT.
2026-11-08: web cap (new) 840, the web client package (shell, generated-type socket store, card model, key map and their tests, F18 PR 1) is 730 lines at the first green run; the cap is that count plus about 15%; the total is 17,580. No other cap changes. F18.
2026-11-08: web cap 840 → 1,580, the pod stream (cards, ask cards with the confirm row, composer, needs-you filter, keyboard glue) and its 30 component tests take the package to 1,373 lines (the gate printed `LOC web 1362 / 1250` before this raise; the cap is that count plus about 15%); the total is 18,320. No other cap changes. F18 PR 2.
2026-11-08: podcore cap 2,500 → 3,000, rulings on an ask (the `rule` command, the decision record an ask now keeps, and the tests that pin each refusal and the old-saved-state load) take the core to 2,585 lines (the gate printed `LOC podcore 2534 / 2500` with the tests alone, before this raise; the cap is that final count plus about 15%); the total is 17,960. No other cap changes. F12.
2026-11-08: e2e cap 4,650 → 4,950, the F15 sign-in test (a stub GitHub for the OAuth web flow and GET /user, a browser leg with a cookie jar, `jellymoon login` run as a black box, the state, next, CSRF, one-exchange and expiry refusals) takes the suite to 4,912 lines (the gate printed `LOC e2e 4912 / 4650` before this raise; the cap is that count plus a little for its fix pass, not 15%). The total was stated as 16,740 but the table added up to 17,460 (the server cap raise was never added to it); corrected, and with this raise the total is 17,760. No other cap changes. F15.
2026-11-08: server cap 1,800 → 1,990, worker cap 1,790 → 1,930 and daemon cap 3,000 → 3,240, sign-in (ADR 0008) in both adapters (`server/src/auth.ts` and `worker/src/auth.ts`: the device code, the approval, GitHub's OAuth web flow, the session cookie, `GET /v1/me`) and `jellymoon login` with its poll-loop and 0600-write tests. The gate printed `LOC server 1959 / 1800` and `LOC worker 1902 / 1790` before this raise; the daemon measures 2060 on this branch, and the open daemon stack it lands on top of (#11, #18, #20) brings main's daemon to 2,996, so the daemon cap is 2,996 plus this unit's 233 lines plus a little. Each cap is the count plus a little for the fix pass, not 15%. The total is 18,330. No other cap changes. F15.
2026-11-08: e2e cap 4,950 → 5,000. The merge of main into the F15 branch brings in main's newer e2e tests (the stray-agent check and the no-leak harness), and the merge result measures `LOC e2e 4966 / 4950`; the cap is that count plus a little, not 15%. The total is 18,380. No other cap changes. F15.
2026-11-08: the merge of main into the F15 branch takes the higher cap for every area, all as they stand in entropy.json: core 4,200 (main), podcore 3,000 (main), worker 1,930 (this branch), server 2,100 (main), daemon 5,000 (main) and e2e 6,000 (main); the table above is restated once. One raise follows from the merge: worker cap 1,930 → 1,950, because main's worker changes (the MCP and hello-resync work) and this branch's sign-in together measure `LOC worker 1940 / 1930`; the cap is that count plus a little, not 15%. The total is 22,250. No other cap changes. F15.
2026-11-08: the merge with main keeps this branch's web (840) cap and main's podcore (3,000) cap, with the core (3,720), worker (1,790), server (1,800) and e2e (4,650) caps, all as they stand in entropy.json; no cap changes; the total is 18,800. F18.
2026-11-08: server cap 2,100 → 2,650, the connector layer (ADR 0009: the vault, the catalogue pin and drift check, the MCP-over-HTTP adapter, the connect and grant routes, connector_call with its write gate, the receipts, and their unit tests) takes the server to 2,544 lines (the gate printed `LOC budget: server 2544 > 2100` before this raise; the cap is that count plus about 4%). The table above was behind entropy.json (core 4,200, worker 1,900, daemon 5,000 and e2e 6,000 were already raised there) and now matches it; the total is 23,590. No other cap changes. F19.
2026-11-08: server cap 2,650 → 2,800, the F19 review follow-ups (an owner-only ruling on a connector write, the scrub of \u-escaped, percent-encoded and base64 forms of the credential, the 1 MiB bound on an upstream answer, and the unit tests for the cross-pod and cross-connection lookups, the IV and the bound) take the server to 2,693 lines (the gate printed `LOC budget: server 2693 > 2650` before this raise; the cap is that count plus about 4%); the total is 23,740. No other cap changes. F19.
2026-11-08: the merge of main (daemon permissions, #18) into the run-workspace branch changes no cap; the caps stand as they are in entropy.json (core 4,200, worker 1,900, server 2,100, podcore 3,000, daemon 5,000, web 840, e2e 6,000) and the total is 23,040. WSP.
2026-11-08: worker cap 1,950 → 2,000, server cap 2,100 → 2,110 and e2e cap 6,000 → 6,380, the merge of main into the F15 branch (the device login and main's e2e tests together): the gate printed `LOC worker 1990 / 1950`, `LOC server 2104 / 2100` and `LOC e2e 6376 / 6000` on the merged tree, before this raise; each cap is the count rounded up to ten; the table caption carries the moved total. No other cap changes. F15.
2026-11-09: server 2,110 -> 2,180; merged tree measures 2,176; #16's server follow-ups meet F15's sign-in. Feature growth, not a refactor; the cap is the count rounded up to ten; no other cap changes; the total is 24,340. F15.
2026-11-09: e2e cap 6,380 -> 6,520; merged tree measures 6,518; #10's replay test meets F15's. The cap is the count rounded up to ten; no other cap changes; the total is 24,480. F15.
2026-11-08: the merge of main into the F19 branch keeps this branch's server cap (2,800) and main's web cap (1,580), with core (4,200), worker (1,900), podcore (3,000), daemon (5,000) and e2e (6,000), all as they stand in entropy.json; no cap changes; the total is 24,480. F19.
2026-11-08: e2e cap 6,000 → 6,700. Main's own suite measures 5,936 lines and TestF19 (the stub Linear, the six steps and their encoding probes) adds the rest: the merge result printed `LOC budget: e2e 6565 > 6000`. The cap is that count plus about 2%; the total is 25,180. No other cap changes. F19.
2026-11-09: the merge of main (F15's sign-in, #30) into the F19 branch: server cap 2,800 -> 3,030 and e2e cap 6,700 -> 7,160. Main measures server 2,176 and e2e 6,518; the connector layer adds 839 and TestF19 629, and the merged tree printed `LOC budget: server 3015 > 2800` and `LOC budget: e2e 7147 > 6700`. Each cap is the count rounded up to ten, as main's F15 notes do; the higher of the two sides is kept for every other budget; no other cap changes; the total is 25,970. F19.
