# A run: `jellymoon run`

A run is the person's own Claude Code, started headless on one made wish, in a copy of the wish's repo (F39,
[ADR 0014](../adr/0014-a-run-is-started-headless.md)). The rules are core's, in `crates/jellymoon-core/src/run.rs`
and `run/child.rs`. The copy is [copy.md](copy.md)'s, and the wish is [wish.md](wish.md)'s.

```
jellymoon run --wish <id> [--date <YYYY-MM-DD>] [--repo <folder>]
```

It waits until the run has ended, prints one line of JSON and exits 0. With no `--date` the day is today.

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
   none, nothing is made, the wish stays as it was, and `why` names the places looked in: `not_started`.
3. **The repo.** `--repo`, a full path whose last name is the wish's project. Without it, the one folder of that
   name that something was typed in that day: core reads the day's session logs for the folders, as the reader
   reads them (a harness worktree counts as the repo it is of, and a switched-off folder is not read). The folders
   are never printed and never reach page data; the one chosen is written in the run's record. The folder must
   be the top of its repo, as `copy make` asks: for one inside a repo, no folder above it is tried.
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
no argument. The variables are the reading call's list and nothing else, by the same function (`call::harness_in`):
`PATH`, `HOME`, `TMPDIR`, `USER`, `LANG`, `LC_ALL`, `TERM` and `CLAUDE_CONFIG_DIR`, or the Windows list.

A Claude Code that answers this command line with an unknown option, naming `--restricted`, is too old. That is
read from its exit with a failure, no result line, and those two fixed pieces in the first 4 KiB of what it said on
stderr. The copy is taken back, branch and all, the wish is as it was, and the outcome is `not_started`. It is never
started again without the flag.

## What is read of the stream

The stream is untrusted. A line is read up to 1 MiB with its line break; the whole stream up to 64 MiB. A line that
is longer, that is not a JSON object, whose `type` is not text, or whose `subtype`, `is_error` or
`permission_denials` has another shape is counted (`skipped`, in the record) and skipped. Of everything else, four
things are taken, from the last line whose `type` is `result`: that it came, its `subtype`, its `is_error`, and how
many entries `permission_denials` lists (`denied`). No text of the agent's is kept, printed or put in the commit
message, and nothing of what it said on stderr is.

**Finished** is all three of: the result's `subtype` is `success`, its `is_error` is the literal `false`, and the
harness exited with 0. A run with denials can still have finished: they are counted, and deciding on them is F40.

| It ended | `why` says |
|---|---|
| With no result line (or one over the line cap) | Claude Code ended without a result |
| With a result that is not a success, or is an error | its result was not a success |
| With a success, and an exit that is not 0 | Claude Code exited with a failure after its result |
| Still going after 30 minutes | it was stopped at its time limit of 1800 seconds |
| Having printed more than 64 MiB | it printed more than Jellymoon reads of a run |
| Finished, and its work could not be committed | its work could not be committed |

## The time limit, and a stop

A run has 30 minutes. One still going then, or one past the stream's cap, is stopped. On a Mac and on Linux the
harness is started in a process group of its own, named by its pid: the group is sent `SIGTERM`, given 5 seconds,
and then killed, so what the harness itself started goes with it. On Windows the harness alone is ended at once
(`TerminateProcess`), since there is no signal to send and no group. The run has failed. A build with debug assertions reads a
shorter limit from `JELLYMOON_TEST_RUN_LIMIT_MS`, for the tests; a release never reads it. F40's stop is one more
reason to end the wait (`child::End`) and uses the same `stop`.

## What is kept

- **The wish**, in `pages/<day>.wishes.json`, behind the day's lock: `run`, the run's id, and `why`, how it
  stands. While the run goes the wish is `working` and `why` says the run is going. Page data is unchanged: a
  page says it with the wish's `state` and `stars`.
- **The record**, `runs/<id>/run.json`, beside the copy's:
  `{wish, day, repo, started, outcome, denied, skipped, pid, mark}`, `outcome` being null while the run goes.
  `pid` is the harness child and `mark` is when that process started, as the system says it. It is written as the tap's files are, and read as one (a plain file
  that only the person can write to).
