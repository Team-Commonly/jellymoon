# 0015 · The art door reads a picture with the renderer's parser

Status: Accepted, by the coordinator's ruling, 2026-11-12, after the review of PR 142.
[ADR 0013](0013-the-shell-dependencies.md) lists what the kit depends on. This note adds one line to that list.

## Decision

The kit depends on `usvg` directly, at the one version GPUI builds: `usvg = "=0.46.0"` with no default features.
The art door (`crates/sticker-kit/src/theme/svg_check.rs`) parses a theme's SVG with it and judges the tree it
makes, which is the tree GPUI hands to `resvg` to draw.

## Why

One parser, not two. The door used to read a file with a tag scanner of its own and count what that scanner saw.
Its counts bound drawing time only while the renderer draws the picture the door counted, and three reviews in a
row found a file the two read differently: a name in another case, a `viewBox` only one of them could read, an
arc's flags, and last a processing instruction named `XML` that hid a whole picture from the door (drawn 8192
pixels a side, 9.7 s, counted as nothing). Each was closed by teaching the scanner one more rule of the renderer.
A second reading of SVG is never finished. The renderer's own is.

The same change takes the door's counts from the tree. A `circle` or a rounded `rect` is counted as the path the
renderer draws it as, a shape's reach is its box with its stroke after every transform, and a stroke's width is
the drawn one. The scanner knew none of these: it counted the numbers in a `d`.

## What it replaces

The tag scanner and its rules in `theme/svg_check.rs`, and `theme/path_cost.rs`, which followed a `d` command by
command. `tests/alike.rs`, which held one case for each way the two readings had differed, now holds the door
against `usvg` called as GPUI calls it.

## What it costs

Nothing is added to the build: `usvg` 0.46.0 was in `Cargo.lock` already, compiled for GPUI. The lock file gains
one line, `usvg` among the kit's dependencies. The kit names it without default features, as GPUI does.

The kit without its `gpui` feature (ADR 0013, decision 2) now builds `usvg` and the few packages under it, since a
theme is loaded and its art judged there too. It still builds no GPUI. Core and the `jellymoon` binary depend on
neither.

## The pin, and what fails when it drifts

GPUI has no versioned release (ADR 0013), so its `usvg` moves when the pinned commit moves. The kit asks for
`=0.46.0` exactly. If GPUI moves to another version, cargo keeps both, and
`tests/alike.rs::the_door_and_the_renderer_build_one_version_of_the_parser` fails: it reads `Cargo.lock` and holds
it to one `usvg` package, at the version `resvg` has. Moving the pin is then one edit in `Cargo.toml`, with
`tests/alike.rs`, `tests/cost.rs` and `tests/art_draw.rs` run again, since the limits were measured on this version.

## What stays different between the door's parse and the renderer's

Only what the door turns off, each so that less is done, never so that a file is read another way:

- **A DTD.** The renderer's parse allows one and expands its entities. The door's refuses the file.
- **Images.** The door's options resolve no image, from a file or from data. No file gets that far: `image` is not
  on the list of elements.
- **Fonts.** The renderer loads the system's fonts when a picture holds text. `text` is not on the list either,
  so neither parse asks for one.
- **Compressed files.** The renderer unpacks a gzip file. The door reads text: such a file is not UTF-8 and is
  refused before the parse.

A file the door takes holds none of the four, so the two parses make the same tree of it.
