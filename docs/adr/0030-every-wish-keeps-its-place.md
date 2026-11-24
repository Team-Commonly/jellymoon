# 0030 · Every wish keeps its place

Status: Built in the app, 2026-11-24. Every made wish of the day has its own track while its run goes and its own
notes when it has ended, under its own typed words, all at once. Proved at the made-up window by
`app/tests/several_wishes.rs`, and with no window by the tests of `app/src/placing.rs`, `showing/tests.rs`,
`answering.rs`, `live.rs`, `scene.rs` and `column.rs`. Nobody has seen it in a real window. It builds on the
column of [ADR 0026](0026-the-page-is-one-column.md) and the pictures of [ADR 0029](0029-a-wish-s-work-is-drawn.md).
Core and the binary are as they were, but for decision 10, which is core's words and the app's carrying
(`app/tests/a_change_that_no_longer_fits.rs`).

## Why

The page is one column with each made wish's words in a bubble of its own, and under the bubbles it still behaved
as the page of one wish. One wish was open at a time: a tap on a second wish took the first one's result away, a
second finished wish waited "its turn" behind the choices of the first, and of several runs at work only the
newest had a track, under words that counted the others ("and 2 more"). A person who made three wishes saw one
answer and two bare bubbles, and could not tell that the other two had finished.

## Decisions

1. **Every working made wish has its own track.** `scene::ruled` puts one drawn note `steps-<wish>` on the page
   for each made wish whose run goes, and `live::tracked` gives each core's count of its own steps and the stage
   of its newest. A wish core has no step for is thinking, with none. The words behind a track are of one wish:
   "Your wish is being worked on." and the line that names that wish. The title for several wishes and the body
   that counted the others are deleted. A page with a clash has no track, as before: a working wish there has
   the pill over its little one and nothing else.
2. **Every ended made wish has its notes, with no tap.** While the day's page is on show, a made wish whose run
   is not going has the notes a tap on its little one gave before (`Today::notes_on_open`): work that waits as
   `told-<wish>` and the drawn `work-<wish>`, closed until tapped; a change kept by itself as `told-<wish>` and
   `kept-<wish>`; else `wish-<wish>`, with what its run said over it when it said something. A note of one wish
   takes no note of another away (`placing::put`), and core's answer to a choice (`said-<wish>`) stays with its
   wish. A wish of the reading is unchanged: its note comes with a tap, one at a time, and a tap on any other
   character takes it away.
3. **A tap goes to the wish of its note.** `Show me`, `Keep it`, `Toss it`, `Undo it`, the second yes and an
   Enter in a field act on the wish whose id the note carries. What they bring (`busy-`, `sure-`, `file-`,
   `said-`) stands under that wish. The looks held after a choice (`HOLD`) are that wish's own: every other
   wish's notes go on being made again. The choice to stop a run holds nothing: the stopped wish has the notes
   of an ended wish at the next look.
4. **The offer of a follow-up is under one wish.** Core says of each wish whether a follow-up may be made, so
   every wish's notes carry the field in the scene the app keeps. Where the scene is drawn
   (`drawing::passed`, `answering::one_offer`) the field stays under one wish: the one the person tapped last (its
   little one, its bubble, its picture, or a choice or a field of its notes), and with no tap the newest made
   wish that may be followed. Why one: "More like this" under every wish is one more line to read for each wish,
   and the next words a person types are for one wish. Why at the drawing: what may be followed is core's and is
   read when the notes are made, while which wish was tapped is the window's own and changes with no call to
   core. A tap on a wish that may not be followed leaves the offer under the newest that may. A field to answer
   a run's question in is never taken away.
5. **A tap on a made wish's little one or bubble** makes it the offer's wish, makes its notes again from core,
   and opens the drawn note of its finished work, or closes it when it is open. Nothing of another wish moves.
   The tap gives the notes an opening gives, so the wish's own `said-`, file notes and `sure-` go with it, as
   they did before; while core is at work for the wish (`busy-<wish>`) the tap does nothing.
6. **When the notes are made again.** Making a wish's notes asks core, which reads the run's record and the repo
   through git. A look is made once a second, so a look does not make them: it shows the notes the scene has.
   They are made again
   - when the window opens: every ended wish, on the first look;
   - when a made wish's run ends, or a made wish that has ended is new to the page: every ended wish, in the
     look that finds it, as a change that landed takes the undo of an older change away;
   - after core answered a choice on any wish's work: every ended wish, at the next look, the answered wish
     itself after its hold (so a keep's or a toss's answer stands alone for three looks, and then how the
     wish stands is drawn over it);
   - for the wish tapped, at the tap.

   A wish with a note in the person's hand (what core is doing, the keep asked once more, a file or the rows
   `Show me` brought) is not made again until that note is answered. `showing/turns.rs` keeps which wishes are
   owed their notes; a look whose scene was dropped owes them again.
