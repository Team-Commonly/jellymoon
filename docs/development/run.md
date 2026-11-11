# A run: `jellymoon run`

A run is the person's own Claude Code, started headless on one made wish, in a copy of the wish's repo (F39,
[ADR 0014](../adr/0014-a-run-is-started-headless.md)). The rules are core's, in `crates/jellymoon-core/src/run.rs`
and `run/` (`child.rs` the child, `stream.rs` what is read, `end.rs` how an end is written down, `dead.rs` a stop). The copy is [copy.md](copy.md)'s, and the wish is [wish.md](wish.md)'s.

```
jellymoon run --wish <id> [--date <YYYY-MM-DD>] [--repo <folder>]
```

It waits until the run has ended, prints one line of JSON and exits 0. (One end exits 1 with a line on stderr: the
run ended, and the day stayed locked. See "The end is written in the record first".) With no `--date` the day is today.

```json
{"run":"r0123456789abcdef","wish":"m1","outcome":"completed","star":"a run reported it finished","denied":0,"why":"run r0123456789abcdef finished: its work is one commit on the branch jellymoon/run-r0123456789abcdef"}
{"run":"r0123456789abcdef","wish":"m1","outcome":"failed","star":null,"denied":1,"why":"run r0123456789abcdef did not finish (its result was not a success): its copy is kept and no star was added"}
{"run":null,"wish":"m1","outcome":"not_started","star":null,"denied":0,"why":"nothing was started: this Claude Code is too old for runs, as it does not know --restricted; update Claude Code and run the wish again"}
```

No word of these is the agent's, a session log's or the wish's: `why` and `star` are fixed words with the run's id
and its branch in them.

## The order

1. **The wish.** A made wish of that day with that id, which has had no run.
2. **The harness.** Found as the reading call finds it ([page.md](page.md)), or named with `JELLYMOON_CLAUDE`. With
   none, nothing is made, the wish stays as it was, and `why` names the places looked in: `not_started`. A `PATH`
   with no folder named by its full path is refused here, before any copy ("What the harness is started with").
