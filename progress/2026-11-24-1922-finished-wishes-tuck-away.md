# Finished wishes tuck away (2026-11-24 19:22 UTC)

Branch `finished-wishes-tuck-away`, on `bb17293`. No row of `feature_list.json` changes. The app only: core and
the kit are as they were.

## Why
ADR 0030 put the notes of every ended made wish on the page at once. After three wishes the column was several
screens of cards, most of them about wishes that need nothing from the person, with the wish being made under
all of them.

## Built
- **Settled** (`app/src/drawing/tucked.rs`, `settled`), read from the scene the app holds: the wish's little
  one is idle or greyed (core says done or let go); no `work-<wish>` note; no note of it in the person's hand
  (`placing::in_hand`); its `said-<wish>` offers no choice; no note of it has a field to answer in. A wish
  core says needs the person, as one whose run failed, is never settled.
- **Tucked away** (`Tucked`, held by `drawing::Open`, so by `showing::Showing`): a set of wish ids. It grows
  when a made wish is kept in the window (`Showing::answered`, an answer that asks for a run: Enter in the
  entry or in a follow-up's field; `Showing::ended`, when core's answer to a choice made a wish, where the wish
  of that answer stays), and when the window opens (`Open::opening`: every settled wish but the newest). It
  shrinks at a tap on a tucked wish (`Open::poked`), and when a tucked wish on the scene is settled no more
  (`Tucked::away`); that wish stays shown when it settles again.
- **Drawn** in `drawing::passed`: the notes of a tucked wish that is settled are left out, and its bubble and
  little one stay. The scene the app keeps holds every note; `layout` tucks nothing away.
- **The tap** on a tucked wish brings its notes back, made again from core, and opens no drawn note; its look
  gives the scene even when core says nothing new, so the page is drawn again.

## Red first
With `tucked.rs` tucking nothing away and leaving every note, the tests as they are now:
- `drawing/tucked.rs`: all four tests. `showing/tests.rs`: the new test.
- `app/tests/finished_wishes_tuck_away.rs`: the first wish's `kept-m1` was under its bubble after the second
  wish was made.

## Mutations (one hold, against the app's `--lib` tests named `tuck`)
| Mutation | Caught by |
|---|---|
| A wish whose work waits is tucked away | the bubble test; settled wishes tuck away when asked |
| A wish is tucked away when it settles (at every drawing) | a window opens; the `Showing` test |
| The newest wish is tucked away at opening | the `Showing` test |
| A tap brings nothing back | a window opens; the `Showing` test |
| A tucked wish's bubble goes with its notes | the bubble test |
| A wish with a choice under core's answer (`Make it again`) is tucked away | the note-in-hand test |

## Picture
`.tools/pictures/finished-wishes-tuck-away-sticker.png` and `-plain.png` (not tracked), made with the snapshot
example and no new flag: `--made 4` and a `--notes` file that holds the newest wish's kept note alone, which is
the page a window draws with three wishes tucked away. Seen: four bubbles at the right, each with its little
one (three with a star, the one let go grey), and under the fourth alone its picture, two sheets going to the
folder, with `Undo it`; the whole day is in one window with the entry under it. Unclear: nothing on a tucked
wish says that a tap shows more, and the newest wish's note stands at the column's left, far from its bubble.

## Tests changed, and why
None changed. New: `app/tests/finished_wishes_tuck_away.rs`, four tests in `drawing/tucked.rs`, one in
`showing/tests.rs`. `several_wishes.rs` passes as it was: its second wish is made outside the window, which
tucks nothing away.

## Lines
Rust: 394 added, 22 removed (175 `drawing/tucked.rs`, 106 the window test, 43 `showing/tests.rs`). The app's
cap 19026 to 19398, the 372 net lines. Of those 91 are outside the two new files, which hold 281 with their
tests. Deleted: nothing.

## Open
- Not run: a real window, a real harness, Windows and Linux. The whole workspace suite was not run.
- The first commit was refused by the hook: the line check counts tracked files, so the cap was short by the two
  new files until they were staged. The cap was corrected and the commit made in a fourth hold of the build lock.
- The window test taps the first wish before the second lands: once a later change is over it, core takes the
  older change's `Undo it` away, so the tap could not show it after.
- A wish made outside the window (`jellymoon wish`) tucks nothing away until a wish is made in the window or a
  window opens.
- A wish that needs the person is never tucked away, so a day of failed runs is as long as before; bubbles
  alone can fill a window on a day of many wishes.
- A tucked wish's notes are still made again from core at the moments of ADR 0030, decision 6.
- The tap that brings a wish back does not open its drawn note; a second tap does. Nobody has seen whether
  that reads well.

## After the review
A review by reading passed the commit and found one moment a person can come to, closed in a second commit: a
wish whose run ended in the second before the next Enter is idle in the scene of that Enter's answer with no
result note yet, and was tucked away unseen. `Tucked::tuck` now takes `drawn`: when a wish is kept, a settled
wish with no note on the scene stays; a window that opens has drawn nothing and tucks as before. Test, seen red
first: `a_wish_that_ended_with_no_note_on_the_scene_yet_is_not_tucked_away`. The app's cap 19398 to 19413.
