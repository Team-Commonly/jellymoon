# 0030 · Every wish keeps its place

Status: Built in the app, 2026-11-24. Every made wish of the day has its own track while its run goes and its own
notes when it has ended, under its own typed words, all at once. Proved at the made-up window by
`app/tests/several_wishes.rs`, and with no window by the tests of `app/src/placing.rs`, `showing/tests.rs`,
`answering.rs`, `live.rs`, `scene.rs` and `column.rs`. Nobody has seen it in a real window. It builds on the
column of [ADR 0026](0026-the-page-is-one-column.md) and the pictures of [ADR 0029](0029-a-wish-s-work-is-drawn.md).
Core and the binary are as they were, but for decision 10, which is core's words and the app's carrying
(`app/tests/a_change_that_no_longer_fits.rs`). Decision 11, that finished wishes tuck away, amends decision 2
and is the app's alone (`app/tests/finished_wishes_tuck_away.rs`, `app/src/drawing/tucked.rs`).

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
       and was read from its repo. Nothing is changed. The installed command a run is started with is looked
       for next: with none, the page says so in the words a typed wish is refused in, and nothing is made or
       tossed.
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
    the run made the toss fail when this was built. So a run that core refuses to start, which cannot be asked
    before the toss, is found after it: the wish is kept with no run, as a typed one is, and the page says so
    with a typed wish's sentence for that and core's reason. A missing installed command, the one such ending
    that can be known before, is found first (step 1).

    It is offered only for the refusal that no later tap can get past: changes in the way go with a save, so
    their note points at `Keep it` again. Amended 2026-11-24: the waiting note offers it too, for one reason
    only. When a run ends and core's keep by itself keeps nothing, core writes why with the run's record
    (`unkept` in `runs/<id>/run.json`, by `run/keep/itself.rs`): its own typed reason and no sentence. Work
    whose record says the change did not fit (`Clash`) says so first on its waiting note, in the two sentences
    the refusal says, and has `Show me`, `Make it again`, `Toss it`: no `Keep it`, which core has just
    refused (`run/waits/unfit.rs`). Every other reason, and a record with none, leaves the note as it was.
    The reason is the run's end's. Whether a change fits is still asked of git only when a keep is tried, not
    at every look, and no later keep writes or clears it: where the folder has changed back since, the note
    says what was true then, and `Make it again` still works. The picture of such work is the one all waiting
    work has; the kit has no picture of a change that does not fit ("Open").
11. **Finished wishes tuck away.** Decision 2 put the notes of every ended wish on the page at once. After
    three wishes the column was several screens of cards, most of them about wishes that need nothing from
    the person, and the wish being made was under all of them. So a made wish that is settled has none of
    its notes drawn once the next wish is made. Its typed words and its little one stay, and the little one
    says how it ended (its star; greyed when it was let go).

    - *Settled* (`drawing/tucked.rs`, `settled`), read from the scene the app holds and from nothing else:
      core says the wish is done or was let go (its little one is neither at work nor needing the person,
      so no run goes and no question waits); no work of it waits (no `work-<wish>`); no note of it is in
      the person's hand (`placing::in_hand`: `busy-`, `sure-`, a file, the rows `Show me` brought); core's
      answer under it (`said-<wish>`) offers no choice, as `Make it again` is one; and no note of it has a
      field to answer in. A wish core says needs the person, which a run that failed is too, is never
      tucked away.
    - *When.* When a new made wish is kept: an Enter in the entry or in a follow-up's field
      (`Showing::answered`, an answer that asks for a run), and core's answer to a choice that made a wish
      (`Showing::ended`: a sharper wish gone with, `Make it again`, where the wish the answer is under
      stays, so that answer is read). Every wish that is settled then is tucked away, but one that ended
      since the last look and has no note on the scene yet: its result was never drawn, so it stays until
      the wish after. And when the window opens on a day that has made wishes: every settled wish but the
      newest.
    - *Why at the making of the next wish, and not when a wish settles.* A wish settles by itself: its run
      ends, its change lands. Tucked away then, the result of what the person asked for would go from the
      page as it came. It stays until they ask for the next thing, which is when they have stopped
      reading it.
    - *What brings it back.* A tap on its little one or its bubble (decision 5's tap, so its notes are also
      made again from core): the notes are back as an ended wish first has them, its drawn note closed, and
      stay until the next wish is made. That tap opens no drawn note; the next one does. A tucked wish
      that is settled no more (a change outside the window makes its work wait again, core has something to
      say of it) is shown again by itself, and stays shown when it settles.
    - *Where.* Which wishes are tucked away is the window's own state, held with which drawn notes are open
      (`drawing::Open`): no call to core, nothing written, and a new window starts from the rule for an
      opening. It is applied where a scene is made ready to be drawn (`drawing::passed`). The scene the app
      keeps holds every note, so a look, a choice and the making again of notes are as they were, and
      `layout`, which has no tap, prints every wish's notes. A tucked wish has no offer of a follow-up
      under it: the offer is under a wish whose notes are drawn (decision 4).

## Open

- A day of many made wishes made a very long column. Settled wishes now tuck away (decision 11); a day of
  many wishes that are not settled (work that waits, runs that failed) is as long as before, and its bubbles
  alone can fill a window. Nobody has seen either page in a real window.
- A tucked wish's notes are still made again from core at the moments of decision 6, though nobody sees them.
  A wish made outside the window (`jellymoon wish`) tucks nothing away: only a wish made in the window does.
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
- Work that did not fit when its run ended is drawn with the picture all waiting work has, whose caption says
  only that it is not in the folder yet. A drawn note places none of its words, so the two sentences are read
  after a tap on the picture or on `?`; with no tap the page shows `Make it again` where `Keep it` would be.
  A picture that says it would be a kit change: one more state of the sheets' picture with its own caption.
- The note a follow-up was typed under stays, with its field. Before, it went, so that the follow-up's work
  could come by itself; now that work comes under its own words whatever is on show.
