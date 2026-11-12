# sticker-kit

A small UI kit on [GPUI](https://www.gpui.rs) that draws a sticker journal
page: flat sticker characters with stars, taped notes on grid paper, springy
motion. A pure function arranges the page; a theme, which is data, styles it.

## The three data types

- `Scene` (`src/scene.rs`): what is on the page. A date, a title, a hero,
  characters, notes (`List`, `Choice`, `Plain`), a legend. Plain serde data.
- `Theme` (`src/theme/`): how it looks. Colours, font names, tilt angles,
  motion amounts and sizes. It names no pictures: the art is the kit's own.
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
left edge to 24 design pixels short of the big character's place, the chips
under the field: in one row when that holds them, else in two. A chip is as
wide as its name, and at least one and a half times its height, while its row
has the room. In a row too narrow for its names the short ones keep their
width and the long ones share what is left; those are cut in the middle with
an ellipsis ("kitc…ntend"), so no name is cut while its row has room and two
names that begin alike are told apart by their ends (`tests/entry_layout.rs`).
Every page keeps that band free, whether or not it has an entry (a page
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

Each of the three is held to a least contrast on its ground (WCAG 2's ratio
of relative luminance, `Color::contrast`):

| Text | On | Floor | sticker | plain |
|---|---|---|---|---|
| the chosen chip's name | `chip_chosen` | 4.5 (`CHOSEN_INK_FLOOR`) | 9.99 | 4.57 |
| the hint, and words that wait | `entry` | 2.5 (`HINT_FLOOR`) | 2.80 | 3.06 |
| the ring | `paper` | 3.0 (`RING_FLOOR`) | 5.35 | 15.80 |

A theme file is held to them at the door (`src/theme/own.rs`), and is never
refused for them: a colour under its floor is replaced by the first of the
file's own that is over it (`soft` then `ink` for the name; `quiet`, `soft`,
`ink` for the hint; `caret`, `accent_dark`, `ink` for the ring), or by black
or white, and one warning names the key.

A key of the entry that a theme file does not set is the file's own colour,
when the file sets one for it: `chip_chosen_ink` is its `soft`, `entry_hint`
its `quiet`, `entry_ring` its `caret`; and `entry` is its `option`,
`entry_edge` its `bubble_ring`, `caret` its `accent_dark`, `chip` its
`dimmed.light`, `chip_chosen` its `star`. So a file written before a key was
is drawn in its own palette. A file that sets neither a key nor the colour it
is taken from is the sticker theme there, and the floors hold what is left. When the chosen chip's fill is close to the other chips' (no
channel 48 apart), a line in the colour of its name is drawn round it;
otherwise, as in both built-in themes, its fill alone tells it.
`tests/colours.rs` holds the built-in themes and theme files to all of this.

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
- **Words are text with a character a person sees** (`text::has_visible`).
  Spaces of any kind, a joiner or a variation selector with nothing beside
  it, a Hangul filler and the blank braille pattern are none: Enter gives
  nothing for them, and the hint stays while they are all the entry holds.
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
```

`Theme::load(path)` returns the theme and its warnings. An unknown key is
ignored and named in a warning, so a theme for a newer kit still loads.

**A theme file names no pictures**
(`docs/adr/0016-theme-files-name-no-pictures.md`). The `[art]` table an older
theme file has is not read: no file it names is opened, looked for or
resolved, by `load`, by `layout` or by the view. It is said in one warning,
`art: pictures in a theme are not read in this version; the kit's own art is
drawn`. Only the text of each name is looked at, and one that leaves the
theme's folder (`..`, a root) still refuses the theme, as it always did.

## The door: what the kit holds its input to

A scene and a theme file may come from anywhere, so each is held to stated
bounds. A theme outside them is **refused**, never clamped, with the key named.

| Input | Bound | Where |
|---|---|---|
| theme file | a regular file of at most 256 KiB | `theme::MAX_THEME_BYTES` |
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

**The art is the kit's own, built in code** (`src/art.rs`), and the only
pictures the view rasterises. Behind the ranges, it is rasterised at no more
than 800 pixels a side with a rim of no more than 64, whatever a theme and a
window multiply out to; a larger piece is that image scaled up.

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
