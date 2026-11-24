# The page is one column (2026-11-24)

Slice 1 of 3: the column from today's data. The same `Scene`, the same calls to core, the same taps. ADR 0026.

## Found
The page was a fixed canvas: a grid of characters, a narrow strip of notes, and a planner that shortened, shrank
and dropped notes to fit the window. A day reads better as one column, top to bottom: what the person typed, then
what came of it, the newest at the bottom, and the box for the next wish always under it.

## Built
- Kit: `layout()` places one column (`layout/column.rs`): the header, a row for each character nobody typed for,
  then the scene's notes in the scene's order, each a card of one width at its full height. `Note::Said` is a
  bubble at the column's right, drawn by the quote's rules, with its character at the bubble's end; a tap on the
  bubble is a tap on that character. The big character stands left of the first card and of the first card after
  each bubble. The entry is a band at the window's bottom (`entry_top`), placed in the window and never moved.
- Scrolling: a `Page` keeps its own offset (`scroll.rs`, `draw/column.rs`): the wheel moves the column between its
  top and its end, and a page at its end stays there while the column grows. The layout stays pure, in the
  column's own places; `Page::shown_at` says where a piece is in the window now.
- App: `src/column.rs` orders a scene for its column, applied in `lines.rs` and `window.rs` only: the day's own
  notes, then each made wish, oldest first, as its typed words and its notes in one fixed order.

Deleted: `layout/grid.rs`, the planner in `layout/notes.rs` (rooms, shortening, the count of notes left out), the
told note's second try in `layout/told.rs`, the legend and the page-wide footer line of `layout/page.rs`, the size
a whole choice's options were written smaller at, and `tests/entry_band.rs`, `tests/told_sweep.rs`. The footer's
words stay, as quiet lines under the title.

## Tests deleted, and why
Kit: `dimmed_characters_come_last`, `eight_longest_labels_fit_at_default_and_small_sizes`,
`notes_stack_on_the_right_clear_of_the_characters`, `a_long_list_gives_up_rows_to_the_notes_after_it_and_says_how_many`,
`more_characters_than_slots_flow_to_more_rows` (the grid and the strip are gone; rows and cards are pinned by
`layout.rs`'s new tests, let-go rows last among them); `a_text_note_with_options_is_given_the_room_of_its_whole_body_before_the_notes_after_it`,
`a_told_note_takes_room_from_the_notes_after_the_one_it_is_over_and_never_from_that_one`,
`with_a_told_note_the_count_of_notes_left_out_is_one_plain_line_and_without_one_the_sheet_it_was`,
`a_result_never_changes_what_the_work_s_note_shows` and the sweep (no note takes room from another);
`a_whole_list_with_no_room_for_even_a_count_draws_nothing_moves_nothing_and_the_footer_is_drawn` (there is always
room); `a_whole_option_written_smaller_is_painted_at_the_size_it_was_measured_at` (nothing is written smaller);
`the_small_characters_begin_under_the_entry_wherever_a_theme_puts_its_big_one` (the entry is in its band).
App: `a_page_with_a_wish_says_once_that_a_jelly_opens_it_and_a_page_with_none_does_not` (the legend is gone).

## Pins changed, and why
Every one of these pinned "left out and counted for room", and now pins "drawn whole":
`f28/door.rs` (three clashes and the list; the fullest page; a quote with no spaces: all of it or its first three
lines), `f35.rs` (two long clashes), `f37.rs` (every made wish is a bubble of its own), `f47/drawn.rs` (long names
keep their yes), `f86.rs` (a long did; rows of many lines), kit `layout.rs`, `told.rs`, `whole.rs`, `text_note.rs`
(a body is cut at its most lines, not at its room). Positions: `entry_first.rs`, `f34.rs`, `f63/chips.rs`, kit
`entry_layout.rs` (the field is in the band at the window's bottom, as wide as the column), `says.rs` (what a busy
character says is on its row), `work_shows.rs` (an option's place is measured from its own card's top: a note
that comes or goes higher up moves a card whole). Legend pins removed from `entry_first.rs`, `f28.rs`,
`open_a_wish.rs`. Clicks in a window go to `shown_at` (`f54/window.rs`, `open_a_wish/window.rs`,
`support/windowed.rs`, kit `draw.rs`, `field_draw.rs`); the test window is still, since a card that opens now moves
the pieces under it and a tap is taken where a moving piece is drawn. Every option a test tapped is still placed.

## Red first
`app/tests/one_column.rs` cannot compile against the code before (`Note::Said`, `column`), so its red is five
one-rule mutations, each killed by the one test file run against it: two prefixes of the order swapped, a wish's
notes put before its typed words, a note dropped when the column is long, the typed words placed as no quote
(`one_column.rs`), and the entry's band moved with the column (kit `tests/column_draw.rs`).

## Open
- The day's own notes stand at the column's top, so one that comes or goes moves every card under it.
- The rows of a reading's wishes are at the top, out of view on a page held at its end. No key scrolls and there
  is no scroll bar; no test reaches a real wheel, a trackpad's momentum or a drag.
- The list of made wishes is still built by `scene::ruled` and taken out again by `column`: slice 2.
- A made wish has no label pill: its words are its bubble. Core cuts what it keeps of a result to two decided and
  two unsure after the day's own, which the old room rule had hidden.
- No real window was opened and no picture taken. Windows and Linux not run here.
