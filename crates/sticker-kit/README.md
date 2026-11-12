# sticker-kit

A small UI kit on [GPUI](https://www.gpui.rs) that draws a sticker journal
page: flat sticker characters with stars, taped notes on grid paper, springy
motion. A pure function arranges the page; a theme, which is data, styles it.

## The three data types

- `Scene` (`src/scene.rs`): what is on the page. A date, a title, a hero,
  characters, notes (`List`, `Choice`, `Plain`), a legend. Plain serde data.
- `Theme` (`src/theme/`): how it looks. Colours, font names, tilt angles,
  motion amounts, sizes, and optional SVG files that replace the drawn art.
  Two are built in: `Theme::sticker()` and `Theme::plain()`.
- `Placed` (`src/layout/`): where each piece goes. `layout(scene, theme,
  viewport, &measure)` returns them; it has no GPUI in it, so it is tested
  without a window. `Measure` is the one trait: the width of a string.
  `BundledMeasure` adds up advance widths from the bundled font files, so it
  gives the same numbers on every system; `draw::GlyphMeasure` asks GPUI, which
  kerns and knows the system's fonts. The view draws with the second.

`draw::Page` is the GPUI view; taps on characters and options reach `on_tap`.

## The entry: one line to type

A page may hold one `Entry` (`src/scene.rs`) beside its scene: an id, a hint, a
few options of which one is always chosen, and a cap on its characters. It has
a spot of its own: in the left column, under the title, from the content's
left edge to short of the big character's place, the chips under the field,
each as wide as its name: in one row when that holds them, else in two, a name
too long for its share of a row cut. Every page keeps that band free, whether or not it has an entry (a page
with no big character begins its small ones under it), so an entry that comes
or goes moves no piece. `layout_entry(entry, theme, viewport, &measure)` places
the field (`PieceKind::Entry`) and a chip for each option (`PieceKind::Chip`),
and nothing for a theme whose big character leaves the field under 160 design
pixels.

Three things about an entry are told apart by more than a shade, and each is
a key of the theme's `[palette]`: the hint is a grey (`entry_hint`, also the
words of an entry that waits), the chosen chip is filled (`chip_chosen`, its
name in `chip_chosen_ink`), and the field that has the keys has a ring, a
line darker than that fill (`entry_ring`; `entry_edge` without the keys).
`tests/entry.rs` holds both built-in themes to it.

```rust
Page::new(scene, theme)
    .with_entry(Entry { id: "wish".into(), hint: "wish for something".into(), options, most: 200 })
    .on_entry(|made, window, cx| { /* made.id, made.text, made.option */ })
```

- **Keys.** A click on the field or a chip gives the entry the keys; Escape, or
  a press anywhere else, gives them back. Text goes in at the caret. Backspace,
  Delete, Left, Right, Home and End (or Left and Right with the platform's
  command key) edit, the command key with V pastes, Tab and Shift-Tab choose the
  next and the previous option, Enter enters. There is a caret and no selection.
- **Enter** calls `on_entry` with the text, trimmed, and the chosen option's
  name (`made.option`), never its place: the options can change before the
  handler acts. Text that is only spaces enters nothing. The handler runs while
  the page is in use, as `on_tap` does: one that changes the page does it in
  `cx.defer`.
- **The entry then waits** for the handler's answer,
  `page.answer_entry(kept, window, cx)`, which every handler gives. Until then
  the words stay in the field, drawn in the hint's grey with no caret,
  and the entry takes no key, Escape included, no click on a chip, no second
  Enter and no composition (`accepts_text_input` is false). It keeps the keys. A key pressed while it waits is
  dropped, not kept for later. Kept: the entry is empty and keeps its option.
  Not kept: the words are still there. A page with no handler answers itself.
- **An input method** (Chinese, Japanese, Korean, a dead key) talks to the page
  through GPUI's `EntityInputHandler`. What it is still composing is underlined,
  and Enter, the editing keys and a click leave it alone until it ends.
