# F45: the five findings of its fourth gate

On branch `f45-keep-fix`, one commit on top of `2387dc5`. The fourth gate failed PR 166 on one scene and named
four smaller things; Windows CI failed one test. The account is `docs/development/keep.md`; what was decided is
ADR 0017, "Found by the fourth gate on F45".

## What changed

- **A name is where work was kept only when the repo lists it at the commit** (`copy/keep/step.rs`, `landed`).
- **`stayed` names a plain file at its path**, under folders that are folders (`copy/keep/attrs.rs`); its words
  claim no cause, and past 200 a fixed clause says more are there (`run/keep.rs`, `MORE`).
- **A checkout on a branch named as a run's is** is refused in those words.

## What the next session should know

1. **A disk that folds case does one of two things when git moves a branch through another spelling**: a Mac
   keeps the file's spelling, Windows gives it the new one. The scenes of `tests/f45/raced.rs` ask the repo what
   it lists after the step and expect what follows from that; a new scene of this kind must too.
2. **F46 and F47**: no name of core's and no printed name changed shape. Three sentences changed.
3. **Not run here**: Windows and Linux; the re-spelled arm of the `Feature` scenes runs only on Windows.
