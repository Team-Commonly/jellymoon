# The answer stays with its wish (2026-11-25 00:58 UTC)

Branch `the-answer-stays`, on `d2c3ac7` (`an-enter-is-not-lost`). No row of `feature_list.json` changes. Core,
the kit and the app.

## Why
A wish kept with an answer to the sharper wish's question is made of more than its typed words: core folds the
answer into the run's prompt. The page showed the typed words only, so the bubble of such a wish did not say what
was asked for.

## Built
- **Core keeps the answer** (ADR 0025, decision 9). A kept wish has `answered` beside `said`: the cleaned answer,
  the one that stands in `shaped`, in the same one write, and only for `ShapeGo::Answer`. Every other wish has
  none and the field is left out of the file, so a day written before reads as it did. A file's `answered` is
  used only while it is what cleaning leaves of itself and stands in the wish's `shaped` under the answer's
  label; otherwise the file is not the day's made wishes. A page's wish carries `answered`; one a page file gives
  that cleaning would change is dropped.
- **The page shows it in the wish's bubble.** The row of "wished here today" is the typed words, a line break and
  the answer (`scene::typed`); `column.rs` gives the kit's `Note::Said` the two apart, as `text` and `answer`.
  The kit lays the answer's lines in the same quote piece under the typed ones, in the same role and by the same
  cleaning, set in by 18 design units behind a small hook the pencil draws (the piece's sketch `answered`, one
  part `hook`). No word, label or new note. A bubble with no answer is the piece it was.
- **A tucked wish** keeps its bubble, so its answer shows there. **An opened wish's note** shows its label and
  what it says and never the typed words, so the answer is not added there.
- **`Make it again`** made the wish of its typed words alone, so a wish that went on with a sharper wish or an
  answer was made again as another wish. It now makes it as it was made (`jellymoon_core::wish_again`): the new
  wish has the old one's `shaped` and `answered`, and its run is given the same prompt. A typed wish made again
  is as before.
- Not changed: the reading call, the question's card and field, the prompt a run is given, keep, toss, undo, the
  entry.

## Red first
The tests were written with the code, so red was shown by switching the behaviour out with every call in place
(the answer not kept, a file's and a page's answer not checked, not on the row, not laid out):
- core `run/shape/tests.rs`: `a_wish_kept_with_an_answer_keeps_the_cleaned_answer_…`,
  `a_day_written_before_answers_were_kept_reads_with_none_…`, `a_wish_made_again_is_kept_with_the_prompt_…`:
  failed. `wish/tests.rs`, `a_page_s_answer_that_cleaning_would_change_is_dropped_…`: failed.
- kit `tests/column_draw.rs`, `an_answer_s_line_is_drawn_in_its_bubble_…`: failed.
- app `src/scene/tests.rs`, `a_made_wish_s_row_holds_the_answer_it_went_on_with_…`: failed.
- Not seen red in that run, because cargo stopped at the first failing file of each crate: kit
  `tests/typed.rs` (`an_answer_is_in_the_bubble_under_the_typed_words_behind_a_hook_…`) and app
  `tests/the_answer_stays.rs` (two tests, through the made-up window). Both were green after.

## Mutations
| Mutation | Run against | Result |
|---|---|---|
| the answer not kept | core `run::shape` tests | killed (2 failed) |
| the raw answer kept, not the cleaned one | core `run::shape` tests | killed (2 failed) |
| an answer kept for a `Shape` go | core `run::shape` tests | killed (2 failed) |
| the answer not drawn | kit `tests/column_draw.rs` | killed (1 failed) |
| the answer drawn for another wish's row | app `column::` unit tests | killed (1 failed) |

## Pictures
`.tools/pictures/the-answer-stays-with-{sticker,plain}.png` and `the-answer-stays-none-{sticker,plain}.png`
(untracked): a made-up day of one finished wish, with and without an answer. With one, a first-time person sees
their sentence and under it, behind a small corner mark, a second line in the same hand: what they added. With
none, the bubble is one line, as it was.

## Open
- The hook is a mark with no words. Whether a first-time person reads the second line as their answer was asked
  of no one.
- The question that was answered is drawn nowhere once the wish is made; a follow-up's run is not given the
  answer; an opened wish's note shows no answer.
- A build from before this refuses a day's made wishes when one has `answered`.
- Not run: a real window, a real harness, Windows, Linux.