- **A window's own keys.** While `page.entry_focused(window)` is true a letter
  is text: a window that acts on a bare key asks first. `page.entry()` is what
  is typed now; `set_entry` changes or removes the entry, and keeps the chosen
  option by its name when that name is still offered (`EntryState::keep_option`).
- **An entry that holds words is not removed.** `set_entry(None)` takes an
  empty entry off the page at once, and one that holds only spaces, which
  are no words: Enter gives nothing for them. One that holds words, or waits
  for its answer, stays as it is, with its options, and goes when it holds
  none: after an answer that kept the words, or when the person has deleted
  them, whatever spaces are left. Words a
  person typed never go with the entry, and an answer always finds the words
  it is for: an entry cannot go and come back empty while it waits. An entry
  set meanwhile takes its place and keeps the words.

The rules are `entry::EntryState`, plain data with no GPUI in it
(`tests/entry.rs`, `tests/entry_waits.rs`). `tests/entry_draw.rs` sends keys and clicks through GPUI's
own dispatch to a headless page. The test platform has no input method, so a
composition is started there by calling the handler, as a platform would; what
a real input method does on a real window is checked by hand.

## Run it

```sh
cargo test --no-default-features          # scene, theme, layout: no GPUI
cargo run --example page                  # a window; press t to switch theme
cargo run --example snapshot --features headless -- out/   # two PNGs, no window
cargo test --features headless            # the drawing tests, headless
```

On macOS without the full Xcode toolchain, add the `runtime_shaders` feature.
`KIT_QUIT_AFTER=6` closes the window by itself; `KIT_CHARACTERS=50` is a crowd.

## A theme file

TOML, for hand editing and comments. Every key is optional.

```toml
name = "dusk"

[palette]
desk = "#2B2440"

[tilt]
enabled = false

[art]
star = "star.svg"   # a file in the theme's own folder
```

`Theme::load(path)` returns the theme and its warnings. An unknown key is
ignored and named in a warning, so a theme for a newer kit still loads.

## The door: what the kit holds its input to

A scene and a theme file may come from anywhere, so each is held to stated
bounds. A theme outside them is **refused**, never clamped, with the key named.

| Input | Bound | Where |
|---|---|---|
| theme file | a regular file of at most 256 KiB | `theme::MAX_THEME_BYTES` |
| art file | a regular file of at most 64 KiB, inside the theme's folder (a symlink out of it counts as outside) | `theme::MAX_ART_BYTES` |
| any number in a theme | finite, and 0 to 10000 unless a line below says otherwise | `theme::RANGES` |
| `metrics.width`, `metrics.height` | 200 to 10000 | |
| `metrics.columns` | 1 to 12 | |
| `metrics.max_scale` | 0.1 to 10 | |
| `metrics.hero_size` | 0 to 800 (the sticker theme: 200) | |
| `metrics.character_size` | 0 to 400 (92) | |
| `metrics.rim` | 0 to 40 (5) | |
| `motion.damping` | 0 to 200 (14); the spring is stable all the way | |
| `labels.max_chars` | 1 to 200 | |
| every angle under `tilt` | -45 to 45 | |
| an art file's SVG | a flat picture, as the renderer's parser reads it, held to the lists below | `svg_refusal` |
| notes in a scene | the first 12 are laid out, the rest only counted | `MAX_NOTES` |
| options of a choice note | the first 4 | |
| rows of a whole list | the first 12 are laid out, the rest only counted | |
| any text | 400 characters, a label `labels.max_chars`; cut with an ellipsis | `layout::MAX_TEXT_CHARS` |
| pieces a layout returns | 5000; what would come after is dropped | `MAX_PIECES` |
| options of an entry | the first 6 are offered | `entry::MAX_ENTRY_OPTIONS` |
| text in an entry | one line of the entry's `most` characters, 400 at most (and for a `most` of 0); a line break or a tab is a space, a control or unseen character is dropped; what is over is not taken, typed or pasted | `entry::MAX_ENTRY_CHARS` |

