# Design: the page moves on to a new day by a tap (ADR 0022, F83 and F84)

**What changed.** On branch `design-midnight`, on main `d31fbf8`. Docs only: one commit, no code and no test.

- `docs/adr/0022-the-page-moves-on-by-a-tap.md`: the coordinator's seven rulings as five decisions, what the
  code is today with its files and functions, every sentence a person will read, what was turned down and the
  limits that are named and not built.
- `docs/GLOSSARY.md`: **page's day** and **quiet** (a day).
- `docs/INDEX.md`: the ADR's line.
- `feature_list.json`: F83 (a run of a later day is read as going only while it is held, with the five notes of
  the review of "jellyfish notes 2") and F84 (the move by a tap), blocked by F83. No other row is touched.

**Where the design leaves a ruling's words.**
- The rulings say "settled". The glossary has a settlement, which is a clash's, so the ADR says a quiet day:
  the day core's look for the jellyfish gives `quiet` for.
- Core's question is a number, not yes or no: the jellyfish's count would say two for one run held and one cut
  off.
- Typed words go with a new view, not with a day in the fields' keys: `more-m1` is the same key on every day,
  the entry keeps its words too, and the kit has no call that empties either.

**What's next.** F83, then F84. After them: an earlier day left with a question or waiting work, which nothing
shows when the app is opened fresh on a later day.

**Not confirmed.** That GPUI at the pinned commit changes a window's root view. Nothing was built or run but
`cargo xtask check`.
