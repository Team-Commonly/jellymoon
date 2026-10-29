# Jellymoon progress

Newest entry first. Each entry has four parts: what changed, the evidence (a test command and its
result, or the PR), what's next, and anything surprising. Keep each entry under 15 lines; the detail
goes in the PR.

## 2026-10-29 · protocol reference doc (unit P01, rebased)
- **Changed:** `make gen` also writes `docs/protocol.md`, a readable reference of the whole protocol
  (101 definitions, 17 events, 7 operations with method, path, auth and MCP name, error codes, rules as
  sentences). `scripts/protocol-doc.mjs` renders it from the data `scripts/gen.mjs` already loads;
  `make check-gen` diffs the doc too. Rebased onto operations.json: the doc lists operations, not tools.
  No new dependency; nothing deleted.
- **Evidence:** `make gen` twice leaves the tree unchanged; a schema description edit makes
  `make check-gen` fail until regenerated; `make preflight` green.
- **Open:** a definition without a schema description shows an empty Notes cell; fixing that is a schema edit.

## 2026-10-29 · Jellymoon names applied (build session)
- **Changed:** decided at the protocol review: MCP adapter prefix `jm_`, tokens `jmu_`/`jmc_`/`jmr_`,
  schema `$id` on `jellymoon.dev/schema/v1`, CLI binary `jellymoon` (alias `jm`), package `jellymoon`.
  Feature step text changed only for these names (F03, F08, F09, F10, F11, F12, F13, F15); no step's
  meaning changed. This resolves the naming hold noted below.
- **Evidence:** `make preflight` green.

## 2026-10-29 · two founder directions applied to the schema (build session)
- **Changed:** asks carry a required grounded `context` (happened, checked, stakes, recommendation) and
  each option a `consequence`; every claim cites `tc_` tool-call or message ids, and a post whose
  numbers aren't in its cited evidence is refused with `ungrounded`. The pod stamps tool calls itself
  (`run.toolcall` from the daemon, MCP calls it sees); agents read them via the `evidence` operation.
  `schema/tools.json` became `schema/operations.json`: plain HTTP operations are the contract, and
  MCP (`x-mcp-prefix`) and the CLI are generated adapters.
- **Evidence:** `make preflight` green; 76 fixtures, 3 new negatives.
- **Next:** the grounding x-rules need Worker tests when F-posting lands; features F08/F15 still quote
  MCP-first wording, changed in the naming PR after the design review.

## 2026-10-29 · portability fixes from loop review (build session)
- **Changed:** the repo now works from a fresh clone anywhere. Stage skills are vendored in
  `.agents/skills/` (pins in `docs/skills/SOURCES.md`); Go and Bun are pinned in `.tool-versions` and
  `init.sh` installs them into `.tools/`; `make portable` (preflight, pre-commit, CI) fails on a symlink
  out of the repo or a machine-local path. The merge rule says what actually runs: coordinator merges on
  green CI plus a ledger verdict, since GitHub Free gives a private repo no merge queue.
- **Evidence:** `./init.sh` green with `HOME` set to an empty dir and only system dirs on `PATH` (35 s
  cold, tools downloaded and checksum-verified). Mutation checks: an outside symlink and a `~/` path
  each fail `make portable`.
- **Surprising:** the first pilot passed its own setup on this machine while depending on a committed
  `node_modules` symlink and skill paths under the operator's home. A pilot proves portability only if
  it runs without the operator's environment, so pilots now run with a scratch `HOME`.
- **Open:** the MCP tool prefix and CLI binary name are an open naming question for the protocol
  review. The prefix is one constant (`x-tool-prefix` in `schema/tools.json`); feature text still
  quotes today's names.

## 2026-10-29 · initializer (build session)
- **Changed:** set up the long-running harness. `feature_list.json` holds 20 slice features as e2e
  tests (F01 passes). Also added `init.sh`, this file, the `AGENTS.md` loop, the PR template and
  `docs/development/workflow.md`.
- **Plan:** one builder session works top-down through the list. Two design docs run alongside as
  subagents (connectors, OSS-first; client options). F18 and F19 are blocked until Sam reviews those
  docs.
- **Evidence:** F01 is `make check-gen test`, green on #2977 (L0).
- **Next:** F02, the `make e2e` harness (wrangler dev, /healthz, E2E_MS).
- **Surprising:** the fleet's husky pre-commit hook doesn't apply here; `make hooks` installs
  this repo's own pre-commit. F17 needs a Cloudflare API token
  secret, an owner action.

## 2026-10-29 · repo move (build session)
- **Changed:** the build moved from the `v4` branch of the Commonly repo to its own private repo,
  `Team-Commonly/jellymoon` (owner ruling;the company stays Commonly). History was rebuilt clean:
  a fixture token that tripped gitleaks became an obviously fake one, and the Go module is
  `github.com/Team-Commonly/jellymoon`.
- **Held:** the CLI binary name (`commonly` in F08/F15) and the npm scope wait for the trademark check.
- **Surprising:** the org is on GitHub Free, so a private repo gets no rulesets or merge queue.
  Until that is decided, the coordinator merges only on green CI plus a ledger verdict.