3. **The repo.** `--repo`, a full path whose last name is the wish's project. Without it, the one folder of that
   name that something was typed in that day: core reads the day's session logs for the folders, as the reader
   reads them (a harness worktree counts as the repo it is of, and a switched-off folder is not read). The folders
   are never printed and never reach page data, and the run's record does not hold the one chosen. Git's own
   words name the folder they are about, so for a folder found this way nothing git says is repeated, now or
   when the copy is ended: the refusal has fixed words in their place (the copy's record keeps `found`). A
   `--repo` the person typed may be named back to them. The folder must be the top of its repo, as `copy make`
   asks: for one inside a repo, no folder above it is tried.
4. **The copy.** `make_copy`, as `jellymoon copy make` does it. The harness is started in the copy.
5. **The run.** Below.
6. **The end.** Finished: `clean_copy` as completed, so the work is one commit on `jellymoon/run-<id>` and the
   folder is gone; the wish is `done` with one more star. A run that changed nothing has still finished, and says
   the branch is where the checkout was. Anything else: `clean_copy` as failed, so the copy is kept for a look;
   the wish is `needs_you` with no star.

## What the harness is started with

The arguments, in this order, and nothing else:

```
-p --restricted --strict-mcp-config --permission-prompts none --permission-mode acceptEdits
--tools Read Edit Write Glob Grep --no-session-persistence --output-format stream-json --verbose
--append-system-prompt <fixed words>
```

The fixed words say the folder is a copy made for one wish, that files in it can be read and edited, and that no
command can be run. The wish's kept words go in on the harness's input, with a line break after them, and are in
no argument. The variables are the reading call's list and nothing else, by the same function (`call::harness_of`):
`PATH`, `HOME`, `TMPDIR`, `USER`, `LANG`, `LC_ALL`, `TERM` and `CLAUDE_CONFIG_DIR`, or the Windows list.

**Only full paths are passed on.** The harness starts in the copy, which the agent writes, so a path that is not
a full one would be looked up there. Of `PATH` only the folders named by a full path are kept, in their order;
with none left nothing is started, and one line says so. A variable that names one folder or file (`HOME`,
`TMPDIR`, `CLAUDE_CONFIG_DIR`, and on Windows `USERPROFILE`, `APPDATA`, `LOCALAPPDATA`, `ProgramData`, `TEMP`,
`TMP`) is dropped unless it is a full path. The reading call and every git child of a copy use the same function
(`call::passed_on`).

A Claude Code that answers this command line with an unknown option, naming `--restricted`, is too old. That is
read from all of: no line at all on its output, an exit with a failure, and those two fixed pieces in the first
4 KiB of what it said on stderr. A harness that printed anything first has worked, and is a failed run; so is
one whose output something it left still holds open, so that its reader never finishes: what was read so far is
what counts, and a reader that did not finish is never the reader of a harness too old. For the wish to be as it
was, the day's lock is waited for as a run's end waits for it, and the copy is taken back whatever came of that.
If the day stayed locked, the wish still names a run of which nothing is kept, not even its folder in `runs`:
the next page or `jellymoon run` that finds such a wish makes it as it was, with no run. The copy is taken back, branch and all, the wish is as it was, and the outcome is `not_started`. It is never
started again without the flag.

## What is read of the stream

The stream is untrusted. A line is read up to 1 MiB with its line break; the whole stream up to 64 MiB. A line that
is longer, that is not a JSON object, whose `type` is not text, or whose `subtype`, `is_error` or
`permission_denials` has another shape is counted (`skipped`, in the record) and skipped. Of everything else this
is taken: how the stream starts, how many lines of `type` `result` came, the first one's `subtype`, `is_error` and
how many entries its `permission_denials` lists (`denied`), and whether a skipped line came after it. No text of
the agent's is kept, printed or put in the commit message, and nothing of what it said on stderr is.

**The start is checked.** The stream's `system`/`init` line says what the harness really started with, and three
parts of it are compared: `tools` is exactly `Edit`, `Glob`, `Grep`, `Read`, `Write` in any order, `mcp_servers`
is an empty list, and `permissionMode` is `acceptEdits`. A part that is missing or of another shape differs.
Every other part of the line is passed over. Before that line only other `system` lines are let by. Every
later init line is measured the same way: one that differs stops the child and fails the run, wherever it comes. Anything
else first (a skipped line, a line of another `type`, an init line that differs) is not a run's stream: the
child is killed at once, with no five seconds, nothing more is read, and the run has failed.

**Finished** is all of: the start is a run's; exactly one result line came; its `subtype` is `success`; its
`is_error` is there and is the literal `false`; no skipped line came after it; and the harness exited with 0. A
run with denials can still have finished: they are counted, and deciding on them is F40.

The first row that fits is the one said:

| It ended | `why` says |
|---|---|
| Still going after 30 minutes | it was stopped at its time limit of 1800 seconds |
| Having printed more than 64 MiB | it printed more than Jellymoon reads of a run |
| Having printed no line at all | Claude Code ended without a result |
| With a start that is not a run's | it did not start with the tools, servers and permission mode of a run |
| With no result line (or one over the line cap) | Claude Code ended without a result |
| With more than one result line | it gave more than one result |
| With a result that is not a success, or does not say `"is_error":false` | its result was not a success |
| With a skipped line after its result | a line that could not be read came after its result |
| With a success, and an exit that is not 0 | Claude Code exited with a failure after its result |
| Finished, and its work could not be committed | its work could not be committed |

## The time limit, and a stop

A run has 30 minutes. One still going then, or one past the stream's cap, is stopped. On a Mac and on Linux the
harness is started in a process group of its own, named by its pid: the group is sent `SIGTERM`, given 5 seconds,
and then killed, so what the harness itself started goes with it. On Windows the harness alone is ended at once
(`TerminateProcess`), since there is no signal to send and no group. The run has failed. A build with debug assertions reads a
shorter limit from `JELLYMOON_TEST_RUN_LIMIT_MS`, for the tests; a release never reads it. F40's stop is one more
reason to end the wait (`child::End`) and uses the same `stop`.

**After any end the group is killed.** A harness that exited by itself, with a success or not, may have left
something going. On a Mac and on Linux the harness is not collected until its group has been sent `SIGKILL`, so
the group's number is still its own, and nothing it started in that group outlives the run. A stream that does
not start as a run's gets the same kill with no `SIGTERM` and no wait. There is one stop sequence, `dead::stop`:
the five seconds are given only inside a live `jellymoon run`, at the limit or the cap. On Windows there is no
group: the harness alone is ended, and what it started goes on until it ends by itself.

## What is kept

- **The wish**, in `pages/<day>.wishes.json`, behind the day's lock: `run`, the run's id, and `why`, how it
  stands. While the run goes the wish is `working` and `why` says the run is going. Page data is unchanged: a
  page says it with the wish's `state` and `stars`.
- **The record**, `runs/<id>/run.json`, beside the copy's:
  `{wish, day, started, outcome, why, denied, skipped, pid, mark}`, `outcome` and `why` being null while the run
  goes. No folder is in it.
  `pid` is the harness child and `mark` is when that process started, as the system says it. It is written as the tap's files are, and read as one (a plain file
  that only the person can write to).
