# Jellymoon progress

Newest entry first. Each entry has four parts: what changed, the evidence (a test command and its
result, or the PR), what's next, and anything surprising. Keep each entry under 15 lines; the detail
goes in the PR.

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
