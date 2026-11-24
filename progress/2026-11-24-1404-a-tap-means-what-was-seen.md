# A tap means what was seen (2026-11-24)

One slice in the kit and the app: a click in the window is safe against a page that changed under the pointer.
ADR 0030, decision 9. No row of `feature_list.json` changes. Core and the binary are as they were.

## Built
- **A choice that has just moved takes no tap** (`crates/sticker-kit/src/draw/settle.rs`). The view keeps, for
  each option and each drawn note's picture (its `?` is inside it), where it stands in the column and since
  when. A click on one is no tap for `SETTLE` (400 ms) after its place in the column changed by more than 4
  design units, and for `SETTLE` after it came onto the page at a window point where another piece took a tap in
  the frame before. A piece is the same one across frames by its kind, its note's id and its index. The wheel,
  a column held at its end and a window that changed size hold nothing back; a new piece where nothing took a
  tap takes one at once. A click that was no tap is not kept and nothing is drawn for it.
- **The clock** is the page's own time, the seconds since it was made, which its drawing goes by;
  `Page::set_settle_time` tells it one, for a test.
- **A tap carries the words that were drawn**: `Tap::Option { note_id, index, words }`, the scene's own string
  (`Scene::option`) as it was when the pieces on the page were laid out. A tap on a picture carries none.
- **The app acts only on what was seen** (`app/src/showing.rs`): `Showing::chose(note, index, words)` carries a
  tap only when the note of that id on show has those words at that place. Otherwise nothing is carried to
  core, nothing is written, and the scene on show is given back to be drawn again.
- The walk passes the words it named the option by; the tests' made-up window carries the words of the scene
  its view holds, and `choose:<note>:<index>:<words>` carries the words of an older drawing.

## Red first
- Kit, `tests/column_draw.rs`, with the guard off and the words left out: a pushed-down choice and a choice
  that came under the pointer both took the tap at once; every tap on an option carried no words (also in
  `tests/draw.rs`, `tests/entry_draw.rs`); the unit test of `settle.rs` failed at "it has just moved".
- App, with the comparison off: `tests/several_wishes.rs` (the tap that carried `Toss it` to the place of
  `Keep it` was carried: the page said the work was being kept), `tests/f43/showing.rs` (a choice not on show
  gave nothing back), `src/showing/tests.rs` (a tap with other words was carried, and core refused it).
- The two tests that say what must NOT be held back (the wheel, a new card at the column's end) were red only
  for the words; they are what the second mutation is killed by.

## Mutations (five, each killed)
| Mutation | Killed by |
|---|---|
| the guard off | `column_draw`: the pushed-down choice, the choice under the pointer |
| the place kept is the window's | `column_draw`: the wheel, then the same scene given again |
| `SETTLE` is 0 | `column_draw`: the pushed-down choice, the choice under the pointer |
| the app compares nothing | `several_wishes`: the tap that carried other words |
| the app asks only that the place has an option | `several_wishes`: the same test, the same line |

## Changed for callers
- `Tap::Option` has a third field, `words`. `Showing::chose` takes the words as a third argument, and a tap
  on a note that is not on show now gives the scene on show back (it gave nothing).

## Deleted
- Nothing was removed but `Page::tap`'s place: it moved from `draw/mod.rs` to `draw/settle.rs`.

## Lines
- Rust: 402 added, 60 removed. Kit budget 12390 to 12673 (+283); app budget 18306 to 18365 (+59).
- Docs: the kit's README, `docs/development/app.md`, ADR 0030 (decision 9, and its open list).

## Open
- No click was made in a real window. Linux and Windows were not run.
- A column held at its end that gets a card ABOVE a choice holds that choice back for `SETTLE` although it did
  not move in the window; and one that gets a card at its END moves its older cards up the window, which is
  not held back (the place kept is the column's).
- A choice whose words change in place (same note, index and place) takes a tap at once: only the words its
  tap carries tell the two apart, and the app refuses the tap.
- A drawn note that opens moves its own options down, and they take no tap for `SETTLE` after the person's own
  tap on the picture.
- The rule is a section of ADR 0030; it has no ADR of its own.

## After the review: the place is the window's, as a layout moved it

The review, and a reading of `relayout`, found the guard wrong both ways for a reader at the column's end, which
is where a reader is. A card at the end left older choices where they were in the column and slid them up the
window, not held back; a card above moved them in the column and not in the window, held back for nothing.

- `Settled::meet` now takes the column's lift before the layout and after it, and a piece has moved when the
  layout put it somewhere else in the window. A turn of the wheel happens between layouts and is in the lift
  before, so it is still never counted; a resize is still only found again.
- Red first, under the old rule: both new tests of `tests/column_draw.rs` (a card at the end; a card above a
  column held at its end) and the two new lines of the unit test. Green: the kit's 172 tests.
- Lines: 75 added, 31 removed with the docs; the kit's budget 12673 to 12716.
- Not run: a click in a real window; Windows and Linux.
