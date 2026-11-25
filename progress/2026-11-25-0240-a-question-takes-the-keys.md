# A question takes the keys (2026-11-25 02:40 UTC)

Branch `a-question-takes-the-keys`, on `93c5262` (main). No row of `feature_list.json` changes. The kit and the
app; core is as it was.

## Why
A wish may draw a question: the note `Only you know`, two choices and a field, `Type your answer, then Enter`.
The keys stayed in the entry. A person who read the question and typed its answer typed it into the entry, and
Enter made those words a new wish in the pair's place: the first wish was gone. Answering took a click on the
small field first, and nothing said so.

## Built
- The kit: a field of a scene may ask for the keys (`Field::wants_keys`, false unless said). At the first
  drawing that places it the field takes them (`Page::keys_asked`, `draw/field.rs`, called at the head of
  `entry_layer`), once. It does not take them while the entry holds words or is owed an answer, or while the
  field that has the keys holds words; it has had its turn then too. A field kept from the scene before, or kept
  apart by its key, never asks again; a field whose note was gone and comes back is a new field and asks again.
- The app: the field of a sharper wish's question says it asks (`shaping::pair`). Nothing else in the app
  changed: Enter in that field is the answer by the path a click-then-type took.
- The seam: the kit owns the keys and knows no wish or question; the app sets one word on one field and
  reaches no focus handle.
- Not done on purpose: the field under a made wish's question (`answering::with_question`) and the field of a
  follow-up do not ask. The first answers a question a run asks: it stands when a look or a tap draws it, not
  as the answer to the person's own Enter, a page can hold several, and an Enter in the entry beside it replaces
  nothing. So the case is not the same.
- Escape is as it was: in a note's field it gives the keys to the entry, in the entry they stay. The asking
  field does not take them back after it. When the pair goes, the keys are the entry's by the kit's old rule
  (`entry_layer`); no line was added for that, and a test now holds it.
- The windowless snapshot (`draw/headless.rs`) gave the entry the keys always; it does so now only when it is
  given words to type, so a picture shows the keys where the page puts them.

## Red first
With the field never given the keys (the one line of `keys_asked` switched out), in one run:
- `tests/field_keys.rs` (kit): three of four failed (first stands; once only; note gone and a new one asks).
  The fourth, words typed keep the keys, passed: it holds what must not change, and mutation 2 is what it
  fails on.
- `app/tests/a_question_takes_the_keys.rs`: both failed then, and failed in the first green run as well, on
  the test's own made-up day (no page kept, so no project on offer and no pair). The day was fixed and the
  file ran green in the next hold; so its red was not the rule's red. Mutation 1 below is the rule's.

## Mutations (five, one hold, against `tests/field_keys.rs`)
| # | Mutation | Killed by |
|---|---|---|
| 1 | The field never takes the keys | `a_field_that_asks_has_the_keys_when_it_first_stands_…`, `it_takes_the_keys_once_…`, `when_the_asking_note_is_gone_…` |
| 2 | It takes them though words are typed | `words_typed_in_the_entry_or_where_the_keys_are_keep_the_keys_there` |
| 3 | It takes them again at every drawing | `it_takes_the_keys_once_…`, `words_typed_…` |
| 4 | A field that does not ask takes them | `a_field_that_asks_…_and_one_that_does_not_ask_never_takes_them`, `it_takes_the_keys_once_…`, `words_typed_…` |
| 5 | The keys do not return to the entry when the note is gone | `when_the_asking_note_is_gone_the_next_letters_are_in_the_entry_…` |

## Remembered Enter
Untouched: `EntryState::again`, `Showing::takes_enter`, and their tests in `tests/shape_first.rs` and
`tests/f43/window.rs` are green. An entry that is owed an answer counts as holding words, so a field that comes
while a wish is read never takes the keys from a remembered Enter's words.

## Tests changed, and why
- `tests/field_draw.rs`, `tests/pencil.rs`, `tests/typed.rs` (kit), `tests/shape_first.rs`,
  `tests/go_on_from_an_answer.rs`, `src/showing/tests.rs` (app): a `Field` written out in full gained the new
  word. `shape_first.rs` expects `wants_keys: true` on the question's field; no assertion was loosened.

## Open
- The app's test plays the page's own window on GPUI's test platform, on the Mac only, in place of a new act
  of the made-up window (`support/tapper.rs`): that window has no view, so an act that types where the keys
  are would hold a second copy of the kit's rule and prove nothing of the kit.
- A refused answer puts the pair back with the same field: it has had its turn and does not take the keys
  again; the words come back in it and a click is needed if the keys were moved.
- Not run: a real window, the real harness, Windows, Linux.
