# The app: `jellymoon-app`

The shell (`app/`) draws page data with the kit. It makes no reading call, opens no network connection and writes no
file; `jellymoon page` makes the page data (`page.md`).

```sh
jellymoon-app layout --page <file, or - for stdin> [--theme sticker|plain] [--size <W>x<H>]
jellymoon-app --page <file, or - for stdin> [--theme sticker|plain] [--size <W>x<H>]
```

The first prints where every piece goes. The second opens a window (default 1440x900); `t` switches the theme, and
`JELLYMOON_QUIT_AFTER=<seconds>` closes it by itself. A wrong command line exits 2 with the usage on stderr; a file
that cannot be read, is over 1 MiB or is not page data exits 1 with one line. Both start `jellymoon-app: ` and print
nothing on stdout.

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
drawn as a plain note with its label or text. Anything else that does not fit core's shapes refuses the whole file.

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
12 clashes.` So the work `layout` does is bounded by these numbers, whatever the file holds.

**Nothing is left out without a word.** Three places can run out of room, and each says how much it left out:

- The list note shows the rows that fit and ends with `+N more`. When it is placed it keeps at least one row.
- The notes are placed in page order: what was left out, came true today, the clashes, then the plain notes. The
  first note that does not fit and every note after it are left out together, and a last small sheet says how many:
  a `note` piece with the id `more`, its `index` the number of notes on the page, and the text `+N more of M notes`.
  No note is skipped to make room for a later one. Measured at 1440x900 and at 960x640, in both themes: the list
  and two clashes with one-line quotes fit; with a third the page shows the list, the first clash and
  `+2 more of 4 notes`; and two clashes with the longest quotes core writes (200 characters, three lines each) do
  not both fit, so the page shows the list, the first clash and `+1 more of 3 notes`.
- A character shows at most 4 stars; every star is a row of the list note.

The kit has its own door for scenes and theme files (`crates/sticker-kit/README.md`).

## `layout` output

One JSON object a line, in the kit's order: `kind`, `id`, `index`, `x`, `y`, `w`, `h` (logical pixels, two decimals),
`tilt` (degrees), `state` on a character, and `text`, a list of runs with `role`, `text` and their own box. Any text
that is longer than its place is cut with an ellipsis: a label to the theme's width and length, a row of the list note
to the note's width (the star's text gets up to 64% of the row, the wish's label what is left), a quote to three lines.
No text holds a control or format character, so no line of the output is broken by one.

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
