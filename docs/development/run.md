# A run: `jellymoon run` and `jellymoon run stop`

A run is the person's own Claude Code, started headless on one made wish, in a copy of the wish's repo (F39,
[ADR 0014](../adr/0014-a-run-is-started-headless.md)). The rules are core's, in `crates/jellymoon-core/src/run.rs`
and `run/` (`child.rs` the child, `stream.rs` what is read, `end.rs` how an end is written down, `dead.rs` the
stop sequence, `stop.rs` a stop the person asks for). The copy is [copy.md](copy.md)'s, and the wish is
[wish.md](wish.md)'s.

```
jellymoon run --wish <id> [--date <YYYY-MM-DD>] [--repo <folder>]
jellymoon run stop --wish <id> [--date <YYYY-MM-DD>]
```

`run` waits until the run has ended, prints one line of JSON and exits 0. `run stop` stops a run that is going
and prints the same line ("A stop the person asks for"). (One end exits 1 with a line on stderr: the
run ended, and the day stayed locked. See "The end is written in the record first".) With no `--date` the day is today.

```json
{"run":"r0123456789abcdef","wish":"m1","outcome":"completed","star":"a run reported it finished","denied":0,"why":"run r0123456789abcdef finished: its work is one commit on the branch jellymoon/run-r0123456789abcdef"}
{"run":"r0123456789abcdef","wish":"m1","outcome":"failed","star":null,"denied":1,"why":"run r0123456789abcdef did not finish (its result was not a success): its copy is kept and no star was added"}
{"run":"r0123456789abcdef","wish":"m1","outcome":"cancelled","star":null,"denied":1,"why":"run r0123456789abcdef was cancelled by a stop: its copy is kept and no star was added"}
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
was, the day's lock is waited for as a run's end waits for it. Whatever came of that, and in this order: the
record takes `not_started` as its outcome, an end of its own kind; the copy is taken back; and only then is the
run's lock let go. So at no moment does a look find a run that nothing holds and that has no end. The run's folder
in `runs`, which holds its lock and its record, goes in one step: it is moved to `gone-<id>`, a name no look takes
for a run, and removed there. Removed file by file, a look could find it with the lock and the record gone and the
folder still there, and take the run for one cut off; on Linux, where files go in no set order, one did. If the day
stayed locked, the wish still names the run. The next page or `jellymoon run` that finds a wish naming a run
whose record says `not_started` (its copy is then taken back), or of which nothing is kept, not even its folder
in `runs`, makes the wish as it was, with no run; a `jellymoon run` that finds it so goes on and starts one. The copy is taken back, branch and all, the wish is as it was, and the outcome is `not_started`. It is never
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
run with denials can still have finished ("What is denied").

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

A run the person stopped is none of these: it is `cancelled`, whatever its stream held.

## What is denied

Nothing is asked or answered while a run goes. What a run may do is the list it is started with ("What the
harness is started with"), and anything outside it the harness itself denies. Jellymoon has no flag, file or
command that adds to that list, for a run or for a stop: every run is started with the one command line, and
`run stop` starts nothing.

- **Counted.** `denied` is how many entries the first result line's `permission_denials` lists. It is printed,
  and kept in the record, for a run that finished, one that failed and one that was cancelled after its result
  came. A stream with no result line counts 0.
- **Never an allow.** A denial changes nothing else: a run that was denied something can still finish, since a
  denial is the list working.
- **What the stream can show of an allow.** One thing was measured: the `system`/`init` line names the tools
  the harness has. Every init line is compared, so a stream that names one tool more, at its start or later, is
  not a run's and earns no star ("The start is checked"). Nothing else in the measured stream says that a tool
  was allowed. The lines in which the agent asks for a tool show what it asked for, not what it was given, and
  their shape was not measured, so no check is made of them.

## The time limit

A run has 30 minutes. One still going then, or one past the stream's cap, is stopped. On a Mac and on Linux the
harness is started in a process group of its own, named by its pid: the group is sent `SIGTERM`, given 5 seconds,
and then killed, so what the harness itself started goes with it. On Windows the harness alone is ended at once
(`TerminateProcess`), since there is no signal to send and no group. The run has failed. A build with debug assertions reads a
shorter limit from `JELLYMOON_TEST_RUN_LIMIT_MS`, and shorter five seconds from `JELLYMOON_TEST_GRACE_MS`, for
the tests; a release never reads them.

**After any end the group is killed.** A harness that exited by itself, with a success or not, may have left
something going. On a Mac and on Linux the harness is not collected until its group has been sent `SIGKILL`, so
the group's number is still its own, and nothing it started in that group outlives the run. A stream that does
not start as a run's gets the same kill with no `SIGTERM` and no wait. There is one stop sequence, `dead::stop`:
the five seconds are given only inside a live `jellymoon run`, at the limit, at the cap or at a stop. On Windows
there is no group: the harness alone is ended, and what it started goes on until it ends by itself.

## A stop the person asks for

`jellymoon run stop --wish <id>` stops the wish's run and prints how the run ended, as `jellymoon run` prints it.

**How it reaches the run.** The harness child belongs to the `jellymoon run` that started it, which is another
process and the only one that can wait for the child. So `run stop` leaves a request, the file `runs/<id>/stop`,
and waits. The `jellymoon run` looks for it every 10 ms while its child goes, and once before it starts the child,
which is then never started. When it finds the request and the child is still going:

1. **The interrupt.** The child's group is sent `SIGINT`, as Ctrl-C would send it, and given 5 seconds.
2. **The signal.** If the child is still there its group is sent `SIGTERM`, and given 5 seconds again.
3. **The kill.** The group is sent `SIGKILL`, whether the child is gone or not.

On Windows there is no interrupt and no signal: the child alone is ended at once (`TerminateProcess`), and what
it started goes on. Then the end is written as any end is ("The end is written in the record first"): the copy
is marked failed, so it is kept; the record takes `cancelled`; the wish is told. `run stop` sees the wish change
and prints the line; the `jellymoon run` prints the same line and exits 0.

**A cancelled run.** The outcome is `cancelled`. The wish is `needs_you`, with no star, and its `why` says the
run was cancelled by a stop. It has had its one run, so `jellymoon run` refuses it. The copy is kept, with what
the agent had done in it and nothing committed on the run's branch, as a failed run's copy is kept: it is one of
the five kept for a look ([copy.md](copy.md)). `denied` is what the stream's result counted, when one came before
the stop. A stop decides the end even after a result that said success: no star is added for a run the person
stopped.

**A stop against the run's own finish.** The `jellymoon run` asks whether its child has ended before it looks
for the request. A child that ended by itself has ended: the run is `completed`, with its star, or `failed`, by
its stream, and the request is never read. A child that was still going when the request was found is
`cancelled`, also when it ends by itself during the five seconds, and also when it prints past the stream's cap
in them: the stop came first. The end is written once, by the one process
that owns the child, so there is always exactly one, and `run stop` prints that one.

**The request is a file, and anyone who can write Jellymoon's home can leave one.** It is made as a new file
with nothing in it, for the person alone to write, and only in a run's folder that is there: `run stop` never
makes that folder. Only its being there is read, never what it holds and never what kind of file it is. So a
request left by hand can do what `run stop` does and nothing more: it stops the run whose folder it is in. It
is read only by the `jellymoon run` of that run, while its child goes: one left in the folder of a run that has
ended changes nothing. No lock is taken to leave it: making a new file either happens or does not, and two
`run stop` at once leave one request and both print the one end.

| `run stop` finds | It prints | Exit |
|---|---|---|
| A run going | the run's end, once it is written: `cancelled`, or how the run ended by itself just before | 0 |
| A run that had ended | that end, as `jellymoon run` printed it, and nothing is changed | 0 |
| A run that was cut off | `failed`, and `why` says it was cut off; it is written down and its child killed, as a page does it ("A run that was cut off") | 0 |
| A wish that names no run: none was started, or the harness was too old, or `jellymoon run` has not got as far as naming one | `wish m1 has no run going, so nothing was stopped` | 1 |
| No made wish of that id | the page has no made wish of that id | 1 |
| A run that has not ended 65 seconds after it was asked | `run <id> was asked to stop and has not ended yet; the page shows its end when it has` | 1 |

The 65 seconds are the two times five of the sequence, two more waits of five for what the child printed, the
40 a run waits for the day's lock, and five. The request stays, so the run still stops. A build with debug
assertions reads a shorter wait from `JELLYMOON_TEST_STOP_WAIT_MS`. A run that ended is printed from its record
when the record's end is core's ("The end is written in the record first") and is the end the wish has; the star
is the wish's. Of any other record nothing is printed: the end is the wish's, `completed` or `failed`, and
`denied` is 0, since the count in a record whose end is not believed can have been written by hand too.

## What is kept

- **The wish**, in `pages/<day>.wishes.json`, behind the day's lock: `run`, the run's id, and `why`, how it
  stands. While the run goes the wish is `working` and `why` says the run is going. Page data is unchanged: a
  page says it with the wish's `state` and `stars`.
- **The record**, `runs/<id>/run.json`, beside the copy's:
  `{wish, day, started, outcome, why, denied, skipped, pid, mark}`, `outcome` and `why` being null while the run
  goes. No folder is in it.
  `pid` is the harness child and `mark` is when that process started, as the system says it. It is written as the tap's files are, and read as one (a plain file
  that only the person can write to).
- **A stop's request**, `runs/<id>/stop`: an empty file, there only when a stop was asked for.
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
it that is core's, the wish is given that end, star included: `done` with its star, or `needs_you` (a failed
run, or a cancelled one). A record is
a file that whoever can write the home can write, so its end is believed only when all of this holds: its `wish`
and `day` are this wish and day; its `why` is, word for word, one of the texts core makes for that outcome and
this run (the text put on the wish is made again by core, never copied from the file); and, for `completed`,
nothing of the copy is kept, as after a finished run, whose work is on its branch. Any other record is a cut-off
run's. The branch itself is not asked for: it is there from the copy's making, so it would prove nothing, and
the record names no repo. The denials counted are the first result's.

Believed or not, the child the record names is killed, if it is still that process ("A run that was cut off"):
nothing holds the run, so nobody reads that child any more, and a record that says the run ended can have been
written by hand over one that was cut off.

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

## A file of the home that is not a plain file

Anyone who can write Jellymoon's home can put something else where one of its files is expected, and opening a
named pipe waits for whoever is at its other end. So of what a page or a stop opens for a run, only a plain file
is opened: what is at the name is asked first, and then it is opened, as the pages' files are read (`tap::plain`).

| At the name | Not a plain file |
|---|---|
| `runs/<id>/run.lock` | nothing holds the run, so it was cut off: it is written down as failed, and the child it left is killed ("A run that was cut off") |
| `runs/<id>/copy.json` | the copy's record cannot be read: the copy is left as it is, and the run's end is written all the same |
| `runs/clean.lock` | no copy is ended or cleaned while it is so: a run that finishes then has failed, as its work could not be committed |
| `runs/<id>/run.json`, the day's wishes, the day's page | not read, as before: a run with no record that can be used was cut off, and wishes that cannot be used are none |
| `runs/<id>/stop` | never opened: only its being there is read |

The question and the opening are two steps, so a file changed between them is not covered. `tests/f40/pipe.rs`
puts a named pipe at each of these, and at the other files of the home, and gives `run stop` and `page` ten seconds.

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
- Whether Claude Code, started headless, ends its work cleanly at `SIGINT` was not measured, nor whether its
  stream mode takes an interrupt on its input: the interrupt is the signal Ctrl-C sends. A harness that does not
  hear it is signalled and killed all the same.
- A stop asked before `jellymoon run` has named its run on the wish (while it finds the harness and makes the
  copy) is refused, since the wish has no run yet, and the run then starts. Asking again stops it.
- A stop that finds its request before the child is started ends the run as cancelled with no child: no test
  reaches that moment, which lasts from the wish naming its run to the child's start.
- A run whose `jellymoon run` is alive and stuck is not ended by `run stop` over its head: the stop says the run
  has not ended yet. Killing that `jellymoon run` makes it a cut-off run, which the next stop or page ends.
- A cancelled run's `denied` is 0 when the stop came before the result line: what was denied until then is in
  the stream's other lines, which are not counted.
- The app does not start or stop a run yet. The 30 minutes are one number for every wish.
