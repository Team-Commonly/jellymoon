# The app: `jellymoon-app`

The shell (`app/`) draws page data with the kit. Opened with nothing, it shows today's page (next section). Given a
page-data file with `--page`, it draws that and does nothing else: no reading call, no file written.

```sh
jellymoon-app [layout] [--date <YYYY-MM-DD>] [--fresh] [--logs <dir>] [--skip <names>] [--model <name>]
              [--theme <theme>] [--size <W>x<H>]
jellymoon-app [layout] --waiting [--date <YYYY-MM-DD>] [--theme <theme>] [--size <W>x<H>]
jellymoon-app [layout] --page <file, or - for stdin> [--theme <theme>] [--size <W>x<H>]
jellymoon-app layout --jelly <file, or - for stdin> --screen <W>x<H> [--theme <theme>] [--date <YYYY-MM-DD>]
```

These are all the flags, with `--help`. `--logs`, `--skip` and `--model` are `jellymoon page`'s; `--waiting` shows
the waiting page and reads nothing; with `--page` the flags of a day are taken and do nothing. `--jelly` and
`--screen` are the jellyfish's, and print it as data ("The jellyfish", below).

`--theme` is `sticker` (the default), `plain`, or the path of a theme file; the two names win over a file called the
same. A theme changes how the page looks and never which pieces are on it. A theme file is TOML in which every key
is optional (the kit's README has the keys and the bounds it is held to). A key the kit does not know falls
back to the sticker theme's, and `art` is not read, whatever it holds (a theme file names no pictures, the kit's own art
is drawn, and no file is refused for what its `art` names), each said in one `jellymoon-app: theme: ` line on stderr, the first 8 of them; a line is cut at 300 characters (`LONGEST_LINE`), with an ellipsis. A theme file that the kit refuses (a number out of its
range) exits 1 with one line that names the file by its last name, the key and the reason: `jellymoon-app: --theme
"<name>" is not sticker, plain or a theme file the kit takes: <why>`.

With `layout` it prints where every piece goes. Without, it opens the page's window (default 1440x900) and the
jellyfish ("The jellyfish's two windows", below); the command key and T (Ctrl and T off the Mac) switches the page
between the two built-in themes, and
`JELLYMOON_QUIT_AFTER=<seconds>` ends the app by itself. `--size` is from 960x640 to 16384x16384, each side a whole
number of pixels written in digits only: 960x640 is the smallest size a page is drawn at, and the window is asked not to shrink below it
(a unit test holds what is asked; that the system obeys was seen by hand on the Mac only). A wrong command line exits 2 with the usage on stderr; a file
that is over 1 MiB or is not page data exits 1 with one line. Both start `jellymoon-app: ` and print
nothing on stdout. A wrong `--size` or `--screen` is said back cleaned and cut at 40 characters, with an ellipsis.

A file that cannot be read, for `--page` or for `--jelly`, exits 1 with `jellymoon-app: the file cannot be read:
<name>`, where the name is the file's last name on one line: no folder that was typed, and no word of the
system's. With no name left it is `jellymoon-app: the file cannot be read`; for stdin it is
`jellymoon-app: the input cannot be read`. A `--theme` that is neither of the two names nor a file that can be
opened may be a name mistyped, so the same words come after what it takes: `jellymoon-app: --theme is sticker,
plain or a theme file: the file cannot be read: <name>`.

## Today's page (`app/src/today.rs`)

With no `--page` the app shows a day: today in the computer's time zone, or `--date`. It calls core's `day` and `page`
itself, with what the `jellymoon` command would use: the logs under `CLAUDE_CONFIG_DIR` or `~/.claude` (or `--logs`),
the harness `JELLYMOON_CLAUDE` names or `claude` on `PATH` or in a well-known place (`page.md`), `--model` (default `sonnet`), a 180 s limit. It never runs
the `jellymoon` binary. Core's rules are core's: only the day goes to the call, with every tool off. The user's home
is the `HOME` variable (`USERPROFILE` on Windows), a full path, and nothing else: with neither that nor `--logs` and
`JELLYMOON_HOME`, the app exits 2 and reads nothing, so a test run with an empty environment cannot reach real logs.
A home that is not an absolute path counts as none, and a `JELLYMOON_HOME` that is not absolute exits 2.

- **Made wishes.** The day's made wishes ([wish.md](wish.md)) are drawn after the reading's on every scene of the
  day: the kept page, a page just read, and, under its one note, the day that is still being read or could not be.
  They are put on by core's `with_made` when the scene is drawn, and are never in the kept page.
- **Where pages are kept.** `<home>/pages/<YYYY-MM-DD>.json`, where `<home>` is `JELLYMOON_HOME` or `~/.jellymoon`.
  The file is the checked page data, one line as `jellymoon page` prints it, and nothing else, in a folder for this
  user only (0700; the file 0600). It is written beside its place, in a file named for the process, and moved there.
  It is read back through the same door as any page file, and first through core's door for a kept page
  (`jellymoon_core::kept_page`), so a kept file that is not page data, is over 1 MiB, is of another day, is a link,
  or can be written to by its group or by others is not drawn: the day is read again.
- **What is shown first.** The kept page, when there is one that read something or whose logs were read. A kept
  page that says no logs were found is not drawn first: it may be stale by now, so the waiting page is shown, and
  the count step draws it again when there are still no logs. That is asked in one place, where the first page
  is chosen (`Today::first`): the steps after it are handed what it chose, so they are never handed such a page.
  On a day with no logs the kept file is read a second time only when no earlier page was handed on and core
  has a page for the day: to see that core's page is the one kept. Otherwise
  a waiting page: the date, the hero and one note, "Reading your day…". The window opens on it at once, before a
  log is opened: reading the logs alone can take seconds. The rest runs off the window's thread in two steps,
  each of which may give the next page to show
  (`today::Next`): the count of the day's turns, then the reading call. While a kept page is read again it stays on
  show under one note, "Reading your day again…", and never gives way to the waiting page. `layout --waiting` prints
  the waiting page; `layout` alone runs the same steps and prints what the window ends on.
- **When the day is read again.** Each time the app opens it reads the session logs (no model) and compares the day's
  count of turns with `read.turns` of the kept page. The same count: the kept page stays and no call is made. Any
  other count, no kept page, or `--fresh`: one reading call, and the new page is kept. The count is the whole test,
  so a day whose turns changed but not their number (a folder switched off and one more turn typed) keeps its page
  until `--fresh`. A day with nothing typed makes no call.
- **A day with no session logs** (F63). The reader's answer is "no session logs", and the app then asks core for
  the day's page all the same (`jellymoon_core::page_with_no_logs`). Core has one when the day has a made wish or
  a project on offer: an empty page that says so (`read.no_logs`), made with no call. It is kept and shown as a
  reading's page is, with the made wishes on it, the entry and its chips; with a made wish on it, its footer is
  "No Claude Code session logs were found, so nothing was read.", then the count of made wishes as on any page. The folder looked in is
  not named on this page: core's page does not carry it. Core has none for a day with nothing made and no project
  on offer, and that day is the plain note it was. A refusal of core's own (a `--model` that is no name) is the
  note, in core's words. A kept page that read something is never replaced by it: that page stays, under "This
  page is from an earlier reading." A kept page that holds nothing a reading gives is replaced by it when the
  logs folder goes; one of 0 turns that holds a wish, a clash or a project read stays, as an earlier reading. A
  kept page that says no logs were found is no earlier reading: once there are logs the day is read as for the
  first time, behind the waiting page, and when there is no page the note is the one of a day with none kept
  ("When the reading cannot run"). The same page as the one kept is drawn and not written again. The field
  counts only on a page that holds nothing a reading gives (no turn, no project, no clash, no wish of its own);
  on any other page it is not looked at, and the footer is the reading's.
