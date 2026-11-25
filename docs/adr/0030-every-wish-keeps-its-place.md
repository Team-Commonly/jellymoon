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
    2. A new made wish of that project is made with those words, by the checks an Enter in the entry meets
       for a typed wish. Every check and limit of a typed wish holds: the day's eight, a project that is
       offered no more. Amended 2026-11-25 (ADR 0025, decision 9): it is made as the old one was made, with
       the prompt and the answer that one was kept with, when it had them.
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
    The reason is the run's end's, and no later keep writes or clears it. The folder can change after it:
    the change the work clashed with is undone, or the person puts their branch back. So where a record says
    a clash, the keep is asked once more whenever the note is read (as `run keep --ask` asks: nothing is
    changed or written), and work a keep would land now has the waiting note again, with `Keep it`. A note
    is read at the moments of decision 6, not at every look. The picture of such work is the kit's `unfit`:
    waiting work with its arrow barred and "does not fit your folder".

    Amended again 2026-11-24: the answer under the old wish ended "Your wish was made again, and it is being
    worked on.", and decision 11 keeps that answer on the page until the next wish is made, so it said so
    long after the new wish's run had ended. The new wish's id is now kept with the answer where it is made,
    with core's words for the same answer afterwards, and once that wish is at work no more the answer is
    drawn ending "Your wish was made again." (`app/src/again.rs`). The sentence is made shorter and not taken
    away: a second wish with the same words is on the page, and the answer is what says why. No sentence is
    matched, nothing is asked of core for it, and neither decision 6 nor decision 11 changes.

    Amended a third time 2026-11-24: walked in a window, one wish made again held three cards and its words
    three times: the old wish's bubble, a card that it was let go, the answer, and then the new wish's bubble
    with the same words. So when all four steps worked (the wish made, the old work tossed, the run started)
    the old wish steps aside: it is tucked away at once, as decision 11 says of it. Whether the work was
    tossed is asked of core after the toss: core keeps the old wish as let go, which a toss writes last, once
    everything else of it was done. No sentence of the answer is read. Where the wish was not made, the toss
    was refused or left unfinished, or no run was started, nothing steps aside and the answer is on the page
    as before: it says something the person has to know. What each step does, and their order, is unchanged.
