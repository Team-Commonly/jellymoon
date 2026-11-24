# A remade wish steps aside (2026-11-24 22:59 UTC)

Branch `a-remade-wish-steps-aside`, on `0c07ea7` (`true-words-on-the-page`). No row of `feature_list.json`
changes. The app only: core and the kit are as they were.

## Why
Walked in a window: a change did not fit, `Make it again` was tapped, and the wish was made again and landed.
For that one wish the page then held the old wish's bubble, a card that it was let go, the answer card, and
the new wish's bubble with the same words a third time. Three cards for one thing asked for once.

## Built
- **The old wish is tucked away at core's answer, when all of it worked**: the wish was made again, the old
  work was tossed and the new wish's run was started. Its bubble and its greyed little one stay, none of its
  notes are drawn (ADR 0030, decision 11's tucked), and the new wish is at work right under it.
- **Decided from core's typed answer.** After the toss, `Today::asked` (`app/src/again.rs`) asks core for the
  day's made wishes and reads the old wish's state: `WishState::LetGo`, which a toss writes last, once
  everything else of it was done. It goes with the answer (`again::Remade.tossed`), which is made only when
  the run was started. No sentence is read.
- **Where.** `drawing::Open::remade` tucks the wish (`Tucked::aside`, the window's own set) as it keeps the
  answer, from `Showing::ended`. The making of a wish still tucks every other settled wish away by its own
  rule, `drawn` included: that rule is not changed, and this one names a single wish for its own reason.
- **Nothing is tucked** when the wish was not made (no installed command, the day's wishes full), when the
  toss was refused or left unfinished, and when no run was started: that answer stays on the page.
- **A tap brings it back** as for any tucked wish, until the next wish is made. That tap makes the wish's
  notes again from core, which leaves an answer out, so the answer that a wish was made again is put back
  under them (`Showing::poked`, `Again::under`): found by being that answer, whole.

## The page after `Make it again`, note by note
The first wish's bubble and its notes as they were; the old wish's bubble with its greyed little one and
nothing under it; the new wish's bubble with the same words, its little one at work, and its track. After a
tap on the old wish: "This wish was let go." and "The change was thrown away." with its two sentences, the
second ending "…and it is being worked on." while the new run goes and "Your wish was made again." after.

## Red first
- `tests/a_change_that_no_longer_fits.rs`, the test of `Make it again`, on the sources as they were: the
  column after the answer had `said-m2` under the old wish's bubble (`left: ["said-m2"]`, `right: []`).
- The other tests of that file are guards and were green on the old sources: nothing was tucked there
  before. Each is killed by a mutation below.
- The new test of a refused toss failed in the first round for a reason of its own making: the made-up
  window draws only at a `column` act, so the wish tucked away as the window opened (a newer wish, the
  follow-up, is on the day) was never drawn while its work waited. It now draws the page once after its
  first look, as a window does.
- `drawing/tucked.rs`, the test of `Tucked::aside`: new with the function; it did not build before.

## Mutations (one hold, each against `tests/a_change_that_no_longer_fits.rs`)
| Mutation | Killed by |
|---|---|
| the old wish is never tucked | the test of `Make it again`, at the column after the answer |
| tucked when the wish could not be made again | the tests of no installed command and of a full day, at the drawn answer (and the refused toss) |
| tucked when the toss failed | the test of a refused toss, at the drawn answer |
| a tap does not bring the notes back | the test of `Make it again`, at the column after the tap |

## Picture
`.tools/pictures/a-remade-wish-steps-aside-sticker.png` and `-plain.png` (not tracked), made with the snapshot
example and no new flag: a page of three made wishes (one done, one let go, one working with the same words),
`--made 3` and a `--steps` file, which is the page a window draws once the old wish stepped aside. Seen: three
bubbles at the right, the first with a starred little one, the second with a grey one and nothing under it, the
third with the same words, a little one at work saying "making the change", and its track under it. Unclear:
nothing says why the same words are there twice or that the grey one was replaced by the one under it, and
nothing on the grey one says a tap shows more.

## Tests changed, and why
`tests/a_change_that_no_longer_fits.rs`: the test of `Make it again` read the answer in the drawn column
right after it; the answer is no longer drawn there, so it reads the tucked column, taps the old wish, and
reads the answer in the column after the tap (while the run goes, and once it ended, as before). The tests
of a full day and of no installed command each print the column and say the old wish's answer is drawn. New:
a toss core refuses after the wish was made again (a follow-up's run stands on the work).

## Lines
The app +112 (141 added, 29 removed); the cap in `entropy.toml` 19694 to 19806. Nothing was deleted but the
lines changed in place.

## Open
- Not run: a real window, a real harness, Windows and Linux.
- A toss that wrote its record and could not tell the wish is not seen in a test: core then does not keep
  the wish as let go, so nothing is tucked.
- A wish core kept as let go before the tap, with work that still waits, would be read as tossed whatever
  the toss answered. Core writes no such state by itself.
- The answer comes back with the notes of any tucked wish it is under, also one tucked by the making of a
  later wish. A tap on a wish that is not tucked leaves it out, as before.
