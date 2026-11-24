# True words on the page (2026-11-24 22:40 UTC)

Branch `true-words-on-the-page`, on `94a7725`. No row of `feature_list.json` changes. The app, one sentence of
core's, and three doc lines owed from a gate. The kit is as it was, and so is everything core does.

## Why
Three sentences on the page were not true when a person read them. The first is met on a person's very first wish.

## Built
- **"Nothing wished for yet today." while the first wish is read.** The note is made by `scene::ruled` for a
  page with no wish, and nothing took it away but a page drawn again with a wish on it: Enter put "Reading your
  wish…" over it, and every look drew it again from the page. Now it is not drawn while a note of the reading
  is on the scene: the reading's note, or a sharper wish with its choice (`shaping::taken`, asked in
  `drawing::passed`, the one place a scene is made ready to be drawn). A tap on a sharper wish's choice, which
  takes both notes away before the wish is kept, takes the note off the scene too. After that the page is
  drawn from what is kept, as before: a kept wish draws none, and a reading that kept no wish has it back.
  The empty page before any wish is unchanged, and so is the scene the app keeps while a wish is read.
- **"Your wish was made again, and it is being worked on." after that work ended.** Core's answer under the old
  wish stays until the next wish is made (ADR 0030, decision 11). Where the answer is made (`Today::asked`,
  `app/src/again.rs`) the new wish's id is now kept with it and with core's words for the same answer
  afterwards (`WorkDone::remade_since`: the same, ending "Your wish was made again."). They are the window's
  own state (`again::Again`, held by `drawing::Open`); where a scene is made ready to be drawn, an answer whose
  new wish is on the scene and not at work is drawn in the words for afterwards. The sentence is made shorter
  and not taken away, as a second wish with the same words is on the page and the answer says why. The note is
  found by being that answer, whole, and the wish by its id: no sentence is matched.
- **Three doc lines**: `unkept` in `run/record.rs` (a note that is read asks the keep once more); the picture
  of work that did not fit in `docs/development/app.md` (the kit's `unfit`); what `run waiting` writes where a
  record says a clash, in `docs/development/show.md`.

## Red first
- `tests/a_change_that_no_longer_fits.rs`, the test of `Make it again`: on the app as it was, the answer once
  the new run had ended still ended "…and it is being worked on." (the first run of the lock).
- `tests/shape_first.rs`, the three new tests: the first run stopped at the file above before it reached them.
  Their red was seen in the second run with the built rule taken out (M1 below: all three failed, at the page
  while the wish is read), and with the line of the tap taken out (the third, at the scene shown at once).

## Mutations (one hold, each against its one test file)
| Mutation | Killed by |
|---|---|
| the note is left while the reading's note is on the page | all three tests of `shape_first.rs` named above |
| the note is taken away on an empty day | the first (the empty day) and the second (no wish kept) |
| the answer is never changed | `a_change_that_no_longer_fits…`, at the answer once the run ended |
| the answer is changed at once, while the new run goes | the same test, at the answer while the run goes |

## Tests changed
The test of `Make it again` has its new wish's run play `steps` in place of `hello`: the run waits for the
test's word, so the window is read while it goes and again after it ended. It changes nothing in the folder,
and what the test said of the folder holds as it did.

## Open
- Not run: a real window, a real harness, Windows and Linux.
- The scene the app keeps still holds the note of an empty day while a wish is read, and the answer as core
  gave it: `layout` of a kept day prints what it printed. Only what is drawn changed.
- Between a tap on a sharper wish's choice and core's answer, a look can draw the empty day's note again for
  the moment before the wish is kept. Not seen in a test.
- A window opened later knows of no answer: `said-<wish>` is not on its page at all, as before.
