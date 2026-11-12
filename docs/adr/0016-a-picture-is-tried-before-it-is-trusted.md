# 0016 · A picture is tried before it is trusted

Status: Accepted, by the coordinator's ruling, 2026-11-13, after the review of PR 147. Amended the same day after
the gate on PR 154: the child has a clock of its own, draws on a stack of the page's size, and accepts a picture
only under a stated cost.

## Decision

The first time the kit sees a theme's picture, another process reads it and draws it, on a stack no larger than
the page's, under a limit of time, and measures what that cost in processor time and memory. Only a picture that
process has accepted is ever read by the XML or SVG parser, or rasterised, in the process that owns the window. A
picture that costs more than the stated cost, or whose trial runs over, crashes, overflows its stack, ends with a
code or says anything but the one line of an answer, is refused with one line, and the kit's own art is drawn for
its piece.

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
alone, which a program needs to start). No argument but the one that selects it and, last, the limit in
milliseconds. Its standard error goes nowhere.
On Windows it is started with `CREATE_NO_WINDOW`, so no console opens. The standard library marks the handles it
opens close-on-exec, and gives a child only the three it is told to; a handle another library opened without that
mark would be inherited, and nothing here closes those: that would take `unsafe`, which the workspace forbids.

**What the child does** (`theme/stand.rs`). Before it reads a byte it starts a clock of its own, which ends the
process with code 124 at the limit: the last argument, and never more than `TRIAL_LIMIT`. A child whose caller was
ended, or that nobody watches, so lives at most the limit (`tests/trial_clock.rs`). The caller still ends it and
waits for it, as before. Then it reads at most 64 KiB and a byte, starts its meter, holds the text to the written
rules (`art_cost`), and draws it with `gpui::SvgRenderer::render_single_frame` at a scale of 1, which is the call
GPUI makes for an SVG image and draws at twice the picture's declared size. A kit built without GPUI has no
renderer and draws no page; its child reads only, and its answers are kept under another key.

**The stack.** The rules are read and the picture is drawn on a thread of `PAGE_STACK`, 512 KiB, and not on the
child's main thread, which has 8 MiB on macOS and Linux. GPUI rasterises a picture on its background executor, and
512 KiB is the smallest stack any system gives that:

| | the thread that draws a page's picture | its stack |
|---|---|---|
| macOS | a queue of the system's (libdispatch) | 512 KiB |
| Linux | threads the standard library starts | 2 MiB |
| Windows | the system's thread pool | the program's own, 1 MiB |

So a picture that would end the page on its stack ends the child first. The gate's picture, 1023 arcs to one
place, is refused by a rule; given to a child that holds it to no rule, it ends that child on its stack and the
caller lives (`tests/ordinary.rs`).

**What the parent trusts** (`theme/trial.rs`): one line of at most 1024 bytes and an exit code of 0. The line is
`accepted <numbers> <paint> <pixels drawn> <ms of processor time> <KiB of memory>`, `refused: <one line>`, or
`unsure: <one line>` when the child could not try the picture here and now. With a renderer, an answer that drew no
pixel is none. It reads no more than that of what a child writes, on a thread of its own, and writes the picture
from another, so a child that never reads or never stops writing holds nothing up. At the limit the child is
killed and waited for. Nothing of a trial is left running when `try_art` returns: `tests/trial.rs` has a child that
never ends hold a file lock, and takes the lock itself the moment the trial has answered.

## What a picture may cost

A picture that ends can still cost the page dear at every draw. The gate's own: pairs of curves under a stroke
far wider than they are long, every number inside the rules. Sixty pairs took 1.0 s and 1.5 GB in a release and
were accepted, since they ended inside the limit. So the child measures its own reading and drawing
(`theme/meter.rs`) and accepts a picture only under both of:

| | the most | the heaviest picture known inside the rules | the lightest dear picture measured (30 pairs) |
|---|---|---|---|
| Processor time, release | 100 ms | 8 ms: 12 times under | 381 ms: 3.8 times over |
| Processor time, debug build | 2000 ms | 551 ms: 3.6 times under | 9474 ms: 4.7 times over |
| Memory, release | 32 MiB | 2.5 MiB: 13 times under | 87 MiB: 2.7 times over |
| Memory, debug build | 32 MiB | 4.6 MiB: 6.9 times under | 75 MiB: 2.3 times over |

An ordinary picture (`tests/ordinary.rs`, the standing set) takes under 1 ms and 1.2 MiB in a release, and at most
10 ms and 3.5 MiB in a debug build. Measured on one Mac; the kit's README has the table by picture. What is over is
`refused: drawing it took more than 100 ms of processor time`, or `… 32 MiB of memory`, said as soon as it is
passed: the draw is not waited for. With the costs on, 60 and 100 pairs are each refused in about 2 s in a debug build, a tenth of the limit.

**Neither number moves with load.** Processor time is what the child's own threads used, and memory is what it
holds, however busy the machine is. So a refusal for cost is an answer about the picture and is kept, as a rule's
is, and it is the cost, not the clock, that says what a picture may take.

**How they are measured, and where not.**
- Processor time: the process's own clock, `clock_gettime(CLOCK_PROCESS_CPUTIME_ID)` through `rustix`, on every
  Unix. Windows has no such call a program may make without `unsafe`: there the time on the wall since the meter
  started bounds it from above, and a picture over by that measure is `unsure`, refused for this run and not kept.
- Memory: what the process holds resident, through `sysinfo`, less what it held before a parser read the
  picture, asked every 2 ms and once more when the draw has ended. A peak given back between two asks is not seen.
- A child that cannot measure refuses: `unsure: its trial could not measure its own memory`. Nothing is drawn
  unmeasured.

