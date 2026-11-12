# 0016 · A picture is tried before it is trusted

Status: Accepted, by the coordinator's ruling, 2026-11-13, after the review of PR 147.

## Decision

The first time the kit sees a theme's picture, another process reads it and draws it, under a limit of time. Only
a picture that process has answered for is ever read by the XML or SVG parser, or rasterised, in the process that
owns the window. A picture whose trial runs over, crashes, overflows its stack, ends with a code or says anything
but the one line of an answer is refused with one line, and the kit's own art is drawn for its piece.

The written rules of [ADR 0015](0015-the-art-door-reads-with-the-renderers-parser.md) stay, as the first thing the
trial does. They are the cheap refusals with a line a person can act on. They are not the proof.

## Why

Four reviews in a row found a picture the rules took and the renderer choked on: a run of closes that overflowed
the stack, a ragged fill at 7.6 s, and in the review of PR 147 a stroke thousands of times wider than its curve
(3.1 s and 1.5 GB in a release, for 60 cubics), a percent of a `viewBox` of 1e36 (a second an ellipse), and an
element of 64 KiB of attributes. Each was closed with one more rule read off the parser's source. The parser, the
stroker and the rasteriser are three libraries nobody here wrote, and ADR 0015 lists what of them was not read. A
list of rules bounds what was thought of. A trial bounds what happens.

## What it replaces

- `Theme::load` reading each picture with the parser in the caller's process, and refusing the whole theme for
  one. A theme now loads whatever its pictures hold, and each picture has its own answer.
- `Theme::art_svgs`, which read and judged the files a second time in the window's process when they were drawn.
  The text a trial accepted is the text drawn (`theme::Accepted`, which only `Theme::try_art` makes).
- The rules as the bound on drawing time and on the stack. The pricing of a picture (`MAX_ART_NUMBERS`,
  `MAX_ART_PAINT` and what feeds them) is still run and still sums a theme's five files; it is left for the
  removal-only change that follows this one, since the trial now holds what it was for.
- The count of every arc in a run (`MAX_ART_IDLE`): only an arc that goes nowhere is counted.

## How a trial runs

**The child is the caller's own program, started again.** The kit takes the program's path and the argument that
selects the trial as plain data (`theme::Trial`), and gives the caller one function to call first in `main`
(`theme::stand_trial`). `jellymoon-app` passes its own `current_exe()` and `--try-art`. No second program is
shipped, and none is looked for on `PATH`.

**What the child is given.** The picture on its standard input. An empty environment (on Windows `SystemRoot`
alone, which a program needs to start). No argument but the one that selects it. Its standard error goes nowhere.
On Windows it is started with `CREATE_NO_WINDOW`, so no console opens. The standard library marks the handles it
opens close-on-exec, and gives a child only the three it is told to; a handle another library opened without that
mark would be inherited, and nothing here closes those: that would take `unsafe`, which the workspace forbids.

**What the child does** (`theme/stand.rs`): reads at most 64 KiB and a byte, holds the text to the written rules
(`art_cost`), and draws it with `gpui::SvgRenderer::render_single_frame` at a scale of 1, which is the call GPUI
makes for an SVG image and draws at twice the picture's declared size. A kit built without GPUI has no renderer
and draws no page; its child reads only, and its answers are kept under another key.

**What the parent trusts** (`theme/trial.rs`): one line of at most 1024 bytes, `accepted <numbers> <paint> <pixels
drawn>` or `refused: <one line>`, and an exit code of 0. With a renderer, an answer that drew no pixel is none. It reads no more than that of what a child writes, on a thread of
its own, and writes the picture from another, so a child that never reads or never stops writing holds nothing
up. At the limit the child is killed and waited for. Nothing of a trial is left running when `try_art` returns:
`tests/trial.rs` has a child that never ends hold a file lock, and takes the lock itself the moment the trial has
answered.

**The limit** is `TRIAL_LIMIT`: 2 s in a release, 20 s in a build with debug assertions. Measured on one Mac
with ten cores and a load of 10 to 18 from other work, child's start included:

| | debug | release |
|---|---|---|
| The heaviest theme known inside the rules, five trials (64 see-through round strokes 600 wide, each of 12 dots) | 6.5 s | 0.55 s |
| The heaviest one trial of those | about 1.4 s | about 0.11 s |
| The five heaviest themes the kit wrote for itself, five trials each | 0.95 to 3.1 s | 64 to 121 ms |
| A picture a rule refuses | 13 to 350 ms | 8 to 10 ms |
| A program's first start after it was built | 2.4 to 4.8 s | 0.44 s, and once over 1 s |