`Theme::load("theme.toml")`, a bare name, reads from the folder the program
runs in. **A theme built in code gets the same range check** (`Theme::check`,
the one function `load` uses): `layout` runs it first and places nothing for
a theme it refuses, and the view draws only what is placed. Behind that,
`layout` returns at most `MAX_PIECES` pieces and punches at most 150 holes,
and the view draws at most 1000 grid lines each way and 64 stripes on a tape.

**An art file is read with the renderer's own parser, and its tree is judged by an allow-list**
(`src/theme/svg_check.rs`, `docs/adr/0015-the-art-door-reads-with-the-renderers-parser.md`), so what a file costs
to draw is bounded at the door and not left to the renderer. The parser is `usvg`, at the version GPUI draws
with; the tree it makes is the tree that is drawn. A size, a shape, a stroke's width and where a transform puts a
thing are what that tree says, so the door has no reading of its own for the renderer to differ from. A file with
anything not named here refuses the whole theme, with the key and the reason. It is read through the door again
when it is drawn, and a file that has gone bad since is left out for the kit's own art.

Three things are held, in this order:

1. **The bytes.** A regular file of at most 64 KiB, UTF-8.
2. **The XML document**, before a picture is made of it. This bounds what the parser is asked to build, and keeps
   out the two things it would load.
   - It is XML with no DTD: no DOCTYPE and no entity. A comment or a processing instruction is passed over, as
     the renderer passes over it, and a character reference is read by the one parser.
   - Elements, the 15 in `theme::SVG_ELEMENTS`, in SVG's namespace and written as SVG writes them: `svg` (one),
     `g`, `defs`, `title`, `desc`, `path`, `rect`, `circle`, `ellipse`, `line`, `polyline`, `polygon`,
     `linearGradient`, `radialGradient`, `stop`. So no `style`, `script`, `use`, `symbol`, `filter` or its
     primitives, `pattern`, `mask`, `clipPath`, `image`, `foreignObject`, `text`.
   - Attributes, the 32 in `theme::SVG_ATTRIBUTES`: `id`, `version`, geometry (`x` `y` `x1` `y1` `x2` `y2` `cx`
     `cy` `r` `rx` `ry` `d` `points` `width` `height` `viewBox`), paint (`fill` `stroke` `stop-color` `offset`
     `fill-rule` `fill-opacity` `stroke-width` `stroke-opacity` `stop-opacity` `stroke-linecap` `stroke-linejoin`
     `gradientUnits`), and `transform`, `gradientTransform`. So no `style`, `href`, `filter`, `opacity` on a
     group, `on…` handler, `xml:base`, dashes or mitre limit. A name in another case (`viewbox`) is another name,
     and is refused.
   - At most 100 elements (`MAX_ART_ELEMENTS`), each at most 12 deep (`MAX_ART_DEPTH`), the `<svg` counted in both.
   - The `d` and `points` values hold at most 16 KiB together (`MAX_ART_PATH_BYTES`).
   - What the paths and shapes write, walked with the parser library's own tokenizer before the parser makes a
     picture of it (`theme/svg_text.rs`). The parser does three things by the length of a value, and one of them
     ends the program:
     - at most 2048 commands in the paths of a file, a pair of `points` being one (`MAX_ART_COMMANDS`);
     - at most 32 closes or arcs in a row in a path with nothing else between them (`MAX_ART_IDLE`). The parser
       calls itself once for each segment it draws nothing for, which is a close after a close and an arc too flat
       to be a curve, and a path of a few thousand `z` overflowed its stack. The first close of a run and an arc
       straight after a close are drawn, and not counted. Every other arc is, flat or not;
     - no number in a `d` or in `points`, and no `x` `y` `x1` `y1` `x2` `y2` `cx` `cy` `r` `rx` `ry` `width`
       `height`, is over 1000000 (`MAX_ART_WRITTEN`). An arc, a circle and a rounded corner are made of more
       curves the larger they are written, all at once: `r="3e38"` was four million;
     - an arc's two radii are within 1000 times of each other (`MAX_ART_FLAT`): the parser makes the smaller
       as much larger as the other is, so a flat arc between far ends is a very large one.
