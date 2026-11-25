# An Enter is not lost (2026-11-25 00:13 UTC)

Branch `an-enter-is-not-lost`, on `9c75d15` (`the-question-gets-an-answer`). No row of `feature_list.json`
changes. The kit and the app; core is as it was.

## Why
Walked twice in a window on the released build: a person types a wish and presses Enter, and for about five
seconds the wish is read. In that time they type the next wish and press Enter. Nothing happened: the entry is
owed an answer for the first words, and its Enter gave nothing until that answer had come. The second wish sat
in the entry with nothing on the page to say why.

## Built
- **One Enter is remembered** (the kit, `EntryState`). Enter while an answer is owed, with words typed since and
  no composition, sets one flag, however often it is pressed. Nothing is sent and the words stay editable.
- **It is given at a kept answer** (`EntryState::answer_and_enter`, called by `Page::answer_entry`): what
  `enter` gives at that moment, by all its rules, to the same `on_entry` handler. Once. The words not kept, it
  is forgotten: they are back in front of the later ones.
- **Whoever answers can refuse it**: `Page::forget_enter()` before the answer. `answer_entry_closed` and a
  note's field (`EntryState::answer`, the plain answer) give none.
- **The app refuses it over a sharper wish that waits** (`shaping::waits`, `Showing::takes_enter`, two lines
  of `window.rs`): the words stay in the entry, and Enter is the person's to press.
- `Page::answer_entry` takes `&mut Window` (it was `&Window`): a handler is called with one. The entry's three
  answer functions moved from `draw/entry.rs` to `draw/field.rs`, beside the fields', for the file's size.
- The made-up window has an entry of its own, the kit's `EntryState`, and the act `press:wish@<project>:<words>`.
- No sentence on the page is new and nothing drawn changes: no picture.

## Red first
With the remembering switched off and every new call in place:
- `crates/sticker-kit/tests/entry_waits.rs`, `one_enter_while_an_answer_is_owed_is_remembered_…`: failed.
- `crates/sticker-kit/tests/entry_draw.rs`, `an_entry_that_is_owed_an_answer_takes_every_key_…`: failed.
- `app/tests/f43/window.rs`, `an_enter_pressed_before_core_has_answered_is_not_lost_…`: failed.
- `app/tests/shape_first.rs`, `a_wish_entered_while_the_one_before_is_read_…`: failed.
The tests of what is refused (`a_remembered_enter_is_not_given_…` in both kit files, `an_enter_remembered_is_
not_given_over_a_sharper_wish_…` in the app) pass in that state, as they must; each is red under its mutation.

## Mutations
| | The rule changed | Killed by |
|---|---|---|
| M1 | the Enter is not remembered | `entry_waits.rs` (1 test), `shape_first.rs` (1 test) |
| M2 | given before the answer: Enter while owed gives the words at once | `entry_waits.rs` (3 tests) |
| M3 | given twice: the flag stays after the Enter was given | `entry_waits.rs` (1 test) |
| M4 | given over a waiting pair: `takes_enter` always yes | `shape_first.rs` (1 test) |
| M5 | given when the first words were refused | `entry_waits.rs` (1 test) |

## Tests changed
- `entry_waits.rs`: a second Enter while owed was compared with the whole state before it; it now changes
  the flag, so the comparison is of the text, the caret, the option and the wait.
- `entry_draw.rs`, `an_entry_that_is_owed_…`: the kept answer after an Enter pressed meanwhile now gives that
  Enter, so the test holds that, and types on without its own Enter.
- `field_draw.rs`: three lines more, that a field gives no Enter at its answer.

## Open
- The window's two lines that forget the Enter over a sharper wish are run by no test: the made-up window makes
  the same two calls. The wiring of a given Enter is run on GPUI's test platform (`f43/window.rs`).
- Not walked in a real window, with a real harness. Windows and Linux were not run.
- A person who pressed Enter twice and whose first words were refused, or answered by a sharper wish, still
  has to press Enter again; the page says nothing of the Enter that was dropped.

## After the review (2026-11-25)
A review by reading failed the first commit on one finding: the Enter was remembered as a flag only, and the
answer gave the words as they stood then. A person who pressed Enter on the next wish and typed on (the start
of a third, or a correction) had a half-typed wish started, and a run with it. A second commit answers it:
- The kit forgets the remembered Enter when the text or the chosen option changes after it (`put`, `cut`,
  `choose` in `entry.rs`; a caret moved, or the same option chosen, changes neither). So the Enter given at
  the answer is always of the words and the option it was pressed on, and words typed after it stay in the
  entry for the person's own Enter. Seen red first: `a_remembered_enter_is_forgotten_when_the_words_or_the_option_change_after_it`
  (`tests/entry_waits.rs`), five edits, each given a wish before the fix.
- The window forgets it when the entry answered is the one a new project's name was typed in (`listing`):
  the Enter would have named a second project from the next words.
- Tests changed: `entry_waits.rs` (the given Enter is of the words at the Enter), `entry_draw.rs` (the Enter
  pressed before a composition and a chip is forgotten; a second pair of Enters holds the given one),
  `f43/window.rs` (the second Enter is pressed after its last word).
- Walked in a real window with a real harness at the first commit: two wishes entered back to back, the second
  while the first was read; both were kept in their order and both runs landed.
