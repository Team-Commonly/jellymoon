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
Local verification is the authority. **GitHub checks are not consulted at all, and nothing GitHub reports counts:**
both workflows are disabled while the repo is private (owner ruling:CI is a bottleneck and not unlimited), and any PR
that touches `.github/workflows/` could fake a green check anyway. `make land PR=<n>` is the only merge path
(`docs/development/landing.md`). It merges only when all of these hold at the exact PR head:
- a clean-shell `make preflight` passes at that head;
- the verifier's ledger line at the head is `PASS` (the check of record, `docs/development/verifier.md`), and the
  preflight ran on the merge result (main plus the head);
- for a protected path, the reviewer's pod verdict also passes. It is a message by a listed reviewer's
  server-stamped username, whose first line is `jellymoon#<pr> <full head>` and whose last line is `GATE: PASS`.
- Workers never merge. GitHub review text is not consulted.
- Protected paths are listed in `landing.md`; they include `land.mjs`, `land-decide.mjs`, `review-gate.mjs`,
  `Makefile`, `init.sh` and `scripts/tools.sh`. A rename out of a protected path counts. Run land from main.
- **Accepted reviewers:** `.github/opus-reviewers.txt` is the only place they are written (`Name username`);
  the test asserts the file parses and pins no name.
- **How the verdict is read** (`scripts/review-gate.mjs`, shared with land). The verdict is the message's LAST
  non-empty line, with HTML comments removed and emphasis and list or heading markers stripped. It passes only
  if that is exactly `GATE: PASS` (case-insensitive). A missing line, a FAIL after the PASS, prose after it, an
  indented line, or a fullwidth or zero-width variant blocks. Cases: `scripts/check-review-gate.mjs`; land's
  cases: `scripts/check-land.mjs`.
- **CI** returns at public launch only as a thin mirror of `make preflight` for outside contributors, never as
  the gate. The workflow files are kept, with a disabled header.
- **Why no merge queue:** the repo is private on GitHub Free. `make land` is the one door, with a lock so two lands cannot run at once.

### Temporary: remove when the reviewers return
While the reviewers are down, `make land PR=<n> ARGS=--reviewer-gate-unavailable` lets a protected-path PR
land without a pod verdict. It needs a second ledger row at the same PR and head: `PASS`, by a verifier on a
different model family from the first PASS, with a note starting `reviewer-gate-unavailable`; the latest row decides. The
second verifier is fresh-context: it has never seen the builder's or the first verifier's reasoning. Every such
land appends a `REGATE-OWED` row; `make land ARGS=--owed` lists the PRs still owed a reviewer re-gate (it blocks
nothing). Whatever a later reviewer re-gate fails is fixed forward. The ledger `by` column is `<model>-verifier`
(`opus`, `sonnet`, `haiku` or `fable`) and `land`; the coordinator never writes `PASS` or `FAIL`.

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