- **One run for a wish.** The wish names its run before the harness is started, behind the day's lock, so of two
  `jellymoon run` at once one starts and the other takes its own copy back and is refused.

## The end is written in the record first

A run that ended is never written down as cut off. When the child is gone, `jellymoon run` writes the end in this
order: a failed run's copy is marked failed; the record takes `outcome` and `why`; then the wish is told, behind
the day's lock. Another change to the day may hold that lock, so it is waited for 40 seconds, which is longer than
a lock can be held (one older than 30 seconds is taken as left behind). A build with debug assertions reads a
shorter wait from `JELLYMOON_TEST_END_WAIT_MS`. If the day stays locked even so, `jellymoon run` exits 1 with one
line that says the run has ended, that its record says how, and that the page shows it when it is next drawn.

Whoever next finds the wish still `working` with nothing holding its run (below) reads the record. With an end in
it that is core's, the wish is given that end, star included: `done` with its star, or `needs_you`. A record is
a file that whoever can write the home can write, so its end is believed only when all of this holds: its `wish`
and `day` are this wish and day; its `why` is, word for word, one of the texts core makes for that outcome and
this run (the text put on the wish is made again by core, never copied from the file); and, for `completed`,
nothing of the copy is kept, as after a finished run, whose work is on its branch. Any other record is a cut-off
run's. The branch itself is not asked for: it is there from the copy's making, so it would prove nothing, and
the record names no repo. The denials counted are the first result's.

## A run that was cut off

While `jellymoon run` lives it holds the system's lock on `runs/<id>/run.lock`, taken before the wish names the
run. The system drops that lock when the process ends, however it ends. So a wish that still says `working`, names
a run whose record has no end, and whose run nothing holds was cut off. The next `jellymoon page`, app page or
`jellymoon run` for that wish that finds it so:

1. writes the wish down, behind the day's lock and only if it still says `working` there: the wish is
   `needs_you` and its `why` says the run was cut off;
2. kills the harness child the run left, if it can be told to be the same process (below): at once, with no
   `SIGTERM` and no wait, so a page is not kept waiting for an agent nobody reads;
3. marks the copy failed, so it is kept, and writes `failed` in the record;
4. draws the wish as `needs_you`, written down or not. A `jellymoon run` is then refused like any second run.