- **A day whose logs hold no turn of it** makes no call, and its page has no footer, with a made wish on it or
  none: every wish on it is the person's own and there is no reading to speak of, so a footer would only say
  so in words a new person does not need. The legend stays while a wish is on the page, for its marks. (Its
  footer once said "Nothing typed on this day was read, so there is no reading. 1 made wish is in your own
  words.", which read as jargon to a person who had typed one wish.)
- **Folders switched off.** `--skip <names>`, and `<home>/skip.txt`, one folder name a line, for an app opened with
  no command line. Core reads the file (`reader.md`), for the app and for `jellymoon day` and `page` alike, so the
  three leave out the same folders. A name matches exactly, and case matters, and a `--skip` name with a character
  that cannot be seen is an argument error: `reader.md` has the rules, and the footer names the projects that were
  read. No `skip.txt` is no names. Anything else under that name that is not a whole, plain list (`reader.md` lists
  what is refused) stops the reading with a plain note and no call, so a folder switched off never reaches the call
  because its list was damaged. A byte order mark at its start is dropped.
- **When the reading cannot run** the page is one plain note that says why. Where no reading call was made its
  title is "There is no page for this day yet.": Claude Code was not found, no session logs (the note names the
  folder looked for, as the command's line does), logs or a `skip.txt` that could not be read, a `--model` that
  is no name, or a call that could not be started. Where a call was made and gave no page it is "Today
  could not be read.": the call failed, its result says it was an error or is no result (core's own line
  follows), it took too long, the model's answer was not a page, or the call was started and the wait for its
  end failed. With a kept page that read something, that page stays and the note is its first, "This page is from
  an earlier reading." A page that could not be written to its file is shown with a first note that says so.
  `layout` prints these pages and exits 0: they are what the window shows.
  - **A call that could not be started.** The body is "Claude Code could not be started, so nothing was read.
    Open Jellymoon again to try once more. What happened: " and core's own line, which ends it and names what
    could not be made or started:
    - the reading folder: "the reading call could not run: the reading folder in Jellymoon's own folder could
      not be made or written to: " and the system's reason;
    - the program `JELLYMOON_CLAUDE` names: "the reading call could not run: JELLYMOON_CLAUDE names no program
      that could be started: " and the system's reason. The path the variable holds is not printed;
    - a `PATH` with no folder named by its full path, or a harness's place that cannot be made a full path:
      "the reading call could not run: " and the reason ([page.md](page.md), "The call");
    - a `claude` that was found: "the reading call could not run: the `claude` that was found could not be
      started: " and the system's reason. Here, and in no other case, the body says "Check that `claude` answers
      in a terminal." before "Open Jellymoon again": only this trouble is one a terminal would show.
  - **A started call that could not be waited for.** The body is "Claude Code was started, but Jellymoon could
    not wait for it to end and stopped it, so nothing from it is shown. Open Jellymoon again to try once more.
    What happened: " and core's line, "the reading call was started, the wait for its end failed, and it was
    stopped: " and the system's reason. Core stops the call and waits for it before the call's folder is removed
    (`crates/jellymoon-core/src/call/wait.rs`). Whether the call had read the day is not known.

The window makes no second reading while it is open, and closing it while a call runs leaves that call to end by
itself. It does look again at what core keeps of the day's made wishes ("A wish in the window").
A tap in the window is one line on stderr, the piece's id and the option's number, never a word of the page.

## Taps (`app/src/taps.rs`)

The rules of a tap are core's (`crates/jellymoon-core/src/tap.rs` and `command.rs`, [delivery.md](delivery.md#a-tap-on-a-clash)):
which statement stands, when a tap makes no ruling, what is kept and in which order, and which `jellymoon` command
the hook may name. They are proved at `jellymoon settle` and `jellymoon hook install --installed`. The app has no
`tap` command. It draws what core says and carries a tap to core by calling it, never by running `jellymoon`. A page
drawn from `--page` has no day, and a tap on it does nothing.

- **What is drawn.** For each clash core gives its two statements in the exact words a ruling would carry
  (`jellymoon_core::standing`), and the side that stands by the person's own ruling. The app puts each between
  quotation marks and adds nothing. A clash's note on a day's page is a whole choice: the kit draws every word of
  both options, smaller when it must, or leaves the note out and counts it in "+N more". So a statement that is
  cut is never on a page that can be tapped. A ruling carries at most 200 characters, and what is drawn is those
  200. A page drawn from `--page` cannot be tapped and keeps its quotes cut to fit. A clash with nothing to choose
  (its two statements are the same once cleaned, or one is empty: `Standing::is_choice`) is not offered: it is one
  plain note, "two things you said clash", with neither statement and nothing to tap, since core would refuse it.
- **What fits.** One clash of two statements of 200 characters is drawn whole at 960x640 and at 1440x900, in words
  and in a script of wide characters with no spaces. At 1440x900 an option is written at 14 pixels. At 960x640 it
  is 11, or 9.35 when the page needs the room (the kit's 0.85 step; the last step, 0.72, is 7.9), which is under
  the 11 pixels every other text keeps. `layout` prints each run's `size`, so the step taken can be read. The
  page scales with its window, so a larger window gives no more room. A clash the person has not chosen on is placed before one they have, so when two long clashes do not
  fit together the open one is drawn, and the other when the first is settled. Past that (four statements of 200
  wide characters) neither is drawn: both are counted, and `jellymoon settle` is the way to rule on them. A
  settled clash that is counted cannot be turned over in the window either.
- **A tap acts on what was drawn.** The window keeps the scene on show (`OnShow`). A tap names a note and an option.
  The note's id must be exactly `clash-<n>` for a clash of the page. The option's drawn text must be one of the
  two statements core gives now; that side is asked of `jellymoon_core::settle`. When it is neither (the page
  changed under the scene) nothing is sent.
- **Settled at once.** The note's title becomes "you chose". The statement that stands is drawn second and the one
  it replaced first, struck in the theme's own way: the kit strikes the first option of a choice and no other, so
  the order of the two is how the app says which stands (`scene::order`). No reading is made.
- **A tap that did nothing** puts a first note on the page, "That tap did nothing.", with core's reason. A ruling
  that was made but that the page could not keep puts "This page could not keep your choice." there: the choice
  stands, waits for the sessions, and is drawn as chosen while it waits.
- **Delivery.** While the hook is not installed (`jellymoon_core::hook_installed`), a page with a clash carries one
  note before its clashes. When core takes the installed command (`jellymoon_core::installed_command`) and the
  settings file has a full path, the note is "your sessions are not told yet" with one option, "Turn on delivery",
  whose tap calls `jellymoon_core::turn_on_delivery`. Otherwise it is plain text that says to run
  `jellymoon hook install` in a terminal. Once the hook is in, the note is gone.

`after_tap` and `OnShow::tap` need no window; `app/tests/f35.rs` follows them.

Not covered: a ruling made while delivery is off waits its 24 hours like any other and is told if the hook goes in
before then; the page does not say afterwards whether a session heard it. The note of the delivery state sits before
the clashes, so on a page too full for all its notes it is a clash that falls into "+1 more", not the note.

## A wish in the window (`app/src/wishing.rs`)

A day's page has one entry (the kit's, `crates/sticker-kit/README.md`): a field to type a wish in, and under it a
chip for each project a wish can be made for that day, whatever the page read, of which one is always
chosen. It stands in the left column, under the title "what I wished for today", where the wishes are listed. The chips
stand in one row when their names fit it and in two when they do not, and after them stands "Choose a folder"
(below, "A project given in the window"). The field is the page's main thing: its row runs from the content's left
edge to the left column's right edge, over the big character, which stands under it, and "Start" ends the row: it
does what Enter does, and with nothing typed it enters nothing and the note "Nothing to start yet." says what to
type first (`starting::UNREADY`); it goes before the notes standing, which stay. While the field has the keys
its ring is a calm colour of the theme, never the red an error would have: the sticker theme's lilac (#6F5BD6,
of the family of the field's edge) and the plain theme's ink (#1F2328), each over 3 to 1 on the paper
(`crates/sticker-kit/tests/colours.rs`). The hint says what the chips are: "What do you want done in the project
below?", and with more than one chip "What do you want done? Choose its project below." (Tab still changes the
chip). Each is whole in both themes from the smallest page (960x640) up as `layout` measures it, by the advances of
the bundled Nunito at the size typed words are drawn (15 design pixels), which also stands in for the plain
theme's system font (`app/tests/f42.rs`, at three sizes; `app/tests/f66.rs` at the smallest); and, for the hint
of F42's words, as the window shapes it on a Mac, in Nunito and in the
system's font (headless pictures at 960x640 and 1440x900). A wider font cuts it with an ellipsis at the
smallest page: one whose every character advances by 0.56 of its size, as the kit's layout tests measure, does. The rules are core's ([wish.md](wish.md), [run.md](run.md#a-run-started-apart)) and are proved at
`jellymoon wish` and `jellymoon wish --run`. The app carries the words and a project's name to core, and draws
what core keeps.

- **When there is an entry.** On a day's page, when a project is on offer that day: a day nothing was typed on
  has one for a listed project (F63). A day with no project on offer, page or none, has the entry a new project
  is named in (F66, below). A day with no page (still being read, or not readable) and a project on offer, the
  waiting page and a `--page` file have none: a wish needs a project, and the app names none of its own. The
  window opens with no entry, and has one as soon as core has said which projects have a chip. The entry takes at most 200 characters,
  core's cap on a wish (`jellymoon_core::MADE_SAID_MAX`). `layout` prints the field as one `entry` piece, its
  hint as text, and a `chip` piece for each option, after the scene's pieces.
- **Which projects have a chip.** Core says (`jellymoon_core::offered`): a name `jellymoon wish --project`
  takes (one folder's name of at most 100 characters, no path, nothing unseen) that is the name of a folder
  something was typed in that day, with the folders switched off left out, or that is on the list of projects
  Jellymoon knows with a folder that stands ([project.md](project.md)). That is asked of the reader, with
  the lookup a run finds its repo by, so `--skip` and `skip.txt` are matched against the folder's whole path: a
  folder switched off by a name above it has no chip. This holds from the first scene, which is the kept page
  of an earlier reading, so a project of a kept page with no folder typed in today, and no line in the list,
  has no chip either. A
  `skip.txt` or a logs folder that cannot be read offers nothing. Whether the folder is there, and is a repo, is
  the run's to say. The names come in core's order: those typed in that day, as the reader lists them, then
  the list's. The app hands the kit every one. The kit draws the first six as chips and counts the rest in words
  after the last chip, "+94 more" for a list of 100 (`layout` prints it as one `more` piece, its `index` the
  count); the count is no chip, and no click or Tab chooses it. A name is drawn as text: markup, the shape of a
  link or a character that cannot be seen changes nothing but the chip's words, and a name too long for its chip
  is cut in the middle by the kit's measure (`app/tests/f63/chips.rs`).
- **What the chips cost.** Every session log of the day is read once more for them, off the window's thread:
  when the window opens and when a reading ends, and once more for each Enter. Measured on a Mac with a release build, as one `jellymoon day` (one read, and more than a lookup does, since it also prints the day): 300 made-up logs of 1000 lines each, 137 MB, with 10,000 turns typed on the day, took 0.9 seconds, and 2.4 the first time, before the system had the files. A day of a few logs is a few milliseconds.
- **Keys.** What a person types goes to the entry whenever nothing else on the page holds the keys: the window
  opens with them in the entry (on a first page, in the field that takes a name), and they are the entry's again
  after a tap on a note's option or on a character, a click anywhere off the fields, and Escape, which in a note's
  field gives them to the entry and in the entry keeps them there. A click on a note's field, or on its offer,
  gives that field the keys. Tab chooses the next project and Shift-Tab the one before; a click on a chip chooses
  it. Enter enters the text. While an input method is composing, Enter confirms the composition and enters
  nothing. A bare `t` is always a letter: the theme is switched with the command key and T (Ctrl and T off the
  Mac), which no field takes (`app/tests/entry_keys.rs`). The window says one line on stderr for an entry, the
  entry's id and how many characters were entered, never a word of them.
- **Enter.** The words and the name on the chosen chip go to `jellymoon_core::wish_offered`, off the window's
  thread (`OnShow::wishing`), so a day that is locked, a slow disk or many logs never hold the window. The app
  passes the name, never a path and never the chip's place: the chips can change while the words are typed.
  Core keeps the wish only for a project it offers at that moment, and refuses any other name. A wish core keeps
  is drawn as soon as core has answered, in the working state, from what core keeps of it: no reading is made.
  Words core refuses write nothing: the page's first note is "That wish was not kept.", with why. Taking the
  answer on the window's thread (`OnShow::answered`) opens no file.
- **While core is asked the entry waits.** From Enter until core's answer the words stay in the field, drawn
  pale, with no caret. The entry takes no key then: no letter, no editing key, no Tab, no Escape, no click on a
  chip, no second Enter, and no input method can begin a composition. The keys stay the entry's. A key pressed
  while it waits is not accepted and
  is not kept for later. The wait is the time of one read of the day's logs, and at most that and the 3 seconds
  core waits for a day that is locked. Kept: the field is empty. Refused: the words are still there, as typed,
  and the entry takes keys again, however many refusals follow one another.
- **An entry with words in it is never taken away.** The window gives the page the entry of each reading step,
  and none when no project is offered any more (every folder switched off in `skip.txt` while the reading runs,
  logs that cannot be read). The kit keeps an entry that holds words, or waits, on the page as it was, with the
  chips it had, until it holds no words and is answered (`Page::set_entry`). So a refusal always has the words to
  leave, and words that were typed and not entered stay. Spaces alone are no words: an entry that holds only them
  goes as an empty one does. Enter on a chip that is still drawn is asked of core like any
  other, and core refuses a project it no longer offers.
- **A wish's note** names its wish: its body is the start of the words between quotation marks, at most 40
  characters, then core's reason. Core cleans and cuts that start (`jellymoon_core::wish_start`), as it cleans
  a kept wish's words, so a credential typed in a wish is not in its note; a kept wish is named by the words
  core kept. The words are the person's own, and the note is text.
- **The run.** For a kept wish the app asks core for a run, off the window's thread. Core checks the installed
  `jellymoon` command (`jellymoon_core::installed_command`, in the folder `install.sh` puts it in) and starts it
  with `jellymoon_core::run_apart` as `jellymoon run --wish <id> --date <day> --logs <the app's logs>`, with the
  app's `--skip` names, in a process and a process group of its own. No run is ever in the app's process, and
  the app starts no other program. `jellymoon run` finds the project's folder in the day's logs as it does for
  `jellymoon wish --run`. Closing the window leaves the run going.
- **When no run was started** the wish is kept and stays as it is, and the page's first note is "Your wish is
  kept, and no run was started for it.", with why in core's words: the command was not found or is not one core
  takes, no harness was found, the folder is not one a run can copy, three runs are going, or the first line
  `jellymoon run` said. `jellymoon run --wish <id>` in a terminal runs it later.
- **The look.** Once a second, off the window's thread, the app draws the day again from what core keeps
  (`jellymoon_core::made`, and the page's rulings) and shows it when it differs from what is shown. So a run's end
  is drawn with no tap and without opening the app again: the star, or the wish needing the person. A wish made
  or run in a terminal shows up the same way. A look can end a run that was cut off, as any look does
  ([run.md](run.md#a-run-that-was-cut-off)), which can take a few seconds; the next look waits for it.
  A look that finds a run ended also shows the work of it that waits ("Work that waits shows itself", below).
- **A look never takes a note away.** The plain notes the app put over the page (a reading that is going or
  failed, a tap that did nothing, a wish that was not kept or has no run, a stop) stay over what a look draws.
  A look made from a scene that is no longer on show is dropped.
- **When a note goes.** A wish's note goes with the next Enter that core answers, kept or not, and with a tap on
  a clash. A reading that ends keeps the notes of a wish and of a stop over its own page (`OnShow::show`). The
  note of an asking that ends late goes under the notes of wishes entered since, and takes none away.

`OnShow::offering`, `wishing`, `answered`, `asked`, `look` and `seen` need no window (`wished` is two of them in
one, for a caller with none); `app/tests/f42.rs` and `f42/notes.rs` follow them, and `f42/run.rs` the run, through
a made-up window that is a program of its own.

## A wish read before its run (`app/src/shaping.rs`)

Enter in the entry does not keep the wish at once (F91, [ADR 0025](../adr/0025-the-wish-is-shaped-before-a-run.md)).
`Showing::wishing` is `OnShow::shaping`: the scene shown at once begins with the text note `shaping`, titled
"Reading your wish…", with the start of the typed words as its quote, and core's `shape` is the work for the other
thread. With no project chosen nothing is read, and the Enter is a typed wish's.

- **`clear` and `as_typed`.** The same work goes on to `OnShow::wishing`: the wish is kept and its run asked for as
  before, the note goes, and nothing more is said. A bad answer, a call that takes too long and no harness all
  end here.
- **`shaped`.** Nothing is kept. The entry is emptied, and two notes stand where the reading's stood:
  `shape-said`, the kit's told note titled "Your wish, sharper", with a row for each text that is not empty under
  the labels "The wish", "Done when", "Not touching", "Only you know" and "Why"; and under it the text note
  `shape`, titled "What you typed", with the typed words as its quote and exactly two options, "Go with this" and
  "Go with what I typed". Every word of the model's is a quote of a row. Every other word is fixed here.
- **A tap** on either option is one arm of `stopping::chose` and an ordinary `Chosen::Work`: both notes go at
  once, and the work asks core's `wish_of_shape` to keep the wish, with the shape or as typed, then asks for its
  run as a typed wish's is asked for. Core keeps it only while that shape still waits for those words; what it
  refuses, and a run that was not started, are said in the notes a typed wish has for them.
- **Enter while they stand** takes the pair away and reads the new words: a page holds one reading or one pair.
  A look leaves them (`working::names`).

`app/tests/shape_first.rs` follows all of it through the made-up window (`support/tapper.rs`, `enter:wish@<project>:…`).

## A project given in the window (`app/src/starting.rs`)

ADR 0020, decisions 3 and 4. The rules are core's, proved at `jellymoon project add` and `jellymoon project new`
([project.md](project.md)). The app carries a folder or a name to core and draws what core says.

- **Start something new (F66).** With no project on offer the entry's place holds one choice, "Start something
  new", as its one chip, and a field for a name, whose hint is "Choose a folder below, or type a name for a new project", the folder first as the page's note
  says it. The chip is a
  choice like any: a tap on it gives the field the keys. "Choose a folder" is the loud choice there, filled as a
  chosen chip is, and the words say to choose the folder first (below). The
  entry's id is `new` and it takes at most 100 characters. Enter carries the name to
  `jellymoon_core::new_project`, off the window's thread. The row says "in core's words": core has no words for
  the choice, so they are ADR 0020's, kept in the app (`starting::START`).
- **A folder dropped (F64).** A folder dropped anywhere on the page's window goes to `jellymoon_core::add_project`,
  off the window's thread, with or without projects on offer. Of several dropped, the first is given. The kit
  tells the app of a drop (`Page::on_paths`).
- **Choose a folder.** Every entry of a day has one thing to do after its chips, "Choose a folder" (`layout`
  prints it as one `action` piece). A tap on it opens the system's folder picker for one folder, and what is
  chosen goes where a drop goes. A picker that is cancelled does nothing.
- **Core's answer is one note**, in place of the one before it (it is a wish's note, `wishing.rs`): "A project was
  added." or "Your new project is ready.", with "“<name>” is ready: type what you want done."; or "That
  folder was not added." or "No project was made.", with core's refusal. Core's words of `--repo` are said of
  "that folder", and of `project new` of "a new project"; nothing else of them changes. The path is carried to
  core and is never drawn: core's refusals do not name it, and the app adds nothing.
- **The chip at once.** Once core has answered, the page's entry is asked again: the project has its chip, which
  is the chosen one, and the entry has the keys. A day that had no page has the one core gives a day with no logs
  and a project on offer (F63), by the first step of a reading, which makes no call; a step that would call the
  model is not taken, and the entry waits for the page, as before.
- **While the day is read.** A drop, a folder chosen or a name entered while "Reading your day…" is on show goes
  to core at once, and its note is shown when core answers. When no entry offers the project yet, the window
  keeps its name (`Showing::wait_for`): its chip is chosen, with the keys, as soon as an entry offers it, the
  entry asked again at once or the one a step of the reading gives.
- **A drop after a key.** GPUI's own drop (`on_drop`) asks whether the page is hovered, and after a key it is
  not until the mouse moves; a drag from another app moves no mouse over the window. So a folder dropped after
  typing was lost. The kit takes a drop from any mouse-up while something is dragged (`Page::on_paths`).
- **Nothing typed is lost.** The kit keeps what is typed when the entry changes (`Page::set_entry`): words typed
  for a wish stay through a drop, and a half-typed name stays in the wish's field. A name core takes empties the
  field; a name it refuses stays.
- **The empty page says what to do here.** The note of a page with no wish is "Nothing wished for yet today.",
  with "Type what you want done in the field at the top." when a project is on offer, and with none (no project
  read on the page and none listed) "Choose the folder you want to work in, or type a name for a new project in
  the field at the top." (`Today::draw`). A day with no session logs says "You can start here: choose the folder
  you want to work in, or drop it on this window. Or type a name for a new project in the field." Words entered
  in the entry of a day with no project are a new project's name and never a wish (`starting::named`). A page
  with nothing read and no wish, projects on offer or not, has no legend and no footer: it has no mark to explain and no reading to speak
  of (`app/tests/entry_first.rs`).

`OnShow::listing` and `listed` need no window. `app/tests/f64.rs` and `f66.rs` carry a folder and a name through
the made-up window of F42 (`tests/support/wisher.rs`), where core starts git with the test's variables;
`f64/window.rs` drops a folder on the page on GPUI's test platform (`FileDropEvent`) and answers the folder
picker (`simulate_path_prompt_response`); `f66/window.rs` enters a name core refuses and cancels the picker. A
name core takes is not entered in the window on the test platform: core would start git with the test's own
variables. What only a person at a window can see is not tested: the system's picker itself, and a drop from
the Finder.

## What the window does, with no window (`app/src/showing.rs`)

`window.rs` is wiring: it opens the window, runs work off the window's thread, and hands each scene to the kit's
view. What each act does is `Showing`, the day on show behind the calls the window makes, with no GPUI in it:

| Act | The window calls | Then, with what the work gave |
|---|---|---|
| a key | `switches_theme(key, command key held)`, `other_theme(name)` | |
| the first page | `offering()` | the entry is set |
| a step of the reading | `reading(step)` | `read(&next)`: the scene, and the step's entry |
| once a second | `look()`, which also asks core of a wish whose run ended | `seen(seen)`: the scene, or none for a look of a scene that is gone |
| Enter | `wishing(words, chip's name)` | `answered(answer)`: what the entry is told, the scene, the asking |
| the asking's end | | `asked(said)`: the scene, when there is something to say |
| a tap on a character | `poked(id)` | `seen(seen)` |
| a tap on an option | `chose(note, index)`: the scene at once, and a stop | `stopped(stopped)` |

A page-data file has no day: every call gives nothing, and an Enter is still answered, not kept, so no entry waits
for ever. The entry is told its words were kept only when core kept the wish. `app/tests/f43/showing.rs` makes these
calls in the window's order, the work run on the test's thread; the two lines the window says on stderr
(`tap_line`, `entry_line`) are there too.

One piece of the wiring is run by a test, on the Mac, with no window on a screen: `app/tests/f43/window.rs` opens
`window::page` on GPUI's test platform, clicks the field, pastes words and presses Enter. It holds that the window
tells the entry what core said (not kept: the words stay; kept: the entry is empty), and that an Enter on a page
with no day is answered. A handler is given the window it was called in, so there is no handle of the window to
wait for: no Enter can go unanswered for want of one.

What stays in `window.rs`, and is run by no test: opening the window on a screen and its options (a unit test
holds what is asked), the one-second timer, the order of the reading's steps, which call a tap is carried to, the
`defer` that lets the view finish a tap before it takes a new scene. The keys (who has them, and the command key
and T) are played through the window on GPUI's test platform (`app/tests/entry_keys.rs`); which view a real
window gives the keys to is still owed a person's walk.

Not covered: a wish for a project past the sixth, or for one neither typed in today nor listed, is made with
`jellymoon wish`. A made wish's question is drawn on its wish's note ("A question answered, and more like
this", below): that a day nothing was typed on prints it as any day does is proved at `jellymoon page`
(`crates/jellymoon/tests/f63.rs`). What only a person at a window can see is not tested: the input method's
candidate window, the caret, the pale words of an entry that waits, and which piece has the keys.

### The walk: the loop step by step, with pictures (`app/examples/walk.rs`)

So that someone looks at the steps of the loop in order before a release: the page when a wish is entered,
while its run goes, when its work waits with its result, and after `Keep it`. The example makes the calls the window
makes, in its order, through `Showing`, against a real core in the home the environment names; it is meant for a
made-up home. It opens no window and starts no program; core starts what the app starts.

```
cargo run -p jellymoon-app --example walk --features sticker-kit/headless -- <folder> 1440x900 \
  type:garden enter "type:<a wish>" enter "until:Your folder is not changed yet.:30" picture:waits \
  "tap:work-m1:Keep it" picture:kept
```

The steps are `type:<words>`, `chip:<name>`, `enter`, `start` (the tap on `Start`), `folder:<path>` (as
`Choose a folder` gives it, with no picker), `look`, `until:<state or note>:<seconds>` (a look once a second, and a
plain line and exit 1 past the bound), `poke:<wish id>` (of a wish of the reading, it chooses the chip of its
project too, as the window's tap does), `tap:<note title or id>:<option>` (only where the page
places it), `say:<note title or id>:<words>` (Enter in that note's field, only where the page places it; the run
it leaves is asked for) and `picture:<name>`. After each it prints one line: the step, its milliseconds, the titles of the notes
and each wish's state; a typed step is counted, never shown. `app/tests/walk.rs` walks a whole loop with the made-up
agent, with no tap on the wish: the look shows its work. A second walk there taps the wish as well (`until:done:30
poke:m1`). A leading `--logs <dir>` is the app's own option: the day is read from that folder's logs, so a walk with
a real harness in a made-up home need not read the person's. It does not prove the window's own wiring: the timer, the picker, keys and focus, the kit's keeping of an
entry with words in it, or the window's other thread.

## A tap that stops a run (`app/src/stopping.rs`)

The rules are core's ([run.md](run.md#a-stop-the-person-asks-for)) and are proved at `jellymoon run stop`. The app
shows the choice and carries its tap to core.

- **The choice.** A tap on the character of a made wish whose run is going puts one note first on the page, with
  one option, "Stop it". Core says which wishes those are (`jellymoon_core::going`: working, and naming a run,
  once a run nothing holds has been given its end), off the window's thread. A tap on any other character, or on
  a wish with no run going, shows nothing and writes nothing. The note's id is `stop-<wish id>`. A look keeps
  the choice while its wish is drawn as working, and takes it away when it is not.
- **`layout`** has no tap, so it prints the choice of every made wish whose run is going, first among the notes,
  and none for any other wish.
- **The tap on "Stop it".** The choice gives way at once to a plain note, "That run is being stopped.". Then
  `jellymoon_core::stop_run` is called off the window's thread: it waits for the run to end, up to 65 seconds.
  The page is drawn again when it answers, with one first note in the first note's place, which also takes
  the place of what the stops before it said: a page holds one such note, the newest. "That run was
  stopped.": the wish needs the person, has no star, and the note's body is core's `why` (the copy is kept).
  "That run had ended, so nothing was stopped.": the run ended by itself meanwhile; nothing was written, and
  the body is how core says it ended. "That stop ended no run.": core refused (no run going, or the run has
  not ended yet and the page shows its end when it has), with core's line.
- **A tap on a choice that is gone** (a second tap on "Stop it", or one that comes after a look took the
  choice away) does nothing and adds no note (`OnShow::chose`).
- **A reading that ends** keeps the choice while its wish is working, and what a stop said. An answer that
  comes when another scene is on show puts only its note on that scene.
- **No word of a stop's note** is the agent's, a log's or the wish's: the titles are the app's and the bodies
  are core's fixed lines, with the run's id in them.

`OnShow::poked`, `chose`, `stopping` and `stopped` need no window; `app/tests/f43.rs` follows them with the
made-up agent of the run tests, and `f43/notes.rs` the notes. Not tested: the tap itself, on a real window.

## A wish opened, and its work shown, kept or tossed (`app/src/working.rs`)

The rules and every word are core's ([show.md](show.md#work-that-waits-jellymoon-run-waiting)). The app carries a
tap to core and puts core's answer in notes. Each note is the kit's text note (`Note::Text`: a title, texts each
from a line of its own, options), whose body is drawn as text and as nothing else; the kit draws the first texts
that fit and counts the rest in a last line, `+N more`.

- **A tap on a little one** whose run is not going opens its wish (`OnShow::poked`, off the window's thread). Work
  that waits: one note, `work-<wish>`, "Your folder is not changed yet.", which says first, in one line, that the change is
  in a copy and what `Keep it` and `Toss it` do (only `Toss it` where a page has no keep), so a result's note over
  it keeps its room (F86); then the wish's own words, the files as a person counts them ("1 file
  changed", "2 new files, 1 changed"), and the choices `Show me`, `Keep it`, `Toss it`. No run's id is
  on it, and no file's name before `Show me`: core's line of names is on the note once a file was shown, and not
  when a look, a tap on the little one or `layout` puts the note on the page ("The result in plain words", below). Work that could not be read: "This change could not be read.", what the person can do next, the wish's
  words and core's reason, and no choice. Any other made wish: one note, `wish-<wish>`, with how
  it stands and its own words, and no choice. Of a made wish whose run did not finish (its record says it
  failed), "This wish needs you." and its words are followed by two fixed sentences of core's (`stands`): "It
  did not finish, and nothing in your folder was changed." (true of every failed run, whether it stopped early
  or had finished when its work could not be committed or its end recorded) and "You can type the wish again, or
  type it with what should be different." No run's id and no word of core's reason are on it. Either note can have a field under it ("A question answered, and
  more like this", below). One wish is open at a time. A tap on no wish, or on a wish whose
  note is on show, shows nothing. Nothing is written by any of it.
- **A wish of the reading** (open a wish). Jellymoon ran nothing for it: it is the model's reading of what was
  typed in Claude Code, and it is read-only, with no choice and no field. Its note, `wish-<wish>`, never says a
  run's state; its title is the reading's take, by state: "This looks unfinished.", "This looks done.", "This
  looks like it needs you.", "This looks like you let it go." (the app's words, `scene::read_said`; core's
  `stands` is for made wishes only). Under the person's quoted words: "You typed this in Claude Code, in
  <project>. Jellymoon did not run it." (with no project, "You typed this in Claude Code. Jellymoon did not run
  it."), and, only when the entry offers that project a chip, "To go on with it here, type what you want next.
  It will start in <project>." The tap also chooses that chip, as a click on the chip does (`Showing::chip_of`,
  `window::poked`): what is typed stays, nothing starts, and the hold after a choice is not changed. A tap on a
  made wish, or on a wish whose project has no chip, chooses nothing (`app/tests/open_a_wish.rs`). The page's
  legend ends, on a page with a wish, in one line with no mark: "tap a jelly to open its wish".
- **Work that waits shows itself** (`Showing::look`, `app/src/showing.rs`). A look that finds a wish's run ended
  puts the wish's work on the page with no tap: the notes a tap on its character gives (`told-<wish>` when there
  is one, and `work-<wish>` with its choices), from the calls to core a tap makes, off the window's thread, in
  the same look that draws the star. A run that finished and changed nothing, and said something that passed,
  shows itself the same way, by the same rules: `told-<wish>` over the wish's own note (told alone, below),
  but never in place of work that waits: the look asks on past it, shows the first wish's work that waits if
  any, and only else the first told-alone note; a wish passed over so keeps its turn. A told-alone note put on
  the page with no work under it holds the three looks after it (`HOLD`), as a choice's answer does, since it
  has nothing to choose and is never in hand. The rules:
  - *Whose work.* A made wish this window saw being worked on or saw made, whose run is not going now. Core is
    asked of it once for that end: a wish with no work waiting is not asked of again until it is worked on
    again, and work that was shown is not shown by itself again. A look whose scene was dropped is made again.
    A page with nothing new is looked at as before: the notes and characters on show are gone through, and no
    call is added.
  - *Work that waited when the window opened.* The first look that is free to shows the newest done wish's
    work that waits, asking the done wishes from the newest, once a window. It asks on past the work it shows,
    and every older wish whose work waits has its turn after it, as a run that ended under this window has: so
    a keep or a toss is followed by the next change that waits, past the looks held back, and nobody is left
    to believe a change is in the folder that is not.
  - *Never over a note in the person's hand.* While the scene on show, placed or not, holds a choice with an
    option (a clash's, the one that turns delivery on, a stop's), a text note with an option or a field (work
    that waits, the same asked once more, a question, a follow-up), or a note core's answer comes in the place
    of (`busy-<wish>`, "That run is being stopped.", a wish's note that says it is being worked on), the look
    is the plain one. The page's own first note that says a run goes ("Your wish is being worked on.") has no
    option and no field and is none of these: work still shows itself under it. The work waits its turn, or
    never comes when the person taps its character first. So a look moves no option and no field, and takes no
    typed word; the characters stand where they stood, since the page had notes before. A page with a clash's
    choice on it therefore opens no work by itself: a tap does.
  - *Not right after the page left the hand.* Every look made while a note is in hand holds the three looks
    after it (`HOLD`), however the page then leaves the hand: by a tap, or by a look that takes a stop's choice
    away when the run ended by itself, or a clash's that a reading or a ruling made elsewhere took away. Work
    comes no sooner than the fourth look after, never where `Stop it` stood a second before.
  - *Not right after the person chose.* After a tap on an option of any note (keep, toss, show me, the second
    yes, a stop, a clash's or delivery's choice), an Enter (a question's answer among them), and core's answer
    to either, the next three looks (`HOLD`) show no work, so about three seconds pass with the answer on the
    page. Without it, another wish's `Keep it` and `Toss it` came, within a second, where the tapped ones stood,
    under a slow second tap. The looks are counted again from core's answer, so a slow keep is held as long as a
    quick one. A turn held back is kept. A tap on a character is the person's own act and opens its work at
    once. A look in flight when the person chooses is dropped whole, its scene being gone, and made again after
    the hold.
  - *Only wishes the page had.* A look puts on the page the work of a wish that was a made wish of the scene it
    was made of. A wish made and ended elsewhere since has its turn at the next look, when its character is
    drawn, so no look adds a first note and a new character at once.
  - *What a look reads.* For each end of a run, once, a look now asks core whether work waits, and core reads
    the run's record and then the repo through its git reads (the branch, the run's commit, what it changed):
    the reads a tap on the character makes, where before this only a tap made them. Nothing is written, and a
    look with no new end reads no repo.
  - *Several ends at once.* One is shown, the first in core's order; the others wait behind its choices.
  - *What stays.* The look's notes go before the others. What was said of work kept or tossed (`said-<wish>`)
    stays under them, where a tap on a character takes it away: a run's end does not take away the answer to
    the person's own tap. The hold above is what makes that safe: the answer has stood alone for three looks.
    Every other note of an opened wish gives way, as to a tap.

  `app/tests/work_shows.rs` holds these with the made-up window's `look` act, and `app/src/showing/tests.rs`
  the turns alone. Not proved: the kit's keeping of the entry's words and keys under a new scene, which is the
  window's (`set_scene`, as every look).
- **What the run says it did** (F86, ADR 0023, `app/src/told.rs`): when core prints a `result` with a piece
  that passed, the work's note has one more note before it, `told-<wish>`, the kit's `Note::Told`. Its title is
  "What it says it did", which is also the label of what the run did: that text is the first thing under it.
  Then, in this order, once `Show me` was tapped ("The result in plain words", below), each change as one block drawn whole or not at all, `Changed in <name>` (the name on the
  label's line only when the two are one line and the name holds no blank; else the name is the block's first
  quote) over its lines as they were and as they are, set close, with the small words "before" and "after" beside the first of each side
  (of a made file `New file <name>`), then `Decided` and `Not sure about`, each over the run's texts,
  every one a quote piece and nothing else; `<name>` is core's cleaned name. It ends with "Nothing was run to
  try this. A run reads and edits files, and no more.", and, when core dropped a piece, "Something the run said
  could not be checked, so it is not here." No result, or every piece dropped: no such note. It has no option
  and no field, so a tap on it finds nothing. The kit plans the work's note first, as it is without a result,
  and the result next, taking room from the notes after them: the work's note is drawn whole, lower, with its
  three choices, and the rows are drawn in their order up to the first that does not fit, which with every
  row after it is counted. While it is on the page the count of the notes left out is one plain line, not a
  sheet. With several works waiting, a result's note takes room the later notes give way to, as `layout` prints
  it; never room from its own work's note, which shows what it shows with no result, or the result is left out. At 1440x900 a typical result (a did of two lines, one change of a line a side, a decision) reads its
  did and, after `Show me`, its whole change; a did too long for the room draws no row: the title, the count and the sentences. It is shown when the wish is opened, with `Show me`'s file, and by `layout`, and goes with the
  work's note when the work is kept, tossed or asked about once more. No file of the work is opened or run.
- **The result in plain words** (`app/src/told.rs`, `app/src/working.rs`). A person who asked for something
  follows it without reading a file's name or a changed line, so neither is on the notes as they first come onto
  the page: by a look that found the run ended, by a tap on the little one, and at `layout`. The result's note
  then holds what the run did, `Decided` and `Not sure about`, and in the place of every change's row one
  sentence of the app's, before the others: "Some of its changed lines were checked against the copy, and they match." It starts the paragraph that
  nothing was run and adds none of its own, so what the run did is still read at 960x640 (as a paragraph of its
  own it took that row's room there). It is
  said when at least one change passed core's check, which proves that each line it quotes is a whole line the
  work took from its file or put in it (`crates/jellymoon-core/src/run/told.rs`, `change`). It says "some" and
  never every line: core looks at a result's first two changes and no further, a later one is neither drawn nor
  counted, and a change core dropped is said by the sentence that something could not be checked, as before. The
  work's note has core's count and not core's line of names: core is asked for its lines of the same work with
  every name among the files it only counts. A tap on `Show me` brings the rows and the names
  (`working::notes_of`, `WorkDone::Shown`), and no file with them: a page has room for two of a wish's notes,
  and with the result and the work on it a file's note was never drawn, so a second tap changed nothing a person
  saw. Each tap after the first brings a file, under the work's choices, and the result goes after them, where
  the kit gives it the room they leave (`told::in_turn`); a result with no row of a change has nothing to show
  first, so its first tap brings the file. Which tap it is, the scene says: a file's note, or a change's
  row. Each row is drawn as it was before this, and the rows of the
  changes come before what the run did, decided and is not sure about: the kit leaves out the rows that do not
  fit from the end, and the changed lines are what the tap asked for. A `Show me` that can show no file (the work only removed files, or none of them is text that can
  be shown) brings them too, beside core's sentence of why no file was shown: no other tap names the files. They
  stay while the file is on
  show, and a tap on the little one shows the first view again, as it always closed the file. Nothing is kept
  to know this: the notes on the scene are the ones the tap's answer made. How a text of the run is checked,
  cleaned and quoted is unchanged. `app/tests/result_in_plain_words.rs`; the rows after `Show me` at
  1440x900, `app/tests/f86.rs`; the names after a `Show me` that showed no file, `app/tests/f47/named.rs`.
- **Told alone: a run that changed nothing** (`jellymoon_core::ended_alone`, [show.md](show.md)). A made wish
  whose run finished and changed nothing has no work's note, so what its run says it did is `told-<wish>` over
  the wish's own note ("This wish is done." and its words), above it and not in its place: the person reads
  what they asked, and what the run said of it. The note is the one above, and its sentences start with "Nothing
  in your folder was changed." (the app's). Core has no file to check a `change` against, so a `change` is
  dropped and counted, and the note then ends with the sentence that something could not be checked. There is
  no `work-<wish>` and no choice. When core lets a follow-up be made of the wish (`jellymoon_core::followable`
  with no work), the wish's note has the field of `More like this` under no offer: the hint "Type what to do
  next, then Enter", the key `more-<wish>`, and Enter there is a follow-up as under work ("A follow-up", below);
  the run goes on in the same conversation where the harness still keeps it (`run.md`, "A session the harness keeps no more"), and the follow-up's wish is drawn working. With a field the note is
  in the person's hand, as every note with one is (`working::in_hand`): while it is on show no look puts another
  wish's work over it. A tap on the
  character shows both notes again; a look shows them once, as work that waits; `layout` does not print them.
  A run that said nothing that passed, or did not finish, has no such note: core keeps no result of a run that
  failed. `app/tests/told_alone.rs`, and the field `app/tests/go_on_from_an_answer.rs`.
- **`layout`** has no tap, so it prints the note of every made wish whose work waits, first among the notes,
  and then the note of every made wish whose run asks or whose work was kept.
- **A tap on a choice** (`OnShow::working`) puts `busy-<wish>` in the note's place at once, with what core says is
  being done, and leaves core's part as work the window runs off its own thread (`showing.rs`, `After::Work`).
  The window's thread opens no file for it. While that note is on show, a second tap on a choice of that wish and
  a tap on its little one do nothing: one tap of a wish is carried at a time. Which tap it is, core says
  (`WorkTap::of`) from what was drawn: the words of the option and the words of its note. A note's id tells the
  app whose wish it is and nothing else.
- **A carried call that panics** is caught where it runs (`OnShow::carrying`). `busy-<wish>` gives way to
  `said-<wish>` with core's words that it did not finish and that what was changed is not known, and the wish
  can be opened again.
- **`Show me`** adds `file-<n>-<wish>` under the work's note: one file's text, as `run show --file` prints it.
  The next tap shows the next file. The app starts no program and hands nothing to the system's opener
  (`the_shell_starts_no_program_and_hands_nothing_to_the_system_s_opener` reads `app/src` for it).
- **`Keep it` and `Toss it`** end in one note, `said-<wish>`, in core's plain words (`run/waits/plain.rs`): "The
  change was kept." and that it is in the folder now, or that the folder had it already and nothing was changed,
  and the wish stays done with its star; or "The change was thrown away." and that the folder is as it was, and
  the wish is drawn let go, with no star and no row under "came true today" (core keeps the star: its run did
  report it finished). One such note stands on a page: an answer names no wish, so it takes the place of
  what was said of another wish's work (`app/src/placing.rs`), and a look takes no answer away. A refused tap
  ("The change was not kept.") says first what the person can do next, then core's reason. The command line's
  `why` is not said, but where the keep's folder was switched while it was at work or a `.gitattributes` stayed,
  after the plain sentence.
- **Files the person's tools obey**: a page keeps one only when its name was drawn, whole, on the note the yes
  was given under. A count is not a name: the agent picks the files, so it picks which would fall behind a count.
  - The first `Keep it` changes nothing and shows `sure-<wish>`. It names every such file of the whole work,
    past the 200 files a list of the work holds too, each name a text of its own, and counts none. It offers
    `Keep it, with those files`.
  - The kit places a text note's options only under a title and a body both drawn whole
    (`layout/text_note.rs`): the title within its two lines, and every text, each to its last character. Where
    the title is cut, the names do not fit the window, or one is cut, no option is placed, and a tap
    finds only a placed option. A text note with options is planned with the room of its whole body, before the
    notes after it, so on a crowded page it is those notes that are counted.
  - Work with more than six such files has no yes on a page, and neither has work with one whose name a person
    does not read whole and as it is: a name that looks like a credential is drawn as `[redacted]`, and one
    with a letter that draws as a blank, with a joiner, or that reads like a count (`+5 more`) is drawn whole
    and is still not the name ([show.md](show.md#work-that-waits-jellymoon-run-waiting), `plain`). Core says so on
    the work's own note and on `sure-<wish>`, in one sentence of its words, and offers `Show me` and `Toss it`
    only: such work is kept from the command line, with `jellymoon run keep`.
  - The tap carries the note's texts to core, and core gives the keep the names the tap carried: under its lock
    the keep lands the work only when those are the files of it the person's tools obey, all of them and no
    other. A yes to a note that names other files is refused there, with nothing changed, but where the
    person's branch holds the run's commit already: nothing is landed then, and the run is said kept. A `Keep it` that was
    not asked once more carries no name, so
    where only the merge would bring such a file (the person renamed a file the work edits to a name their tools
    obey), it is refused, with nothing changed.
- **The repo is not named by the app.** Core finds it as the run did: the folder of the wish's project typed in
  the day's logs, with the folders switched off left out, or else the one listed under its name
  ([project.md](project.md)), and checked against the run's record. When it is not
  found the note says so in core's words and has no choice.

`app/tests/f47.rs`, `f47/asked.rs`, `f47/named.rs` and `f47/drawn.rs` follow all of it through `Showing`, in a
made-up window that is a process of its own (`tests/support/tapper.rs`), since the git core starts takes the
variables of the process it is in. That window lays the scene on show out as `layout` does, at its size, and
carries a tap only on an option that is placed, as the kit's view gives a tap only on a placed piece;
`f47/drawn.rs` asserts on those placed pieces. `f47/gone.rs` follows, in the test's own process, what asks no
repo: a choice of work that waits no more, and a carried call that panics.
Not tested: the tap itself, on a real window. Not done: a file's text is not scrolled, so of a long file the
first lines are drawn and the rest counted.

## A question answered, and more like this (`app/src/answering.rs`)

A made wish's note can have a field under it (F54, ADR 0018). The rules and every sentence but three titles and
two hints are core's ([run.md](run.md), "An answer"; [wish.md](wish.md), "A follow-up"). The app puts the field
where core says, carries what is typed to core off the window's thread, and draws what core answers.

- **A wish that asks.** Core keeps the run's question with the wish (`Wish.question`, there only while the run
  asks). The wish's note, `wish-<wish>`, has core's words for how it stands, the question under them, and a
  field with the hint "Type your answer, then Enter". The app does not look at the wish's state for it. Only
  a made wish (`jellymoon_core::made`) is drawn so: a question a kept page file gives a wish of the reading is
  not drawn, and that wish gets no field. The field is the wish's and the question's, not the note's (the
  kit's `Field.key`, `answer-<wish>-<question>`): a half-typed answer is still there when the wish is opened
  again after a tap elsewhere, and another question has a field of its own, with no words.
- **The question is a run's text.** It is the note's `quoted` words: the kit places it as one `quote` piece, on a
  ground of its own, every run in the `quoted` role, so it is told apart from Jellymoon's sentences. The kit
  cleans it and cuts it to eight lines. It is text: nothing in it is a link, an option or a field, and nothing
  of it is run or read for meaning. Core has cleaned it already and holds it to 300 characters
  ([run.md](run.md), "A run that asks").
- **An answer.** Enter in that field gives the words to `jellymoon_core::answer_run`, the call `jellymoon run
  answer` makes, off the window's thread (`OnShow::typing`). The answer is in no note and no line the app
  prints. Core's call waits until the run has ended again. From Enter until then the wish's note says what
  core says of a wish at work ("This wish is being worked on."), with no question and no field, so the note is
  true for as long as the run goes on and no key is held for it. Then the wish's note is as core says the wish
  stands: work that waits, with its choices; another question, with a field; or how it failed.
- **An answer core gives an error for.** Core's error says why and not whether the run was given the answer, so
  the app looks again: did the run wait on the note's question before the call, and does it after? A look
  that does not find the wish among the day's made wishes, which is also what a day that cannot be read
  gives, is "not known". Where the run waited and is known to wait no longer, the answer was given and what
  failed came after: the first note is "The answer was given, but what came after it failed.", with core's
  sentence, the field is emptied so the same answer is not sent twice, and the wish's note is as core says the
  wish stands. In every other case, not known among them, the answer is held not to have been given: the
  page's first note is "That answer was not taken.", with core's reason (the run asks no question, having
  ended or been answered meanwhile; the answer is over 600 characters; nothing is left of it once cleaned).
  Then the note is back with its question and its field, which keeps what was typed and takes keys again;
  it is put directly under that first note when no note of the wish is on show any more, as after a tap on
  another little one during the wait. Only where the run is known not to wait on that question any more is
  the question not put back: the wish's note is then as core says the wish stands, and when no note of the
  wish is on show any more none is put back, so a wish the person closed is not opened again, unless the wish
  asks another question by then, whose note is put there with an empty field. Where the words typed are not
  back under a note, they are not emptied either: the kit keeps them apart, undrawn, under the field's key, and
  they are on show again only if the same wish asks the same words again.
- **More like this.** Core says which made wishes a follow-up may be made of
  (`jellymoon_core::followable`, printed as `follows` by `jellymoon run waiting`: work that waits, and work
  that was kept). Under the note of such a wish, and of no other, is the offer `More like this` over a field
  with the hint "Type what to do next, then Enter". Under work that waits it stands below the three choices,
  and a tap on a choice leaves it there: the note that says what is being done carries the same field, and
  Enter under it is carried as under the work. The field is the wish's, not the note's (the kit's `Field.key`,
  `more-<wish>`), so what was typed in it is still there when the work's note is back, also after a note that
  asks once more and has no field. The app does not work it out from the wish's state.
- **A follow-up.** Enter in that field gives the words and the wish's id to `jellymoon_core::wish_after`, the
  call `jellymoon wish --after` makes. A follow-up core keeps is drawn where core keeps it, and its run is
  asked for as a typed wish's is, a process of its own ("The run", above). The field is emptied. A note with
  choices keeps it: another follow-up may be typed. A done wish's own note, with the field alone under it, goes
  once core kept the follow-up: a field on show keeps the page in the person's hand, so no look would bring the
  follow-up's work when its run ends. A tap on the wish opens the note again.
- **A follow-up core refuses** writes nothing. The page's first note is "That wish was not kept.", with the
  start of the words and core's reason: the page has its 8 made wishes, the words are not a wish, the work was
  tossed meanwhile. The field keeps what was typed.
- **Which call an Enter is.** The kit gives the note's id with the words. The field on show under that note
  says the rest: one to answer in, by its key `answer-<wish>-<question>`, is an answer, any other a follow-up. An Enter for a note
  with no field on show is nothing. A field's words go when its offer changes: what was typed as an answer is
  never there to be sent as a follow-up under the same note.
- **A page closed and shown again.** The page's view outlives its window (F57), and a field's words are the
  view's: they are there when the page is shown again. A field answered while the window is closed is answered
  all the same (`window.rs`, `answer_entry`; the kit's `answer_field_closed`), so it does not wait for ever.
- **What can be acted on.** Of all this only the `field` and `offer` pieces take a click, and a click on the
  offer only gives its field the keys. `layout` prints them, and the `quote` piece, with the note's id.

`app/tests/f54.rs`, `f54/drawn.rs`, `f54/answered.rs`, `f54/more.rs` and `f54/window.rs` follow it: `drawn.rs`
at `jellymoon-app layout`, `window.rs` at the page's window on GPUI's test platform (Enter in a field, and an
answer that comes after the window was closed), the others in the made-up window of `tests/support/tapper.rs`
(`type:<note>:<words>`, or `enter:<note>:<words>` and `answer` with acts between them; `start`), which gives an
Enter only from a field `layout` places. `crates/jellymoon/tests/f54.rs` proves `follows` at the binary.

Not done, and not tested at a real window:
- An answered run lives in the app. Core has one call to answer, and it waits for the run's end. Closing the
  page's window ends nothing: the app lives on and so does the run. Quit meanwhile cuts the run off, and the
  next look ends it as any run that was cut off.
- A field takes at most 400 characters, the kit's cap on a line, where core takes an answer of 600.
- One wish is open at a time in the window, so of two wishes that ask one is on show; `layout` prints both.
- A note left on show while its wish changed elsewhere keeps its field until the wish is opened again. An Enter
  there is refused by core, in its words. One case is not refused: a note left on show with one question while
  the run, answered elsewhere, now asks another gives its words to the new question.
- Whether an answer was given is told by the question's words. A run that asks the very same words again
  after an answer that then failed reads as not given: "That answer was not taken.", and the words stay.
- Not known may be an answer the run did get. The person is then told "That answer was not taken." of an
  answer that was given. Where the run ended, a second Enter is refused by core in its own words; where it
  asks again, the limits above hold. The note stays until the next tap or Enter.
- "Directly under" what core said holds when the asking note is put, not after. When a second answer's
  refusal takes the first one's place, the asking note stands under a refusal of another answer, or under none
  when that answer was taken; a run that could not be started puts its note between the two. A refusal's body
  names no wish.
- Two cases only core can tell apart read as given, with the field emptied, though no harness got the answer:
  another answer (from a terminal) took the run between the app's first look and core's own lock, and core
  cleared the wish's question and then failed to write the run's record. Until core says whether it started
  a harness, the app cannot know.
- An answer started apart from the app's process, as `wish --run` starts a run, so that Quit would not cut the
  run off: open (ADR 0018).

## The jellyfish (`app/src/jelly.rs`)

`layout --jelly` prints what the jellyfish draws for one line of `jellymoon hear` ([hear.md](hear.md)), and where
its two windows stand on a screen of the size given. It opens no window, holds no key and hears nothing (ADR 0019,
"Found while building F56").

```sh
jellymoon hear ... | head -1 | jellymoon-app layout --jelly - --screen 1440x900
```

Core decides everything: `Jelly::read` says whether the input is a line, `jellyfish` which state is drawn when a day
is read too, and `Jelly::drawn` what the bubble holds. `jelly.rs` turns that into the kit's edge scene, one name
for one name, and prints what the kit placed.

| State | `mood` | The bubble | `ring` |
|---|---|---|---|
| `quiet` | `floats` | none | |
| `listening` | `leans_in` | the words so far (`row_main`); none until there are words | |
| `heard` | `holds_up` | the words (`row_main`), the project (`row_sub`), how to take it back (`body`) | `left` of 4000 |
| `working` | `swims` | none; a `count` piece on the jellyfish when more than one run goes | |
| `asks` | `waves` | core's sentence that a run asks (`body`), then the question (`quoted`) | |
| `waits` | `glows` | none | |
| `cannot` | `droops` | core's sentence for what was refused (`body`) | |

**The lines.** One JSON object a line. A `window` line has `id` (`jelly` or `bubble`), where the window stands on
the screen, and `opens`: what the window is asked for when the app opens it (next section). The lines after it are that window's pieces, with the same `id`, placed from the window's own top left:
`character` (with `state` and `mood`) and `count` in `jelly`; `bubble` and `ring` (with `ring`, from 1, full, to 0)
in `bubble`. `text` is a list of runs as in the page's output. With no bubble, the `bubble` window is not printed.

```json
{"kind":"window","id":"jelly","x":1328.0,"y":788.0,"w":96.0,"h":96.0,"opens":{"focus":false,"see_through":true,"moves":false,"resizes":false,"tap":"page"}}
{"kind":"character","id":"jelly","x":8.0,"y":8.0,"w":80.0,"h":80.0,"state":"quiet","mood":"floats","text":[]}
```

**Where the windows stand.** The jellyfish's window is 96 by 96, 16 pixels from the right and the lower edge of the
screen. The bubble's is 300 wide, one height for a theme whatever is said, 8 pixels left of the jellyfish with their
lower edges level; on a screen too narrow for both it stands over the jellyfish. On a screen that holds the two
apart only with less than 16 pixels at the edge, the jellyfish's margin gives way first, at its right and its
lower edge alike though one of them would do, so the bubble is never laid over the jellyfish while there is room
for both (`place_pair`). A window larger than the
screen keeps its size and starts at the screen's top left. Here the room is the whole screen: the open app gives
the same function the room a Dock and a menu bar leave. A theme's text sizes are held to 9 to 22 pixels in the
edge scene, so no theme file makes a window that a text leaves.

**Text.** A person's words take at most 3 lines, a project's name 1, a sentence of Jellymoon's 4 and a question 3,
each cut with an ellipsis between two characters a reader sees. A question is a run's own words: it has the role
`quoted` and stands apart from Jellymoon's sentence. Markup, a link's shape, an escape or a control character in
any of them changes the letters drawn and nothing else.

**`--date <YYYY-MM-DD>`** also reads that day's made wishes in Jellymoon's home, through core, and writes nothing.
A line in `listening`, `heard` or `cannot` is still drawn as it is; any other gives way to what the day gives now.
With `--date` an input of nothing is no hearing at all, and the day alone is drawn. Without `--date` nothing but the
input is read.

**Refused.** An input that is not one line of `jellymoon hear` exits 1 with
`jellymoon-app: the input is not one line of `jellymoon hear`` and prints nothing: so does a heard wish whose
words or project hold nothing a person sees, and a line with a carriage return or a form feed inside it (a line
may end in `\r\n`). A `--jelly` file that cannot be read exits 1 with `jellymoon-app: the file cannot be read:
<name>`, as above. A `--screen` that is not `<W>x<H>` with each side a whole number from 1 to 16384 in digits only,
`--jelly` without `layout` or without `--screen`, and either with a page's flags (`--page`, `--waiting`, `--size`,
`--logs`, `--skip`, `--model`, `--fresh`) exit 2 with the usage.

Tests: `app/tests/f56.rs` with `f56/jelly.rs`, `words.rs` and `day.rs`, and `app/tests/f57.rs`; the kit's half is
`crates/sticker-kit/tests/edge.rs`. The smoke chain's `jelly` step draws the line its `hear` step printed.

## The jellyfish's two windows (`app/src/popup.rs`, `jellyfish.rs`, `shell.rs`)

The open app shows the jellyfish in a window of its own, and its bubble in a second (ADR 0019, decision 4 and
"Found while building F57").

**What each window is asked for** is data, `popup::JELLY` and `popup::BUBBLE`, printed as `opens` by
`layout --jelly`. `popup::options` is the one place that turns it into what GPUI is asked.

| | `focus` | `see_through` | `moves` | `resizes` | `tap` |
|---|---|---|---|---|---|
| the jellyfish | no | yes | no | no | `page` |
| the bubble | no | yes | no | no | `none` |

Both are opened as pop-ups (above other windows, on every space) without a title bar, on the primary display, at the place and the size `jelly::stands` gives, which
is the kit's `place_pair` in the room the system says the menu bar and the Dock leave (`Room::primary`). Neither is
ever asked for the focus, and neither has a key: a key pressed anywhere goes to the page's window or nowhere.

**The bubble** is opened when core's state has words a person sees, and removed when it has none. No window is
moved or resized: the bubble's place is fixed when the jellyfish is opened, and what changes inside either window
is drawn in it. A bubble that something else removed is opened again at the next words. A bubble that cannot be
opened is said in one line on stderr (`jellymoon-app: no bubble: <why>`), once for as long as it goes on failing,
and is tried again at the next look.

**A tap on the jellyfish shows the page and does nothing else**: the page's window is brought to the front, or
opened again when it was closed. A tap on the bubble does nothing. A text in the bubble is drawn as text: a run's
question keeps its own role and its own ink (the palette's `quoted`), below Jellymoon's sentence.

**Closing the page's window ends nothing.** The app and the jellyfish stay; the page is the same view in a new
window when it is shown again, with its scene, its theme and what was typed in its entry. A day goes on being read
and looked at while its window is closed, and an answer to a wish entered just before is taken. Opening the app
again (its icon in the Dock) is wired to show the page as a tap does, and whether the system ever asks is not
known: GPUI passes that on only when the system says the app has no window to be seen, and the jellyfish always
stands (the list for a person, below). **Quit** (the menu, or Cmd-Q) removes all three windows
and ends the app; so does `JELLYMOON_QUIT_AFTER`. If the jellyfish could not be opened, one line on stderr says
so, and closing the page then ends the app, since nothing is left to show it from.

**Once a second** what core keeps for the jellyfish is read again: `jellymoon_core::jellyfish` with no line, in
the day's home, which is that day's made wishes and their runs (`working` with its count, `asks` with the
question, `waits`, else `quiet`). The reading is off the window's thread, as the page's look is, and what it gives
is drawn. A look that fails draws `quiet`. A day named with `--date` is the one day looked at. With no day named
the jellyfish looks at the page's day, which is the day the app was opened on, and asks at each look which day it
is. While that is the page's day there is one reading. Past midnight there are two, and the new day is read for
one thing only: a run at work. It is drawn when the page's day has no question and no run at work of its own,
which come first in core's order (`JellyState::ORDER`: `asks`, `working`, `waits`, `quiet`). So a wish made on the page past midnight is shown
while its run goes, a run begun before midnight is still shown and still asks, and a run of the new day, started
from a terminal, is shown while it works. A day before the page's, which a clock set back or a move west gives,
is not read. The days are compared as text.

What the jellyfish does not show past midnight:

- **A question, or finished work, of the new day.** It is on that day's page, which the app shows when it is
  opened again. The page does not move on to the new day by itself: that is not built. It shows the day the app
  was opened on, and files what is made on it under that day. So the jellyfish never says that something of the
  new day asks, or waits, "on the page".
- **A run at work on the new day while that day also has a question.** Core gives one state for a day, the
  question first, and the question is not shown.
- **That a run of the new day was cut off.** Core's look for the jellyfish writes nothing and takes the made
  wish's word that its run goes. Only the page's own look, which is of the page's day, finds that nothing holds
  a run and writes it down as cut off. So a run of the new day that was killed still swims until the app is
  opened again. Core's question for this (`run::held`) is not public.
- **One count for both days.** Runs that go on both days are not counted together: the count is one day's.

With `--page` or `--waiting` there is no day: the jellyfish is quiet and nothing is looked at. The jellyfish keeps
the theme the app was opened with; the command key and T switches the page only.

Tests: `app/tests/f57/window.rs` opens all of it on GPUI's test platform, on the Mac, where there is no screen:
both windows placed, each of the seven states drawn, the bubble opened and removed with nothing moved, the page
closed and shown again by a tap and by opening again, Quit, the end by itself, a display that is not at the top
left of the displays, the room of the primary display, and a bubble removed from outside. `app/tests/f57/looks.rs`
opens it there on a day's page, with a clock of the test's own: the look once a second off the window's thread,
with work that waits read from a run's record; and past midnight a wish made by the page's own call, a run that
still asks, each of the sixteen pairs of what the page's day and the new day hold, the new day's question beside
its run at work, the count, a day before the page's, and a named day that stays. `popup.rs` holds the options
asked in a unit test, and `jellyfish.rs` that a failed open of the bubble is said once for a row of the same
failure, and again when the reason changes or after a draw with no failure, a quiet one too: no open fails on the
test platform.

**Not tested, and not seen**: nothing here was opened on a screen. Four things are for a person at one to check.

1. Whether a pop-up brings the app to the front when it is opened or tapped. The app asks to come to the front
   on a tap in `shell.rs` (`cx.activate(true)` in `show_page`).
2. Whether the system draws a shadow or a frame round a window one sees through. GPUI's window options have
   nothing for a shadow, so if one is drawn the answer is not in `popup::options`.
3. Whether the part one sees through takes clicks. What the app takes a tap in is the kit's 80 by 80 box for
   the character (`draw/edge.rs`), given a tap in `jellyfish.rs`; `tap` in `popup.rs` is only what is printed.
4. Whether a click on the app's icon in the Dock, with the page closed, shows the page. If the system counts the
   jellyfish's window as one to be seen, GPUI is never told and the click does nothing; a tap on the jellyfish
   still shows the page.

No test sees that Quit ends the process (`cx.quit()`) or that a tap brings the app to the front
(`cx.activate(true)`): the test platform does nothing for either, so with either call taken out every test stays
green, and these lines are their only guard. A test sees the windows removed and nothing opened after. The menu
and Cmd-Q are the system's and are not run by a test.

## From page data to pieces (`app/src/scene.rs`)

| Page data | Piece |
|---|---|
| a wish | a character with the wish's label, in page order; the kit places let-go ones last |
| its state | working: busy bubble; done: idle; needs you: badge; let go: dimmed, label struck |
| each star | a star on its wish's character (the kit draws at most 4) and a row in the list note "came true today"; none of a made wish that was let go, whose change was thrown away |
| a clash | a choice note `clash-<n>` with both quotes, each drawn whole; `settled` strikes the first |
| `read` | the footer: how many things were typed, in which projects, and that the reading is unedited |
| the day's made wishes | a character each, like any wish, and one whole list note, "wished here today", after the clashes: each made wish's own words after the mark of its state, drawn to the last word or counted in the note's last line, `+N more`, never cut. The footer adds "N made wishes are in your own words."; on a day not read yet that is all the footer says |
| a made wish whose run goes, on a page with no clash | the first note (after "not everything is drawn"), a plain one: "Your wish is being worked on." over "“<the start of its words>”. What it did will show here when it is done." With more, one note for all: "Your wishes are being worked on." over "“<the newest's>”, the newest, and N more. What each did will show here when it is done." The wish is named as every note of a wish names it, by its first 40 characters as core cleans them. No time is promised. It goes with the last run, and a look never carries it over as the app's own. A page with a clash has none: it is a note planned before the made wishes' list, so it would move or shrink what a person rules on, which a made wish never does (F37) |
| a made wish whose run goes, with steps (live work, `app/src/live.rs`) | its little one says on a small pill over it the stage its run is at, from the tool of its last step, in fixed words: "looking around" (`Read`, `Glob`, `Grep`), "making the change" (`Edit`, `Write`); "thinking" before the first step. On a page with the note that says it is being worked on, a told note `steps-<wish>` goes just before that note, of the newest working wish: "What it is doing", then the stage as a sentence, "Looking around your project." or "Making the change.", and "1 step so far." or "N steps so far.". It has no row. No file's name is drawn, on the pill, in the note or on an opened wish: a person who asked for something wants to know how far along it is, not which file is open. Core keeps the names (`runs/<id>/steps.json`); the page does not draw them. A pill is drawn only where the room over the little one holds one line and it has no star (stars stand there). A step is a tool use the run asked for, as its stream says, not proof that it happened. Nothing of it can be tapped, and once the run has ended none of it is drawn: the notes are the ones they were |
| no wish, and nothing set aside | one plain note: "Nothing wished for yet today." |
| more than the app draws | the first note, "not everything is drawn", with the counts (next section) |

**What the app has no piece for becomes a plain note** (`app/src/read.rs`). A wish whose `state` is a word this app
does not know, and each item of a top-level part other than `day`, `wishes`, `clashes` and `read`, is set aside and
drawn as a plain note with its label or text. Such a wish gets no character, no star and no row in the list note, so
its note counts its stars: `A wish in a state this app cannot draw yet: paused. Its 3 stars are not drawn.` Anything
else that does not fit core's shapes refuses the whole file.

## The door: a size, four lengths, and a count of what was left out

Page data is untrusted input. `app/src/read.rs` holds it to these before anything else looks at it:

| Bound | Number | Core writes at most |
|---|---|---|
| the page file, or what is piped in | 1 MiB (`MAX_PAGE_BYTES`); over it the file is refused | under 10 KiB |
| wishes | the first 24 (`MAX_WISHES`) | 8 |
| clashes | the first 8 (`MAX_CLASHES`) | 2 |
| stars of one wish | the first 12 (`MAX_STARS`) | 4 |
| items of parts the app does not know, all parts together in name order | the first 8 (`MAX_UNKNOWN`) | none |

A list over its length is cut, not refused, and what is cut is never checked against core's shapes. The page then
starts with a plain note, "not everything is drawn", whose body counts what went: `Left out of this page: 9976 wishes,
12 clashes.` One of a thing is said in the singular (`1 wish, 1 star`). So the work `layout` does is bounded by these
numbers, whatever the file holds. What is piped in is read only to the size and one byte more; the rest stays unread.

**Nothing is left out without a word.** Three places can run out of room, and each says how much it left out:

- The list note shows the rows that fit and ends with `+N more`. When it is placed it keeps at least one row.
- The notes are placed in page order: what was left out, came true today, the clashes, then the plain notes. The
  first note that does not fit and every note after it are left out together, and a last small sheet says how many:
  a `note` piece with the id `more`, its `index` the number of notes on the page, and the text `+N more of M notes`.
  No note is skipped to make room for a later one.
- A character shows at most 4 stars; every star of a wish that has a character is a row of the list note. The stars
  of a wish in an unknown state are counted in its plain note.

**The fullest page core can write is drawn whole**: 8 wishes with labels of 40 characters, 4 stars of 60 each, and
2 clashes with two quotes of 200 each, at 1440x900 and at 960x640, in both themes. Every wish is a character, both
clashes are choice notes with both quotes, the list note is there, and nothing is counted as left out. An F28 case
holds this three times: with quotes in words, in Chinese with no space, and as one unbroken word. The kit makes the
room in this order: the list gives up rows, down to its title, one row and `+N more`; then each quote is shown in two
lines, not three, ending in `…`; then the gaps between notes close by a third. The page data keeps the whole quote. Only past what core writes (a third clash, parts the app
does not know, a list over its length) does `+N more of M notes` appear: with three clashes, the list, the first one
or two of them, and `+1` or `+2 more of 4 notes`.

The kit has its own door for scenes and theme files (`crates/sticker-kit/README.md`).

## `layout` output

One JSON object a line, in the kit's order: `kind`, `id`, `index`, `x`, `y`, `w`, `h` (logical pixels, two decimals),
`tilt` (degrees), `state` on a character, and `text`, a list of runs with `role`, `text`, `size` (the font size the
run is drawn at) and their own box. Any text
that is longer than its place is cut with an ellipsis: a label to the theme's width and length, a row of the list note
to the note's width (the star's text gets up to 64% of the row, the wish's label what is left), a quote to three
lines, or to two when the notes need the room. A quote breaks at a space. A word too long for a line of its own (an
address, or a script written without spaces) carries on to the next line, broken between characters and never inside
one (the kit's README says what counts as one), so such a quote fills its lines like any other.
No text holds a control character, a line or paragraph separator, or a format character other than the two joiners
U+200C and U+200D, which are kept because emoji and several scripts are written with them. So no line of the output is
broken by one.

## Two ways to measure text

`layout` measures with the kit's `BundledMeasure`: advance widths read from the bundled font files with `ttf-parser`,
so the output is the same bytes on every system. A character no bundled font has gets a fixed width: 1 em for East
Asian scripts, 1.4 em for emoji, 0 for joiners and marks, 0.6 em otherwise. A theme font that is not bundled is
measured as Nunito.

The window measures with GPUI's shaped glyphs instead, because the gap is too wide to draw by. Measured on a Mac over
35 texts in 8 roles: in the sticker theme the bundled widths run 0.6 to 3.7 px wider on average (no kerning), 17 px
(11%) at worst on capitals that kern; in the plain theme, which draws in the system's font, they are off by up to 16%.
So `layout` says where pieces go to within a few pixels of the window, not to the pixel. A kit test holds the gap.

## Building

The window is the default cargo feature `window`; `--no-default-features` builds `layout` alone, without GPUI.
GPUI compiles its Metal shaders with Xcode's `metal`. On a Mac without it, `make` adds `runtime_shaders` by itself;
with cargo directly, pass `--features runtime_shaders`. The headless draw tests and the picture example run on the Mac:

```sh
cargo run -p jellymoon-app --example snapshot --features headless -- <page file> <folder> <name> [<width> <height> [<words> [<option> [<theme file>]]]]
```

It writes `<name>-sticker.png` and `<name>-plain.png`. The page has the entry a day's page has, with a chip for
each of the file's first six projects (a picture reads no logs) and its hint in the field. With `<words>` the field holds
them, with the caret after them, and the chip of place `<option>` (from 0; 0 when left out) is the chosen one. With `<theme file>` a third picture is
written, named after that theme, and the file's warnings are said on stderr, each as one line of text, cleaned as the app's are
(`jellymoon_app::one_line`).

`--work <waiting file>` before the page file puts the notes of the work that waits, as `jellymoon run waiting`
prints it, over the page's notes, and `--told <shown file>` gives the first work the `result` of what
`jellymoon run show` printed, so a page with work that waits and what its run says can be looked at from core's
examples (`crates/jellymoon-core/tests/fixtures/waiting.json`; `shown-told.json`, a result with a dropped piece; and
`shown-told-typical.json`, a typical one: a did, one change, a decision, nothing dropped). The notes are drawn as
they first come onto the page: no row of a change, and no file's name.

Linux needs the packages the CI job installs. The dependencies and GPUI's pin are in `docs/adr/0013-the-shell-dependencies.md`.