**Dependencies.** `sysinfo` 0.31.4, with its `system` feature alone, is new to the kit and not to the workspace:
a package GPUI brings (`zed-scap`) already names that version in `Cargo.lock`, which gains the one line that names
it under the kit. It is what
reads a process's memory on all three systems without `unsafe` here. `rustix`, which core already depends on, is
now named for every Unix (it was Linux alone) with `time` beside `process`.

**The limit of time** is `TRIAL_LIMIT`: 4 s in a release, 20 s in a build with debug assertions. It was 2 s in a
release when it was the only bound, and was reconsidered now that the cost is. The clock no longer says what a
picture may take. It is for what the meter does not see: a child that waits, one that does not get its turn on a
busy machine, and a program's first start, which took 0.44 s and once over 1 s in a release. So it is 40 times
the processor time a picture may take in a release and 10 times in a debug build, and a trial it ends says nothing
of the picture: `its trial took longer than 4000 ms and was ended, and is tried again at the next start`, and
nothing is kept. Wall time, child's start included, on one Mac with ten cores and a load of 10 to 18 from other
work:

| | debug | release |
|---|---|---|
| The heaviest theme known inside the rules, five trials (64 see-through round strokes 600 wide, each of 12 dots) | 6.5 s | 0.55 s |
| The heaviest one trial of those | about 1.4 s | about 0.11 s |
| The five heaviest themes the kit wrote for itself, five trials each | 0.95 to 3.1 s | 64 to 121 ms |
| A picture a rule refuses | 13 to 350 ms | 8 to 10 ms |
| A program's first start after it was built | 2.4 to 4.8 s | 0.44 s, and once over 1 s |

The app tries with its own running program, which has been started already; the kit's tests start their helper
once before a trial counts its time. A test's build reads the limit in milliseconds from
`STICKER_KIT_TEST_TRIAL_MS`; a release ignores it.

With the rules switched off (`tests/trial.rs`, a child that only draws), under a limit of 1.5 s: the stroke
thousands of times wider than its curve and the percent of a huge `viewBox` were ended at the limit in a debug
build and in a release alike; the run of 60000 closes ended its child on its stack in 10 to 57 ms; the ragged
fills were ended at the limit in a debug build and drawn in time in a release. With the rules on each is refused
by a rule, in 8 to 350 ms.

**Memory the system holds.** On Linux the child also limits its own address space to 2 GiB before it reads a byte
(`setrlimit(RLIMIT_AS)` through `rustix`). macOS does not enforce that limit, and Windows has none short of a job
object, which needs `unsafe` or a new dependency. On those two the meter is the bound: it ends the child within
one ask, 2 ms, of 32 MiB being passed.

**One at a time.** A process runs one trial at a time, whichever thread asks, and a theme's pictures one after
another.

**When the child cannot be started** the picture is refused ("its trial could not be started"). It is never read
in the caller's process instead.

## Kept answers

A caller may name a folder (`Trial::kept`; the app's is `art-trials` in Jellymoon's own folder). An answer is kept
there in a file named by a hash of the key and the picture, holding the key, the answer's line and the whole
picture. The key names the kit's version, the parser's (`theme::USVG`, held to `Cargo.lock` by `tests/alike.rs`),
what the child did and the size it drew at, the build (`debug` or `release`, since the costs differ), and the
caller's own stamp (`Trial::stamp`; the app's is `jellymoon-app` and its version), since another version of a
caller may draw with another renderer. A kept file is the answer only when its key is this build's and its picture
is this picture, byte for byte: the 64 bits of the name only find the file. Anything else in the folder is ignored
and the picture is tried.

**What is kept** is an answer about the picture: `accepted`, and a refusal a child said and ended well after,
which is a rule's, the renderer's or the cost's. Those hold whenever the picture is asked again. What says nothing
of the picture is not kept: a trial a clock ended, one that could not start, a child the system ended or that
ended with a code, and `unsure`. On a busy machine the next may go otherwise, so each is tried again at the next
start. A folder holds 64 answers at most and is emptied of them before a 65th.

**The folder is the person's alone** (`theme/kept.rs`). It is a folder and no link to one, asked of the name
itself, and a link in its place is not followed: nothing is read or kept and every picture is tried. On Unix it is
made with mode 0700 and set to it when it is found more open, and an answer is a plain file made with mode 0600
beside its name and moved onto it. Windows has no modes; the folder is under the person's own profile there.

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

- **A peak of memory between two asks.** The meter asks every 2 ms; what is taken and given back inside that is
  not seen. Linux has the system's own limit of 2 GiB behind it, macOS and Windows have none.
- **Processor time on Windows** is bounded by time on the wall, so a picture refused there for time is not kept
  and is tried at every start.
- **A child whose caller is gone goes on to its limit**, 4 s at most in a release. It does not end the moment its
  caller's pipe closes: once it has the picture it reads nothing more.
- **The Linux and Windows halves were not compiled into a running program on the Mac this was written on**: the
  Linux limit, the meter on both, and the modes of a folder on Linux. CI is the first to run them.
- **The first trial of a program the system has not started before** can take seconds on macOS (the table
  above). The app tries with its own running program, which the system has already looked over. A trial that
  runs over for that reason is not kept, and is tried at the next start.
- **A machine so busy that an honest picture runs over the clock.** The picture is refused for that run, said in
  one line, and tried again at the next start. Its cost does not move with the load.
- **The kept folder between the look and the use.** Whoever can swap the folder for a link after it was looked at
  can already write in the person's own folder.
- **Handles** another library left inheritable, as above.
- **A picture that passes its trial and costs more in the window** than it did in the child. The child makes the
  call the page makes, on the same bytes and on a stack no larger than the page's; the page then scales the image,
  which the trial does not.
