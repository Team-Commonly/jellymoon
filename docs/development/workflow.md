# How Jellymoon is built

One long-running builder session builds Jellymoon, with subagents for parallel pieces, after Anthropic's
[Effective harnesses for long-running agents](https://www.anthropic.com/engineering/effective-harnesses-for-long-running-agents).
`AGENTS.md` has the rules; this file has the reasons and the two protocols.

| File | Role |
|---|---|
| `feature_list.json` | Every feature, written as a test at the `jellymoon` binary's boundary. Only `passes` may change |
| `init.sh` | Installs the pinned tools into `.tools/`, runs the baseline and prints the scoreboard |
| `progress/` | The hand-off between sessions: one file per unit, named `YYYY-MM-DD-HHMM-<unit>.md` (UTC), so the newest sorts last. Four parts (what changed, the evidence, what's next, anything surprising), under 15 lines; the detail goes in the PR. Nothing else lives in the folder |
| git log | The record. Small commits, one feature per PR |

## Why each rule exists
- **The feature list is JSON and its text is never edited.** Models rewrite Markdown checklists, and removing a test
  to make it pass hides missing behaviour.
- **One feature at a time.** Long runs fail by declaring done too early or by half-finishing several features.
- **The baseline runs first.** A session that builds on a broken tree compounds the break.
- **Prove it at the binary.** Agents mark features done on a unit test. Here a feature passes only on its `test`
  command, which runs the built `jellymoon` binary as a person would and reads what it prints.
- **One progress file per unit.** Separate files never conflict, so a merge never moves another PR's head.
- **Lessons land in the PR.** A lesson kept only in a chat or a private note is lost to the next contributor.

## Size
`make preflight` runs `cargo xtask check`: each file at most 400 lines, a LOC budget per crate in `entropy.toml`, no
machine-local path in a tracked file, and the crate direction (core, then the binary; the kit, then the app). Clippy holds
a function to 60 lines.

- A budget grows only in the PR of the feature that needs it, with the measured count and the reason in the PR. Never
  for a refactor, never by deleting tests.
- A new dependency or crate gets a short note in `docs/adr/`: what it is for and what was rejected.
- Every report states lines added and removed (`git diff --shortstat`), and answers: what did this change make
  obsolete, and was it deleted?

## The fresh-context review
Every PR head that is to merge gets one review by a subagent that never saw the builder's reasoning, on a different
model from the builder where possible. It is given the PR number and head SHA, the feature from `feature_list.json`
(or the unit's brief), the diff, and this file. In order:

1. **Trust.** For a PR from outside the maintainers, read the code with `git show`, `git diff` or on GitHub. Never
   check it out and run `make`, `cargo` or `./init.sh`: a build script and a test are code. A maintainer's PR may be run.
2. **Reproduce** (a maintainer's PR only). From a clean checkout of the head: `make preflight`, then the feature's `test` command. Red ends
   the review with `GATE: FAIL`.
3. **Acceptance.** Walk each step of the feature against what the test asserts. A step no assertion covers is a finding.
4. **Review.** Apply `code-review` to the diff against `main`. Correctness first, then the repo's rules: logic in core,
   the privacy rules (synthetic fixtures, only typed text kept, credentials replaced, model text never run), every system.
5. **Deslop.** Apply `deslop`: dead code, needless abstraction, comments that restate code, over-defensive guards.
6. **Subtract.** Could the diff be smaller, and does it delete what it made obsolete? Report lines added and removed.
7. **Secrets the scanner skips.** A scan can refuse but cannot prove a secret absent: read binaries, images, lockfiles
   and commit messages in the range yourself.

The last lines of the report are always these two:
```
FINDINGS <n blocking> <n non-blocking>
GATE: PASS | GATE: FAIL
```
`GATE: PASS` requires zero blocking findings. The reviewer never pushes or merges. The coordinator posts the report as
a PR comment whose first line is `Review at <full head SHA>`, changing nothing in it.

## Landing
A PR is squash-merged when both hold at its current head:
- CI is green: `check` on macOS, Linux and Windows, `secrets`, and `dco`;
- a review comment for that exact head ends in `GATE: PASS`.

A blocking finding gets a fix, which is a new head, and a new review of it. A head that only merges `main` needs CI
again and a review of what the merge changed. Only the coordinator merges. Workers commit on their own branch with
`git commit -s`, in a worktree fenced by `make hooks fence=1`; the coordinator pushes and opens the PR.

## Entropy
After every few features, one PR removes and adds nothing: `deslop` plus `improve-codebase-architecture`, a check of
`docs/INDEX.md` against the files under `docs/`, and any flaky test quarantined with a fix noted in `progress/`.