A pid is given out again once its process is gone, so the pid alone is never signalled. The record keeps the
child's **mark**, when the process started, and the pid is signalled only while its mark is the recorded one; it
is read just before the signal. A record with no pid or no mark kills nothing.

| | What is guaranteed | What is not |
|---|---|---|
| **Linux** | The mark is the start time in `/proc/<pid>/stat`, in clock ticks since boot. The child's group gets `SIGKILL`. So after the next page or run, the harness and what it started in its group are gone. | Between the kill of `jellymoon run` and that next page, the harness goes on editing its copy: nothing in safe Rust makes a child die with its parent (`PR_SET_PDEATHSIG` is set in the child before it starts, which is `unsafe`). A process the harness moved to another group is not reached. With no `/proc`, `ps` is asked as on a Mac. |
| **macOS** | The same, with the mark read from `/bin/ps -o lstart=`, to the second. | The same, and macOS has no way at all for a child to die with its parent. A pid given out again within the same second as the first start would have the same mark; the system does not reuse a pid that fast. |
| **Windows** | The wish is drawn as `needs_you` and the run is written down as failed. | Nothing is stopped: there is no mark and no signal without `unsafe` or another crate (a job object would do it). The harness goes on until it ends by itself, or until it next prints and finds nobody reading. |

On every system the copy is the run's own, so what such a harness edits is never the person's checkout. A harness
that outlives its `jellymoon run` and still prints is ended by the system at its next line, as before. Because the
harness is in a group of its own, Ctrl-C in the terminal of `jellymoon run` ends `jellymoon run` and not the
harness: that is a cut-off run like any other.

## What is refused

One line on stderr, exit 1. No copy is left, no branch is made and the wish is as it was. The first refusal past
the wish and the harness does make the folders a copy needs under the Jellymoon home (`copies`, `runs` and the
empty `runs/no-hooks`), and they stay.

| Refused | The line says |
|---|---|
| No made wish of that id on that day | the page has no made wish of that id |
| A wish that names a run | the wish has had its run, and which |
| `--repo` that is not a full path, or whose last name is not the wish's project | which of the two |
| No `--repo`, and no folder of the name typed in that day, or two of them | how many, and to name one with `--repo` |
| No `--repo`, and no session logs | the reader's own line |
| A `PATH` with no folder named by its full path | nothing was started, and why |
| No `--repo`, and the one folder is inside a repo, or is no repo | that it is not one a run can copy, and `copy make`'s reason, with fixed words where git's own would be |
| A folder that is not a checkout git can copy | `copy make`'s own line ([copy.md](copy.md)) |

## Not covered

- A `jellymoon run` that is killed leaves its harness editing its copy until the next page or run ("A run that
  was cut off"), and on Windows for as long as that harness goes.
- The test of a child deaf to `SIGTERM` needs `/usr/bin/perl` to start one, and says so where it is skipped.
- A process the harness moved to another process group, or to another session, is not reached by any kill.
- On Windows nothing the harness started is ended with it. A full path is told by `Path::is_absolute`, which
  there wants a drive with a root, or a share: `C:\bin` and `\\server\share\bin` are kept (so is anything else
  that starts with two separators, which is read as a share), and `\bin` (no drive), `C:bin` (no root), `bin`
  and `.` are dropped.
- A harness whose init line names more than the three compared parts differently (a new field, say) still passes:
  only tools, MCP servers and permission mode are compared.
- What the real harness does with the command line is measured once, on a Mac (ADR 0014, "Measured"). A repo with
  its own `CLAUDE.md`, settings or hooks, Windows, and an older Claude Code's exact words were not measured.
- A run does not follow a project's own rules: on the measured Claude Code the restricted command line does not
  run a repo's own hooks and does not read its `CLAUDE.md`. Measured on one version only (2.1.322).
- Denials are counted and nothing is done about them. A run cannot be stopped by the person. Both are F40.
- The app does not start a run yet. The 30 minutes are one number for every wish.
