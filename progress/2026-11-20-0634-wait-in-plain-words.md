# The wait in plain words (2026-11-20)

## Built
Live work's pill ("reading menu.txt") and note (a row per file) asked a person to follow file by file, and a
person who asked for something does not. A working wish now says the stage its run is at, in fixed words, and never a file's name:

- the pill: "thinking" before the first step; "looking around" after `Read`, `Glob` or `Grep`; "making the change"
  after `Edit` or `Write`;
- the note "What it is doing": the stage as a sentence ("Looking around your project." or "Making the change."),
  then "1 step so far." or "N steps so far.". No rows.

File names are left off the page entirely, opened wish included: showing them only on a tapped working wish would
need the page's drawing to know a stop choice is on show (`stopping.rs`, `taps.rs`), more than a few lines. Core
keeps the names as before; nothing in core, the kit or the binary changed.

Deleted: the per-tool words and nameless sentences (`doing`), the pill's blank rule, the note's rows.

## Red first
`app/src/live.rs`'s unit test: `left: "reading hours.txt"`, `right: "looking around"`. `app/tests/live_work.rs`:
the pill mid-way said "reading b.txt", wanted "looking around".

## Mutations (against `tests/live_work.rs` only, one hold)
| | mutation | killed by |
|---|---|---|
| M1 | `Glob`, `Grep` say "making the change" | the glob/grep case: `["making the change"]` vs `["looking around"]` |
| M2 | the pill takes the oldest kept step | the read-then-edit case: `["looking around"]` vs `["making the change"]` |
| M3 | the note takes the oldest kept step | the same case: "Looking around your project. 7 steps so far." |
| M4 | the note drops its stage sentence | mid-way: "What it is doing 1 step so far." |

## Open
No real window and no real harness were used; Windows and Linux not run here.
