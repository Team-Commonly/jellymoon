# The result in plain words (2026-11-20)

## Built
When a run ended, a look brought the note "What it says it did" with a row per change (a file's name, the lines
before and after) and the work's note with every file's name. Following and trusting a wish took reading files.
Now the first view of both notes names no file and holds no changed line:

- "What it says it did": the run's own sentence, `Decided`, `Not sure about`, then the app's last sentences. In
  the place of every change's row one fixed sentence, "The lines it quotes match the copy.", said only when at
  least one change passed core's check. It starts the paragraph "Nothing was run to try this. ..." and adds none of
  its own; as a paragraph of its own it took the room of the run's own sentence at 960x640.
- the work's note: core's count ("1 new file, 1 changed, 1 removed.") and no name.
- a tap on `Show me` brings the change rows and the names, drawn as before, and the checked sentence is not said
  beside the rows.

The first view is what a look, a tap on the wish's character and `layout` show; only the answer to `Show me`
(`working.rs`, `notes_of`) asks for the rows. No state was added: a look leaves the notes on show as they are.
A result with no work (told alone) reads as before. Nothing in core, the kit or the binary changed, and nothing
about how a text of the run is checked, cleaned or quoted.

What the sentence claims is what core's check proves and no more: each line a passed change quotes is a whole
line the work took from its file or put in it. Core looks at a result's first two changes only, so the sentence
does not say "every".

Deleted: nothing but the change rows and the names from the first view, and their pins there.

## Red first
Against the source before the change: `tests/result_in_plain_words.rs` 2 of 3 failed (the first view held the
rows "Changed in a.txt" and "New file n.txt" and no checked sentence); `tests/f86.rs` 5 of 7 and `tests/f47.rs`
4 of 20 failed on their updated first-view pins. The third new test (the rows after `Show me`) passed before the
change, as it should: those rows are drawn as they were.

## Mutations (against `tests/result_in_plain_words.rs` only, one hold)
| | mutation | killed by |
|---|---|---|
| M1 | a look draws the change rows | the first view and the checked sentence tests: a row "Changed in a.txt" |
| M2 | the checked sentence is said with no change that passed | the checked sentence test, and the rows after `Show me` |
| M3 | the checked sentence is said when a change was only dropped | the checked sentence test |
| M4 | `Show me` gives the first view again | the `Show me` test: no change row |
| M5 | the work's note names the files on its first view | the first view test: "a.txt, b.txt, n.txt" |

## Open
No real window and no real harness were used; Windows and Linux not run here. The whole workspace's tests and the
smoke chain were not run, only the app's. `Show me` on a result with more rows than the room still counts the
rest ("+N more"), as before. A tap on the wish's character shows the first view, not the rows.