So the limit is 14 times the heaviest honest trial in a debug build and 18 times in a release. A second in a
release was tried first and refused an honest picture: the test helper had just been built, and macOS took over a
second to start it the first time. The app tries with its own running program, which has been started already;
the kit's tests start their helper once before a trial counts its time. A test's build reads the limit in
milliseconds from `STICKER_KIT_TEST_TRIAL_MS`; a release ignores it.

With the rules switched off (`tests/trial.rs`, a child that only draws), under a limit of 1.5 s: the stroke
thousands of times wider than its curve and the percent of a huge `viewBox` were ended at the limit in a debug
build and in a release alike; the run of 60000 closes ended its child on its stack in 10 to 57 ms; the ragged
fills were ended at the limit in a debug build and drawn in time in a release. With the rules on each is refused
by a rule, in 8 to 350 ms.

**Memory.** On Linux the child limits its own address space to 2 GiB before it reads a byte
(`setrlimit(RLIMIT_AS)` through `rustix`, which core already depends on; the kit names it for Linux alone and the
lock file gains one line). macOS does not enforce that limit, and Windows has none short of a job object, which
needs `unsafe` or a new dependency. On those two the limit of time is the only bound: the worst picture a review
measured grew by about half a gigabyte a second, so about a gigabyte before it is ended.

**One at a time.** A process runs one trial at a time, whichever thread asks, and a theme's pictures one after
another.

**When the child cannot be started** the picture is refused ("its trial could not be started"). It is never read
in the caller's process instead.

## Kept answers

A caller may name a folder (`Trial::kept`; the app's is `art-trials` in Jellymoon's own folder). An answer is kept
there in a file named by a hash of the key and the picture, holding the key, the answer's line and the whole
picture. The key names the kit's version, the parser's (`theme::USVG`, held to `Cargo.lock` by `tests/alike.rs`),
what the child did and the size it drew at. A kept file is the answer only when its key is this build's and its
picture is this picture, byte for byte: the 64 bits of the name only find the file. Anything else in the folder is
ignored and the picture is tried.

Only what a child said before it ended well is kept. A trial that ran over, could not start or was ended by the
system is not: on a busy machine the next may go otherwise. A folder holds 64 answers at most and is emptied of
them before a 65th.

A kept answer gives no new trust. Whoever can write that folder can write the theme file and Jellymoon's own
files too, and a forged "accepted" is what a hostile picture would have been with no trial: the harm the trial is
there to stop, for a person who already has the machine.

## What a person sees

The page is drawn with the kit's art from the start. The theme's pictures replace it once all of them have been
tried: at once when the answers are kept, after the trials the first time. The trials run off the window's thread
(`app/src/window.rs`, `arts`). A refused picture is one line on stderr, `jellymoon-app: theme: the SVG for art.star
is refused: <why>; the built-in art is drawn`, and the run goes on. `jellymoon-app layout --theme` tries the art
too and says the same lines, so a theme can be looked at before a window is opened on it; it exits 0.

## There is no side door

`tests/trial.rs::no_shipped_code_reads_or_draws_a_picture_but_the_child_and_the_one_place_given_accepted_ones`
reads the kit's and the app's sources: the rules are called only from the child, the renderer only in the child,
and GPUI is handed SVG bytes in one function (`draw/sprites.rs`), which takes `Accepted` pictures and the kit's own
drawings. The kit's own art is built in code from a theme's checked numbers and does not come through a trial.

## What is not closed

- **Memory on macOS and Windows**, as above.
- **The Linux limit was not compiled or run on the Mac this was written on**: no Linux target is installed there.
  CI builds and runs it.
- **The first trial of a program the system has not started before** can take seconds on macOS (the table
  above). The app tries with its own running program, which the system has already looked over. A trial that
  runs over for that reason is not kept, and is tried at the next start.
- **A machine so busy that an honest picture runs over.** The picture is refused for that run, said in one line,
  and tried again at the next start.
- **Handles** another library left inheritable, as above.
- **A picture that passes its trial and costs more in the window** than it did in the child. The child makes the
  call the page makes, on the same bytes; the page then scales the image, which the trial does not.
