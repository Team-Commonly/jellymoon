# How Jellymoon is built

Jellymoon is built by one long-running builder session, using the harness from Anthropic's
[Effective harnesses for long-running agents](https://www.anthropic.com/engineering/effective-harnesses-for-long-running-agents).

| File | Role |
|---|---|
| `feature_list.json` | Every slice feature, written as an end-to-end test. Only `passes` may change. |
| `init.sh` | Brings up the environment, runs the baseline and prints the scoreboard. |
| `progress.md` | The hand-off between sessions and across context compactions. |
| git log | The record. Small commits, one feature per PR. |

## Why each rule exists
- **The feature list is JSON and tests are never edited.** The article found that models rewrite
  Markdown checklists. Removing a test to make it pass is the failure that hides missing
  functionality.
- **One feature at a time.** Long runs fail by declaring done too early or by half-finishing several
  features. Taking the top unfinished feature every time prevents both.
- **The baseline runs first.** A session that builds on a broken tree compounds the break.
  `init.sh` makes the state visible before any edit.
- **Verify at the API, not with unit tests.** The article saw agents mark features done on unit
  tests or a single curl. Here a feature passes only on its `test` command. For the slice, that is
  the Go e2e that drives the real Worker and daemon. Browser checks join when the client lands (F18).
- **Compact at feature boundaries.** `progress.md` plus git carry the state, so a fresh context can
  resume without the transcript.

## Stage skills
The table in `AGENTS.md` maps each stage to one skill in `.agents/skills/`. They are vendored, not
installed, so the loop works from a fresh clone; pins and update rules are in `docs/skills/SOURCES.md`.
`code-review` and `deslop` run in a fresh-context subagent; `retro` runs at the end of each wave or
after a surprise.

## Review and merge
- Only the coordinator merges, on green CI plus a verifier ledger verdict at the PR head
  (`docs/development/verifier.md`). Workers never merge.
- A PR touching a protected path also needs two reviews before it merges:
  - one fresh-context review: a subagent given the diff and the feature, not the builder's reasoning;
  - an Opus `GATE: PASS` at the PR head from an accepted reviewer (below).
- Protected paths: `schema/`, `worker/src/auth`, `worker/src/mcp`, `daemon/internal/{acp,run}`,
  `install.sh`, `.github/workflows`, `.github/opus-reviewers.txt` and `scripts/review-gate.mjs`. A rename out
  of a protected path counts. Listing the gate's own files here means a change to them needs a review; it does
  not protect them from the gate (see below).
- **Accepted Opus reviewers:** the names in `.github/opus-reviewers.txt`, the only place they are written
  (today Ben, Ada, Eli, Cleo and Dana; Ada is the usual gate). The gate reads that file from the base
  branch.
- **How the gate decides** (`.github/workflows/review-gate.yml` runs the base branch's copy of
  `scripts/review-gate.mjs`; the cases are `scripts/check-review-gate.mjs`, its own `make test` step). It fails
  closed, at the exact PR head, ignoring code fences and blockquotes: a review is credited to N when a
  `Written by <N>, a Commonly agent` line names a listed reviewer; one credited to two names blocks; dismissed
  reviews are ignored; each reviewer's latest credited review decides, and it passes only with a last line of
  exactly `GATE: PASS` (case and markdown emphasis ignored) and a state other than CHANGES_REQUESTED. The gate
  passes when at least one reviewer's latest review passes and none blocks.
- **The gate cannot judge its own change.** The script and reviewer list come from the base branch, so a PR
  cannot rewrite its judge, but `review-gate.yml` itself runs from the PR's merge ref. Any PR touching
  `review-gate.yml`, `scripts/review-gate.mjs` or `.github/opus-reviewers.txt` is judged by a person at press
  time, never by the green check. Bootstrap: when the base branch has no `scripts/review-gate.mjs` (the first
  merge of the gate into `main`), the check fails closed; a person verifies the Opus `GATE: PASS` at the head
  by hand, merges that one PR by hand and records it in `progress.md`.
- **Advisory, not required:** GitHub Free has no required checks on a private repo, so a red gate does not stop
  a merge by itself. The coordinator's press rule above does.
- **Known limit:** the credit line is a pointer, not an identity. Every reviewer posts to GitHub as one account,
  so the gate does not verify who reviewed. Until it checks a server-stamped verdict (the reviewer's
  post in the pod, naming the PR and head SHA), any holder of the shared GitHub token can satisfy it.
  That check is the next unit.
- **Why no merge queue:** the repo is private on GitHub Free, which allows no rulesets or merge queue.
  The coordinator presses by hand under the rule above. When the repo goes public or moves to GitHub
  Team, a ruleset plus merge queue takes over and this section changes.

## Design-gated features
- **F18 (client) and F19 (connectors)** stay blocked until their options docs are reviewed with Sam.
  The docs are `docs/design/client-options.md` and `docs/design/connector-options.md`.
- **After the review:**
  1. add the steps and test to the existing F18 or F19 entry, and clear `blockedBy`;
  2. note in `progress.md` that the review set them.

  This is the only time a feature's text changes, and only for these placeholders.

## Entropy gate
Maintainability is a gate in the loop, not a cleanup phase. `make preflight` runs `make entropy`, and
so does CI.
- **Budgets** live in `entropy.json`:
  - each file at most 400 lines and each function at most 60;
  - a LOC cap per area;
  - `TODO` only with a row id, like `TODO(F07)`;
  - Go import boundaries;
  - exceptions only shrink, unless a new one cites a `docs/adr/` note.
- **Dead code:** knip (TS), staticcheck and deadcode (Go).
- **Boundaries:** dependency-cruiser (TS).
- **Every unit report** states its lines added and removed, and answers the subtract-before-you-add
  question: what did this change make obsolete, and was it deleted?
- **New dependencies:** a new dependency or module gets a short note in `docs/adr/`.
- **Each wave** includes one removal-only unit:
  - `deslop` plus Pocock `codebase-design`, with no new behaviour;
  - a `docs/INDEX.md` drift check;
  - quarantining any flaky test, with a fix row.
- **Next, once the slice is green:** jscpd duplication and a full ratchet report.
