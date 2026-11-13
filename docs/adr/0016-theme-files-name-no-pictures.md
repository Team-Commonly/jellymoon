# 0016 · Theme files name no pictures

Status: Accepted, by the coordinator's ruling, 2026-11-12, when PR 154 was closed without merging. It supersedes
[ADR 0015](0015-the-art-door-reads-with-the-renderers-parser.md).

## Decision

A theme file is colours, fonts, paper, sizes and motion. It names no pictures. The `[art]` table an older theme file
has (`hero`, `character`, `star`, `bubble`, `badge`) is not read: no file it names is opened, looked for or
resolved, by `Theme::load`, by `layout`, by the view or by the picture example. A file that still has the table
loads, with one warning through the path warnings already take: `art: pictures in a theme are not read in this
version; the kit's own art is drawn`. Only the text of each name is looked at, and one that leaves the theme's
folder still refuses the theme, which is how F32's step is kept.

Corrected 2026-11-13, by the coordinator's ruling on the review of PR 156: the last sentence no longer holds. No
name under `art` is looked at, and no theme file is refused for what its `art` names. The check guarded nothing,
since nothing named is opened; it refused one spelling and loaded another, and answered differently on a Mac and
on Windows. `art` in any shape, naming anything, is not read, is said in the one warning, and the file loads the
same on every system. F32's step 4 is replaced by feature F49.

The art on a page is the kit's own, built in code (`crates/sticker-kit/src/art.rs`). The window's process now
parses no picture it did not draw itself.

## Why

A picture from a theme was drawn by GPUI's renderer in the window's own process. The door in front of it tried
to say, before the draw, which pictures the renderer would draw quickly and without a fault. Four reviews after a
merge each found a picture the door took and the renderer choked on: slow to draw, or deep enough to end the
process. Each fix was a new count or limit at the door (a byte cap, an allow-list, a counted cost, then the
renderer's own parser, ADR 0015), and each review found the next picture. The door was about 630 lines with
about 1360 lines of tests, and it was still a guess about another program's cost.

The design that does not guess was built in PR 154: the picture is drawn once in a child process, and taken only
when the child drew it under a measured cost. It was closed on its fifth round with two faults in CI on its head
`bdfc909`. On Linux, a process's CPU time counts the meter's own 2 ms sampling thread, so a slow picture was
refused for processor time after 1.39 s where the test wanted the wall-clock limit. On Windows a dear picture
came back `unsure` and was never kept. Both are faults of measuring on three systems, not of the idea.

Nobody has asked for a theme with its own pictures. Until someone does, the smallest safe thing is to take none.

## What was removed

- `theme/svg_check.rs` and `theme/svg_text.rs`, the art bounds, `ArtCost`, `art_cost`, `svg_refusal`, every
  `MAX_ART_*`, `MIN_ART_*`, `ROW_PAINT` and `SVG_*` name, `Theme::art`, `Theme::art_svgs`, `Theme::folder`,
  `ThemeError::RefusedSvg`, the second reading at draw time and the branch of `draw/sprites.rs` that drew a
  theme's picture.
- `tests/cost.rs`, `tests/limits.rs`, `tests/alike.rs`, `tests/art_draw.rs`, and the door's cases in
  `tests/theme.rs`, `tests/draw.rs` and the two F32 tests.
- `usvg` and `svgtypes` from the kit's dependencies and the workspace's. GPUI still builds both for its own
  drawing, so `Cargo.lock` keeps the packages and loses only the two lines that named them under `sticker-kit`.

## When it comes back

When someone wants sticker packs: pictures a person adds to the kit. The design to start from is the trial, not
the door: PR 154 (closed), the branch `art-trial`, the text of that branch's own ADR 0016 (the trial in a child,
accepted under a measured cost), and the four findings of that PR's first gate. The two CI faults above are the
first things to settle. ADR 0015 and the kit README before this change say what the door held and what each
limit was measured at.
