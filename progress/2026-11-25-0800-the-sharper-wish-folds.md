# The sharper wish folds

One slice on the page of a wish that asks (ADR 0025, decision 12). No row of `feature_list.json` changes.

## What was built

- **Under a question, the sharper wish is one line.** `shaping::folded` is one pure pass at the end of
  `Open::column`: with the question's note on the column, the note `shape-said` is replaced by the drawn note
  `shape-fold`, the kit's tally with no rows, titled `your wish, sharper`, with its chevron. Unfolded, the line
  stays, its chevron turned, and `shape-said` stands under it as it was. The scene that is kept is untouched,
  so what the answer and the two taps hand core is what it was, and `layout` prints both notes whole.
- **The fold is the window's.** A sixth cell of `drawing::Open`. A tap on the line is `Open::fold`, the arm of
  `Showing::chose` that turns the day's tally: no call to core, nothing written. `Showing::wishing` folds it
  again at every Enter in the entry, so each question starts folded and a refused answer finds it as it was.
- **The kit: a tally can stand in its place** (`Picture::Tally::in_place`). It is not lifted to the column's
  top, it is not the tally the view opens at or holds for, and a press on it moves no key. The last was found
  red: a press off a field gives the entry the keys, so after a tap on the line the answer's letters went to
  the entry, where Enter is a new wish in the pair's place.
- **What the fold never hides.** The sharper wish's note holds its three rows and nothing else
  (`shaping::pair`, `end` empty; a refusal is a note of its own), and the pass leaves a note with an `end`
  alone. The question's note is not touched. With no question nothing is folded: while the wish is read (one
  note, `shaping`), with its two choices (the sharper wish over "What you typed"), and after a refused tap
  (both notes gone, core's reason in a plain note).
- **Sentences.** Folded: "Only you know", the question as a quote, "Type your answer, then Enter", "Skip the
  question", "Use my own words", "your wish, sharper". Unfolded: those, and "Your wish, sharper", "The wish",
  "Done when", "Not touching" with the three quotes.
- The picture example draws the folded page with `--tucked` and the unfolded one with `--sharper`.

## Tests

Red first, at the parent with only the kit's new field added and read by nothing: the kit's
`a_tally_in_its_place_is_one_line_among_the_notes_and_a_tally_of_the_column_still_stands_first` (the line stood
at the column's top) and `a_tally_in_its_place_neither_opens_the_page_at_its_top_nor_holds_the_view_when_it_turns`
(the page opened at its top), and in `shape_first` the two tests named below (the column still held the sharper
wish's note). Each failed at the assertion of this rule.

`app/tests/the_sharper_wish_folds.rs` did not compile in that round, a closure's type in the test itself, so its
six tests were not seen red at the parent. They were seen red in the next round instead: four of them under the
mutation that folds nothing, which is the parent's behaviour, and the test of the keys by itself, before the
kit's rule for a press on the line.

New: that file (the folded column and its pieces, a tap and a second tap, the unfolded note piece for piece the
one `layout` prints, the answer and both ways round folded and unfolded with what is kept and what the run is
given, a refused answer, a new question, the three cases with no question, and the keys in the page's window on
GPUI's test platform), and the kit's two tests above.

Existing tests whose expectation changed, by name, both in `shape_first`:

- `a_sharper_wish_that_waits_and_the_choice_on_it_are_not_under_that_note_either`
- `an_enter_remembered_is_not_given_over_a_sharper_wish_that_asks_and_its_words_stay_in_the_entry`

Neither is loosened: where each named the pair on the window's column it now names the question's note and the
line, whole.

## Mutations

Each run against `the_sharper_wish_folds` alone.

| mutation | killed by |
|---|---|
| nothing is folded | `under_a_question_…`, `a_refused_answer_…`, `a_new_question_starts_folded_…` |
| the fold also hides the question's field and options | the same three |
| a tap on the line skips the question | the same three, and `the_answer_and_both_ways_round_…` |
| the fold is applied with no question | `with_no_question_…` |
| a row is left out when unfolded | `under_a_question_…`, `a_refused_answer_…`, `a_new_question_starts_folded_…` |
| folded, the run is handed another form | `the_answer_and_both_ways_round_…` |

The test of the keys failed in every one of those runs for its own reason (the kit's rule came after), so it is
not counted as a killer.

## After the first walk in a real window

The first head passed its gate and failed its walk. With a real harness's question on the page: a tap on the
line unfolded it and the view followed the column's new end, so the line was no longer under the pointer; the
second tap in the same place landed on the rows' card, which gave the entry the keys; the rest of the answer
went into the entry, and Enter there read a new wish in the pair's place. The answer half typed was gone.
The gate had named both halves as notes (the press beside the line's words, and the view that follows the end).

- **The view is held when a tally in its place is turned** (`draw/column.rs`, `turn`), as for the day's tally.
  The line stays where it was tapped.
- **A press on the line's card, or on the note it opened, moves no key** (`Page::keeps_keys`): any piece of
  the line's note, and any piece but an option of the note under an open line.
- Held by the kit's `a_tally_in_its_place_does_not_open_the_page_at_its_top_and_holds_the_view_only_when_it_is_turned`
  and by the app's test of the keys, which now clicks the rows' card and the line's card beside its words.

## Open

- Windows and Linux were not run.
- A press on the question's own card, off its field, still gives the entry the keys (ADR 0025's rule for
  every press off a field): letters typed after it go to the entry.
- The kit's rule that a press on a tally in its place moves no key is held by the app's test of the keys only;
  the kit has no test of its own for it.
- Unfolded, the line `your wish, sharper` stands over a card titled "Your wish, sharper": the same words twice.
- The line is a card with its strip of tape, as every card is: quiet, but not small. Its chevron is small, and
  whether a first-time person reads the line as something to tap was not asked of anyone.
- A person can now answer, or skip the question, without having read the sharper wish their run is given
  (ADR 0025, decision 12, says so).
- A look at the estimate: the app was to grow by about 60 lines and the kit by none. The app's code grew by 56
  and its tests by 339; the kit grew by 77, 59 of them tests.
