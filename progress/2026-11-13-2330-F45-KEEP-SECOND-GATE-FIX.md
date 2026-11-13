# F45: the ten findings of its second gate

On branch `f45-keep-fix`, one commit on top of `d8096e3`. The second gate failed PR 166 with ten findings, three
of them blocking. This commit closes them. The account is `docs/development/keep.md`; what was decided and
measured is ADR 0017, "Found by the second gate on F45".

## What changed

- **Attributes are asked of the index the step would leave** (`copy/keep/attrs.rs`, `copy/keep/fits.rs`). The
  trial, once passed, is taken again into the copy of the index (`keep.trial`); a `.gitattributes` the work
  removes is no longer read, and one it changes or adds is read from that index.
- **A run's branch is never where work was kept** (`copy::runs`, `Unkept::OnRun`, `copy/keep/step.rs`), for the
  step and for a record's `onto`.
- **Why and how moved to `copy/keep/why.rs`**: `Unkept` and `How`, still `copy::Unkept` and `copy::How`. Four new
  reasons: `Spelled`, `Unmerged`, `Name`, `OnRun`.
- **A keep begun again asks `check-ignore`** of what is where a made file would go.
- **The list of a change is one function** (`copy/work.rs`: `changed`), for a look and a keep.
- **`or_test` reads its variable only in a build with debug assertions** (`busy.rs`).

## What the next session should know

1. **F46 (`run toss`) builds on this.** `copy::alone`, `copy::nameable`, `copy::LOOSE`, `Git::opened`,
   `Repo::seen_from`, `How` and the constants of `run/keep.rs` are as they were. `Unkept` has four variants more,
   and `copy::runs` is new: a toss that reads a record's `onto` can use it.
2. **The made-up git has two new files**: `after` (lines the real git does just after the step) and `fails` (the
   step is not taken and ends with that number). `tests/f45/raced.rs` has `told` for both.
3. **The red run of a new test must not share a target folder with a copy of the sources**: cargo takes the two
   for one, and the next build runs the old binary.
4. **Not run**: Windows and Linux by hand, a disk that keeps case apart (the test of a branch under another
   spelling skips there), git before 2.50.1, the outcome of `HEAD` pointed at another worktree's branch.
