# The run follow-up, after its gate: git is heard out, and ADR 0017 says what a record names

Branch `run-follow-up-fix`, one commit on `6db8e48` (pull request 165). The gate of `6db8e48` failed with two
blocking and six non-blocking findings; this closes all eight as the coordinator ruled. No feature row changes.
It changes what a look and a run read of git, so it waits for `GATE: PASS`.

## What changed

**What a command says on stderr (`call.rs`), findings 1 and 8.** One reader for a look's reads and a run's own
git steps. It keeps the first bytes, hands them over, and then reads the pipe to its end into nothing. The pipe
is never let go of and never full, so git is neither ended by a write nor left waiting on one. Kept: no more
than the size a bounded read stops at, and never more than 128 KiB (`SAID_KEPT`). An error's one line is made
by `redact::line` from the first 300 + 16,384 characters, at most four bytes each, which is under 128 KiB: the
build fails if the bound is ever set below that. So no error line changes. Nothing in core chooses a sentence
by matching git's words; the one use of stderr besides the line is "exit 1 and nothing said" (`copy/git.rs`,
`printed`), asked of that same line.

**ADR 0017, finding 2.** The dated section now says a going, failed, cancelled or cut-off run names `repo` and
`start` and no `end`, or, written before F48 or written down as failed by v0.2.0-rc1, none of the three. The
decision of 2026-11-12 is there: `run toss` of a record of none removes its kept copy and leaves the branch.
`run.md` says the same under its table of what a run writes.

**Tests, findings 3 to 6.** `copy clean`'s fixed line for a branch that is gone and for one that is symbolic
(`f38.rs`); a look refuses a repo whose config says `core.bare=true` (`f44/refused.rs`), and `copy make` does
(`f38.rs`); a run that changed nothing, its name made a name for `main` before the last check, leaves a plain
branch at `start` (`f48/raced.rs`); an 11-byte file that cleaning shortens is not `cut` (`f44/bounds.rs`).
`raced()` now also asserts the made-up git was at its step: before, a run that never came to the step passed.

**Docs, findings 5 and 6.** The unchanged path "moves nothing; a name that was made a name for another branch
is a plain branch again" (`run.md`, `copy.md`). `cut` is true whenever the file was not read whole or the
cleaned text was cut at `most` (`show.md`).

**`init.sh`, finding 7.** The scoreboard reads every name on a `blockedBy` line.

## What is not closed

- Windows and Linux were not run for this change: only a Mac. CI is the first run of the new tests there. The
  test at the binary needs a git that warns of an unknown `core.fsync` part (git 2.36 or later) and says so
  when it does not.
- The baseline `./init.sh` ran while the first new test was already in the tree, and went red on that test
  alone (the stderr scene at the binary, red before the fix). The whole of `make preflight` was run once more
  at the end.
- What a run's own git steps print (stdout) is still read whole into memory. Bounding it means deciding what a
  cut answer of `ls-files` or `write-tree` means, which is more than this change.
- The scoreboard has no test: its proof is a made-up feature list run by hand, pasted in the pull request.
- F46's third step says a failed or cancelled run's branch goes "by the same rule". For a record that names
  none, the decision above leaves the branch. The row was not edited.
- ADR 0017's earlier sentence, that `run keep` and `run toss` refuse a record that names no commits, is now
  read as scoped by the decision: `run keep` refuses, `run toss` takes back the copy and no more.

## What the next session does

F45 (`run keep`) and F46 (`run toss`) build on ADR 0017's dated section, with a record of three, of two and
of none each given its rule.