3. **The tree.**
   - Groups and paths only. A group that is faded, clipped, masked, filtered or blended is refused, and so is a
     dashed stroke, a pattern, text and an image. The two lists keep each of these out already; the tree is judged
     all the same, since it is what is drawn.
   - The picture is at most 256 a side (`MAX_ART_SIDE`), at the size the renderer gives it: its `width` and
     `height`, else its `viewBox`'s, else as large as what is in it. A unit (`1in`) is what the renderer takes it for.
     And at least 0.5 a side (`MIN_ART_SIDE`): it is drawn at twice its size cut to whole pixels, and the renderer
     refuses a picture that comes to none.
   - Every path's box after every transform, and twice its stroke's drawn width past it, is within 4096 of the
     picture's edges (`MAX_ART_REACH`), in the picture's own pixels. Twice the width is as far as a mitre's point
     goes. The renderer's own box of a stroke is not used: under a slant it is the path's box and no more.
   - A stroke is drawn at least 0.51 wide (`MIN_ART_STROKE`), in the picture's own pixels: just over one drawn
     pixel. A group that makes a picture smaller makes its strokes thinner, and that is the width held.
   - The paths hold at most 3072 numbers (`MAX_ART_NUMBERS`): two for every point, once the renderer has made
     each shape a path. A `rect` is 8, a `circle` or an `ellipse` 26, a rounded `rect` 34, an arc as many as the
     curves it is drawn as. 99 circles are 2574.
   - The paint is at most 4194304 (`MAX_ART_PAINT`, 64 fills of 256 by 256). Every fill and every stroke is the
     picture's width times its height, and 16 (`ROW_PAINT`) for every row of the picture an edge of the path
     crosses, each edge after the last and back to where a piece began; a stroke's rows count twice, for its two
     sides. One painted with a gradient is four times all of that. A curve's edges are counted through the
     points that pull it. So a fill that covers a picture of 256 is 73728, and 56 of them fit; a line down the
     whole picture is 4096 under a fill.
- A theme's art files together are held to the same two counts, so five files
  cost no more than one. The refusal names the file that went over.
- The files are read again when the theme is drawn (`Theme::art_svgs`: one pass, each file read
  once, one sum), and are held
  together to the two counts again, in the same order: a file changed since the load
  that is refused, or would take the files before it past a count, is not drawn, and
  the kit's own art is drawn for its piece.

**What stays different between the door's parse and the renderer's** is only what the door turns off, each so
that less is done and never so that a file is read another way. The renderer allows a DTD and expands its
entities; the door refuses the file. The renderer loads the image an `<image` names and the system's fonts for a
`<text`; the door's options resolve no image, and neither element is on the list, so neither parse is asked. The
renderer unpacks a gzip file; the door reads text, and such a file is not UTF-8. A file the door takes holds none
of these, so the two make the same tree of it. `tests/alike.rs` holds the door against `usvg` called as GPUI calls
it, size for size and point for point, on each picture the door's own reader once took another way, and
`tests/art_draw.rs` rasterises them and compares the sizes. The kit asks for `usvg` at exactly GPUI's version, and
a test fails when the lock file holds two.

What an address in a value does: nothing. The renderer has no network, and a paint it cannot find in the file
(`fill="url(https://…)"`) is painted as a plain colour, which is how the door counts it.

Those two counts are what a file costs to draw (`theme::art_cost`): a shape may paint the whole picture however
small it is written, every point is a line or a curve the renderer walks, and every edge is walked down every row
it crosses, the picture being painted in the pieces the edges cut a row into. Before the counts the bound was the
file's size, and five files at it took 38 s to draw in a debug build on a Mac. `tests/cost.rs` holds each count,
the reach and the stroke at its edge. Measured on one Mac, in a debug build and in a release:

- **The heaviest themes the kit knows how to write** (`tests/draw.rs`): five ways to spend both counts to the last
  unit over five files, each read at the door and rasterised. They are the dearest of what was measured, and not
  a proof that nothing dearer is inside the counts.

  | All the paint and every point spent on | debug | release |
  |---|---|---|
  | 52 see-through fills that cover the picture, the rest a ragged fill | 1.30 s | 56 ms |
  | one ragged see-through fill a file, 241664 rows under its edges | 1.10 s | 80 ms |
  | the same under a see-through gradient, 45056 rows | 0.64 s | 24 ms |
  | the same under a fill and a round stroke 2.6 wide, 73728 rows | 0.77 s | 35 ms |
  | the same written as curves, under a fill and the thinnest stroke | 0.84 s | 32 ms |

  The door reads the five files in 3 to 16 ms (0.4 to 0.9 ms in a release). The heaviest is the first in a debug
  build and the second in a release: 1.3 s and 80 ms. One paint of 256 by 256 with no edge in it: an opaque
  fill 1.5 ms in a debug build, a see-through one 14 to 19 ms, a gradient 45 to 49 ms.
- **What a row costs.** Before a row was counted, a fill cost its area and no more, and a review found eight
  ragged shapes of 192 points, each edge as tall as the picture, under a see-through gradient and a gradient
  stroke: all the paint of a file and no more, and 7.6 s to draw in a debug build (14 s on a busy
  machine, 0.46 s in a release). It is now 19 times the paint of a file and refused, with its three neighbours
  (`tests/cost.rs`). One ragged fill of 1535 points took 0.78 s opaque and 1.4 s under a see-through gradient, and
  the time followed the points; the same points along the rows took 30 ms. A gradient stroke on such edges was
  the dearest thing measured, 4.6 s for the eight.
- **The worst file to read**: 64 KiB of arcs took the parser 0.36 s before its points could be counted. The 16 KiB
  held for `d` and `points` refuses it before that parse (`tests/limits.rs`). **What ends the program** is refused
  before the parse too. In a debug build the parser overflows a stack of 2 MiB on elements nested 700 deep (600 are
  read), and one of 256 KiB on 80 deep or on a run of 96 closes; at 1024 deep it refuses a file itself. The door
  takes 12 deep and a run of 32, and `tests/art_draw.rs` reads and draws the worst it takes on a stack of 256 KiB.
- **What a review measured at seconds each** is refused (`tests/art_draw.rs`): 99 ellipses of a radius of 4096
  under a thin stroke (4.5 to 6.5 s), circles and a rounded `rect` as large, a line under a stroke 4096 wide,
  each for its reach; 511 lines under a stroke of 0.015 or thinner on a picture of 8 drawn 256 (1.6 s), for the
  stroke. Their nearest kin that pass: 504 lines as tall as the picture at 0.016, the whole paint of a file,
  0.21 s (17 ms in a release); one ellipse of 100, 36 to 53 ms; 40 of them filled see-through on 204 by 204, the
  whole paint of a file, 1.1 s (32 ms in a release).
- **Reach**: 30 circles under a thin stroke took 0.42 s at a radius of 8, 64 and 512 (16384 pixels past the
  picture) alike, and 0.75 s at 4096. The reach held is a quarter of the largest that cost nothing.
- **A stroke under a drawn pixel** is a hairline to the renderer: 511 lines took 64 ms at 0.016 and 0.74 s at
  0.015, the same at 0.0001.