11. **Finished wishes tuck away.** Decision 2 put the notes of every ended wish on the page at once. After
    three wishes the column was several screens of cards, most of them about wishes that need nothing from
    the person, and the wish being made was under all of them. So a made wish that is settled has none of
    its notes drawn once the next wish is made. Its typed words and its little one stay, and the little one
    says how it ended (its star; greyed when it was let go).

    - *The mark* (amended 2026-11-24; `drawing/ended.rs`). A star and a grey tell too little: nothing said
      what became of a tucked wish, or that a tap brings it back. So a tucked wish has one small pencil mark
      right under its bubble, at the bubble's end: a sign, a few words on one line, and a chevron that says
      it opens. A tap on it is the tap on the bubble. It is the kit's small form of a note's picture
      (`Note::Said`'s `small`): the app hands the kit a picture and never a word, so each word is a caption
      the card of that picture already has. Three ends have one. The change is in the folder: the wish's
      `kept-<wish>` note is drawn with its sheets in the folder, and the mark is that very picture, a small
      folder with its tick, the card's count and "in your folder". The wish was let go: core says so (the
      little one is greyed) and a note of the wish is on the scene; the mark is a sheet crossed out and
      "thrown away", in the quiet ink, with no count, as the scene holds none. The run finished and changed
      nothing: the note of what it says it did begins with the app's own sentence for that; the mark is an
      empty sheet and "nothing changed". Every fact is read from the scene the app holds: no call to core,
      nothing written. No sentence of a run and no typed word is read. Where the scene says none of the
      three (a change kept by a tap, a kept change the folder no longer shows, no note of the wish) there
      is no mark: never a guess, and the folder is drawn only from a note that draws the folder. Only a
      tucked wish has it: when the wish comes back the mark goes in the same drawing, and a wish whose
      work waits is never tucked away, so it never has one.

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
      newest. A wish that is at work at such a moment, under a later one, is tucked away when it settles
      (below): that tucks a result that may never have been drawn open.
    - *A remade wish steps aside* (amended 2026-11-24; decision 10). One wish is tucked away at another
      moment: the wish `Make it again` made again, once its old work was tossed and the new wish's run was
      started, at core's answer (`Tucked::aside`, from `Open::remade`). The rule above keeps the wish an
      answer is under on the page so that the answer is read; here the answer only repeats what the new
      wish, right under it with the same words and at work, already shows. The two rules sit side by side:
      the making of a wish still tucks every other settled wish away and still leaves one whose result was
      never drawn, and this one names a single wish, for its own reason, whether or not a note of it was
      drawn. It is hidden only while it is settled, as every tucked wish is, and the same tap brings it
      back, until the next wish is made. That tap's notes are made again from core and would leave the
      answer out, so the answer that a wish was made again is put back under them (`Showing::poked`): it
      ends "Your wish was made again." once the new wish is at work no more.
    - *Why at the making of the next wish, and not when a wish settles.* A wish settles by itself: its run
      ends, its change lands. Tucked away then, the result of what the person asked for would go from the
      page as it came. It stays until they ask for the next thing, which is when they have stopped
      reading it.
    - *A wish at work under a later one* (amended 2026-11-25). The rule above looked at a wish only when
      the next was made. Two wishes entered back to back, the second while the first was still at work,
      ended as two open results: the first settled after the second was made, and nothing looked at it
      again. The person had already asked for the next thing, so the reason above does not hold for it.
      Now every made wish but the newest that is at work when a wish is made, or when the window opens, is
      owed the tuck (`Tucked::tuck`; the wish an answer is under is left out, as above). At the first
      drawing where its run goes no more it is tucked away if it is settled then, and owed nothing after
      that either way (`Tucked::away`). So the page is what it would be had each wish been made after the
      one before it ended, whichever run ends first: one that needs the person when its run ends (a
      question, work that waits for a tap or does not fit, a run that failed) stays on the page, and
      stays once the person has answered or kept it, until the next wish is made. The newest wish is
      never owed it. A wish tucked away so never had its result drawn open; its little one says how it
      ended, and a tap brings the notes back. A wish stopped by the person while it is owed is tucked
      away when it is let go. The wishes owed it are the ones that stand before the new wish, counted on
      the scene core gave the answer (`Answer::earlier`), not every one but the last of the scene on show:
      when the page changed while core was asked (a look drew a step of the earlier wish's run), the scene
      on show does not hold the new wish yet, and its last made wish is the earlier one (amended after a
      walk in a real window, where that left two results open).
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

12. **The day folds** (2026-11-25; `drawing/folded.rs`). Decision 11 tucked the made wishes away and left
    the reading as it was: a read day opened as a row for each of its wishes, each with a label, a list of
    what came true as long as the day was busy, and its clashes, and the window opened at the end of all
    that. The entry, and whatever the person had made, was under a wall of the model's sentences. So what
    the reading put on a day's page is first one drawn note at the column's top, a **tally**, and a page
    with no made wish opens at its top.

    - *What is folded.* The reading's pieces and no other: the little ones nobody typed for, with their
      labels and stars; the list of what came true; a change of mind the reading found settled; a clash
      with nothing to choose; and the note a tap on a read wish brought, while it has no choice and no
      field. A made wish is never folded: its typed words, its little one, its notes, its tuck and its
      mark stand under the tally as they stood.
    - *The tally.* A picture first (the kit's `Picture::Tally`). A row for each project the reading has a
      wish in, in the order the day's projects were first typed in, which is the order of the entry's
      chips: the project's name in the hand, cleaned and cut in its middle as a chip's is, then one small
      pencil mark for one piece the open page draws. A `?` in a ring for a little one that needs the
      person and for a clash nobody chose on; a ring with three dots for a little one at work; a star for
      each row of what came true; a cross in the quiet ink for a little one that was let go. They are the
      marks of a whole list's rows, which is what the page already means by them. A row is one line: past
      twelve marks the rest are a count (`+9`). Past five rows one quiet line of Jellymoon's counts the
      projects left ("and 3 more projects"). Its title is Jellymoon's, "today so far".
    - *The marks have the page's colours.* A row of hollow stars read as an empty rating, so a star is
      filled with the theme's star colour, the one a star of the open day has, under the pencil's outline;
      the outline is what tells it from the note's ground, so no theme's star colour is refused. The `?`
      is the one mark that asks for the person: its ring is filled with the theme's accent, the colour of
      the badge on a little one that asks, where the pencil's `?` on that accent has a contrast of 3 to 1.
      Where it has not (the plain theme), the ring alone is in the accent if the accent has 3 to 1 on
      the note's ground, and else the mark is all pencil. The mark at work and the cross stay pencil and
      quiet. No colour is new.
    - *Whose reading, said once.* The footer under the page's title ends in "This is the model's reading,
      unedited.", right over the tally, with no tap. The app gives the tally that sentence as its last
      line too, and the kit leaves the line out where the footer it placed says it whole. So it stands
      once; and where the footer was cut at its four lines before the sentence (a day of many projects
      with long names), the tally says it.
    - *No word of the model's.* The tally draws no label, no star's text and no quote. A project's name
      is a folder's name from the session logs, which core holds a read wish to (`read.projects`).
    - *A tap opens it, and a second folds it.* A small chevron at the end of the title's line says so, and
      turns. Open, the tally stays, and under it is every piece of the page as it was before this
      decision: the tally is the scene's last note, which the kit stands at the column's top, so no other
      note's place among the notes, and so no id and no tilt, is another. The big character stands beside
      a folded tally, as beside a first card, and where it stood once the day is open.
    - *Never hidden.* The fold never hides a thing that waits for the person's tap. A clash nobody chose
      on is drawn under the folded tally, with its `?` in the row of its first statement's wish. A clash
      the person chose on is theirs, with the tap that changes it, and stays too. A note of a read wish
      that has a choice or a field stays (none has one today: `working.rs`, `read_note`). The note that
      delivery is off, the note of what was left out, and every status note are not the reading's and are
      never folded.
    - *Where.* Whether the day is open is the window's own state, held with which notes are open and
      which wishes are tucked away (`drawing::Open`): no call to core, nothing written, and a new window
      starts folded. It is applied last where a scene is made ready to be drawn (`Open::day`, from
      `Showing::column`), on the column decision 11 left. A page that is read again stays as the person
      left it. `layout`, which has no tap, prints no tally and every piece of the reading.
    - *Counted from what is drawn.* Which pieces are the reading's is read from the scene the app holds.
      The page data that scene was made from gives the two facts it does not carry, by the wish's id: a
      wish's project and how many of its jobs came true. Where a little one of the reading is no wish of
      that page data, nothing is folded and there is no tally: never a count of something not drawn.
    - *The page opens at its top.* A view opened at its column's end and stayed there (`Scroll::default`,
      ADR 0026). A page that opens on a tally and no made wish is held instead (`Scroll::hold`): at its
      top, with the date, the title and the tally. A page that holds a made wish opens at its end as it
      always did: that wish may wait for a tap there, and the tally above it is short now. When the tally
      is opened or folded the view is held where it is, so the tally stays under the pointer. A view that
      is at the end of its column is at its end again when the column grows; and a held view no wheel has
      moved goes to the end when the page holds one more made wish than before, wherever it was made
      (the entry, a card's field, `More like this`, `Make it again`, the command), so a new wish is not
      left under the window's edge. One thing stops it: while a field of a note holds words the person is
      typing, the page does not move by itself, whoever made the wish.
    - *Rejected.* Folding the made wishes too: they are the person's own, and decision 11 already puts
      away the ones that need nothing. A sentence in place of the picture ("8 wishes in 4 projects"): it
      is the wall again, shorter. A row for each wish: that is the page it folds. Keeping the fold in a
      file, so the day opens as it was left: the reading changes under it, and a window is cheap to tap.
      A second vocabulary of marks for the tally: the page has one already.

## Open

- A folded day shows no label, so which wish a `?` in the tally is of is seen only once the day is open. A
  day with no made wish whose open clashes are longer than the window opens at its top, with the last of
  them below the window's edge until the person scrolls.
- A day of many made wishes made a very long column. Settled wishes now tuck away (decision 11); a day of
  many wishes that are not settled (work that waits, runs that failed) is as long as before, and its bubbles
  alone can fill a window. Nobody has seen either page in a real window.
- Two results can still be open at once: a wish that ended in the second before the next was made, with no
  look between, has no note on the scene yet and stays (decision 11, *When*), and so does an earlier wish the
  person answered or kept after a later one was made.
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
  once more whether the change still does not fit before it says the wish's words. Which answer speaks of
  which new wish is the window's own state: a window opened later has no such answer on its page at all.
- Work that did not fit when its run ended has a picture of its own (the kit's `Kept::Unfit`: waiting work with
  its arrow barred and the caption "does not fit your folder"), since a drawn note places none of its words
  while it is closed. The two sentences are still read only after a tap on the picture or on `?`. Nobody has
  seen whether the barred arrow and those four words are enough.
- The note a follow-up was typed under stays, with its field. Before, it went, so that the follow-up's work
  could come by itself; now that work comes under its own words whatever is on show.
