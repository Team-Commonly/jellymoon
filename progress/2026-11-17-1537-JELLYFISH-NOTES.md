# Jellyfish notes: the blocker and five notes of the review after the F57 notes were merged

**What changed.** On branch `jellyfish-notes`, on main `935f6ce`. One commit; no row of `feature_list.json` changes.

- **Past midnight the jellyfish looks at the page's day and at today.** The page files what it writes under the
  day the app was opened on, and since the F57 notes the jellyfish asked for today alone, so a wish made on the
  page after midnight was missing from it and a run begun before midnight dropped off it. With no day named the
  look now reads both days and shows the first in core's order (`JellyState::ORDER`, read by the app, not copied);
  of two in one state, the page's day's. A day named with `--date` is one day, as before. Core did not change for
  this.
- **The look is built in `shell::wire`**, from the page's `Today`, a `named` flag and a clock (`shell::Asked`), so
  the tests on GPUI's test platform reach it. `jelly_look` is private, and `main.rs` hands over the flag and the
  system's clock and nothing else.
- **A failed open of the bubble is said again when its reason changes**, and after any draw with no failure, a
  quiet one too (`jellyfish.rs`, `news`). Every draw goes through the one rule; a row of the same failure is
  still said once.
- **One search answers both harness questions.** `run_apart`'s search hands on what it missed
  (`run::asked_apart`, crate-only), and the hearing's sentence is a function of that (`hear/day.rs`, `unstarted`).
  The long line that names folders cannot reach the bubble for a missing harness.
- **Docs**: ADR 0019's F57 note, point 1; `app.md` on what the jellyfish looks at past midnight.
- **Deslop**: `Source::Day { day, named }`; the closure in `input` is `why`.

**The evidence.** Red first: a wish made by the page's own call after the clock moved, a run that asks begun on
the page's day, the order between two days (`app/tests/f57/looks.rs`); a different reason said (`jellyfish.rs`);
the three sentences given as data on a machine that is in one of them at most (`hear/day.rs`). 13 one-rule
mutations, 13 killed.

**What's next.** The page moving on to a new day at midnight is open, and `app.md` says so.

**Surprising.**
- The page's own look writes a working wish of its day down as cut off when nothing holds its run, so a made-up
  run that goes on the page's day needs its `run.lock` held by the test, as a `jellymoon run` that lives holds
  it. A wish typed in as working and nothing else is quiet a second later.
- A wish made through the page has no run until its asking is run, and the asking cannot be run in a test's own
  process, so the test types the run into the kept wish. No test sees a real run begun from the page past
  midnight.
- Runs that go on both days are not counted together: the count drawn is one day's.
- Not proved: that `main.rs` hands `wire` the flag it parsed and the system's clock (it opens a real window), and
  that a failed open reaches stderr (no open fails on the test platform).