The kit's own art, counted the same
way at the built-in themes' sizes, is 2260 numbers and 2551696 paint: 74% and
61% of the counts. 163440 of that paint is the rows its edges cross. Behind
the ranges, the view rasterises the kit's own art at no more than 800 pixels a
side with a rim of no more than 64, whatever a theme and a window multiply out
to; a larger piece is that image scaled up. The kit's own art
is built in code and does not come through this door. It alone blurs a shadow,
scales, and fades a group, which a file may not ask for: the renderer panics
on a blurred group whose box overflows, and keeps a layer of memory for every
faded group.

**Text is cleaned before it is measured or drawn** (`text::clean`): control
characters, the line and paragraph separators U+2028 and U+2029, and every
format character (direction marks, zero-width space, soft hyphen, word joiner,
byte-order mark, tag characters) are removed, except the joiners U+200C and
U+200D, which emoji and several scripts are written with. The kit carries no
Unicode tables, so the list is written out in `src/text.rs`.

**Notes that do not fit are counted, never dropped in silence.** Notes are
placed in the scene's order, and room is made in this order. A list shows the
rows that fit, down to one, and ends with `+N more`. Then every option of a
choice note is shown in two lines, not three. Then the gap under each note
goes from 48 to 32 design units. A tighter step is taken only when it places
more notes. Text that is cut ends in `…`; the scene keeps the whole text.
A choice with `whole: true` is for options a person acts on by what they
read: no option of it is ever cut. Its sheet is as wide as the note column,
its options take the lines they need, and after the gaps close they are
written at 0.85 and then 0.72 of their size when that places more notes.
A `Whole` list is that promise for rows: each is drawn after its mark to its
last word, or passed over and counted in the list's last line, `+N more`.
It never moves or shrinks another note: the other notes are placed as if the
scene had no whole list, and it comes below them in the room they leave. It
makes room for itself alone, by a closer gap above it (32, then 16) and rows
written at 0.85 and 0.72, taking a step only when it draws more rows. With no
room for even its count line it is counted with the notes left out, where
that sheet fits. Where that sheet does not fit either, and every other note
is drawn, nothing is drawn for the list: it is the one note that can go
without a word, since a word for it would move another note. A scene's maker
that must have its rows counted says the count elsewhere, as in the footer.
`layout` returns one `mark` piece a row drawn.
That can go under the theme's `min_font_size`, which holds for every other
text: 9.35 pixels at 960x640 where the smallest is 11. Each text run carries
the `size` it was measured at, and the drawing paints it at that size, so a
line always ends inside its option. A
whole choice that still does not fit is left out and counted, never drawn in
part. Last, the first note that does not fit and every
note after it are left out together, and a level sheet (a `Note` piece, id
`MORE_NOTES_ID`, index the number of notes) says `+N more of M notes`. Layout
time does not grow with the number of notes or rows.

**Text breaks at a space, and never inside a character** (`text::wrap`,
`text::fit`). A word too long for a line of its own, such as an address or
text in a script written without spaces, carries on to the next line while
lines remain, broken between characters; the last line ends in `…` when text
is left. A character here is a base with the marks drawn onto it (the
combining marks of Latin, Greek, Cyrillic and symbols, the kana voicing marks,
variation selectors, emoji skin tones), whatever U+200D joins to it, or the
two regional indicators of a flag. That is not all of Unicode's rule, which
needs tables the kit does not carry: a mark of Hebrew, Arabic, Thai or an
Indic script, and each jamo of a decomposed Hangul syllable, counts as a
character of its own and a break can fall before it.

## Limits

GPUI rotates drawn paths only. Paper and tape tilt; **text never tilts**, and
the kit does not fake it. GPUI cannot pick a weight on a variable font, so
`assets/fonts/` holds static cuts made from the Google Fonts variable files
(Fredoka 500, Nunito 600 and 700, Caveat 600 and 700; OFL texts beside them).
What is known of their source is what the files say in their own name tables:
Fredoka `Version 2.001`, Nunito `Version 3.602`, Caveat `Version 2.000`. The
commit of the Google Fonts repository they were cut from was not recorded, and
the tool that cut them is not in this repository.
