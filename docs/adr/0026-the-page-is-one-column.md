# 0026 · The page is one column

Status: Built, 2026-11-23, the first of three slices. Proved at the API by `app/tests/one_column.rs` and in the
kit's headless view by `crates/sticker-kit/tests/column_draw.rs`. Not seen in a real window. It replaces the
arrangement of the page that F28, F32 and F37 described (a canvas of two columns scaled to the window) and
changes [ADR 0022](0022-the-page-moves-on-by-a-tap.md) and [ADR 0023](0023-a-runs-work-is-told-as-a-result.md)
only in where a note stands, never in what it says or what a tap on it does.

## Why

The page was a fixed canvas of 1440 by 900 scaled to its window: a grid of characters on the left, a narrow strip
of notes on the right, the entry over the big character. Everything a wish made had to fit that strip at once, so
the layout shortened lists, closed gaps, wrote options smaller, laid the notes out a second time for a result,
and at the end left notes out and counted them. Each of those was a rule with a test, and each made a page a
person had to learn to read: where is the thing I just typed, which note is its answer, what was dropped.

A person already knows how to read one column: what I said, then what came of it, in order, the newest at the
bottom, and the place to say the next thing under it.

## Decisions

1. **One column, read from its top.** The header (the date, the title, the reading's footer as one quiet line),
   then a row for each wish nobody typed here, then the notes in order. A note is a card of one width
   (`metrics.note_width`, 760 design units) at its full height. The column is as long as what it holds.
2. **Nothing gives way for room.** No card is moved beside another, shortened, written smaller or left out
   because the page is long, and no layout is tried twice. What bounds a card is what bounds its own text, and
   it is the same at every window size (the kit's README lists each bound).
3. **What a person typed is on the page as theirs.** `Note::Said { id, text }`: a bubble at the column's right,
   drawn by the rules the kit draws every quoted text by, never as the page's own words. The wish's character
   stands small at the bubble's end with its state and stars. A tap on the bubble or on the character is the tap
   on that wish's character, as before.
4. **The app orders the column; the kit only lays out what it is given** (`app/src/column.rs`, pure). The day's
   own notes first; then each made wish, oldest first: its `Said`, `typed-<wish>`, then its notes in one fixed
   order of their ids' prefixes (`column::ORDER`); last the notes of a wish that is not made yet (ADR 0025).
   `scene::ruled`, the core calls and every tap are unchanged:
   the column is a reordering of the same `Scene`, applied where the scene reaches the layout
   (`lines::lines`, `window`).
5. **The entry is the window's, not the column's.** A band at the window's bottom holds the field and its
   chips. Every window keeps it, entry or none. The column is shown over it only: painted in a layer clipped
   at the band's top, so a card is never drawn or tapped under the entry.
6. **Scrolling is the view's, and the layout is pure.** A piece's place is the column's own (`y` from the
   column's top), which is what `layout` prints. `Page` keeps a `Scroll` (`crates/sticker-kit/src/scroll.rs`):
   the wheel moves the column between its top and its end; a page opens at its end; a page at its end stays
   there when the column grows, and one a reader moved stays where it was.

## What was deleted, and why it existed

| Deleted | Why it was there |
|---|---|
| the grid of characters (`layout/grid.rs`), `metrics.columns`, `row_gap`, `hero_x`, `hero_top` | to fit a day's wishes left of the notes on one canvas |
| the notes planner: closing gaps, two-line options, options written at 0.85 and 0.72, a list cut to the rows that fit, `MAX_NOTES`, the `+N more of M notes` sheet | to fit a strip of notes in one window's height |
| the told note's second layout (`told::guarded`, `told::in_turn`) | so a result never cost another note its room |
| a whole list making room for itself, and going without a word | the same, for the made wishes' list |
| the legend (`Scene::legend`, `LegendItem`, `PieceKind::Legend`) | to explain marks at the foot of a canvas; the footer stays, under the title |
| the entry's band under the title, and the big character standing under it | the entry was the first thing on a canvas read from its top |
| the made wishes' list, "wished here today", on the page | each made wish is now its own bubble in the column; `scene::ruled` still builds the list and the column replaces it |
| the tests that pinned only those | listed in `progress/` for this unit |

Kept, with their tests: every rule about text that is not the page's own (the still door, quoted words as quote
pieces and nothing else, the marks beside a quote, the cut lengths, no option under a body or title that is cut,
a told row whole or counted).

## Open

- **Turns and holds remain until the next slice.** A note still appears and goes by the app's present rules
  (what a look carries over, the hold after a choice); this slice changes only where a note stands. A wish of
  the reading has no `Said`: it is a row under the header, and its note when opened stands with the day's own.
- **Scrolling was not seen in a real window.** The headless view proves the wheel, the clip and the taps
  through GPUI's own dispatch; no test reaches a trackpad's momentum, a window resized while scrolled part way
  on a real display, or how the motion reads.
- **The day's own notes stand at the column's top**, so on a long day a clash a person must rule on can be out
  of view while the page is held at its end. The next slice decides where such a note goes.
- No key scrolls the column, and there is no scroll bar.
- Windows and Linux were not run.
