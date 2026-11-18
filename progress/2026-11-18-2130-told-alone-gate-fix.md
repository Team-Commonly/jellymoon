# Told alone, gate fix

## Fixed
- **The blocker.** A look stopped at the first wish that answered, so a told-alone note took the first look's
  slot from an older wish's work that waits, for the whole day. `Showing::look` now asks on past a told-alone
  answer (`waiting_over` says whether its scene is work), shows the first work that waits, and only else the
  first told-alone note. A wish passed over is not marked as having nothing, so it keeps its turn.
- **A told-alone note gone a second later.** It has nothing to choose, so it was never in hand and the next look
  could put another wish's work over it. `Turns::saw` now holds the three looks after a scene that carried a
  `told-` note with no `work-` note (`working::told_on`), as a choice's answer is held. A tap that opens one holds
  them too.
- **The sentence.** "It stopped before it was done, and nothing in your folder was changed." is now "It did not
  finish, and nothing in your folder was changed.": a run whose work could not be committed, whose branch was
  moved or whose end was not recorded had finished.

## Red first (`app/tests/told_alone.rs`)
- `the_first_look_shows_an_older_wish_s_work_that_waits_before_a_newer_wish_told_alone`: the first look showed
  `told-m2`, `wish-m2` in place of `work-m1`.
- `a_wish_told_alone_passed_over_for_work_that_waits_is_shown_after_the_person_chose`: the look showed `told-m1`,
  so `Toss it` of `work-m2` was never there.
- `a_note_told_alone_shown_by_a_look_holds_the_looks_after_it`: the look after the first put `work-m2` over it.
- The two did-not-finish tests, on the old sentence.

## Mutations (one hold, `tests/told_alone.rs`)
| # | mutation | killed by |
|---|---|---|
| 1 | the look stops at a told-alone answer | the first-look test and the passed-over test |
| 2 | a wish passed over is marked as having nothing | the passed-over test |
| 3 | no hold after a told-alone note | the hold test |

## Holds
Three, one over the brief's two: the second stopped at its first test, since the did-not-finish test forbade the
words "did not finish" (meant for core's reason, "run <id> did not finish (..."), which the new sentence holds. The
test now forbids "did not finish (".

## Budgets
core 17905 -> 17906 (+1), app 15087 -> 15219 (+132, 113 of it tests). The binary is unchanged.

## Open
- The other gate notes (the day's eight wishes, a tossed unchanged run in the app, `ended_alone` reading the
  home twice) are in the backlog, untouched.
- Windows and Linux not run; no real window.
