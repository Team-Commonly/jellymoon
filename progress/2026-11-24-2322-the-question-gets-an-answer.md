# The question gets an answer (2026-11-24 23:22 UTC)

Branch `the-question-gets-an-answer`, on `fc2110a` (`a-remade-wish-steps-aside`). No row of `feature_list.json`
changes. Core and the app; the kit is as it was.

## Why
Walked in a window on the released build: a typed wish left something open, and the sharper wish said so in
a row, "Only you know", with a question only the person can answer. Under it were two choices and nowhere to
answer: the person could go on with the hole or type the whole wish again. And the card was the most text a
first-time person meets: five rows, their own words again, two choices.

## Built
- **A field under the choices.** When the sharper wish has a question, the note `shape` has a field under
  `Go with this` and `Go with what I typed`: the offer "Or go with this and your answer to the question" and
  the hint "Type your answer, then Enter". Enter with words keeps the wish with the sharper wish and the
  answer, and its run is asked for as after any Enter. An empty field gives nothing; with no question there
  is no field.
- **Core builds the prompt** (`run/shape.rs`, `prompt`). With an answer, two parts stand before the person's
  own words: ` | Asked of the person: <the question> | The person's answer: “<the answer>”`. The question is
  bare, as every text of the model's; the answer is in the person's quote marks.
- **The typed API** is `ShapeGo`: `Typed`, `Shape`, `Answer(words)`, in place of a bool.
- **The answer is cleaned as a wish's words are** (the same door) and is 200 characters at most. It is
  refused, nothing written and the shape still waiting, with a fixed sentence: the shape asks nothing; nothing
  is left once cleaned; it is longer; no prompt can be built with it.
- **`SHAPED_MAX` is 1300** (it was 1000). The longest of everything is 1280 characters.
- **The card says less.** The row "Why" is not drawn. Core still reads and checks it. The rows are "The wish",
  "Done when", "Not touching", "Only you know".
- **A refused answer** leaves the pair on the page under "That wish was not kept." with core's reason, and
  the field keeps its words.

## Found
- The door looks at no text over 4096 bytes. The longest of everything fits in characters of up to three
  bytes (3228 bytes); written wholly in characters of four bytes it is 4198 bytes and no prompt is built. That
  answer is refused with the fixed sentence and a shorter one builds. Lengths in characters alone could not
  promise a prompt for every answer; a test holds both cases.
- The person's own words may hold `|`, `“` and `”` today, and cleaning leaves them. The answer is treated the
  same way. Only a model's text is refused for those marks.
- A wish made again is made of the typed words alone, and a follow-up's run is given the follow-up's words:
  neither is given the answer. Not changed.

## Red first
- The app's tests against the typed API alone: five of ten in `tests/shape_first.rs` failed (the four rows,
  the field, Enter with an answer, a refused answer).
- Core's first red run did not compile, by a mistake in the test itself. Its tests were then run against the
  typed API alone in the next round: five of seven failed (the prompt with an answer, the cleaning, the two
  refusals, the longest lengths, the prompt past the door's bytes).

## Mutations
| Mutation | Killed by |
|---|---|
| the answer left out of the prompt | `an_answer_stands_in_the_prompt_…` and three more (core) |
| the question inside the person's quote marks | `an_answer_stands_in_the_prompt_…` and two more (core) |
| the answer not cleaned | `an_answer_is_cleaned_as_a_wish_s_words_are_…` (core) |
| a field with no question | `a_sharper_wish_is_shown_as_quotes_…` (app) |
| "Why" still drawn | `a_sharper_wish_is_shown_as_quotes_…` and four more (app) |
| an answer kept when the shape waits for other words | `an_answer_is_refused_when_the_shape_asks_nothing_or_waits_for_other_words` (core) |

## Lines
Core +203 (233 added, 30 removed), cap 19757 to 19960. The app +124 (144 added, 20 removed), cap 19806 to
19930. Deleted: the row "Why" and its label; the bool of `wish_of_shape`.

## Open
- `wish --shaped` has no answer.
- A refused answer whose reason is that the shape waits no more leaves the pair standing until the next tap.
- An answer goes with the question of the shape that waits for those words, which may not be the one on show
  when the command made another for the same words meanwhile.
- The answer is nowhere on the page after Enter: the wish's bubble shows the typed words.
- Not run: a real window, a real harness, Windows, Linux.

## After the walk

A walk with a real harness (not by this unit) found that the answer reached the run and the run obeyed it, and
that the page was the fault: the field stood a whole note and a quote of the typed words away from its
question, under a sentence for an offer. A second commit, in the app only:

- With a question, "Your wish, sharper" has three rows ("The wish", "Done when", "Not touching"), and the note
  `shape` is the question's: titled "Only you know", the question its quote, the two choices, then the field
  with the offer "Answer" and the hint "Type your answer, then Enter". The typed words are on neither note.
- With no question both notes are as before the first commit, but for the reason, which stays undrawn: `shape`
  is titled "What you typed" and quotes the typed words, with no field.
- The typed words are held by the page on show (`OnShow::typed`), set when the pair is shown and cleared when
  it goes, and both taps and the field take them from there. Nothing reads them back from a note. Core's check
  that the shape waits for those words is as it was.
- Red first: five of the eleven tests of `shape_first`. One mutation, the typed words taken from the note's
  quote, was seen killed.
- Not proved: a real window, a real harness on this head, Windows, Linux. The picture is the headless example's.

## After the verifier

A verifier failed the second commit on one finding: the window carried only the typed words to core, so a shape
made since for the same words (the command, the same day) was the one an answer was kept with, and its
question was one the person never saw. A third commit:

- Core: `wish_of_shape` takes the shape the page has on show, with its words and project (`ShapeSeen`), and
  refuses, with nothing written, when the shape that waits is not that one, in all three ways. The older hole
  of a tap is closed by the same line, and so is a wish kept for another project than the page's.
- The app holds that in `OnShow::typed` in place of the words alone, and hands it to core at a tap and at Enter.
- Red first: `a_shape_made_since_for_the_same_words_is_not_the_one_on_show_and_keeps_nothing` (core), against
  the new signature with no comparison. The app's tests of the pair are green only while the window hands core
  the very shape core wrote.
- "Built" and "Lines" above are the first commit's: the head has three rows, the offer "Answer", and the caps
  in `entropy.toml`.
- Still open: an answer refused because the shape waits no more brings the older pair back once, and the next
  tap takes it away; the newer shape is on no page.

