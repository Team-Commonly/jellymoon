# The kit draws in pencil (2026-11-24)

One slice in the kit: a note can be a picture in pencil where a text note would say the same in sentences.
ADR 0027. No row of `feature_list.json` changes; the app does not make a drawn note yet.

## Built
- The pencil (`src/pencil.rs`, plain geometry, no window): a stroke is a line of points pushed off true by a
  seed made from the note's id, the stroke's place in its picture and the pass. Each stroke is laid twice, a wide
  faint pass and a narrow dark one, and the view paints each pass as one stroked path. Marks: line, dashes, box,
  hatch, circle, squiggle, arrow, tick, cross, plus, tally. No point leaves its mark by more than
  `pencil::REACH`.
- A picture draws itself stroke after stroke in 1.5 s at most (`pencil::DRAWN_IN`); it is whole at once when
  motion is reduced or the page is still, and in the headless picture unless a time is asked (`snapshot_at`).
- `Note::Drawn { id, picture, words, open, options }` and `enum Picture`: `Track` (the steps as tallies of five,
  the tool at work, a dashed line to the place), `Sheets` (up to six files, an arrow, the folder; kept, waiting or
  thrown away), `Bars` (lines added and taken of up to six files, one scale, a ruler). Captions are fixed words
  in the handwriting face; a file's name is drawn only by the told row's name rule (`layout/told.rs`, reused).
- The words of a drawn note are placed only while it is open; a `?` in a pencil ring says there are words. A
  tap on the picture is the existing `Tap::Option` with `PICTURE_PLACE`.
- Three palette roles, `pencil`, `added`, `taken`, in both builtin themes; `Role::Caption`.
- `layout::print` prints a layout as lines of text; `Scene::pictures()` is the sample page of every picture; the
  snapshot example writes it as `pencil-<theme>.png` and takes `--at <seconds>`.
- App: one arm in the test module of `src/column.rs`, so its match on a note stays whole. The library needed
  none.

Deleted: nothing but the lines the changes replaced (39 removed, most of them in `layout/told.rs`, where the
name rule and the one-line fit became two functions the pictures share).

## Red first
`tests/pencil.rs` (9 tests) and one test in `tests/column_draw.rs` were run against a drawn note that placed an
empty card. 8 of the 9 failed, the first on "a picture is placed"; the tap test failed on "no Option of
work-garden is placed". The ninth, the seed's test, passed at once: `src/pencil.rs` was written before it. The
first mutation below is what shows that test can fail.

The first green run found two faults: a stroke asked for whole was cut by a rounding error and was not its full
line, and a tap on the `?` was reported twice. Both are fixed and pinned.

## Mutations (each one rule, against `tests/pencil.rs`, patched in and put back)
| Mutation | Killed by |
|---|---|
| the seed without the stroke's place | `the_same_id_and_place_give_the_same_strokes_and_another_id_or_place_gives_others` |
| a stroke not yet begun is whole | `a_picture_draws_itself_stroke_after_stroke_and_is_whole_after_its_time_or_with_no_time` |
| a seventh sheet is drawn | `sheets_draw_six_files_and_count_the_rest_and_each_state_has_its_own_arrow_folder_and_caption` |
| each bar to its own scale | `bars_are_drawn_to_one_scale_with_a_ruler_and_a_row_of_nothing_has_no_box` |
| the words drawn while closed | `a_drawn_note_s_words_are_drawn_only_when_it_is_open_and_then_to_their_last_word` |

The final round was run twice. The first found one clippy line (`is_multiple_of`) and, with the machine busy,
`a_theme_at_the_top_of_its_size_ranges_is_drawn` (`tests/draw.rs`) over its 110 s backstop at 226 s; in the second
the whole kit was green and that file took 48 s. That test draws the sample page, which holds no drawn note.

## Budget
The kit's cap went from 10805 to 12322 (1517 lines of Rust, 320 of them `tests/pencil.rs`); the app's from 16725
to 16727 (the one arm, as rustfmt lays it out).

## Open
- No real window was opened: the drawing over time is proved by the pure timing and by headless pictures at a
  time, not by watching it. The rocking tool is covered only by a unit test of `motion::rock` and the view's
  `moving`.
- The app's test module was not compiled here (the host has no Metal compiler for the app's tests); the app's
  library was checked.
- macOS only; Windows and Linux were not run.
- The app makes no drawn note yet: which of its notes become pictures is the next slice.
- A picture's parts have no words for a screen reader.
