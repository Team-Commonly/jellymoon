# The app: `jellymoon-app`

The shell (`app/`) draws page data with the kit. Opened with nothing, it shows today's page (next section). Given a
page-data file with `--page`, it draws that and does nothing else: no reading call, no file written.

```sh
jellymoon-app [layout] [--date <YYYY-MM-DD>] [--fresh] [--logs <dir>] [--skip <names>] [--model <name>]
              [--theme <theme>] [--size <W>x<H>]
jellymoon-app [layout] --waiting [--date <YYYY-MM-DD>] [--theme <theme>] [--size <W>x<H>]
jellymoon-app [layout] --page <file, or - for stdin> [--theme <theme>] [--size <W>x<H>]
```

These are all the flags, with `--help`. `--logs`, `--skip` and `--model` are `jellymoon page`'s; `--waiting` shows
the waiting page and reads nothing; with `--page` the flags of a day are taken and do nothing.

`--theme` is `sticker` (the default), `plain`, or the path of a theme file; the two names win over a file called the
same. A theme changes how the page looks and never which pieces are on it. A theme file is TOML in which every key
is optional (the kit's README has the keys and the bounds it is held to). A key the kit does not know falls
back to the sticker theme's, and `art` is not read, whatever it holds (a theme file names no pictures, the kit's own art
is drawn, and no file is refused for what its `art` names), each said in one `jellymoon-app: theme: ` line on stderr, the first 8 of them; a line is cut at 300 characters (`LONGEST_LINE`), with an ellipsis. A theme file that cannot be read or that the kit refuses (a number out of its
range) exits 1 with one line that names the key and the reason.

With `layout` it prints where every piece goes. Without, it opens a window (default 1440x900); `t` switches between the two built-in themes, and
`JELLYMOON_QUIT_AFTER=<seconds>` closes it by itself. `--size` is from 960x640 to 16384x16384, each side a whole
number of pixels: 960x640 is the smallest size a page is drawn at, and the window is asked not to shrink below it
(a unit test holds what is asked; that the system obeys was seen by hand on the Mac only). A wrong command line exits 2 with the usage on stderr; a file
that cannot be read, is over 1 MiB or is not page data exits 1 with one line. Both start `jellymoon-app: ` and print
nothing on stdout.

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
- **What is shown first.** The kept page, when there is one. Otherwise a waiting page: the date, the hero and one
  note, "Reading your day…". The window opens on it at once, before a log is opened: reading the logs alone can take
  seconds. The rest runs off the window's thread in two steps, each of which may give the next page to show
  (`today::Next`): the count of the day's turns, then the reading call. While a kept page is read again it stays on
  show under one note, "Reading your day again…", and never gives way to the waiting page. `layout --waiting` prints
  the waiting page; `layout` alone runs the same steps and prints what the window ends on.
- **When the day is read again.** Each time the app opens it reads the session logs (no model) and compares the day's
  count of turns with `read.turns` of the kept page. The same count: the kept page stays and no call is made. Any
  other count, no kept page, or `--fresh`: one reading call, and the new page is kept. The count is the whole test,
  so a day whose turns changed but not their number (a folder switched off and one more turn typed) keeps its page
  until `--fresh`. A day with nothing typed makes no call.
- **Folders switched off.** `--skip <names>`, and `<home>/skip.txt`, one folder name a line, for an app opened with
  no command line. Core reads the file (`reader.md`), for the app and for `jellymoon day` and `page` alike, so the
  three leave out the same folders. A name matches exactly, and case matters, and a `--skip` name with a character
  that cannot be seen is an argument error: `reader.md` has the rules, and the footer names the projects that were
  read. No `skip.txt` is no names. Anything else under that name that is not a whole, plain list (`reader.md` lists
  what is refused) stops the reading with a plain note and no call, so a folder switched off never reaches the call
  because its list was damaged. A byte order mark at its start is dropped.
