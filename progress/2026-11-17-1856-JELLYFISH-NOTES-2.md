# Jellyfish notes 2: the blocker and four notes of the review after the jellyfish notes were merged

**What changed.** On branch `jellyfish-notes-2`, on main `5d4711a`. One commit; no row of `feature_list.json`
changes. Core did not change.

- **Of a later day the jellyfish shows only a run at work.** The page shows the day the app was opened on and no
  other, so a question or finished work of the new day was said to wait "on the page", and the page did not hold
  it. The look (`shell.rs`, `jelly_look`) now takes from the later day a run at work and nothing else, and shows
  it when the page's day has no question and no run at work of its own.
- **Only a later day is read.** A clock set back, or a move west, gave a day before the page's, and its old
  question could be shown. The days are ISO dates and are compared as text.
- **Waiting work goes through the real look again.** The once-a-second test types a done wish and a completed
  run's record (core's own example) for its fourth step, on the page's day.
- **Deslop**: `JellyLook` is private; the doc comment of `window()` in `main.rs` is wrapped; the module doc of
  `shell.rs` names `looks.rs` too.
- **Docs**: `app.md` lists what the jellyfish does not show past midnight, and says the page moving on by itself
  is not built. ADR 0019 has no sentence this makes false.

**The evidence.** Red first on the look as merged: the sixteen pairs (a later day's question drawn over a quiet
page's day), the later day's question beside its run at work, and a day before the page's whose run was drawn.
13 one-rule mutations of the look, 13 killed (`app/tests/f57/looks.rs`).

**What's next.** The page moving on from the day the app was opened on, which needs a decision first (the
review's second step).

**Surprising.**
- Core gives one state for a day, the question first. So a run at work on the later day is not seen while that
  day also has a question: the app cannot ask core for the day's runs at work alone. A test pins it and `app.md`
  says it.
- A cut-off run of the later day still swims until the app is opened again. Core's question for whether a run
  still goes (`run::held`) is private to its `run` module, and the public calls that use it (`made`, `going`)
  write the run down. Asking it from the app needs one new public function in core that reads and writes
  nothing, so it was not built; `app.md` says the limit.
- `>` against `>=` for the later day cannot be told apart by what is drawn: reading the page's day twice shows
  the page's day.
