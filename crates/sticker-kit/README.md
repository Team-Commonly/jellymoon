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
| art file | a regular file of at most 1 MiB, inside the theme's folder (a symlink out of it counts as outside) | `theme::MAX_ART_BYTES` |
| any number in a theme | finite, and 0 to 10000 unless a line below says otherwise | `theme::RANGES` |
| `metrics.width`, `metrics.height` | 200 to 10000 | |
| `metrics.columns` | 1 to 12 | |
| `metrics.max_scale` | 0.1 to 10 | |
| `labels.max_chars` | 1 to 200 | |
| every angle under `tilt` | -45 to 45 | |
| an art file's SVG | a flat picture, held to the list below | `svg_refusal` |
| notes in a scene | the first 12 are laid out, the rest only counted | `MAX_NOTES` |
| options of a choice note | the first 4 | |
| any text | 400 characters, a label `labels.max_chars`; cut with an ellipsis | `layout::MAX_TEXT_CHARS` |
| pieces a layout returns | 5000; what would come after is dropped | `MAX_PIECES` |

`Theme::load("theme.toml")`, a bare name, reads from the folder the program
runs in. **A theme built in code gets the same range check** (`Theme::check`,
the one function `load` uses): `layout` runs it first and places nothing for
a theme it refuses, and the view draws only what is placed. Behind that,
`layout` returns at most `MAX_PIECES` pieces and punches at most 150 holes,
and the view draws at most 1000 grid lines each way and 64 stripes on a tape.

**An art file is an allow-list, not a block-list** (`src/theme/svg_check.rs`),
so what a file costs to draw is bounded at the door and not left to the
renderer. A file with anything not named here refuses the whole theme, with
the key and the reason. It is read through the door again when it is drawn,
and a file that has gone bad since is left out for the kit's own art:

- Elements, the 15 in `theme::SVG_ELEMENTS`: `svg` (one), `g`, `defs`, `title`,
  `desc`, `path`, `rect`, `circle`, `ellipse`, `line`, `polyline`, `polygon`,
  `linearGradient`, `radialGradient`, `stop`. So no `style`, `script`, `use`,
  `symbol`, `filter` or its primitives, `pattern`, `mask`, `clipPath`, `image`,
  `foreignObject`, `text`, and no DOCTYPE, entity or processing instruction.
- Attributes, the 33 in `theme::SVG_ATTRIBUTES`: `xmlns`, `id`, `version`,
  geometry (`x` `y` `x1` `y1` `x2` `y2` `cx` `cy` `r` `rx` `ry` `d` `points`
  `width` `height` `viewBox`), paint (`fill` `stroke` `stop-color` `offset`
  `fill-rule` `fill-opacity` `stroke-width` `stroke-opacity` `stop-opacity`
  `stroke-linecap` `stroke-linejoin` `gradientUnits`), and `transform`,
  `gradientTransform`. So no `style`, `href`, `filter`, `opacity` on a group,
  `on…` handler, `xml:base`, dashes or mitre limit.
- A value is in quotes and holds no backslash and no character reference
  (`&#…`), so `&#64;import`, `@\69mport` and `\75rl(` are refused as written.
  The letters `url` may only start `url(#`, a paint server in the same file.
- At most 100 elements (`MAX_ART_ELEMENTS`), nested at most 12 deep
  (`MAX_ART_DEPTH`), the `<svg` counted in both.
- `width`, `height` and the size in `viewBox` are 1 to 256
  (`MAX_ART_SIDE`), with no unit but `px`. Every other number is plain digits
  with no exponent, at most 4096 (`MAX_ART_NUMBER`).
- A transform is only `translate` and `rotate`.

The heaviest file tried inside these (560 KiB: one path of 60000 segments, 87
circles 4096 wide under 11 nested groups) is drawn five times over in 7 s by a
debug build on a Mac; a draw test holds it under a minute. The kit's own art
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
more notes. Text that is cut ends in `…`; the scene keeps the whole text. Last, the first note that does not fit and every
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
