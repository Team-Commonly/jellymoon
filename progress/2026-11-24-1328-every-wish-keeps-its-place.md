# Every wish keeps its place (2026-11-24)

One slice in the app: every made wish of the day keeps its own track while its run goes and its own notes when it
has ended, under its own typed words, all at once. ADR 0030. No row of `feature_list.json` changes. Core, the kit
and the binary are as they were.

## Built
- **A track for every working wish** (`app/src/scene.rs`, `live.rs`): one drawn note `steps-<wish>` for each made
  wish whose run goes, each with core's count of its own steps and the stage of its newest; a wish core has no
  step for is thinking, with none. The words behind a track name that one wish. A page with a clash has no
  track, as before: a working wish there shows its pill only.
- **Notes for every ended wish, with no tap** (`app/src/placing.rs`): `Today::looked` makes the notes of ended
  made wishes again on a look, `placing::put` puts a wish's notes where its own were and takes nothing of another
  wish. A wish of the reading is as it was: its note comes with a tap, one at a time.
- **A tap goes to its note's wish** (`placing::whose`, `showing.rs`): a choice, the second yes and an Enter act on
  the wish whose id the note carries; `busy-`, `sure-`, `file-`, `said-` and the hold after a choice are per wish.
- **One offer** (`answering::offered`, `one_offer`, `drawing::passed`): the field of a follow-up is drawn under
  the wish tapped last while core says it may be followed, else under the newest that may. A field to answer a
  question in always stays. `drawing::Open` keeps which wish was tapped beside which drawn notes are open.
- **When core is asked** (`app/src/showing/turns.rs`): every ended wish at the window's first look, in the look
  that finds a made wish's run ended, and at the look after core answered a choice on any wish's work; one wish
  at a tap on it. A look with nothing new asks core nothing of an ended wish. A wish with a note in the person's
  hand, or in the three looks after the person acted on it, is left and stays owed.
- **What came true** lists the reading's stars only; a made wish's row is left out, in the window and `layout`.
- The picture example has `--notes <file>`; the made-up window has the act `column`.

## Deleted
- The plural title of several wishes at work and the body that counted the others ("and N more").
- The rules of one open wish: the global `in_hand`, `settled`, `shown`, `told_on`, `busy`, `waiting_over` and
  `opened` of `working.rs`; the turn, the first-look state and the page-wide hold of `showing.rs`; the note a
  follow-up was typed under being taken away (`Answer::under`).
- 733 lines removed, 802 added in tracked files, and two new files of 94 and 271 lines; the app's count of Rust
  went up by 438 (365 of them the two new files).

## Tests changed, and why
Each pinned a rule this slice replaces; none was changed to pass on anything else.
- `f42/run.rs`: a made wish's star is no row of "came true today" (it is on its little one).
- `f47.rs`: a tap on a second wish leaves the first wish's note; a tap on a wish whose note is on show is nothing.
- `f54/answered.rs` (two tests, renamed): the wish's note stays while another is opened, so a refusal puts the
  question back into it, and an answer given to a run that asks again has the new question in it.
- `f54/more.rs`, `go_on_from_an_answer.rs` (two tests, one renamed): the note a follow-up was typed under stays.
- `one_column.rs`: a look shows the work of both wishes, each under its own words.
- `told_alone.rs` (five tests, three renamed): a look shows an ended wish with no tap; a wish told alone and
  another's work are shown together; the turn and the field that held every look are gone.
- `work_shows.rs` (seven of eight tests, six renamed): the rules of "its turn", "the newest alone", "one answer
  on the page" and "not over a note on show" are replaced by both wishes' notes at once. One of them now holds
  only the options of the work on show in place, not its field: `placed` knows no tap, so it draws the offer
  under the newest wish.
- `showing/tests.rs`, `live.rs`, `scene/tests.rs`: rewritten for the per-wish rules.

## Red first
`app/tests/several_wishes.rs` was written first and run before the code: all six red (the first try did not
compile, for a type in the test). The unit tests of `placing.rs`, `turns.rs`, `answering.rs`, `live.rs`,
`scene.rs` and `column.rs` were written with the code; the mutations below show they can fail.

## Mutations (each patched in and put back, against `several_wishes` and the app's library tests)
| Mutation | Killed by |
|---|---|
| a tap carried to the newest wish | `a_keep_and_a_toss_go_each_to_the_wish_of_the_note_tapped…`, `show_me_shows_the_work_of_the_wish_it_was_tapped_on…`, `two_wishes_at_work_have_each_their_track…`, and one of `showing::tests` |
| an older kept note not made again when a second lands | `two_changes_kept_by_themselves_have_each_their_note…` |
| the offer on every wish | `two_changes_kept_by_themselves_have_each_their_note…`, `answering::tests::one_wish_has_the_field_of_a_follow_up…` |
| only the newest working wish tracked | `two_wishes_at_work_have_each_their_track…`, `scene::tests::every_made_wish_whose_run_goes_has_its_own_track…` |
| finished notes dropped on a character tap | `a_wish_of_the_reading_opens_and_goes_as_before…`, and three more of `several_wishes` |
| core asked on every look | `a_look_with_nothing_changed_asks_core_nothing_of_a_wish_that_has_ended`, and four of `showing::tests` |

## Budget
The app's cap went from 17868 to 18306 (438: the two new files are 365 of them). The work took more holds of
the build lock than planned: the red run twice, the green run five times, as old tests that pinned the replaced
rules were found and rewritten, and the commit twice, as the first count of lines left the new files out.

## Open
- No real window was opened. The two pictures of the example (two at work; three done, one waiting and two kept)
  are the only look.
- A day of many made wishes is a very long column; nothing folds an older wish away. The big character is drawn
  beside each wish's first card.
- After a keep or a toss the wish shows core's answer alone for three looks, then how it stands over it.
- A tap on a made wish's little one takes that wish's own `said-`, file notes and `sure-` away, as before.
- The offer stays with the tapped wish and does not follow a newer wish; `layout` and `placed` know no tap.
- A change made outside the window is seen at the next run's end, answer or tap, not by a plain look.
- Windows and Linux were not run.