- **One run for a wish.** The wish names its run before the harness is started, behind the day's lock, so of two
  `jellymoon run` at once one starts and the other takes its own copy back and is refused.

## A run that was cut off

While `jellymoon run` lives it holds the system's lock on `runs/<id>/run.lock`, taken before the wish names the
run. The system drops that lock when the process ends, however it ends. So a wish that still says `working`, names
a run, and whose run nothing holds was cut off. The next `jellymoon page`, app page or `jellymoon run` for that
wish that finds it so:

1. stops the harness child the run left, if it can be told to be the same process (below);
2. writes the run down, behind the day's lock and only if the wish still says `working` there: the wish is
   `needs_you`, its `why` says the run was cut off, the copy is marked failed so it is kept, and the record's
   outcome is `failed`;
3. draws the wish as `needs_you`, written down or not. A `jellymoon run` is then refused like any second run.

A pid is given out again once its process is gone, so the pid alone is never signalled. The record keeps the
child's **mark**, when the process started, and the pid is signalled only while its mark is the recorded one; it
is read again just before each signal. A record with no pid or no mark stops nothing.

| | What is guaranteed | What is not |
|---|---|---|
| **Linux** | The mark is the start time in `/proc/<pid>/stat`, in clock ticks since boot. The child's group gets `SIGTERM`, 5 seconds, then `SIGKILL`. So after the next page or run, the harness and what it started in its group are gone. | Between the kill of `jellymoon run` and that next page, the harness goes on editing its copy: nothing in safe Rust makes a child die with its parent (`PR_SET_PDEATHSIG` is set in the child before it starts, which is `unsafe`). A process the harness moved to another group is not reached. With no `/proc`, `ps` is asked as on a Mac. |
| **macOS** | The same, with the mark read from `/bin/ps -o lstart=`, to the second. | The same, and macOS has no way at all for a child to die with its parent. A pid given out again within the same second as the first start would have the same mark; the system does not reuse a pid that fast. |
| **Windows** | The wish is drawn as `needs_you` and the run is written down as failed. | Nothing is stopped: there is no mark and no signal without `unsafe` or another crate (a job object would do it). The harness goes on until it ends by itself, or until it next prints and finds nobody reading. |

On every system the copy is the run's own, so what such a harness edits is never the person's checkout. A harness
that outlives its `jellymoon run` and still prints is ended by the system at its next line, as before. Because the
harness is in a group of its own, Ctrl-C in the terminal of `jellymoon run` ends `jellymoon run` and not the
harness: that is a cut-off run like any other.

## What is refused

One line on stderr, exit 1, nothing made and nothing written.

| Refused | The line says |
|---|---|
| No made wish of that id on that day | the page has no made wish of that id |
| A wish that names a run | the wish has had its run, and which |
| `--repo` that is not a full path, or whose last name is not the wish's project | which of the two |
| No `--repo`, and no folder of the name typed in that day, or two of them | how many, and to name one with `--repo` |
| No `--repo`, and no session logs | the reader's own line |
| No `--repo`, and the one folder is inside a repo, or is no repo | that it is not one a run can copy, and `copy make`'s reason |
| A folder that is not a checkout git can copy | `copy make`'s own line ([copy.md](copy.md)) |

## Not covered

- A `jellymoon run` that is killed leaves its harness editing its copy until the next page or run ("A run that
  was cut off"), and on Windows for as long as that harness goes.
- The test of a child deaf to `SIGTERM` needs `/usr/bin/perl` to start one, and says so where it is skipped.
- What the real harness does with the command line is measured once, on a Mac (ADR 0014, "Measured"). A repo with
  its own `CLAUDE.md`, settings or hooks, Windows, and an older Claude Code's exact words were not measured.
- Denials are counted and nothing is done about them. A run cannot be stopped by the person. Both are F40.
- The app does not start a run yet. The 30 minutes are one number for every wish.
