# 0001 · Dev tooling and its dependencies

Status: Accepted (maintainers, 2026-10-29). Rule: every new dependency or module gets a note like this one.
Update (2026-11-09, maintainers): the cap table, the running total and the dated cap notes moved out of this ADR into
`docs/entropy/caps/`, one file per change. Every cap change edited the same rows and the same total sentence, so any two
PRs that touched a cap conflicted, and the table drifted from `entropy.json`.

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
| @playwright/test 1.64.1 | The browser specs in `web-e2e/` (`make e2e-web`, CI's `browser` job): real keyboard input and a real socket in Chromium, at 390 and 1440 px. Its Chromium is installed only inside the CI job, at the version this package pins; `bun install` downloads no browser | Testing Library alone: jsdom has no layout, no focus model of a real browser and no WebSocket to the server |
| ws 8.21.0 | The Node-side pod socket of the browser specs: a browser cannot put a token in `Authorization` on the upgrade, so the spec bridge opens it from Node with the token (`web-e2e/fixtures.ts`). Already in the lockfile under wrangler; now named | Node's built-in WebSocket: whether it sends custom upgrade headers depends on the Node release |
| Node 20 or newer (land's runtime, not pinned) | `scripts/land.mjs` runs under `node`, never bun, so bun does not auto-load a `.env` from the checkout; land refuses to start on an older Node | bun: loads `.env` and `bunfig.toml` from the cwd into the environment |
| deadcode, x/tools v0.51.0 (`go run`) | Unreachable Go functions from `main`; starts enforcing when a main package exists | — |

Go tools run through `go run pkg@version`. They resolve from the module cache, with no install. Go and
Bun themselves are pinned in `.tool-versions` and installed into `.tools/` by `scripts/tools.sh`
(checksum-verified, repo-local). The tools themselves are repo-local, but Go and Bun keep their caches (module cache, build cache, bun install cache) under HOME, so a first run writes there; the preflight points HOME at its throwaway dir.

The budgets they enforce live in `entropy.json`: a file is at most 400 lines and a function at most 60;
LOC per area is capped; exceptions only shrink unless the exception cites a note here.

The LOC budget for the slice is a deliberate choice to stay small. Agent-written comparables grew without
bounds: herdr reached 291k lines in seven months, and Tutti 1.6M (owner's figures).

`entropy.json` is the only place the caps are written, and `make entropy` prints their total. This note holds no
table of caps and no total, because a copy drifts and, as a shared row, conflicts on every PR that touches a cap.

The rule: a cap grows only by adding one new file in `docs/entropy/caps/` in the same change as the
`entropy.json` edit. The file is named `YYYY-MM-DD-<area>-<old>-to-<new>.md` and says the area, the old and new
cap, the measured count, the PR or feature that grew it, and why. A cap grows for feature growth, never for a
refactor and never by deleting tests. An old note is never edited. `scripts/check-entropy.mjs` fails a changed cap
that has no new file there. The notes written before this rule (every dated line this ADR used to carry) were
moved there with their words and numbers unchanged (four lost their list dash).
