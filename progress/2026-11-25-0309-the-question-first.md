# The question first (2026-11-25 03:09 UTC)

Branch `the-question-first`, on `7c21d49` (the keys slice, itself on `93c5262`). No row of `feature_list.json`
changes. The kit and the app; core is as it was.

## Why
A wish that drew a question showed "Your wish, sharper" (three labelled rows, each sentence in a rounded outlined
box), then "Only you know" with the question in the same box, two wide choices, a chip "Answer" and the field.
Five outlined boxes, four of them alike and one of those the field; the answer, the one thing asked, at the
bottom.

## Built
- **The kit's seam: `Field::first`.** One word of a text note's field. With it the field is placed directly
  under the quoted words and the options after it, as small pills on one row (wrapped when they do not fit),
  each in the role of an offer's word and drawn as an offer is. They are `option` pieces of their index still:
  the tap, its words and the settle guard are the old code. A note without the word, and every drawn note,
  places and draws as before (`layout/text_note.rs` `under`, `layout/typed.rs` `quiet`, `draw/shapes.rs`).
- **A quote never looks like a field.** Every `quote` piece of a text note and of a told note is drawn on a
  soft wash of the pencil's colour over the bubble, corners of 4, no line round it, and one line of the pencil
  down its left before the words (the piece's sketch `quoted`, part `line`; `layout/typed.rs` `lined`). The
  wash is `Palette::quote_ground`: used only where the `quoted` ink reads on it at `QUOTED_FLOOR`, else the
  bubble the ink was settled on, so a theme file cannot make a quote unreadable by it. Built-in themes: the
  wash. What a person typed (`Note::Said`) keeps its bubble and ring.
- **The app.** With a question `shaping::pair` gives the question's note first, then the sharper wish; the
  field is first and has no offer (the const `ANSWER` is deleted); the two options read "Skip the question"
  and "Use my own words" (`SKIP`, `OWN`), in the old order, so option 0 is still `ShapeGo::Shape` and option
  1 `ShapeGo::Typed`. With no question nothing but the quote's look changes.
- **The sharper wish.** Three rows, every word, as before; the labels keep their role and size. They were
  left alone on purpose: the told note is also what a result is told in, and a smaller label there would have
  changed the result's note.

## Pages a quote is on (each looked at, both looks)
The question's note; the sharper wish with and without a question; "What you typed"; "Reading your wish…";
the note of a made wish whose run asks; "What it says it did" of a result. The bubble of typed words is a
quote piece too and is unchanged. A day's page with a change of mind draws options, not quotes: unchanged.

## Red first
On the tree with the field's word added and no behaviour (the notes as the parent builds them):
- kit `src/draw/shapes.rs`: `a_note_s_quote_has_no_line_round_it_…` failed on the quote's outline.
- kit `tests/typed.rs`: `a_field_that_is_first_stands_between_the_quote_and_the_options_…` failed on the
  order; `a_note_s_quote_has_one_line_of_the_pencil_down_its_left_…` failed on no sketch.
- app `tests/shape_first.rs`: 8 of 13 failed (the order of the pair, the words, the field before the options,
  the taps by their new words). The 5 that passed hold the no-question case and the reading, which must not
  change.
- `tests/told.rs`, `tests/pencil.rs`, `tests/field_draw.rs` were green before and after.

## Mutations (six)
| # | Mutation | Against | Killed by |
|---|---|---|---|
| 1 | A quote drawn with a field's outline again | kit `--lib` | `a_note_s_quote_has_no_line_round_it_…` |
| 2 | The question's note under the sharper wish | app `shape_first` | 6 tests, `a_question_is_the_quote_of_the_note_it_is_answered_in_…` first |
| 3 | The field after the options | kit `tests/typed.rs` | `a_field_that_is_first_stands_between_the_quote_and_the_options_…` |
| 4 | The two words swapped | app `shape_first` | 7 tests, `skip_the_question_goes_on_with_the_shape_and_use_my_own_words_…` among them |
| 5 | The question drawn as the note's own text | app `shape_first` | 6 tests |
| 6 | The third row of the sharper wish left out | app `shape_first` | 6 tests |

The app's four took a second hold: the first switch was a variable of the environment, which the made-up
window's process is not given, so all four passed unmutated. The switch was made a file and they were run
again. The switches are removed.

## Tests changed, and why
- `app/tests/shape_first.rs`: the expected pair is the question's note first, with the new words, a first
  field and no offer; the placed order is field, option, option; the tap test is renamed and taps by the
  words seen. The test of the question's note also holds each row's text now. No assertion was loosened.
- `app/tests/the_answer_stays.rs`: a comment names the new words. `go_on_from_an_answer.rs`,
  kit `tests/field_draw.rs`, `tests/pencil.rs`: a `Field` written out gained the word; `pencil.rs` now holds
  that a drawn note's field is never first.

## Open
- The question page is lighter and still wordy: the sharper wish's three sentences outweigh the question
  above them, and "Use my own words" names words the page does not show.
- The wash is a full-width grey bar; on the plain look it can read as a switched-off field. No person was
  asked.
- A long quiet option is cut to one line of the note with an ellipsis; the app's two are short.
- A theme whose `quoted` ink fails on the wash gets the plain bubble with the pencil's line and no outline:
  where the bubble is the paper's colour the ground is then not seen.
- Not run: a real window, the real harness, Windows, Linux.
