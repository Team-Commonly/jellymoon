# sticker-kit

A small UI kit on [GPUI](https://www.gpui.rs) that draws a sticker journal
page: flat sticker characters with stars, taped notes on grid paper, springy
motion. A pure function arranges the page; a theme, which is data, styles it.

## The three data types

- `Scene` (`src/scene.rs`): what is on the page. A date, a title, a hero,
  characters, notes (`List`, `Choice`, `Plain`, `Text`, `Whole`, `Told`, `Drawn`: a picture in pencil, and
  `Said`: what a person typed). Plain serde data.
- `Theme` (`src/theme/`): how it looks. Colours, font names, tilt angles,
  motion amounts and sizes. It names no pictures: the art is the kit's own.
  Two are built in: `Theme::sticker()` and `Theme::plain()`.
- `Placed` (`src/layout/`): where each piece goes. `layout(scene, theme,
  viewport, &measure)` returns them; it has no GPUI in it, so it is tested
  without a window. `Measure` is the one trait: the width of a string.
  `BundledMeasure` adds up advance widths from the bundled font files, so it
  gives the same numbers on every system; `draw::GlyphMeasure` asks GPUI, which
  kerns and knows the system's fonts. The view draws with the second.

`draw::Page` is the GPUI view; taps on characters, options and pictures reach `on_tap`. Files or folders dropped on it
from outside the app reach `on_paths`, with their paths, called while the page is in use as `on_tap` is.

**A tap means what was seen** (`draw/settle.rs`). A page changes while a pointer is on its way, so:

- A tap on an option is `Tap::Option { note_id, index, words }`: `words` is the scene's own string for that
  option (`Scene::option`), whole, as the scene held it when the pieces on the page were laid out. Whoever acts
  on the tap can compare it with what the note says now. A tap on a picture carries no words.
- A click on an option, or on a drawn note's picture or its `?`, is no tap while the piece has not settled: for
  `SETTLE` (400 ms) after a new layout moved it in the window by more than 4 design units, and for `SETTLE`
  after it
  came onto the page at a point of the window where another piece took a tap in the frame before. A piece is the
  same one across frames by its kind, its note's id and its index.
- What counts is what a new layout did in the window. A card that comes at the end of a column held at its end
  slides the older choices up, and they have moved; a card that comes above while the column holds its end leaves
  them where they were. A turn of the wheel and a window that changed size are the person's own moves and hold
  nothing back. A new piece where nothing took a tap takes one at once.
- A click that was no tap is not kept for later and nothing is drawn for it: the person clicks again. Characters,
  bubbles, chips, the entry and typing are never held back.
- The time is the page's own, the clock its drawing goes by; `page.set_settle_time(Some(seconds))` tells it one,
  for a test.

Not done: a choice whose words change in place (same note, same index, same place) takes a tap at once, and only
the words it carries tell the two apart. No real window was clicked.

## The entry: one line to type

A page may hold one `Entry` (`src/scene.rs`) beside its scene: an id, a hint, a
few options of which one is always chosen, and a cap on its characters. It has
a spot of its own: a band at the window's bottom, as wide as the column. It is the one thing on a page that is
placed in the window and not in the column, so it is where it was however long the column is and however far it
is scrolled. The field's row is 44 design pixels high, 12 under the band's top, and the chips stand under it: in
one row when that holds them, else in two. A chip is as
wide as its name, and at least one and a half times its height, while its row
has the room. In a row too narrow for its names the short ones keep their
width and the long ones share what is left; those are cut in the middle with
an ellipsis ("kitc…ntend"), so no name is cut while its row has room and two
names that begin alike are told apart by their ends (`tests/entry_layout.rs`).
The end never begins with a mark that is drawn onto the character before it
(`tests/text.rs`).
Every window keeps that band (`entry_top(theme, viewport)` is its top), whether or not the page has an entry, and
the column is shown over it only, so an entry that comes or goes moves no piece. `layout_entry(entry, theme, viewport, &measure)` places
the field (`PieceKind::Entry`) and a chip for each of the first six options
(`PieceKind::Chip`). The options past those are counted: `+N more` (`PieceKind::More`), in words, after the last
chip, whose row keeps its width free. It is no option: no click chooses it. It
is left out only where that row could not hold its chips at their least width
beside it. An entry's `action`, when it is not empty, is one thing to do that is no option
(`PieceKind::Action`): whole, after the chips and the count, in the last row, whose chips share what it leaves. A
tap on it reaches `on_tap` as `Tap::Action` with the entry's id; it is never chosen, and Enter never gives it.
An entry's `go`, when it is not empty, is the choice that does what Enter does (`PieceKind::Go`): whole, at
the right end of the field's row and as high as the field, which has what it leaves. A tap on it gives the field
the keys and enters its words, as Enter does; while the field holds no words it is drawn as not ready (a plain
chip, its words in the hint's grey), and a tap enters nothing and reaches `on_tap` as `Tap::Go` with the entry's
id, so whoever made the entry can say what to type first. With `loud`, the action is the entry's loud choice:
filled as a chosen chip is, while no chip is drawn chosen. The hint, and `go`, are written at the size typed words
are. `page.choose_option(name, cx)` chooses the option of that name when it is offered, and says whether it was.
It places nothing for a theme whose column is under 160 design pixels wide.

Three things about an entry are told apart by more than a shade, and each is
a key of the theme's `[palette]`: the hint is a grey (`entry_hint`, also the
words an entry keeps aside for its answer), the chosen chip is filled (`chip_chosen`, its
name in `chip_chosen_ink`), and the field that has the keys has a ring, a
line darker than that fill (`entry_ring`; `entry_edge` without the keys), calm and never a red that reads as an error: the sticker theme's lilac, the plain theme's ink.

Every text of the entry is held to a least contrast on its ground (WCAG 2's
ratio of relative luminance, `Color::contrast`). Each floor is what WCAG 2 asks
of text (4.5) or of a line (3), but the hint's, which a theme file may make pale; the built-in themes' hints
are dark enough to read as text:

| Text | On | Floor | sticker | plain |
|---|---|---|---|---|
| the chosen chip's name | `chip_chosen` | 4.5 (`CHOSEN_INK_FLOOR`) | 9.99 | 4.57 |
| a chip's name, the palette's `soft` | `chip` | 4.5 (`CHIP_INK_FLOOR`) | 5.66 | 5.45 |
| what is typed, the palette's `ink` | `entry` | 4.5 (`TYPED_FLOOR`) | 14.39 | 15.80 |
| the hint, and words that wait | `entry` | 2.5 (`HINT_FLOOR`) | 5.36 | 5.37 |
| the caret | `entry` | 3.0 (`CARET_FLOOR`) | 3.67 | 4.57 |
| the ring | `paper` | 3.0 (`RING_FLOOR`) | 4.98 | 15.80 |
| `quoted`, someone else's words | `bubble` | 4.5 (`QUOTED_FLOOR`) | 6.14 | 6.66 |

A colour one can see through is laid on what is under it before it is
measured: a chip and the field on the paper, the paper on the desk, and the
desk on white, since the kit does not know what a window has under its page
(`Palette::ground`). A text one can see through is laid on its ground the same
way, so it is measured as it is drawn.

A theme file never draws one of these pairs under its floor for a key it did
not set (`src/theme/own.rs`). Over its floor a pair may be drawn worse than
before: `accent_dark = "#808080"` alone moves the ring from 4.98 to 3.88. A
key of the entry the file lacks takes the
file's own colour only when the file sets that colour and the pair reads with
it: `entry` is its `option` under its `ink`, `caret` its `accent_dark`,
`entry_hint` its `quiet`, `entry_ring` its caret's, `chip` its `dimmed.light`
(the light shade itself, not another shade of `dimmed`) under its `soft`, and
`chip_chosen` and `chip_chosen_ink` its `star` and its `soft`, both or
neither. `entry_edge`, which no text is on, is its `bubble_ring`. Where the
pair does not read, or the file does not set the colour, the key is the
sticker theme's, and nothing is said. The edge of a field without the keys is
held to no floor: it is the file's own `bubble_ring`, unchecked, and a
`bubble_ring` of the paper's colour draws a field with no edge.

Then every pair is held to its floor, and a file is never refused for one: a
colour under its floor is replaced by the first of the palette's that is over
it (`soft` then `ink` for the chosen name; `quiet`, `soft`, `ink` for the
hint; `accent_dark`, `ink` for the caret; the caret, `accent_dark`, `ink` for
the ring), or by black or white. `soft` and `ink` are the whole page's, so
under them it is the ground that gives way: a chip to `dimmed.light`,
`option`, `paper`, `ink`, and the field to `option`, `paper`, `ink`. An `ink`
or a `soft` one can see through, which no ground is read under
(`ink = "#2B235020"`), is replaced itself, for the whole page: by the same
colour with nothing seen through where that reads, else by black or white. One
warning names the key, and only a key the file set: a replacement for a key
the file never set is not said. So a file that sets only a pale `ink` or
`soft` gets a different field and chips with nothing said (`ink = "#C0C0C0"`
alone draws a black field): the field and the chips are keys it did not set.

When the chosen chip's fill does not stand out from the other chips' and from
the paper under it (a contrast under 1.2, `CHOSEN_FILL_FLOOR`: the sticker
theme's yellow on its lilac chips is 1.22), a line in the colour of its name
is drawn round it; otherwise, as in both built-in themes, its fill alone tells
it. `tests/colours.rs` and `tests/floors.rs` hold the built-in themes and
theme files to all of this.

```rust
Page::new(scene, theme)
    .with_entry(Entry { id: "wish".into(), hint: "wish for something".into(), options, most: 200, go: "Start".into(), ..Entry::default() })
    .on_entry(|made, window, cx| { /* made.id, made.text, made.option */ })
```

- **Keys.** A page with an entry gives it the keys whenever nothing else on the
  page holds them: when the window opens, when the entry arrives, after a press
  anywhere off the fields, and after Escape, which in a note's field gives them
  to the entry. A click on a note's field or its offer gives that field the
  keys, and a field that asks for them takes them once, when it first stands
  ("A text note's quoted words, field and offer"). What is typed is never lost for want of a holder. Text goes in at the caret. Backspace,
  Delete, Left, Right, Home and End (or Left and Right with the platform's
  command key) edit, the command key with V pastes, Tab and Shift-Tab choose the
  next and the previous option, Enter enters. There is a caret and no selection.
  One Backspace or Delete removes a whole character as a reader sees it, by
  the rule no cut falls inside (`text.rs`, `ends`): a letter with its accents,
  a joined emoji, a flag, and a whole cluster of Thai, Devanagari, Hebrew,
  Arabic or Hangul in conjoining jamo, not one mark of it (`tests/entry.rs`).
- **Enter** calls `on_entry` with the text, trimmed, and the chosen option's
  name (`made.option`), never its place: the options can change before the
  handler acts. Text that is only spaces enters nothing. The handler runs while
  the page is in use, as `on_tap` does: one that changes the page does it in
  `cx.defer`.
- **The entry is then owed an answer**, `page.answer_entry(kept, window, cx)`, which every handler gives. The
  words Enter gave are kept aside (`EntryState::owed`), and the typed text is empty from then on.
  - *Every key is taken meanwhile*, as at any other time: text, a paste, a composition (`accepts_text_input`
    stays true, and every range the input method is told is of the typed text, never of the words kept aside),
    the editing keys, the caret, Tab, a click on a chip, Escape. Only Enter gives nothing while the answer is
    owed, and what was typed since stays. No key is dropped.
  - *One Enter is remembered.* An Enter pressed while the answer is owed, with words typed since and no
    composition, is remembered: once, however often it is pressed. Nothing is sent then and the words stay
    editable. When the answer is *kept*, `answer_entry` gives that Enter as one pressed at that moment is, to
    the same `on_entry` handler, with the words and the option it was pressed on. A change to the text or to
    the chosen option after that Enter forgets it (a caret moved does not): nothing is sent that no Enter was
    pressed on, and the words stay for the person's own Enter. So does a change of the entry under it
    (`set_entry`, `EntryState::moved`): another entry in its place, its option no longer offered, a cap that
    cuts the words, or the entry taken away. An option that only moved keeps it, by its name. It is given once, and the entry is then owed
    the answer to those words. When the answer is *not kept* the Enter is forgotten: the owed words are back in
    front of the later ones, and an Enter would send both as one. A handler that does not want it calls
    `page.forget_enter()` before it answers, and the words stay as they are. `answer_entry_closed` forgets it
    too (a handler is called in a window), and so does a note's field: `EntryState::answer` is the plain
    answer, `EntryState::answer_and_enter` the one that gives the Enter (`tests/entry_waits.rs`).
  - *Drawn:* while nothing was typed since, the field shows the words kept aside in the hint's grey, with the
    caret where an entry that shows its hint has it. With the first key the typed text is drawn in ink in their
    place. `go` is not lit while the answer is owed.
  - *Kept:* the words kept aside are forgotten. What was typed since stays as it is, with its caret.
  - *Not kept:* the words come back as they were typed, the caret at their end. When something was typed since,
    they come back in front of it, one space between, as much of them as the cap leaves room for beside what
    was typed since, which is never cut; the caret is at the end, and a composition keeps its own. An option
    chosen since stays.
  - A page with no handler answers itself.
  - Until 2026-11-24 the entry took no key between Enter and its answer, and a key pressed then was dropped
    with no sign: the start of the next words a person typed was lost. That was a decision, and it was changed
    on purpose.
- **An input method** (Chinese, Japanese, Korean, a dead key) talks to the page
  through GPUI's `EntityInputHandler`. What it is still composing is underlined,
  and Enter, the editing keys and a click leave it alone until it ends.
- **A window's own keys.** On a page with an entry a letter is always text: a
  window's own key needs a modifier no field takes (the app's is the command key
  and T). `page.entry_focused(window)` says whether the entry or a field has the keys. `page.entry()` is what
  is typed now; `set_entry` changes or removes the entry, and keeps the chosen
  option by its name when that name is still offered (`EntryState::keep_option`).
- **Words are text with a character a person sees** (`text::has_visible`).
  Spaces of any kind, a joiner or a variation selector with nothing beside
  it, a Hangul filler and the blank braille pattern are none: Enter gives
  nothing for them, and the hint stays while they are all the entry holds.
- **An entry that holds words is not removed.** `set_entry(None)` takes an
  empty entry off the page at once, and one that holds only spaces, which
  are no words: Enter gives nothing for them. One that holds words, or is owed
  its answer, stays as it is, with its options, and goes when it holds
  none: after an answer that kept the words, or when the person has deleted
  them, whatever spaces are left. Words a
  person typed never go with the entry, and an answer always finds the words
  it is for: an entry cannot go and come back empty while an answer is owed. An entry
  set meanwhile takes its place and keeps the words.

The rules are `entry::EntryState`, plain data with no GPUI in it
(`tests/entry.rs`, `tests/entry_waits.rs`). `tests/entry_draw.rs` sends keys and clicks through GPUI's
own dispatch to a headless page. The test platform has no input method, so a
composition is started there by calling the handler, as a platform would; what
a real input method does on a real window is checked by hand.

## A text note's quoted words, field and offer

A text note (`Note::Text`) can hold two things besides its title, body and
options (`layout/typed.rs`, `draw/field.rs`).

- **`quoted`: someone else's words.** They are placed as one piece
  (`PieceKind::Quote`) under the body, on a ground of their own, every run in
  `Role::Quoted`, so they are told apart from the note's own words. They are
  cleaned as every text is, broken into at most `MAX_QUOTED_LINES` (8) lines
  and cut there with an ellipsis. They are text and nothing else: the piece
  takes no tap and no key, whatever the words say. Words that clean to nothing
  place no piece.
- **A quote never looks like a field.** Every `quote` piece of a note (a text
  note's, and each of a told note's) is drawn on a ground of its own with no
  line round it: a soft wash of the palette's `pencil` on its `bubble`
  (`Palette::quote_ground`), and one line of the pencil down its left, before
  the words (the piece's sketch, `quoted`, with one part, `line`). A line
  round a ground is a field's and the entry's, so only they look like a place
  to type. The wash is used only where the `quoted` ink still reads on it at
  `QUOTED_FLOOR`; under a theme where it would not, the ground is the plain
  `bubble` the ink was measured on. What a person typed (`Note::Said`) keeps
  its bubble and its ring.
- **`field`: one line to type in** (`Field`: a `hint`, a cap `most`, and an
  `offer`). It is placed under the options (`PieceKind::Field`), with the
  hint in it. An `offer` that has words is a small pill over the field
  (`PieceKind::Offer`); a click on it gives the field the keys and nothing
  else.
- **A field that is first** (`Field::first`, a text note's only). The note
  asks something and the field is where it is answered, so the field stands
  directly under the quoted words, and the options come after it as the ways
  round it: small pills on one row, wrapped to another when they do not fit,
  each drawn as an offer is and each still an `option` piece of its index,
  with its tap. A long option is cut to one line of the note's width. A note
  whose field is not first, and every drawn note, places and draws as before.
- **A long body.** The quoted words and the field are placed whether the
  body is drawn whole or counted; the options still go when it is counted.
- **Typing.** A field is typed in as the entry is, by the same code and under
  the same rules: a click gives it the keys, Escape or a press elsewhere takes
  them, an input method composes in it, `most` caps it (0 is the kit's own
  cap, `MAX_ENTRY_CHARS`). Each field and the entry hold their own words.
- **Enter** calls the `on_entry` handler with the note's id as `made.id`,
  the words, and no option. The field is then owed its answer as the entry is,
  `page.answer_field(id, kept, window, cx)`, and takes every key meanwhile: kept, what was typed since stays;
  not kept, the words come back in front of it. An Enter pressed in a field meanwhile gives nothing, then or at
  the answer: only the entry is given the one it remembered. `page.answer_field_closed(id, kept, cx)` is the same for a page whose window was closed since
  Enter, as `answer_entry_closed` is the entry's. `page.field(id)` is what is typed in it now.
- **A field lives with its note.** `set_scene` keeps what is typed in a field
  whose note still has one, by the note's id, and drops the field of a note
  that is gone or has none.
- **A field that asks for the keys** (`Field::wants_keys`, false unless a
  scene says so) takes them at the first drawing that places it: what is
  typed then is in the field, with no click. Once only: after a click
  elsewhere or Escape it does not take them back at a later drawing. Never
  out of typed words: while the entry holds words or is owed an answer, or
  the field that has the keys holds words, the keys stay where they are, and
  the asking field has had its turn. When its note goes the keys are the
  entry's again, by the rule under "Keys". A field that comes again after its
  note was gone is a new field, and asks again (`tests/field_keys.rs`).

A drawn note (`Note::Drawn`) can hold a `field` too. It is a text note's:
placed under the options, whether the note is open or closed, and drawn,
typed in and reported by the same code (`tests/pencil.rs`,
`tests/field_draw.rs`).

`tests/typed.rs` places them; `tests/field_draw.rs` types in them through
GPUI's dispatch on a headless page. Not done: a field's element stands where
the layout placed it and does not follow its note while the note moves into
place.

## A drawn note: a picture in pencil

`Note::Drawn { id, picture, words, open, options, field }` is a card that shows a thing as a small drawing where a text
note would say it in sentences (ADR 0027). The kit draws it with a pencil (`src/pencil.rs`, plain geometry):

- **A stroke** bows a little and ends a little past both its ends, and is laid twice: a wide faint pass under a
  narrow darker one. Its wobble follows from the note's id, the stroke's place in its picture and the pass, and
  from nothing else, so a scene is drawn alike in every frame and every run. The marks built from strokes: a
  line, dashes, a box whose sides pass its corners, hatching at 45 degrees, a circle, a squiggle, an arrow, a
  tick, a cross, a plus, and tally marks (four upright, the fifth across them).
- **A picture draws itself** when its note first comes onto a page: stroke after stroke, in a second and a half at
  most (`pencil::DRAWN_IN`). Then it is still, but for the tool at work in a `Track`, which rocks by the theme's
  motion. With motion reduced, or in a theme where nothing moves while idle, it is whole at once and nothing rocks.
- **The pictures** (`enum Picture`, laid out in `src/layout/pictures/`):
  `Track { steps, stage, file }`, a run at work: tallies for its steps (25 at most, then a small plus), a sheet
  under a magnifier (`looking`) or a pencil (`making`) or three dots (`thinking`), the stage in two words
  ("thinking", "looking around", "making the change") with the file's name under it, and a dashed line to the
  empty circle its result will stand in.
  `Sheets { made, changed, removed, state }`, finished work: a sheet a file (six at most, then a stack and
  `+N`), "1 file" or "3 files" under them ("nothing changed" for none), and an arrow to a folder: dashed with
  "not in your folder yet" while it waits, solid with a tick and "in your folder" once it is in, and for work
  thrown away no arrow, no folder, every sheet crossed out and "thrown away". Work that does not fit the folder
  (`unfit`) is drawn as waiting work with the arrow barred in the taken ink and "does not fit your folder".
  `Bars { rows, more }`, changed lines a file: a name, a box in the added ink and one in the taken ink to one
  scale, `+6 −1`, at most six rows and then `+N more`, and a ruler labelled in lines ("0", "20", "40 lines").
- **Words.** A caption is the kit's own or a number, in the theme's handwriting (`Role::Caption`). A file's name
  is someone else's word: it is drawn by the rule a told row's `name` is (no blank, unseen or control character,
  whole on one line), else left out of a track and written "a file" in a bar's row. The note's `words` are not
  placed while `open` is false: a `?` in a pencil ring (`PieceKind::Ask`) stands at the picture's top right when
  there are any. With `open` each is drawn under the picture to its last word. The kit keeps no state for it.
- **Taps.** An option is a text note's: `Tap::Option` with the note's id, the option's place and its words. A
  tap on the picture or the `?` is `Tap::Option` with the note's id, `PICTURE_PLACE`, the place of no option, and
  no words. Both wait until the piece has settled ("A tap means what was seen", above).
- **Inks.** `palette.pencil`, `palette.added`, `palette.taken`. A theme file that sets none gets the sticker
  theme's; they are held to no floor.
- **Reading one without a window.** The `Picture` piece carries its `sketch`: the strokes, and its parts by name
  (`tally`, `sheet made`, `folder dashed`, `bar added`, ...). `print(placed)` writes a line a piece, such as
  `picture work-1 0 [track steps=7 stage=making file=hello.py] (tally×6, across×1, sheet×1, pencil×1, dashed×1,
  place×1) "making the change" "hello.py"`. `tests/pencil.rs` reads these.

Not done: no app builds these notes yet, and no stroke was seen in a real window.

## The edge scene: a character at a corner of the screen

`Edge` (`src/edge.rs`) is a second, smaller scene: one character with a `mood` (`floats`, `leans_in`, `holds_up`,
`swims`, `waves`, `glows`, `droops`) and a `count`, a `bubble` of texts, and a `ring` from 1, full, to 0, empty.
`layout_edge(edge, theme, corner, measure)` places it in two windows, each from its own top left: the character and
its count in one of 96 by 96, the bubble with its lines and its ring in a second, 300 wide. `edge_windows(theme)`
gives both sizes, which never depend on what is said: a window is opened and removed, never resized.

- A text of the bubble has a `tone`: `plain` (the role `body`, at most 4 lines), `strong` (`row_main`, 3), `small`
  (`row_sub`, 1) or `quoted` (the role `quoted`, 3), for someone else's words, which stand 10 pixels apart from the
  texts around them. A bubble draws the first 4 texts that hold something a person sees, on 8 lines at most; each
  is cleaned and cut like every text of the kit, and every text keeps a line. With none there is no bubble, no
  second window and no ring.
- A count of 0 draws nothing, and one over 99 is drawn as `99+`. A ring that is no number is empty. The kit reads
  no clock: whoever counts the time gives the share that is left.
- `place_corner(screen, room, size, corner, margin)` says where a window stands in a corner, and
  `place_beside(screen, room, anchor, size, corner, gap)` where a second stands beside it. The room is what the
  system leaves of the screen (a menu bar, a dock, a task bar), given as numbers: the kit asks the system nothing.
  A room that is nothing, off the screen or not numbers is the whole screen. The margin gives way before a window
  leaves the room; a window larger than the room keeps its size and starts at the room's top left. The second
  window stands away from the corner's edge, level with the first; where the room is too narrow it stands over a
  lower anchor and under an upper one.

- `place_pair(screen, room, first, second, corner, margin, gap)` places both. In a room that holds the two apart
  only with less than the margin, the first window's margin gives way before the second is laid over the first.
  The margin is one number, so it is made smaller at both of the corner's edges, though one would do. A size, a margin or a gap below nothing, or that is no number, is nothing.
- A text of the edge scene is drawn at its role's size held to 9 to 22 pixels, whatever a theme asks, so a window
  of the size `edge_windows` gives always holds its texts.
- `Edge::speaks()` says whether there is a bubble, and so a second window.

`EdgeView` (`src/draw/edge.rs`, feature `gpui`) paints one window of the scene: `EdgePart::Character` or
`EdgePart::Bubble`. It is given the scene with `set_edge` and draws it again; `on_tap` is told a tap anywhere in
it, and a view with no `on_tap` takes none. The character is the theme's art in one of seven poses, one a mood,
with the count on a pill; the bubble is a rounded shape with the ring as an arc that empties. Every run of text is
clipped to its piece, and is text and nothing else. A `quoted` run is drawn in the palette's `quoted` ink
(`edge_ink`), the others in `ink`, `soft` and `quiet`, and the count in `bubble` on its pill; `painted()` gives
the ink and the clipped box of every run of the last frame, beside `placed()`. A theme file's `quoted` falls back to its `accent_dark` where that
reads on its `bubble`, and is held to its floor like the pairs above, on a bubble laid on white.

**Open: the bubble's other inks have no floor.** Only `quoted` is measured on `bubble`, and only on white.
Nothing is built for it while nobody has a theme file of their own. What was measured with theme files:

| A theme file's `bubble` | What it gives | Said |
|---|---|---|
| `#808080` | `quoted` 5.32, but `ink` 3.64, `soft` 1.69 and `quiet` 1.28 | nothing |
| `#202020` | the scene's own sentence, in `ink`, at 1.13 | nothing |
| `#E8B0A0` | `quoted` falls back to exactly `ink`: someone else's words and the scene's own share ink, size, family and weight, with the 10 pixels between them | nothing |
| `#FFFFFF20` | `quoted` 6.14 on white, and 2.21 over a dark desktop | nothing |

In the two built-in themes `quiet`, the small line under a heard text, is 3.10 and 3.04 on the bubble. What
would close it: a floor for `ink` and `soft` on `bubble` as for `quoted`, a fallback that is never `ink` or
`soft`, and a bubble one sees through settled against black as well as white.

## Run it

```sh
cargo test --no-default-features          # scene, theme, layout: no GPUI
cargo run --example page                  # a window; press t to switch theme
cargo run --example snapshot --features headless -- out/   # four PNGs, no window: the sample page and the
                                                           # pencil's pictures, in both themes
cargo run --example snapshot --features headless -- out/ --at 0.6   # the same, each picture 0.6 s into drawing itself
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
(`docs/adr/0016-theme-files-name-no-pictures.md`). The `art` an older theme
file has is not read, in any shape (a table of names, a string, an array) and
whatever it names (a path out of the folder, a root, a script, nothing): no
file it names is opened, looked for or resolved, by `load`, by `layout` or by
the view, and no name is looked at. It is said in one warning, `art: pictures
in a theme are not read in this version; the kit's own art is drawn`, and the
file loads, the same on every system. No theme file is refused for its `art`.

## The door: what the kit holds its input to

A scene and a theme file may come from anywhere, so each is held to stated
bounds. A theme outside them is **refused**, never clamped, with the key named.

| Input | Bound | Where |
|---|---|---|
| theme file | a regular file of at most 256 KiB | `theme::MAX_THEME_BYTES` |
| any number in a theme | finite, and 0 to 10000 unless a line below says otherwise | `theme::RANGES` |
| `metrics.width`, `metrics.height` | 200 to 10000 | |
| `metrics.max_scale` | 0.1 to 10 | |
| `metrics.hero_size` | 0 to 800 (the sticker theme: 120) | |
| `metrics.character_size` | 0 to 400 (48) | |
| `metrics.rim` | 0 to 40 (5) | |
| `motion.damping` | 0 to 200 (14); the spring is stable all the way | |
| `labels.max_chars` | 1 to 200 | |
| every angle under `tilt` | -45 to 45 | |
| rows of a list | the first 24 are laid out, the rest only counted | |
| texts of a text note's body | 24 lines of them, the rest only counted | `MAX_TEXT_LINES` |
| options of a choice note | the first 4 | |
| rows of a whole list | the first 12 are laid out, the rest only counted | |
| rows of a told note | the first 12 are laid out, the rest only counted | |
| any text | 400 characters, a label `labels.max_chars`; cut with an ellipsis | `layout::MAX_TEXT_CHARS` |
| pieces a layout returns | 5000; what would come after is dropped | `MAX_PIECES` |
| options of an entry | the first 6 are offered; the rest are counted, `+N more` | `entry::MAX_ENTRY_OPTIONS` |
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

**The page is one column, and nothing is left out of it for room** (`layout/column.rs`, ADR 0026). From its top:
the header (the date's tape, the title, the footer as one quiet line); a row for each small character no `Said`
is of (the character, its label, what it says); then the scene's notes in the scene's order, each a card of
`metrics.note_width` at its full height, one under the other. The column is as long as what it holds. No card is
moved beside another, made smaller, written smaller or left out because the page is long, and no layout is tried
twice. What bounds a card is what bounds its own text, the same at every window size: a list's first 24 rows, a
whole list's and a told note's first 12, 24 lines of a text note's body, four options, three lines an option
(every line of one in a `whole` choice), six lines of a plain note, and `…` on a text over 400 characters. Each
count that is cut says so in the card's last line, `+N more`. Behind all of it `layout` returns at most
`MAX_PIECES` pieces.

`Note::Said { id, text, answer }` is what a person typed. It is no card: a bubble at the column's right, one `quote`
piece in `Role::Quoted` with the note's id, drawn by the rules every quoted text is drawn by (cleaned, at most
`MAX_QUOTED_LINES` lines, cut with `…`). The character it is of (`speaker(id)`: the id after its first `-`)
stands small at the bubble's end with its stars and its state, and its label and what it says under them.
`answer` is what the person answered to go on, when they did: lines of the same piece under the typed ones, by
the same rules, set in behind a small hook the pencil draws (the piece's sketch, `answered`, with one part,
`hook`). No word tells the two apart, and with an empty `answer` the bubble is `text` alone. A tap
on the bubble is a tap on that character; a bubble of no character takes none. The big character stands left of
the first card after each `Said`, and of the column's first card, no larger than the room left of the column.

A `Whole` list draws each of its first 12 rows after its mark, to its last word; `layout` returns one `mark`
piece a row drawn. A choice with `whole: true` cuts no option. A `Told` note is someone else's words under the
note's own labels (`layout/told.rs`): each row is a label in `row_main` and the other's texts under it, each a
`quote` piece in `Role::Quoted`, never a run of the sheet; a row with an empty label starts with its first
quote. Rows are drawn in their order, each to its last character, up to the first that the cleaning every text
is drawn with would cut; it and every row after it are counted, `+N more`, and no later row is drawn in its
place. A row's `marks` are the note's own small words in `row_sub`, each level with the quote of its place and
before it; a row with one sets its quotes close together, each ground after the widest word. A row's `name` (a
file's, the other's word) shares the label's line only when the two lay out as one line and it holds no blank,
unseen or control character; else the label stands alone and the name is the row's first quote. The note's own
sentences, `end`, come last and are drawn whole, or the note is left out. It has no option, field or mark.

**A piece's place is the column's own, and a view scrolls it** (`src/scroll.rs`, `draw/column.rs`). `layout` is
pure and knows nothing of scrolling: `y` is from the column's top. `scroll_most(placed, theme, viewport)` is how
far a column scrolls: what of it is below `entry_top`. `Page` keeps a `Scroll`: a turn of the wheel moves the
column, held between its top and its end; a page opens at its end, and one that is at its end stays there when
the column grows, while one a reader has moved stays where it was. The view paints the column in a layer clipped
at the band's top and the entry in a second layer over it, so no card is drawn or tapped under the entry.
`page.shown_at(piece)` says where a piece is in the window now.

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
