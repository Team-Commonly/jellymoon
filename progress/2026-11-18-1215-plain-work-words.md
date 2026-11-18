# The work's note speaks to a person (plain work words)

On branch `plain-work-words`, one commit on `origin/walk-fixes`. Not gated; no row of `feature_list.json` changes.

## What was built

The window's notes of work that waits, kept and tossed, in words for a person who has never seen Jellymoon. The
words are core's, as every word of these notes is (`show.md`): `crates/jellymoon-core/src/run/waits/plain.rs`.
The command line's lines (`run waiting`, `run show`, `run keep`, `run toss`) are unchanged.

| Note | Old | New |
|---|---|---|
| work that waits, title | Work waits for this wish. | Your folder is not changed yet. |
| its first line | (none) | It is in a copy: Keep it puts it in; Toss it throws it away. (no keep on a page: "It is in a copy: Toss it throws it away.") One line of a note, whatever the project, so a result's note over it keeps its room (F86). |
| its count | the work of run r… waits: files made 1, changed 1, removed 1. | 1 new file, 1 changed, 1 removed. (cut list: "…, counted in the first 200 files.") |
| not read, title | Work waits for this wish, and it was not read. | This change could not be read. |
| its first line | (none) | Your folder alpha is not changed. To keep or toss the change here, put right what is said below, then open the wish again. (core's reason follows) |
| kept, title | That work was kept. | The change was kept. |
| moved forward, joined, finished | run r… was kept: … | The change is in your folder alpha now. |
| held already | run r… was kept: the branch held the run's work already, and nothing was landed. | Your folder alpha had this change already, so nothing in it was changed. |
| elsewhere, loose | run r… was kept, on another branch … | The change is in your folder alpha now. The folder was switched while it was kept, and the change went where it stood: see below. (core's sentence follows) |
| kept before | the work of run r… was kept before: nothing was changed. | This change was kept before, so nothing was changed now. |
| a `.gitattributes` stayed | (core's sentence) | the plain sentence, then core's sentence |
| tossed, title | That work was tossed. | The change was thrown away. |
| tossed | the work of run r… was tossed: its branch was deleted, … | The change was thrown away, and your folder alpha is as it was. |
| tossed before | run r… was tossed before, … | This change was thrown away before, so nothing was changed now. |
| refused, titles | That work was not kept. / not tossed. / No file of that work was shown. | The change was not kept. / The change was not thrown away. / No file of the change was shown. |
| refused, first line | (none) | Open the wish again to see where the change stands. Here is why it was not kept: (… it was not thrown away: / … no file was shown:), then core's reason |

The second yes (`sure-<wish>`) names no run and is unchanged. `WorkDone::Said` now carries the note's texts.
`Kept` carries how the keep landed and `Waits` the project, both `serde(skip)`: never printed.

## What was red first

Against the old core and app: 15 tests of `app/tests/f47.rs` (with `f47/`), 5 of `f54.rs` and 1 of `f86.rs`.
The count's unit test (`plain.rs`) did not exist before.

## Mutations

One hold, each against the one test file that should kill it.

| # | Mutation | Killed by |
|---|---|---|
| M1 | a keep that found the change in the folder already says it is in the folder now | `f47/named.rs`, the yes to other names (the held keep) |
| M2 | one file counted as several ("1 files changed") | `plain.rs`, the count's test |
| M3 | the first sentence moved after the count | `f47.rs`, layout's note of work that waits |

`read` in `waits.rs` now builds `Waits` once for work read and not read (the literal is not written twice).

## Found in the final round

- The brief's grep misses the refusals' old words: `f47/gone.rs` ("That work was not tossed.") and `f47/named.rs`
  ("No file of that work was shown.") pinned them, and `crates/jellymoon-core/tests/shapes.rs` counts the note's
  lines. All three changed with the words.
- The plain words took the room a result's note had at 1440x900 (F86: the typical result no longer read its whole
  change). Probed: one line more than before still fits, two do not. So the title says the folder is not changed
  yet, and the first line, one line with no project's name in it, says the rest.
- `tests/f86.rs` compared the work's note with and without a result to a hundredth, and two layouts of the same
  note rounded 17.33 and 17.34; it compares to a tenth.
- `cargo test --test walk` runs the walk example as last built: build it first (`--example walk`).

## What is open

Windows and Linux were not run. The command line's test files (`crates/jellymoon/tests/f45.rs`, `f47.rs`, `f53.rs`)
were run unchanged and are green. The work's note is one line taller than before: a result with a longer did than
the typical fixture's gives up its change row a line sooner. The kit's `tests/told.rs` was not touched: its work
note is a made-up one.
