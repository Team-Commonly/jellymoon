# 0027 · The kit draws in pencil

Status: Built in the kit, 2026-11-24: the pencil, one note kind and three pictures. Proved on the placed pieces
and their print by `crates/sticker-kit/tests/pencil.rs`, in the headless view by
`crates/sticker-kit/tests/column_draw.rs`, and for the view's clock by the test in
`crates/sticker-kit/src/draw/pencil.rs`. The app builds none of these notes yet, and nobody has seen a stroke in a
real window. It adds to [ADR 0026](0026-the-page-is-one-column.md): a drawn note is one more card of the column.

## Why

The column explains a wish in sentences: that it is being worked on, that the change is in a copy and not in the
folder yet, how many files changed, that the change was kept. A person has to read each one, every time, to learn
what a glance could tell. A notebook page tells the same with marks: tallies for steps, a little sheet for each
file, an arrow into a folder, a tick.

So a wish's progress and its result are shown mostly as small drawings that draw themselves, the way a hand
sketches on paper. The sentences stay, and are drawn when the person asks for them.

## Decisions

1. **A pencil that is geometry** (`src/pencil.rs`, pure: no GPUI, no clock, no random source). A stroke from one
   point to another bows a little and ends a little past both its ends. It is laid twice: a wide faint pass under
   a narrow darker one whose wobble differs a little. That is the graphite. The view paints each pass as one
   stroked path (`PathBuilder::stroke` of the pinned GPUI), so a stroke costs two paths.
   *Not a shader:* the kit has none of its own, GPUI's are compiled into it, and a look that lives in a shader
   cannot be read by a test or printed. *Not a texture file:* a theme names no pictures
   ([ADR 0016](0016-theme-files-name-no-pictures.md)), a texture is one more door for a file, and it would not
   scale with the page as every piece does. Strokes are points: a test reads them, the print names them, and
   they scale by the one number everything scales by.
2. **What is seeded, and why.** The wobble of a stroke follows from the note's id, the stroke's place in its
   picture and which pass it is, and from nothing else: never the time, never a random source. The same scene
   gives the same strokes in every frame, so a picture does not shimmer while the page repaints, and in every
   run, so a test and a picture file are the same on every machine. The place is in the seed so that two equal
   lines of one picture are two strokes and not one stroke twice; the id so that two notes are two hands.
3. **A picture draws itself once.** Every stroke has a start and a length in seconds from when its note first
   came onto a page; the strokes follow one another in the order a hand would draw them, and a whole picture
   takes at most a second and a half. The layout gives the strokes and their times; the view keeps when each
   picture first came (by the note's id and the picture's kind) and draws each stroke as far as that time has
   reached. A picture seen before is not drawn again when its scene changes: what is new in it is simply there.
4. **What moves, and when it stops.** After its time a picture is still. One thing keeps moving: the tool at
   work in a `Track` (the magnifier or the pencil) rocks three design units to each side, by the theme's motion
   (`motion::rock`). With motion reduced, or in a theme where nothing moves while the page is idle, every
   stroke is whole at once and nothing rocks. Every headless print and picture file is whole too, unless a time
   is asked for (`Page::sketched_at`, the snapshot example's `--at`).
5. **One note kind: `Note::Drawn { id, picture, words, open, options }`.** A card of the column's width, as high
   as its picture and, when open, its words. The options are a text note's, tapped by the note's id and the
   option's place. `words` are the note's own sentences: closed, none of them is placed, and a small `?` in a
   pencil ring stands at the picture's top right when there are any; open, each is drawn under the picture to
   its last word. A tap on the picture or on the `?` is the note's id at `PICTURE_PLACE`, the place of no
   option, through the same `Tap::Option` an option uses. The scene carries `open`; the kit keeps no state for
   it, so whoever builds the scene decides what a tap opens.
6. **Three pictures** (`enum Picture`, `src/layout/pictures/`), each a `Picture` piece whose `sketch` holds its
   strokes and its named parts:
   - `Track { steps, stage, file }`, a run at work: tally marks for its steps (25 at most, then a small plus);
     a sheet with a magnifier or a pencil over it, or three dots while it thinks; the stage in two words and
     the file's name under it; a dashed line to an empty circle where the result will stand.
   - `Sheets { made, changed, removed, state }`, finished work: a sheet a file (six at most, then a stack and
     `+N`), their count in words, an arrow to a folder. Waiting, the arrow and the folder are dashed; in, they
     are solid with a tick; tossed, there is no arrow and no folder and every sheet is crossed out.
   - `Bars { rows, more }`, changed lines a file to one scale, with a ruler labelled in lines.
7. **Words in a picture are few, fixed and handwritten.** Every caption is the kit's own (the three stages, the
   counts, the three states) or a number, set in the theme's handwriting family and measured as every text is.
   The one word that is someone else's, a file's name, is drawn by the rule a told row's name is drawn by
   (`layout/told.rs`, `plain` and `one_line`: no blank, unseen or control character, whole on one line), or
   left out. There is one such rule in the kit, called from both places.
8. **Three inks, from the theme.** `palette.pencil`, `palette.added` and `palette.taken`, with values in both
   built-in themes. No existing role meant the same: `soft` is text, `accent` is the page's call to act and
   `quoted` is someone else's words.
9. **The print names what a picture counts.** `sticker_kit::print(placed)` writes one line a piece: for a
   picture, what it is and counts (`track steps=7 stage=making file=hello.py`), its parts with their numbers,
   and every caption; the words only when they are placed. Tests read parts and captions, never pixels.

## Open

- **The app builds none of these notes.** `Note::Drawn` reaches no page yet; the next slice turns core's data
  into pictures and decides what a tap on a picture opens.
- **Nobody has seen the strokes in a real window.** The picture files of the snapshot example are the only
  look so far. How the drawing reads at speed, whether a second and a half is right, and whether the two passes
  read as graphite on a real display are not known.
- **Line counts for `Bars` do not exist in core.** The picture is built and tested on made-up rows.
- A theme file that sets none of the three inks gets the sticker theme's, with nothing said, and they are held
  to no floor of contrast on its paper.
- Captions are placed whole from the first frame: only the strokes draw themselves.
- A file's name too long for its line is left out of a `Track` and is `a file` in `Bars`; it is never cut.
- Windows and Linux were not run.
