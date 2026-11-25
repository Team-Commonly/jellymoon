# The question keeps the keys

One small rule of the kit, found while walking the folded sharper wish in a real window (ADR 0025, decision
12). No row of `feature_list.json` changes.

## What was built

- **A press on the question a person is reading moves no key.** `Page::keeps_keys`
  (`crates/sticker-kit/src/draw/column.rs`) now also covers every part but an option of a note whose field
  asks for the keys (`Field::wants_keys`). Before, a click on the question's own card gave the entry the keys:
  the answer typed next went into the entry, and Enter there was a new wish in the pair's place.
- **Unchanged:** a press off the notes (the paper, the page's title, a character) gives the entry the keys; a
  tap on `Skip the question` or `Use my own words` does too; a click on the field gives it the keys; a field
  that does not ask for the keys (a result's follow-up field) is as it was.

## How it is held

- `app/tests/a_question_takes_the_keys.rs`,
  `a_click_on_the_questions_own_card_leaves_its_field_the_keys_and_a_click_off_the_notes_gives_the_entry_them`:
  in the page's own window on GPUI's test platform, a click on the card and then letters, which are the
  answer; then a click on the page's title and a letter, which is the entry's. Seen red with the rule taken out
  (the letters went to the entry).

## Open

- The rule has no test in the kit itself: the keys are in a view, and the kit's tests have no window.
- Windows and Linux were not run in a window.