- **When the reading cannot run** the page is one plain note, "Today could not be read.", that says why: Claude Code
  was not found, no session logs (the note names the folder looked for, as the command's line does), the call failed
  or took too long (core's own line follows), or the model's answer was not a page. With a kept page, that page
  stays and the note is its first, "This page is from an earlier reading." A page that could not be written to its
  file is shown with a first note that says so. `layout` prints these pages and exits 0: they are what the window
  shows.

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
chip for each project of the page's reading (`read.projects`) a wish can be made for, of which one is always
chosen. It stands in the left column, under the title "what I wished for today", where the wishes are listed. The chips
stand in one row when their names fit it and in two when they do not; the hint is "Type a wish: Tab for its project,
Enter". It is whole in both themes from the smallest page (960x640) up as `layout` measures it, by the advances of
the bundled Nunito at the kit's small size (13 design pixels, never under 11), which also stands in for the plain
theme's system font (`app/tests/f42.rs`, at three sizes); and as the window shapes it on a Mac, in Nunito and in the
system's font (headless pictures at 960x640 and 1440x900). A wider font cuts it with an ellipsis at the
smallest page: one whose every character advances by 0.56 of its size, as the kit's layout tests measure, does. The rules are core's ([wish.md](wish.md), [run.md](run.md#a-run-started-apart)) and are proved at
`jellymoon wish` and `jellymoon wish --run`. The app carries the words and a project's name to core, and draws
what core keeps.

- **When there is an entry.** On a day's page whose reading names a project a wish can be made for. A day with no
  page (still being read, or not readable), a page with no such project, the waiting page and a `--page` file
  have none: a wish needs a project, and the app names none of its own. The window opens with no entry, and has
  one as soon as core has said which projects have a chip. The entry takes at most 200 characters,
  core's cap on a wish (`jellymoon_core::MADE_SAID_MAX`). `layout` prints the field as one `entry` piece, its
  hint as text, and a `chip` piece for each option, after the scene's pieces.
- **Which projects have a chip.** Core says (`jellymoon_core::wishable`): a name `jellymoon wish --project`
  takes (one folder's name of at most 100 characters, no path, nothing unseen) that is the name of a folder
  something was typed in that day, with the folders switched off left out. That is asked of the reader, with
  the lookup a run finds its repo by, so `--skip` and `skip.txt` are matched against the folder's whole path: a
  folder switched off by a name above it has no chip. This holds from the first scene, which is the kept page
  of an earlier reading, so a project of a kept page with no folder typed in today has no chip either. A
  `skip.txt` or a logs folder that cannot be read offers nothing. Whether the folder is there, and is a repo, is
  the run's to say. The first six of these are offered, which are the chips the kit draws.
- **What the chips cost.** Every session log of the day is read once more for them, off the window's thread:
  when the window opens and when a reading ends, and once more for each Enter. Measured on a Mac with a release build, as one `jellymoon day` (one read, and more than a lookup does, since it also prints the day): 300 made-up logs of 1000 lines each, 137 MB, with 10,000 turns typed on the day, took 0.9 seconds, and 2.4 the first time, before the system had the files. A day of a few logs is a few milliseconds.
- **Keys.** A click on the field gives it the keys; Escape, or a click elsewhere, gives them back. Tab chooses
  the next project and Shift-Tab the one before; a click on a chip chooses it. Enter enters the text. While an
  input method is composing, Enter confirms the composition and enters nothing. While the field has the keys, `t`
  is a letter and does not switch the theme. The window says one line on stderr for an entry, the entry's id and
  how many characters were entered, never a word of them.
- **Enter.** The words and the name on the chosen chip go to `jellymoon_core::wish_offered`, off the window's
  thread (`OnShow::wishing`), so a day that is locked, a slow disk or many logs never hold the window. The app
  passes the name, never a path and never the chip's place: the chips can change while the words are typed.
  Core keeps the wish only for a project it offers at that moment, and refuses any other name. A wish core keeps
  is drawn as soon as core has answered, in the working state, from what core keeps of it: no reading is made.
  Words core refuses write nothing: the page's first note is "That wish was not kept.", with why. Taking the
  answer on the window's thread (`OnShow::answered`) opens no file.
- **While core is asked the entry waits.** From Enter until core's answer the words stay in the field, drawn
  pale, with no caret. The entry takes no key then: no letter, no editing key, no Tab, no Escape, no click on a
  chip, no second Enter, and no input method can begin a composition. The keys stay the entry's, so `t` does
  not switch the theme. A key pressed while it waits is not accepted and
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
- **A look never takes a note away.** The plain notes the app put over the page (a reading that is going or
  failed, a tap that did nothing, a wish that was not kept or has no run, a stop) stay over what a look draws.
  A look made from a scene that is no longer on show is dropped.
- **When a note goes.** A wish's note goes with the next Enter that core answers, kept or not, and with a tap on
  a clash. A reading that ends keeps the notes of a wish and of a stop over its own page (`OnShow::show`). The
  note of an asking that ends late goes under the notes of wishes entered since, and takes none away.

`OnShow::offering`, `wishing`, `answered`, `asked`, `look` and `seen` need no window (`wished` is two of them in
one, for a caller with none); `app/tests/f42.rs` and `f42/notes.rs` follow them, and `f42/run.rs` the run, through
a made-up window that is a program of its own.

## What the window does, with no window (`app/src/showing.rs`)

`window.rs` is wiring: it opens the window, runs work off the window's thread, and hands each scene to the kit's
view. What each act does is `Showing`, the day on show behind the calls the window makes, with no GPUI in it:

| Act | The window calls | Then, with what the work gave |
|---|---|---|
| a key | `switches_theme(key, entry has the keys)`, `other_theme(name)` | |
| the first page | `offering()` | the entry is set |
| a step of the reading | `reading(step)` | `read(&next)`: the scene, and the step's entry |
| once a second | `look()` | `seen(seen)`: the scene, or none for a look of a scene that is gone |
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
`defer` that lets the view finish a tap before it takes a new scene, and the calls `set_theme` and
`entry_focused`. A person's walk at a real window is still owed for it.

Not covered: a wish for a project past the sixth, or for one nothing was typed in today, is made with
`jellymoon wish`. What only a person at a window can see is not tested: the input method's candidate window,
the caret, the pale words of an entry that waits, and which piece has the keys.

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

## From page data to pieces (`app/src/scene.rs`)

| Page data | Piece |
|---|---|
| a wish | a character with the wish's label, in page order; the kit places let-go ones last |
| its state | working: busy bubble; done: idle; needs you: badge; let go: dimmed, label struck |
| each star | a star on its wish's character (the kit draws at most 4) and a row in the list note "came true today" |
| a clash | a choice note `clash-<n>` with both quotes, each drawn whole; `settled` strikes the first |
| `read` | the footer: how many things were typed, in which projects, and that the reading is unedited |
| the day's made wishes | a character each, like any wish, and one whole list note, "wished here today", after the clashes: each made wish's own words after the mark of its state, drawn to the last word or counted in the note's last line, `+N more`, never cut. The footer adds "N made wishes are in your own words."; on a day not read yet that is all the footer says |
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

Linux needs the packages the CI job installs. The dependencies and GPUI's pin are in `docs/adr/0013-the-shell-dependencies.md`.
