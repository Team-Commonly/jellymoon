# F45: the nine findings of its third gate

On branch `f45-keep-fix`, one commit on top of `27565e5`. The third gate failed PR 166 on two narrow scenes and
named seven smaller things. This commit closes them. The account is `docs/development/keep.md`; what was decided
is ADR 0017, "Found by the third gate on F45".

## What changed

- **Only a name the repo lists is where work was kept** (`copy/keep/step.rs`, `landed`). A name `HEAD` gives
  that `for-each-ref` does not list is a run's when it is under `refs/heads/jellymoon/run-` in either case
  (`copy::runs`, now without regard to ASCII case everywhere), and otherwise work kept on no branch.
- **A `.gitattributes` git could not remove is said after the step** (`copy/keep/attrs.rs`, `stayed`;
  `Note::Stayed`). What `run keep` prints has a new `stayed`, left out when empty, and `why` then ends with
  fixed words. Still `work: kept`, exit 0.
- **What stops the second take of the trial is in the way** (`copy/keep/fits.rs`), as what stops the first is.
- **Gone**: the list of what the step's index holds (`attrs.rs`), which changed no answer.
- **Moved**: the unit tests of `run/keep.rs` to `run/keep/tests.rs`, to keep the file under 400 lines; the keep
  and its answer are `landed` there, to keep `keep_run` under 60.

## What the next session should know

1. **F46 and F47**: `Kept` has one new name, `stayed`, which no answer has but that of a keep that wrote past a
   `.gitattributes` git left. `How` and `Unkept` are as they were. `Note` has a third kind, `Stayed`, which
   writes nothing to the record. `copy::runs` has the same shape and now takes `Jellymoon/run-x` too.
2. **The stand-in keep of `tests/f45/raced.rs` (`told`) passes `--with-settings`**, so a raced scene may touch
   a `.gitattributes`.
3. **A split index's shared part has its time changed by every read of the index**, a question's too. Nothing
   stops it; `keep.md`, "Asked first", says so.
4. **Not run**: Windows and Linux by hand; a disk that keeps case apart (the spelling scenes skip there); a
   system with no folder that can be shut (the scene of the file git cannot remove skips there).
