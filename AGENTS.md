# Jellymoon

A multiplayer harness for the agents people already use: a pod where humans and their Claude Code or Codex
agents share tasks, ask for decisions and leave receipts. Cloudflare Workers with one Durable Object per pod;
a headless Go daemon runs local agents over ACP; agents use pod operations over HTTP; MCP and the CLI are generated adapters. The main
check is an API-level e2e. Terms are defined in `docs/GLOSSARY.md`; use them exactly.

## Start every session here
1. `./init.sh` — dependencies, the baseline (`make preflight`) and the feature scoreboard. Red baseline first.
2. Read `progress.md` (newest first) and `git log --oneline -15`.
3. Take the first `feature_list.json` feature with `passes: false` and no `blockedBy`. One feature at a time.

## The loop: one skill per stage
Skills are vendored in `.agents/skills/<name>/SKILL.md` (pins in `docs/skills/SOURCES.md`). Read it before the stage.

| Stage | Skill | Output |
|---|---|---|
| Design | `grill-with-docs` | ADRs in `docs/adr/`, terms in `docs/GLOSSARY.md` |
| Plan | `to-tickets` | features in `feature_list.json` with `blockedBy` edges |
| Build | `tdd` | e2e red from the feature's steps, then red/green inward |
| Verify | the feature's `test` command (`docs/development/verifier.md`) | `passes: true`, ledger row |
| Review | `code-review` + `deslop` | `GATE:` verdict in a fresh context |
| Land | `make-pr-easy-to-review` | PR from `.github/pull_request_template.md` |
| Learn | `retro` | a test, lint rule or doc in the same PR |
| Continuity | `handoff` | a `progress.md` entry at each boundary |
| Entropy | `improve-codebase-architecture` | one removal-only unit per wave |

## Rules
- **Only `passes` changes** in `feature_list.json`. Never delete, reword or reorder a feature to make it pass.
- **Verify at the API.** A unit test alone never marks a feature passing.
- **The schema is the contract.** Edit `schema/`, run `make gen`, commit both. No `oneOf`/`anyOf`/`allOf`;
  unions are a `type` enum plus `x-variants`; cross-field rules are `x-rules`, tested in `worker/`.
- **Tokens.** An agent holds only a `cmr_` run token, never `cmc_`, `cmu_` or a third-party credential. The
  server stamps receipts and actors and never trusts them from input. Push wakes only; nothing polls.
- **Stay small.** `make entropy` enforces `entropy.json`: files ≤ 400 lines, functions ≤ 60, LOC caps per
  area, `TODO(F07)`-style ids, import boundaries. A new dependency or module gets a `docs/adr/` note.
  Every report states lines added and removed, and what it deleted.
- **Merging.** Only the coordinator merges, and only on green CI plus a verifier ledger verdict at the head.
  Protected paths also need an Opus `GATE: PASS` at the head: `schema/`, `worker/src/auth`, `worker/src/mcp`,
  `daemon/internal/{acp,run}`, `install.sh`, `.github/workflows`. (No merge queue while the repo is private
  on GitHub Free; this rule changes when it goes public or moves to Team.)
- **Workers** (subagents) commit on their own branch and never push, merge, rebase or reset; `make hooks`
  installs the pre-commit preflight and the push fence in a worktree.
- **Lessons** land in the same PR as a test, lint rule or doc. Never only in private memory.

## Map
- `schema/` wire shapes; `schema/fixtures/` examples every test loads. `gen/`, `daemon/internal/protocol/gen.go`
  are generated. `worker/` Worker, PodDO, the operations and their MCP adapter, auth. `daemon/` Go daemon, ACP. `e2e/` slice tests.
- `docs/INDEX.md` lists every doc. Go and Bun are pinned in `.tool-versions`; `init.sh` installs them into
  `.tools/` (repo-local). Never install anything globally. No machine-local paths (`make portable`).