7. **A stale choice is refused by core.** Between those moments a kept note may show a choice that no longer
   holds: an `Undo it` on an older change after a newer one landed outside this window. The tap is carried to
   core as any tap, core refuses it in its own sentence, and the wish's notes are made again under that
   sentence. The app decides nothing about a choice from what it drew.
8. **What came true lists the reading's stars only.** A made wish's stars are on its little one and its result
   is under its own words, so its rows are left out of "came true today", in the window and in `layout`. A list
   with no row is not drawn.
9. **A tap means what was seen.** With every wish's notes on the page at once, a card can come above a choice
   and move it under a click already on its way, and a note can be made again with another list of choices
   between a frame and a click. One of the choices throws work away. So the kit's view takes no tap on a choice
   for 400 ms after a new layout moved it in the window or it came where another piece took a tap (the wheel
   is the person's own move and holds nothing back), and a tap on an option carries the words that were drawn,
   which the app
   compares with the note it holds now: other words, and nothing is carried to core and the scene is drawn
   again. This is about which choice a click was on. Whether a choice still holds stays core's (decision 7).
10. **A change that no longer fits can be made again.** With several wishes at once, two can change the same
    place of one file. The first lands; a `Keep it` of the second is refused every time, and the page said so
    in the command line's sentence, which names a run and git. Now core says two refusals of a tap's keep in
    the window's own sentences (the folder changed in the same place; changes of the person's own are in the
    way), and the first has one way on, `Make it again`. Every other reason is told as it was, and `run keep`
    prints what it printed. The reason reaches the page as core's own typed reason beside the sentence
    (`run/keep/tapped.rs`); no sentence is matched.

    What a tap on `Make it again` does, in this order:
    1. Core is asked, and answers with the wish's own words and its project only while the wish's work waits
       and was read from its repo. Nothing is changed.
    2. A new made wish of that project is made with those words, by the call an Enter in the entry makes for a
       typed wish. Every check and limit of a typed wish holds: the day's eight, a project that is offered no
       more.
    3. Only then the work that does not fit is tossed, by the call a tap on `Toss it` is carried by, with its
       own checks. A toss that is refused is said as `Toss it` says it, and the new wish is run all the same.
    4. The new wish's run is started as a typed wish's is.

    Why this order: the work that does not fit is the only thing the person has of that wish. Tossed first, a
    day that has its eight wishes would leave them with neither the old change nor a new wish. So nothing is
    tossed unless the new wish was made; a refusal is said in the words a typed wish is refused in, and the
    old work waits as before, with its choices. The other way round costs at worst a new wish beside work that
    could not be tossed, which `Toss it` still throws away. The run comes after the toss because a toss asks
    where every copy of the repo stands and core refuses it while a run's copy is being made: started first,
    the run made the toss fail when this was built. So a run that cannot be started (no installed command)
    is found after the toss: the wish is kept with no run, as a typed one is, and the page says so.

    It is offered only for the refusal that no later tap can get past: changes in the way go with a save, so
    their note points at `Keep it` again. The waiting note does not offer it: whether a change fits is asked
    of git when a keep is tapped, not at every look, and the reason a change was not kept by itself is written
    nowhere (`run/keep/itself.rs` keeps only whether it landed).

## Open

- A day of many made wishes makes a very long column: each ended wish holds a picture and its choices. Nobody
  has seen such a page in a real window, and nothing folds an older wish away.
- A wish whose notes are made again higher up moves the cards under it. The window holds the column's end while
  the reader is there; what a reader in the middle of a long page sees was not looked at. A click that such a
  move would have put on another choice is closed (decision 9); what is still open of it: a choice whose words
  change in place is told apart only by the words its tap carries; and no click was made in a real window.
- A change made outside the window between two of the moments above is seen at the next one, or at a tap.
- The offer does not follow the newest wish once a wish was tapped: it stays with the tapped wish while a
  follow-up may be made of it.
- `Make it again` (decision 10) makes the wish with its words alone: they are not read again before the run,
  a sharper wish the old one was made with is not carried over, and a follow-up is made again as a wish of its
  own. A wish that was kept and whose run could not be started stays kept, with the old work tossed; that end
  was not run in a test. The choice stays on the page after the folder changed again, and core does not ask git
  once more whether the change still does not fit before it says the wish's words.
- The note a follow-up was typed under stays, with its field. Before, it went, so that the follow-up's work
  could come by itself; now that work comes under its own words whatever is on show.
