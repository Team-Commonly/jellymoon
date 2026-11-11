# The app: `jellymoon-app`

The shell (`app/`) draws page data with the kit. Opened with nothing, it shows today's page (next section). Given a
page-data file with `--page`, it draws that and does nothing else: no reading call, no file written.

```sh
jellymoon-app [layout] [--date <YYYY-MM-DD>] [--fresh] [--logs <dir>] [--skip <names>] [--model <name>]
              [--theme <theme>] [--size <W>x<H>]
jellymoon-app [layout] --waiting [--date <YYYY-MM-DD>] [--theme <theme>] [--size <W>x<H>]
jellymoon-app [layout] --page <file, or - for stdin> [--theme <theme>] [--size <W>x<H>]
jellymoon-app tap [--date <YYYY-MM-DD>] --clash <n> --option <1|2>
jellymoon-app tap --delivery-on
```

These are all the flags, with `--help`. `--logs`, `--skip` and `--model` are `jellymoon page`'s; `--waiting` shows
the waiting page and reads nothing; with `--page` the flags of a day are taken and do nothing.

`--theme` is `sticker` (the default), `plain`, or the path of a theme file; the two names win over a file called the
same. A theme changes how the page looks and never which pieces are on it. A theme file is TOML in which every key
is optional (the kit's README has the keys and the bounds it is held to). A key the kit does not know and an art file
that is not there fall back to the sticker theme's, each said in one `jellymoon-app: theme: ` line on stderr, the
first 8 of them. A theme file that cannot be read or that the kit refuses (a number out of its range, art that is
more than a flat picture, a path out of the theme's folder) exits 1 with one line that names the key and the reason.

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

- **Where pages are kept.** `<home>/pages/<YYYY-MM-DD>.json`, where `<home>` is `JELLYMOON_HOME` or `~/.jellymoon`.
  The file is the checked page data, one line as `jellymoon page` prints it, and nothing else, in a folder for this
  user only (0700; the file 0600). It is written beside its place, in a file named for the process, and moved there.
  It is read back through the same door as any page file, so a kept file that is not page data, is over 1 MiB or is
  of another day is not drawn: the day is read again.
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

The window does not look again while it is open, and closing it while a call runs leaves that call to end by itself.
A tap in the window is one line on stderr, the piece's id and the option's number, never a word of the page.

## Taps (`app/src/taps.rs`)

A tap on an option of a day's page does one of two things, both by calling core, never by running `jellymoon`.
The window's handler and the `tap` command both call `Today::tapped`; the command reads the day's kept page first
and prints one line of JSON. A page drawn from `--page` has no day, and a tap on it does nothing.

- **A clash's option makes a ruling.** The option tapped stands and the other is what it replaces. Both statements
  are the clash's `quote_a` and `quote_b` in the page data on show, handed to `jellymoon_core::rule` as they are:
  core makes each one line, replaces credential shapes and cuts it ([delivery.md](delivery.md)). The app adds no
  word of its own, and one tap makes at most one ruling. `tap` prints `{"ruling":{"id","stands","over"}}`.
- **The page shows it settled at once.** The note's title becomes "you chose". The statement that stands is drawn
  second and the one it replaced first, struck in the theme's own way: the kit strikes the first option of a choice
  and no other, so the order of the two is how the app says which stands (`scene::order`). No reading is made.
- **It stays settled.** Which side stands is kept beside the day's page, in `<home>/pages/<YYYY-MM-DD>.ruled.json`:
  the day, and for each clash ruled on its two statements and `"stands": "a"` or `"b"`. It is written like the kept
  page (a file of the process's own, 0600, moved into place) and read back through a door: at most 64 KiB, exactly
  this shape, this day, at most 8 clashes; anything else is no rulings. A clash is known by its two statements, so
  a later reading that words it differently shows that clash open again. The file is never sent anywhere.
- **The other way.** A tap on the struck statement makes a new ruling with the two the other way round, and the page
  turns over. A tap on the statement that already stands by the person's own ruling makes none: `{"ruling":null}`.
  A clash the reading found settled has no ruling of the person's yet, so a tap on either side makes one.
- **Refusals write nothing.** No kept page for the day, a clash that is not on it, an option that is not 1 or 2:
  one line on stderr, exit 1, no ruling, no file. In the window the page gets a first note, "That tap did nothing."
- **Delivery.** A ruling reaches a session only through the hook. A tap on a clash never touches Claude Code's
  settings. While the hook is not installed (`jellymoon_core::hook_installed`), a page with a clash carries one
  note before its clashes: "your sessions are not told yet", with one option, "Turn on delivery". That tap, or
  `tap --delivery-on`, calls `jellymoon_core::install_hook`, the function behind `jellymoon hook install`, with
  the settings file that command would take (`settings.json` under `CLAUDE_CONFIG_DIR` or `~/.claude`) and the
  `jellymoon` command where `install.sh` puts it (`JELLYMOON_BIN_DIR` or `~/.local/bin`). When no file is there the
  app cannot name a program for the hook to run, so the note is plain text that says to run `jellymoon hook install`
  in a terminal, and `tap --delivery-on` exits 1. Once the hook is in, the note is gone.

Not covered: a ruling made while delivery is off waits its 24 hours like any other and is told if the hook goes in
before then; the page does not say afterwards whether a session heard it. The note of the delivery state sits before
the clashes, so on a page too full for all its notes it is a clash that falls into "+1 more", not the note.

## From page data to pieces (`app/src/scene.rs`)

| Page data | Piece |
|---|---|
| a wish | a character with the wish's label, in page order; the kit places let-go ones last |
| its state | working: busy bubble; done: idle; needs you: badge; let go: dimmed, label struck |
| each star | a star on its wish's character (the kit draws at most 4) and a row in the list note "came true today" |
| a clash | a choice note `clash-<n>` with both quotes; `settled` strikes the first |
| `read` | the footer: how many things were typed, in which projects, and that the reading is unedited |
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
`tilt` (degrees), `state` on a character, and `text`, a list of runs with `role`, `text` and their own box. Any text
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
cargo run -p jellymoon-app --example snapshot --features headless -- <page file> <folder> <name>
```

Linux needs the packages the CI job installs. The dependencies and GPUI's pin are in `docs/adr/0013-the-shell-dependencies.md`.
