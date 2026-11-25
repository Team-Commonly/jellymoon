# An earlier wish tucks when it settles (2026-11-25 01:30 UTC)

Branch `an-earlier-wish-tucks`, on `9214ab2` (`the-answer-stays`). No row of `feature_list.json` changes. The app
only: core and the kit are as they were.

## Why
Two wishes entered back to back, the second while the first was still at work, ended as two open results, one
under each wish. Finished wishes tuck away so that only the newest work is open (ADR 0030, decision 11), and the
rule looked at a wish at one moment only: when the next wish was made. The first wish was at work then, so it was
not settled and not tucked; it settled later, and nothing looked at it again.

## Built
- `Tucked` (`app/src/drawing/tucked.rs`) holds a second set: the wishes that are owed the tuck. When a wish is
  made in the window, or the window opens, every made wish but the newest that is at work is owed it (the wish an
  answer is under is left out, as it was). At the first drawing where an owed wish's run goes no more
  (`Tucked::away`) it is tucked away if it is settled then, and it is owed nothing after that either way.
- So the page is what it would be had each wish been made after the one before it ended, whichever run ends
  first. Nothing of undo, of an open card, of the order of wishes or of a single wish changed.
- Kept, and read from the scene as before (`tucked::settled`): a wish is never tucked while its little one says
  it needs the person (a question, a run that failed), while work of it waits (`work-<wish>`: a tap to keep, a
  change that does not fit and has `Make it again`), while a note of it is in the person's hand, while core's
  answer under it offers a choice, or while a note of it has a field to answer in. Also kept: the wish an answer
  is under when that answer made a wish, and a wish that ended since the last look and has no note yet.
- `Undo it` on an earlier card: core offers it only for a change at the tip of its branch
  (`run/waits/itself.rs`, `Stands::Here`); a change with a later commit over it says it cannot be undone here. A
  tucked earlier wish opens with a tap, with the same rule: no `Undo it` while the newer change is over it, and
  `Undo it` when it is the last one in (the run order reversed in the test).

## Red first
With the owing switched out (today's behaviour), in one run:
- `drawing::tucked::tests::a_wish_at_work_under_a_later_one_tucks_away_…`: failed, nothing tucked.
- `finished_wishes_tuck_away`: `an_earlier_wish_that_ends_after_a_later_wish_was_made_tucks_away_whichever_…`
  failed (`m1:kept-m1[Undo it] m2:steps-m2`, then both results open); `of_three_wishes_entered_back_to_back_…`
  failed (three results open); `a_settled_wish_tucks_away_when_the_next_wish_is_made_…` failed at its changed
  line.
- The two tests of a wish that needs the person passed before and after: they hold what must not change, and
  mutations 2 and 3 are what they fail on.

## Mutations (five, one hold, against `tests/finished_wishes_tuck_away.rs`)
| # | Mutation | Killed by |
|---|---|---|
| 1 | no wish is owed the tuck (today's behaviour) | the red run above |
| 2 | a wish that needs the person or has a field to answer in counts as settled | `…needs_the_person…` (and the window opened on four wishes) |
| 3 | work that waits counts as settled | `…needs_the_person…`, `…does_not_fit…` |
| 4 | the newest wish is owed it too | `…whichever_of_the_two_ends_first`, `of_three_wishes…`, `…does_not_fit…` |
| 5 | the tuck is undone at the next drawing | `…whichever_of_the_two_ends_first` (the second look) |

## Tests changed
`a_settled_wish_tucks_away_when_the_next_wish_is_made_…` said that a wish which settled after the next was made
stays on the page. That was the behaviour this change removes; it now says the wish is tucked away.

## Open
- Two results can still be open at once: a wish that ended in the second before the next was made, with no look
  between (it has no note yet, and the rule for that is kept), and an earlier wish the person answered or kept
  after a later one was made.
- A wish the person stops while it is owed the tuck is tucked away when it is let go.
- Not seen in a real window, not run with a real harness, not run on Windows or Linux.
