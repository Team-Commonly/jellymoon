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

## What a worker's own runs taught
- **`HOME` is never empty or unset in a probe or a test.** The platform then finds the real account's home another
  way, and the run reads a real person's files. Ours reads the variable only, and only a full path
  (`jellymoon_core::user_home`); a run still gets a scratch home, `PATH` and `JELLYMOON_SYSTEM_BIN`.
- **A test starts `jellymoon` or `jellymoon-app` through the door, and only there.** Each program's tests have
  one: `crates/jellymoon/tests/support/door.rs` and `app/tests/support/door.rs`, the one file of each that names a
  built program. Their rule is one file, `crates/jellymoon/tests/support/door/world.rs`, which the app's door
  takes as a module of its own, since the two are different crates. It refuses, in one fixed sentence and before
  anything is started, a world whose home (`HOME`, `USERPROFILE`), `JELLYMOON_HOME`, `JELLYMOON_APP_DIR`,
  `JELLYMOON_BIN_DIR`, `CLAUDE_CONFIG_DIR`, or value of `--logs`, `--settings` or `--repo` is a full path outside
  the temporary folders, with links resolved on both sides as far as the path is there; a path through more than
  32 links whose end is not there is refused. The temporary folders are the one the system gives the process and
  the one Cargo gives tests, and for the binary's tests `/tmp` on Unix too; the app's door has no third. No home,
  an empty one and one that is not a full path still start, but a `--logs` or `CLAUDE_CONFIG_DIR` that is no full
  path is refused when it climbs with `..` or is rooted with no drive: the program reads it from the folder it is
  started in. On Windows a variable is the same variable in any case. The door sets `JELLYMOON_SHELL` and
  `JELLYMOON_SYSTEM_BIN` to nothing in every world; a test may name a made-up login shell under a temporary
  folder, and folders of its own to look for a harness in. A temporary folder that is a file system's root, or
  that is or holds the home of the person running the tests, is taken for none: the door then says so and starts
  nothing. A program that starts `jellymoon` itself (the hook's entry, the installer, a made-up asker, the app's
  made-up windows, git) is started through `door::command`, with the same check. A test's folder comes from the
  support too (`scratch`, `system_scratch`, `kept_scratch`).
- **A home of another owner is `door::not_ours()`**, which is `/tmp` where that is not the tests' own. Where it is
  (a sandbox with a `/tmp` of its own, or tests run as root) there is none, and the cases that need one say so
  and are left out (`f35/delivery.rs`, `f41/refused.rs`, `f62/refused.rs`); before the door they ran against the
  root folder. A world that takes it as the home names Jellymoon's folder, the harness's folder and a harness.
- **A test of the app that holds the day in its own process names its harness** (`support::day::World::claude`):
  no `PATH` is the world's there, so a look would be made with the variables of the person running the tests.
- **`cargo xtask check` holds tests to the door** (`crates/xtask/src/rules/door.rs`). It reads lines, not Rust. In a
  file under `crates/jellymoon/tests` or `app/tests` other than that program's `support/door.rs` it refuses a line
  that names a built program of the door (`CARGO_BIN_EXE_jellymoon`, and for the app also `f42-jellymoon`,
  `f42-wisher` and `f47-tapper`), and outside `support/` a line that makes a temporary folder (`tempdir`,
  `TempDir::new`, `temp_dir(`). In the tests of core and of the app (a file under `tests/`, a `tests.rs`, and what
  `mod tests` holds under `#[cfg(test)]` in a file) it refuses the harness looked for with the process's own
  environment (`harness()` called by that name, `named_or_found(None`, `harness: None`). It cannot see a program
  started from a path a test read out of a file or took from `door::built()`, an argument or a variable added to
  a command after the door made it, a home put in a variable by hand and given to another program with
  `Command::new`, options built with `..Default::default()`, or the same thing written across two lines. The
  ignored test of F29 that runs the real Claude Code starts it with the person's own variables, by design, and is
  not covered.
- **Builders on different trees never share one `CARGO_TARGET_DIR`.** The programs Cargo lifts to the folder's top,
  `xtask` among them, overwrite each other, and a test runs another tree's program. Take a copy-on-write copy of a
  warm folder and clear the workspace crates' builds in it.
- **A wall-clock limit is never a test's only bound.** A slow runner fails it and a fast one hides the cost. Assert a
  count (calls made, pieces drawn, units spent) and keep the clock as a generous backstop.

## Size
`make preflight` runs `cargo xtask check`: each file at most 400 lines, a LOC budget per crate in `entropy.toml`, no
machine-local path in a tracked file, the crate direction (core, then the binary; the kit, then the app), and no
package under `[forbidden]` (GPUI, for core and the binary) listed in a crate's manifest. Clippy holds a function to 60 lines.

- A budget grows only in the PR of the feature that needs it, with the measured count and the reason in the PR. Never
  for a refactor, never by deleting tests.
- A new dependency or crate gets a short note in `docs/adr/`: what it is for and what was rejected.
- Every report states lines added and removed (`git diff --shortstat`), and answers: what did this change make
  obsolete, and was it deleted?

## The fresh-context review
Every feature gets one review by a subagent that never saw the builder's reasoning, on a different model from the
builder where possible. It runs after the merge, at the merged head, and the features after it do not wait for it.
Two kinds of PR are reviewed before they merge instead: one that changes what leaves the machine (the reader,
redaction, the reading call, delivery), and one from outside the maintainers. The reviewer is given the PR number and
head SHA, the feature from `feature_list.json` (or the unit's brief), the diff, and this file. In order:

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

What a review costs is bounded, because PR 109 took three rounds and one of them nearly took down the machine:
- **One round.** Blocking findings are fixed in the next PR and proved by its tests, not by a second review. Only
  the two kinds of PR reviewed before merging get a new review of the fix.
- **Blocking means reachable.** A fault in code that no command reaches yet (a loader nothing calls, a file kind
  nothing opens) is non-blocking: it is the first work of the feature that opens that door.
- **Every run has a time limit**, one test binary at a time. A mutation changes a cap by one and never removes or
  widens a loop, size or allocation cap: with the cap gone, the test that reaches the loop does not fail, it grows.

## Landing
A PR is squash-merged as soon as both hold at its current head:
- `make preflight` passed on the builder's machine;
- CI is green: `check` on macOS, Linux and Windows, `secrets`, and `dco`.

Nothing waits on CI but the merge itself: the next feature starts from the branch while the checks run. A PR that
changes what leaves the machine or what an agent may do on it, or comes from outside the maintainers, also needs a
review comment for its exact head ending in `GATE: PASS`. CI is read on that head before the gate's pass counts: F39
passed its gate on a head whose Windows check had failed unread, and the fix came after the pass. Only the coordinator merges. Workers commit on their own branch with `git commit -s`, in
a worktree fenced by `make hooks fence=1`; the coordinator pushes and opens the PR.

## Entropy
After every few features, one PR removes and adds nothing: `deslop` plus `improve-codebase-architecture`, a check of
`docs/INDEX.md` against the files under `docs/`, and any flaky test quarantined with a fix noted in `progress/`.
