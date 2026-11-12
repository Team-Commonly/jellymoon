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

## What the parser does by the length of a value (added 2026-11-12, after the review of PR 145)

**Superseded in part by [ADR 0016](0016-a-picture-is-tried-before-it-is-trusted.md), after the review of PR 147.**
The rules below are kept as the cheap first filter, with a line a person can act on. They are no longer what makes
a picture safe to read and draw: that review found three more shapes the table did not hold, and a fourth review
would find a fifth. A picture is now tried in a process of its own, and these rules run inside it.

Sharing the parser closes every difference between the door and the renderer. It does not make the parser safe to
call. The door calls it on a file nobody has judged yet, and a parse that ends the program cannot be judged
afterwards: `M0 0L1 1` followed by a few thousand `z` overflowed the stack inside `Tree::from_xmltree`, in the
door and in the renderer alike. The lists of elements and attributes bound what is built by the count and nesting
of elements. They do not bound what one value makes the parser do.

So `svgtypes` 0.16.2 and `usvg` 0.46.0's conversion were read for everything they do by the length or size of a
value, and each is held at the door before the parse (`crates/sticker-kit/src/theme/svg_text.rs`):

| What the parser does | Where | Held by |
|---|---|---|
| Calls itself once for every path segment it draws nothing for: a close after a close | `svgtypes` `SimplifyingPathParser::next` | at most 32 in a row (`MAX_ART_IDLE`) |
| The same for an arc `kurbo` makes no curve of (its ends closer than about 1e-16 of its radius), unless it follows a close | the same, and `kurbo` 0.13.1 `Arc::append_iter` | counted in the same run when its ends are closer than a millionth of its larger radius and it does not follow a close. The door first counted every arc, which refused a scalloped edge of forty honest ones (the review of PR 147) |
| Makes an arc of a number of curves that grows with its radius, all at once: 1.5 million at a radius of 1e36 | the same | a written number is at most 1000000 (`MAX_ART_WRITTEN`), and an arc's radii are within 1000 times of each other (`MAX_ART_FLAT`), since the parser makes the smaller as much larger as the other is. Radii of 1000000 and 1 between far ends were 81 curves. This is no count of an arc's curves: relative moves add up to a place near 2e9, and an arc from there was 80 (`tests/limits.rs` pins it). What a path comes to is held by the count of commands, the count of numbers on the tree, and the trial |
| Takes those curves from the front of a list one at a time | `svgtypes`, `buffer.remove(0)` | the same |
| Makes a `circle`, an `ellipse` and a rounded `rect` of the same arcs by their written size: four million curves at `r="3e38"` | `usvg` `shapes.rs` | the same limit on `x y x1 y1 x2 y2 cx cy r rx ry width height`, and none of them a percent, which is a part of the `viewBox` and not of the number written: `rx="100%"` of a `viewBox` of 1e36 was a second an ellipse (the review of PR 147). The `viewBox`'s four numbers are held to the same limit |
| Walks every command of every `d` and every pair of `points` | `svgtypes` `PathParser`, `PointsParser` | at most 2048 commands a file (`MAX_ART_COMMANDS`), within the 16 KiB of `MAX_ART_PATH_BYTES` |
| Runs the stroker over each stroked path while the tree is built, twice under a slant | `usvg` `Path::calculate_stroke_bbox` | the same counts, and `stroke-width` held as a written number. The stroker is not linear in the path's segments under a stroke thousands of times wider than its curve: 60 cubics at `stroke-width="2e9"` under `scale(1e-6)` took 3.1 s and 1.5 GB in a release (the review of PR 147). Whether it ends soon on every width the limit allows was not read: the trial holds it |
| Calls itself once for each level of nesting, 3.2 KiB a level in a debug build | `usvg` svgtree and converter | 12 levels (`MAX_ART_DEPTH`), counted before the parse |
| Compares every stop of a gradient with every other | `usvg` `convert_stops` | 100 elements a file |
| Compares every attribute and namespace declaration of an element with every other | `roxmltree` | at most 40 `=` between one `<` and the next (`MAX_ART_ATTRIBUTES`), counted on the text before the XML is read. 64 KiB of declarations on one root was 0.3 s to read in a debug build (the review of PR 147) |

Read and found to be plain loops over the value, bounded by the file's 64 KiB: numbers and lengths, a transform
list, a colour, a paint and its `url(`, a `viewBox`. Not reachable: the loops that follow an `href`, which is not
on the list of attributes. Not read: `roxmltree` itself beyond what a review measured, the text modules (no `text` passes), and whether the
stroker's own loops always end on the largest numbers the door lets through. What was not read is why the rules
are not the proof (ADR 0016).

The door walks a `d` with `svgtypes::PathParser`, the library's plain tokenizer, which the parser itself reads a
path with and which is a loop. So there is still one reading of a path: the door asks the same library what the
segments are, and counts them. It compares an arc's two radii, and the distance between its ends with the larger of them, and does no other
arithmetic on one: an arc whose ends are closer than a millionth of its radius is taken for one the parser draws
nothing for, ten orders above where `kurbo` in fact makes no curve. A run of arcs that go somewhere is not counted,
however long.

This is a second dependency by name and none by weight: `svgtypes = "=0.16.2"` is the version `usvg` 0.46.0
builds, already in `Cargo.lock`, and the lock file gains one line. `usvg` does not hand it on, so the kit names
it. The test that holds `usvg` to one version holds `svgtypes` to one too.

`tests/limits.rs` has the table of every segment and whether the parser draws for it, and pins the review's
file. `tests/art_draw.rs` reads and rasterises the worst files the door takes (the longest runs, the most
commands, the deepest nesting) on a thread with a stack of 256 KiB, in a debug build. Measured there: the run that
overflows such a stack is 96 long (80 does not), and the nesting that does is 80 deep (60 does not).
