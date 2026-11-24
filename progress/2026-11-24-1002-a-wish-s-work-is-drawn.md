# A wish's work is drawn (2026-11-24)

One slice in the app, with a field on the kit's drawn note: a run at work is a track and work that waits is its
sheets, with the sentences behind a tap on the picture. ADR 0029. No row of `feature_list.json` changes. Core
and the binary are as they were.

## Built
- **A run at work** (`app/src/live.rs`): the newest working made wish has one drawn note, `steps-<wish>`, a
  `Track` of core's count of its steps at the stage of the newest (thinking before the first), in the place of
  the plain note that says the wish is being worked on. That note's title and body are its first words, then
  the stage's sentence and the count. No file's name. The pills are as they were.
- **Work that waits** (`app/src/drawing.rs`, `working.rs`): `work-<wish>` of work that was read is a drawn note,
  `Sheets` of core's three counts in the state `Waiting`, core's choices in core's order, core's title and lines
  as its words. A tap on a choice reaches core as it did. Work that could not be read, the keep asked once more,
  a question, `busy-`, `wish-`, `file-` and `said-` stay text notes.
- **Sentences on a tap** (`drawing.rs`, `showing.rs`): a tap on a picture or its `?` opens the note, the next
  closes it; the open set is the window's own (`drawing::Open` in `Showing`). `drawing::pass` sets each drawn
  note open or closed and takes `told-<wish>` off the page while that wish's drawn `work-` is closed;
  `drawing::column` applies it and then the column's order, and is the only caller of `column::column`.
  `Show me` opens the note it was tapped on. A look closes nothing; a note that left comes back closed.
- **The kit**: `field: Option<Field>` on `Note::Drawn`, placed under the options and typed in by the text note's
  own code (`layout/drawn.rs` calls `typed::field`; `draw/field.rs` fits a drawn note's field as a text note's).
- `layout`, the made-up window's `placed` and the walk print every drawn note open, as they have no tap.
- The walk has `open:<note title or id>`; the picture example has `--made`, `--steps` and `--open`.

Deleted: the told note "What it is doing" of a wish at work, which the track replaces, and the lines the changes
replaced. `After` and `Ended` moved from `showing.rs` to `showing/after.rs` to keep the file under its limit.

## Left out, by a ruling
A done keep or toss as a picture (`In`, `Tossed`). Core's answer to a tap is `WorkDone::Said` for a done keep, a
done toss and a refusal alike, and the app must not tell them apart by their sentences. `said-<wish>` is the text
note it was for every answer; one test pins that.

## Red first
The tests were written with the code, not before it, so the red that was seen is this: the first whole run had
one new test red (`work_that_was_not_read_and_every_answer_to_a_keep_or_a_toss_stay_text_notes`: work that could
not be read had become a drawn note; it is a text note now) and six old ones red on the new behaviour (the note
of a wish at work, and helpers that listed a page's notes by kind) in `f42`, `f47`, `live_work` and `work_shows`.
Every other new test is shown able to fail by the mutations below.

## Mutations (each one rule, patched in and put back, against the app's library tests)
| Mutation | Killed by |
|---|---|
| the note that says the wish is worked on left beside the track | `a_working_wish_is_one_track_with_core_s_count_in_the_place_of_the_note_that_says_it_works` |
| the choices in another order | `work_that_waits_is_drawn_with_core_s_counts_and_core_s_choices_in_their_order` |
| the told note placed while the work's note is closed | `the_sentences_and_what_the_run_says_come_with_a_tap_on_the_picture_and_go_with_the_next`, `a_tap_at_the_picture_s_place_opens_and_closes_the_note_on_show` |
| the open set cleared by a look | the same two, and `a_note_that_left_the_page_comes_back_closed_and_show_me_opens_the_work_it_shows` |
| a count taken from how many names there are | `work_that_waits_is_drawn_with_core_s_counts_and_core_s_choices_in_their_order` |

## Budget
The app's cap went from 17078 to 17616 (538 lines of Rust, 228 of them `src/drawing/tests.rs`); the kit's from
12322 to 12390 (68, of them 56 its two tests).

## Open
- A done keep or toss is still words; `In` and `Tossed` wait for a typed answer from core.
- No step's file is drawn, and `Bars` has no data.
- No real window was opened: the pictures of the snapshot example are the only look. The tap on a picture is
  proved at `Showing::chose` and the kit's own tests, not through a real window's wiring.
- `layout` prints the page with every picture open, which is not what a window first shows.
- A page with a clash has no track, as it had no note that the wish is being worked on.
- macOS only; Windows and Linux were not run.

## Joined with F92 (kept by itself)
- The note `kept-<wish>` of a change kept by itself is drawn too: `Sheets` in the state `In`, core's counts and
  `Undo it`; closed, the told note is folded. Only while core says the folder has the change and it was read;
  otherwise it is the text note it was (`landing::kept_note`, test in `src/drawing/tests.rs`, seen red first).
- Core: `Stands` is re-exported (two lines, no rule changed) so the app can ask where a kept change stands.
- F92's page test reads a drawn note as F47's does.
- Caps: the app 17868 (the two sums, 4 for the test's reader, 57 for the drawn kept note), core 19412.
