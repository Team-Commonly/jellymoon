# Jellymoon

A multiplayer harness for the agents people already use: a pod where humans and their Claude Code or Codex
agents share tasks, ask for decisions and leave receipts. Cloudflare Workers with one Durable Object per pod;
a headless Go daemon runs local agents over ACP; agents use pod tools through one remote MCP server. The main
check is an API-level e2e. Terms are defined in `docs/GLOSSARY.md`; use them exactly.

## Start every session here
1. `./init.sh` — dependencies, the baseline (`make preflight`) and the feature scoreboard. Red baseline first.
2. Read `progress.md` (newest first) and `git log --oneline -15`.
3. Take the first `feature_list.json` feature with `passes: false` and no `blockedBy`. One feature at a time.

## The loop: one skill per stage
`P` = `<plugin cache>/claude-plugins-official/mattpocock-skills/1.3.1/skills` (Pocock).
`S` = `<vendored pstack>/skills` (pstack, pinned, read-only). Read the SKILL.md before the stage.

| Stage | Skill | Output |
|---|---|---|
| Design | `P/engineering/grill-with-docs` | ADRs in `docs/adr/`, terms in `docs/GLOSSARY.md` |
| Plan | `P/engineering/to-tickets` | features in `feature_list.json` with `blockedBy` edges |
| Build | `P/engineering/tdd` | e2e red from the feature's steps, then red/green inward |
| Verify | the feature's `test` command (`docs/development/verifier.md`) | `passes: true`, ledger row |
| Review | `P/engineering/code-review` + `docs/skills/pstack-deslop` | `GATE:` verdict in a fresh context |
| Land | `S/make-pr-easy-to-review` | PR from `.github/pull_request_template.md` |
| Learn | `S/reflect` | a test, lint rule or doc in the same PR |
| Continuity | `S/poteto-mode/playbooks/session-pickup.md` | a `progress.md` entry at each boundary |
| Entropy | `P/engineering/improve-codebase-architecture` | one removal-only unit per wave |

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
- **Merging.** Green in the merge queue merges. Protected paths also need a fresh-context review and an Opus
  `GATE: PASS` at the head: `schema/`, `worker/src/auth`, `worker/src/mcp`, `daemon/internal/{acp,run}`,
  `install.sh`, `.github/workflows`.
- **Workers** (subagents) commit on their own branch and never push, merge, rebase or reset; `make hooks`
  installs the pre-commit preflight and the push fence in a worktree.
- **Lessons** land in the same PR as a test, lint rule or doc. Never only in private memory.

## Map
- `schema/` wire shapes; `schema/fixtures/` examples every test loads. `gen/`, `daemon/internal/protocol/gen.go`
  are generated. `worker/` Worker, PodDO, MCP, auth. `daemon/` Go daemon, ACP. `e2e/` slice tests.
- `docs/INDEX.md` lists every doc. Go and Bun come from a shared toolchain dir on the build host
  (the Makefile adds it). Never install anything globally.
