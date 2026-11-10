# The app: `jellymoon-app`

The shell (`app/`) draws page data with the kit. It makes no reading call, opens no network connection and writes no
file; `jellymoon page` makes the page data (`page.md`).

```sh
jellymoon-app layout --page <file, or - for stdin> [--theme sticker|plain] [--size <W>x<H>]
jellymoon-app --page <file, or - for stdin> [--theme sticker|plain] [--size <W>x<H>]
```

The first prints where every piece goes. The second opens a window (default 1440x900); `t` switches the theme, and
`JELLYMOON_QUIT_AFTER=<seconds>` closes it by itself. A wrong command line exits 2 with the usage on stderr; a file
that cannot be read or is not page data exits 1 with one line. Both start `jellymoon-app: ` and print nothing on stdout.

## From page data to pieces (`app/src/scene.rs`)

| Page data | Piece |
|---|---|
| a wish | a character with the wish's label, in page order; the kit places let-go ones last |
| its state | working: busy bubble; done: idle; needs you: badge; let go: dimmed, label struck |
| each star | a star on its wish's character (the kit draws at most 4) and a row in the list note "came true today" |
| a clash | a choice note `clash-<n>` with both quotes; `settled` strikes the first |
| `read` | the footer: how many things were typed, in which projects, and that the reading is unedited |
| nothing at all | one plain note: "Nothing wished for yet today." |

**What the app has no piece for becomes a plain note** (`app/src/read.rs`). A wish whose `state` is a word this app
does not know, and each item of a top-level part other than `day`, `wishes`, `clashes` and `read`, is set aside and
drawn as a plain note with its label or text. Anything else that does not fit core's shapes refuses the whole file.

## `layout` output

One JSON object a line, in the kit's order: `kind`, `id`, `index`, `x`, `y`, `w`, `h` (logical pixels, two decimals),
`tilt` (degrees), `state` on a character, and `text`, a list of runs with `role`, `text` and their own box. A label
longer than the theme allows is cut with an ellipsis; the list note keeps the whole label.

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
