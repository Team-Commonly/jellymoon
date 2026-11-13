# A run: `jellymoon run` and `jellymoon run stop`

A run is the person's own Claude Code, started headless on one made wish, in a copy of the wish's repo (F39,
[ADR 0014](../adr/0014-a-run-is-started-headless.md)). The rules are core's, in `crates/jellymoon-core/src/run.rs`
and `run/` (`child.rs` the child, `stream.rs` what is read, `end.rs` how an end is written down, `dead.rs` the
stop sequence, `stop.rs` a stop the person asks for, `place.rs` the folder, `never.rs` a run never started,
`left.rs` what a killed command left, `apart.rs` and `cap.rs` F41). The copy is [copy.md](copy.md)'s, and the wish is
[wish.md](wish.md)'s. What a finished run made is shown by `jellymoon run show`, which is [show.md](show.md)'s.

```
jellymoon run --wish <id> [--date <YYYY-MM-DD>] [--repo <folder>] [--logs <dir>] [--skip <names>]
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
   reads them (a harness worktree counts as the repo it is of, and a switched-off folder is not read: one named
   with `--skip` or in `skip.txt`, and neither is a run's copy: [reader.md](reader.md)). `--logs` names the logs, in place of Claude Code's own. A folder found so is
   a log's word, and a log is untrusted, so it must be the person's own: on Unix its owner must be the owner of
   the person's home folder, and one that is not is refused and never copied. A `--repo` is not asked. The folders
   are never printed and never reach page data. Of the one chosen, the run's record keeps the git folder of its
   repo ("What is kept"), and that is the one place it is kept once a finished run's copy is gone. Git's own
   words name the folder they are about, so for a folder found this way nothing git says is repeated, now or
   when the copy is ended: the refusal has fixed words in their place (the copy's record keeps `found`). Of
   what any git step of a run says on stderr, the first 128 KiB are kept and the rest is read to its end and
   dropped: git is never ended for what it says, and it cannot fill memory by it ([copy.md](copy.md), "A git
   that says a lot"). A
   `--repo` the person typed may be named back to them. The folder must be the top of its repo, as `copy make`
   asks: for one inside a repo, no folder above it is tried.
   **A folder found in a log is resolved once** (`run/place.rs`): every link and every `..` in its path is
   followed. The log's folder was found by its last name, as the page names a folder's project by the last
   name of its path. Where that last name is itself a link, the folder it leads to must have the project as
   its last name too: a link named `alpha` that leads to a repo named `secrets` is refused; one that leads to
   a repo named `alpha` runs there; one that leads nowhere is refused. A last name that is no link is the
   folder's own name, and is not compared with the spelling the file system keeps: a folder stored as `Alpha`
   and typed as `alpha`, or stored and typed in two Unicode forms, is one folder wherever the system takes the
   two for one name, and runs. The owner is asked of the resolved folder, and the same resolved path is what
   the copy is made of: the log's own path is not looked up a second time, so a link put in its place
   afterwards is not followed (`tests/f41/named.rs`). A git worktree in a folder named for the project is a
   folder of that name, and runs as before.
   **What is not closed.** The folder is looked up after what killed commands left is gone over ("A copy
   nothing names"), so none of that pass's waits stands between the lookup and git. What is left: the look at
   the last name and the lookup are two calls; after the lookup come the count of the runs going and the start
   of git, which opens the folder by that path; and from then git names the repo by its git folder's path,
   through the wait for the lock copies are made behind, which lasts as long as another command's git
   children, until the worktree is added. Whoever can rename the folder, or a folder above it, in that span
   can have the copy made of another repo: the agent's commit then lands on a `jellymoon/run-<id>` branch
   there, and no checkout is written. That is someone who can write the folder's parent, usually the person.
4. **The copy.** `make_copy`, as `jellymoon copy make` does it. The harness is started in the copy.
5. **The run.** Below.
6. **The end.** Finished: `clean_copy` as completed, so the work is one commit on `jellymoon/run-<id>` and the
   folder is gone; the wish is `done` with one more star. A run that changed nothing has still finished, and says
   the branch is where the checkout was. Both hold only of a branch that is still at the commit the copy was
   made from: one that was moved while the run worked fails the run ("What is kept"). Anything else:
   `clean_copy` as failed, so the copy is kept for a look; the wish is `needs_you` with no star.

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

## A run that was never started

A Claude Code that answers this command line with an unknown option, naming `--restricted`, is too old. That is
read from all of: no line at all on its output, an exit with a failure, and those two fixed pieces in the first
4 KiB of what it said on stderr. A harness that printed anything first has worked, and is a failed run; so is
one whose output something it left still holds open, so that its reader never finishes: what was read so far is
what counts, and a reader that did not finish is never the reader of a harness too old. A harness the system
refuses to start at all is the other run that was never started.

By then the wish names the run. It is told (`run/never.rs`), behind the day's lock, which is waited for as a
run's end waits for it: it names no run, its state is `needs_you`, and its `why` is one of three fixed lines.

| `why` | Written by |
|---|---|
| `nothing was started: this Claude Code is too old for runs, as it does not know --restricted; update Claude Code and run the wish again` | `jellymoon run`, which prints the same line as its own `why`, with `not_started` |
| `nothing was started: Claude Code could not be started` | `jellymoon run`, which exits 1 with this line and the system's reason after it; the wish keeps the fixed part only |
| `nothing was started: Claude Code is too old for runs or could not be started; run the wish again when that is mended` | a look, for a run whose command could not tell its wish: the day stayed locked, or the command was killed |

The wish is not left as it was made. A made wish says `working`, and a page is drawn from the wish: left so, it
would show work that nothing is doing, and whoever asked for the run apart would have nothing to read the
reason from. The wish can be run again: `jellymoon run` asks only whether a wish names a run, and the run that
starts makes it `working` again. (With no harness found at all nothing was made and the wish is as it was
made: "The order", 2.)

Whatever came of telling the wish, and in this order: the
record takes `not_started` as its outcome, an end of its own kind; the copy is taken back; and only then is the
run's lock let go. So at no moment does a look find a run that nothing holds and that has no end. The run's folder
in `runs`, which holds its lock and its record, goes in one step: it is moved to `gone-<id>`, a name no look takes
for a run, and removed there. Removed file by file, a look could find it with the lock and the record gone and the
folder still there, and take the run for one cut off; on Linux, where files go in no set order, one did. If the day
stayed locked, the wish still names the run. The next page or `jellymoon run` that finds a wish naming a run
whose record says `not_started` and names no child (its copy
is then taken back), or of which nothing is kept, not even its folder
in `runs`, tells the wish, in the third line; a `jellymoon run` that finds it so goes on and starts one. If
the lock a copy is taken back behind stays held for that look's 3 seconds, the wish is not written: where it is
kept it goes on naming the run, so the copy, its branch and the run's folder are not left with nothing naming
them, and the next look tries again. That look draws the wish as it will be kept, `needs_you` with no run, and never as one going; a
`jellymoon run` asked for then is refused, as the wish has its run still, before any copy is made. If
the copy cannot be taken back for any other reason (the repo was moved away, git fails, the copy's place is not a
folder), it never will be: the look removes what Jellymoon's home holds of the run, as `jellymoon run` does below,
and tells the wish, so a wish is never refused for a run it is not drawn with. The copy is taken back, branch
and all, and the outcome is `not_started`. The harness is never started again without the flag.

A record that says `not_started` is written with no child named: a harness that was started and found too old is
gone by then. One that says so and names a child is a record changed by hand, of a run that did start, and a
look does not remove a copy on its word, since the copy can hold work. That run was cut off ("A run that was cut
off"): its child is killed, its copy is kept and marked failed, and the wish needs the person. A record made by
hand to say `not_started` with no child named has its copy removed, work and all, and the agent of that run, if
one still goes, is not stopped: nothing in the home says otherwise.

If the repo was moved away while the harness ran, git cannot take the copy back, and `jellymoon run` exits 1 with
git's reason: the copy's folder and the run's folder in `runs` are removed all the same and the wish is told,
but the branch `jellymoon/run-<id>` and git's registration of the copy are in the moved repo, which Jellymoon
cannot find, and stay there (`git worktree prune` and `git branch -D` in that repo remove them).

A moved repo is one case of it. What Jellymoon's home holds of the run is removed so (`remove_ours`) after any
take-back that failed twice, the second time with the run's lock let go: a copy record that cannot be read, a git
that fails or runs past its limit, a copy's place that is no longer a folder Jellymoon made (then that place is
left as it is). `jellymoon run` exits 1 with the reason of the second failure. The branch and git's registration
of the copy then stay in the repo, where it still is, and nothing in Jellymoon's home records them any more: the
run's id is in that one line, and in the branch's name.

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
| Finished, and its branch was no longer at the commit the run started from ("What is kept") | its branch was moved while it worked |
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
| A wish that names another wish's run | `wish m2 names run <id>, which is another wish's run, so nothing was stopped`; no request is left | 1 |
| A run that has not ended 65 seconds after it was asked | `run <id> was asked to stop and has not ended yet; the page shows its end when it has` | 1 |

The 65 seconds are the two times five of the sequence, two more waits of five for what the child printed, the
40 a run waits for the day's lock, and five. The request stays, so the run still stops. A build with debug
assertions reads a shorter wait from `JELLYMOON_TEST_STOP_WAIT_MS`. A run that ended is printed from its record
when the record's end is core's ("The end is written in the record first") and is the end the wish has; the star
is the wish's. Of any other record nothing is printed: the end is the wish's, `completed` for a wish that is
done, `cancelled` for one whose `why` is the one line of a cancelled run, and `failed` for any other, so `run
stop` and the run's own `jellymoon run` print one line for one end. `denied` is then 0, since the count in a
record whose end is not believed can have been written by hand too.

## What is kept

- **The wish**, in `pages/<day>.wishes.json`, behind the day's lock: `run`, the run's id, and `why`, how it
  stands. While the run goes the wish is `working` and `why` says the run is going. Page data is unchanged: a
  page says it with the wish's `state` and `stars`.
- **The record**, `runs/<id>/run.json`, beside the copy's:
  `{wish, day, started, outcome, why, denied, skipped, pid, mark, began, repo, start, end}`, `outcome` and `why`
  being null while the run goes (example `crates/jellymoon-core/tests/fixtures/run-record.json`).
  `pid` is the harness child and `mark` is when that process started, as the system says it. It is written as the tap's files are, and read as one (a plain file
  that only the person can write to).
- **What the record names of the repo** (F48): `repo`, the git folder the repo's checkouts share, as the system
  resolved it when the copy was made (the copy's own record names the same folder, and goes with a finished run's
  copy);
  `start`, the commit the copy was made from; `end`, the commit the copy ended as. `repo` and `start` are in the
  record from its first writing, before the harness is started. `end` is written with `outcome` and `why`, in the
  one writing of the run's end, and only for a run that finished: the commit made of its work, or, when it
  changed nothing, `start` itself. Each is left out when it is not known, never written as null,
  and a record from before F48 has none of them (example `run-record-before.json`) and is read as it always was.
  The repo's folder is a path of the person's machine: it is in this one file, and is not printed, not in page
  data and not in what is sent to the model. Only `run show` reads the three, and it trusts none of them
  ([show.md](show.md)).

  | A run that | writes `repo`, `start` | writes `end` |
  |---|---|---|
  | was refused before its copy was made, or behind the lock runs are counted at | no record at all | |
  | was never started (the harness too old, or not startable) | yes, then the record goes with the copy | no |
  | finished, with work | yes | the one commit of its work, made on top of `start` |
  | finished, and changed nothing | yes | `start` |
  | finished, with its copy's folder already gone | yes | `start`: nothing was there to commit |
  | would have finished, and its branch was not at `start` by then | yes | no: it failed, as its branch was moved |
  | failed, was stopped, ran out of time, or could not be committed | yes | no |
  | was cut off | yes | no, and a look that writes it down as failed adds none |

  The table is what this build writes. Not every record of a run that failed, was cancelled or was cut off
  names `repo` and `start`: one written before F48 names none of the three, and neither does one the released
  v0.2.0-rc1 wrote down as failed (below). What `run toss` does with a record of none is decided in ADR 0017
  ("What `run keep` and `run toss` may build on").

  **At a run's end its branch must still be at `start`.** Before anything of a finished run is staged or
  committed, core asks git which commit `refs/heads/jellymoon/run-<id>` names, by that whole name, as a look does
  (`copy/work.rs`), and compares what git prints with the commit this command made the copy from, the one it
  wrote in the record as `start`. No word of a record is given to git. If the two differ, or the branch is gone
  or is a name for another branch, the run has failed, whoever moved it: a commit put there by hand, in a run's
  own shape or not, and the person's own commit inside the copy alike. `why` says `its branch was moved while it
  worked`, nothing is staged or committed, the branch is left where it was moved to, the copy is kept as any
  failed run's is, and no `end` is written. So an `end` is only ever `start` itself or the one commit core made
  on top of `start`, by how it is made and not only by a later check. The commit is put on the branch only if
  the branch still names `start` at that moment (`update-ref --no-deref` with the old value); a branch moved in
  between fails the run as one whose work could not be committed. `jellymoon copy clean` knows no start and
  commits onto the branch as it is ([copy.md](copy.md)).

  **The run's name is written as itself.** Between the read of the branch and that `update-ref`, a hand in the
  store can make the run's name a symbolic ref, a name for another branch. Git checks the old value through
  such a name, and without `--no-deref` it would then move the branch the name leads to: the person's `main`,
  when that is still at `start`. With it, git writes the run's own name and nothing else: the name becomes a
  plain branch again, at core's commit, the run finishes, and the branch it was pointed at is not moved. When
  the name leads to a branch that is not at `start`, the old value does not hold, nothing is written, and the
  run has failed as one whose work could not be committed (`f48/raced.rs`).

  **A run that changed nothing is checked once more.** Its end is the commit its branch was read at, and no
  commit is made. After the copy's files were staged and found to be that commit's own, git is asked the same
  `update-ref --no-deref`, with that commit as the old value and as the new: it moves nothing; a name that
  was made a name for another branch is a plain branch again (`f48/raced.rs`). It fails
  when the branch no longer names that commit. The run has then failed as one whose work could not be
  committed, and no `end` is written. What is left open is the time after that last check: the copy's folder
  is removed and its listing dropped, then the end is written, and a branch moved by hand in that time, or
  ever after, is not where the record's `end` says. The words of a finished run say only where the branch was
  when git was last asked. A look goes by what the repo says when it looks, and so must `run keep` and `run
  toss`, which compare the branch with `end` again under their own check of the old value (ADR 0017).

  **A git that does not answer is not a moved branch.** When git fails, or runs out of time, at the read of
  the branch, the run has failed as one whose work could not be committed. Only an answer that names another
  commit, no branch, or a name for another branch says `its branch was moved while it worked`.

  **`its copy is kept` is said of every failed run, and is not always so.** The words of a run that failed,
  was cancelled or was cut off say its copy is kept, and they are fixed when the run ends. The folder can be
  gone all the same: removed by hand, removed by the run's own agent, or lost to a kill between the folder's
  removal and git dropping its listing. Nothing checks the folder before the words are written. A second form
  of the words, for a failed run whose folder is not standing, comes with `run toss`.

  The released v0.2.0-rc1 reads a record that names the three, since it reads past names it does not know. It
  writes a record again in one case only, a look that writes a cut-off run down as failed, and the three are
  dropped from that record then: such a run has no work to show.
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
When the wish could not be told for any other reason (the made wishes were changed while the run worked and
cannot be used, or the wish no longer names this run as going), the line says instead that the made wishes of the
day could not be used, so the wish does not show it. The record has the end in both cases. One case has words
of its own: the wish names this run and no longer says `working`, because a look that took the run for one
nothing holds told it first (below). The line then says `run <id> has ended; a look had taken it for cut off
and told its wish first, so the wish shows what that look wrote`, the exit is 1 as before, and nothing is
changed.

Whoever next finds the wish still `working` with nothing holding its run (below) reads the record. With an end in
it that is core's, the wish is given that end, star included: `done` with its star, or `needs_you` (a failed
run, or a cancelled one). A record is
a file that whoever can write the home can write, so its end is believed only when all of this holds: its `wish`
and `day` are this wish and day; its `why` is, word for word, one of the texts core makes for that outcome and
this run (the text put on the wish is made again by core, never copied from the file); and, for `completed`,
nothing of the copy is kept, as after a finished run, whose work is on its branch. Any other record is a cut-off
run's. The branch itself is not asked for: it is there from the copy's making, so it would prove nothing. What
the record names of the repo is not read here either, and no repo is asked anything for a page: `run show`
compares the three with the repo it finds. The denials counted are the first result's.

Believed or not, the child the record names is killed, if it is still that process ("A run that was cut off"):
nothing holds the run, so nobody reads that child any more, and a record that says the run ended can have been
written by hand over one that was cut off. For the same reason the copy of a run whose believed end is `failed`
or `cancelled` is marked failed, if it is not marked yet, so that it is one of the five kept for a look. That is
done before the wish is told, behind `runs/clean.lock` and with a cut-off run's wait for it: if the lock is not
had in time nothing is written, and the next look does the rest.

**A look between the end of the copy and the end in the record.** A finished run commits its copy, behind
`runs/clean.lock`, and only then writes its end. A look that takes the run for one nothing holds can come
between the two: it finds no end in the record, and nothing of the copy to mark. It then reads the record
again, every 10 ms for one second at most, and gives the wish the end it finds, believed by the same rule. One
second is many times the single file write between the two (the commit is over by then, since the look waited
for its lock), and a third of what a look waits for anything. A run whose record has no end even then was cut
off after its work was committed, or its copy was removed by hand, and its `why` does not say a copy is kept:
`run <id> was cut off before it ended: no copy of it is kept, what was committed of it was on the branch
jellymoon/run-<id>, and no star was added`. The branch is named as where the work was put: a look asks git
nothing, and a branch can be removed by hand. The end is written in the record only after the wish took it,
so while the day's wishes cannot be written the one-second wait is paid again on every look, once for each
such run. A build with
debug assertions waits before the run writes its end for `JELLYMOON_TEST_NOTE_WAIT_MS`, after a look's
first read of the record for `JELLYMOON_TEST_LOOK_WAIT_MS`, and after every writing of a run's record for
`JELLYMOON_TEST_RECORD_WAIT_MS`, so that a test reads each state the record ever has.

**A test does not depend on an idle machine.** Each such wait (`busy::pause`: those three, and
`JELLYMOON_TEST_CLAIM_WAIT_MS`, `JELLYMOON_TEST_NAME_WAIT_MS` and `JELLYMOON_TEST_PLACE_WAIT_MS` in a run's
start, `JELLYMOON_TEST_MADE_WAIT_MS` once git has made a copy, and `JELLYMOON_TEST_UNMADE_WAIT_MS` between
the two git steps that take back one it could not make) ends early when the file `JELLYMOON_TEST_GO` names is
there, and leaves a file of that name ending in `.at` when the command has come to it: a test holds a command
at a place until it has done what it came to do, and no longer. A short limit under test is counted from when a file is there that the made-up program leaves
once it has stalled (`JELLYMOON_TEST_RUN_LIMIT_FROM`, `JELLYMOON_TEST_GIT_LIMIT_FROM`), so nothing else has to
be done within it. A test that expects an asker to be told of a run that never started sets the asker's wait
for a silent harness to a minute (`JELLYMOON_TEST_APART_QUIET_MS`), and one whose agent says nothing by design
sets it to a tenth of a second. None of these is read by a release.

**One limit ends a test's command that hangs:** `LIMIT` in `tests/support/mod.rs`, 90 seconds, for a command a
test waits for and for one it started and left going (`ended`). It stands above every bound a test states of
its own, of which the longest is a minute, so a bound that is passed is what fails. **A test that ends leaves
nothing going**, a test that panics too: a command it started and did not see end is killed, every made-up
agent ends itself when the test's folder is given up, and the folder is removed only once no agent and no
`jellymoon run` of it is left (`tests/f39/guard.rs`, proved by `tests/f39/panics.rs`).

**A wish that names another wish's run.** The day's wishes are a file too. A run is another wish's when its
record names another wish, or another day, and either that wish names the run or a `jellymoon run` still holds
it: while its command lives a run is stopped on the word of the wish its record names and no other, whatever the
day's wishes say by then, two wishes made to name each other's runs included. (Once nothing holds it, a record
that names a wish which does not name the run is the file that was changed: the run is this wish's, and was cut
off.) So a record changed by hand to name another wish while its run goes makes that run one no `run stop`
reaches until it has ended or was cut off; a request left by hand and the run's limit end it still. A wish changed by hand
to name another wish's run is given no say over that run: `run stop` for it is refused and
leaves no request, `jellymoon run` for it is refused as for any wish that names a run, and neither they nor a
page write the run down or kill its child on that wish's word. The run is stopped, and written down when it was
cut off, for its own wish only. The wish that was changed is kept as it was written, and nothing is written for
it. One made to say `working` is never drawn so, since no run of its own will ever end: every look draws it as
`needs_you`, with the one line `run <id> is another wish's run: this wish has had no run of its own`, while the
other wish's run goes and after it has ended.

## A run that was cut off

While `jellymoon run` lives it holds the system's lock on `runs/<id>/run.lock`, taken before the wish names the
run. The system drops that lock when the process ends, however it ends. So a wish that still says `working`, names
a run whose record has no end, and whose run nothing holds was cut off. The next `jellymoon page`, app page or
`jellymoon run` for that wish that finds it so:

1. kills the harness child the run left, if it can be told to be the same process (below): at once, with no
   `SIGTERM` and no wait, so a page is not kept waiting for an agent nobody reads;
2. marks the copy failed, so it is kept. That is done behind `runs/clean.lock` ([copy.md](copy.md)), which a
   `copy make`, a `copy clean` or another `run` may hold for as long as its git children take. A look waits for
   it 3 seconds and no more (a build with debug assertions reads a shorter wait from
   `JELLYMOON_TEST_COPY_WAIT_MS`). If it is still held then, nothing more is written, and the next look, which
   finds the wish as this one did, does the rest;
3. writes the wish down, behind the day's lock and only if it still says `working` there: the wish is
   `needs_you` and its `why` says the run was cut off;
4. writes `failed` in the record;
5. draws the wish as `needs_you`, written down or not. A `jellymoon run` is then refused like any second run.

A pid is given out again once its process is gone, so the pid alone is never signalled. The record keeps the
child's **mark**, when the process started, and the pid is signalled only while its mark is the recorded one; it
is read just before the signal. A record with no pid or no mark kills nothing. That is the whole boundary: a
process is signalled when its pid and its start mark are the record's, whatever that process is, and whoever can
write the home, and so the record, can already signal it.

| | What is guaranteed | What is not |
|---|---|---|
| **Linux** | The mark is the start time in `/proc/<pid>/stat`, in clock ticks since boot. The child's group gets `SIGKILL`. So after the next page or run, the harness and what it started in its group are gone. | Between the kill of `jellymoon run` and that next page, the harness goes on editing its copy: nothing in safe Rust makes a child die with its parent (`PR_SET_PDEATHSIG` is set in the child before it starts, which is `unsafe`). A process the harness moved to another group is not reached. With no `/proc`, `ps` is asked as on a Mac. |
| **macOS** | The same, with the mark read from `/bin/ps -o lstart=`, to the second. | The same, and macOS has no way at all for a child to die with its parent. A pid given out again within the same second as the first start would have the same mark; the system does not reuse a pid that fast. |
| **Windows** | The wish is drawn as `needs_you` and the run is written down as failed. | Nothing is stopped: there is no mark and no signal without `unsafe` or another crate (a job object would do it). The harness goes on until it ends by itself, or until it next prints and finds nobody reading. |

On every system the copy is the run's own, so what such a harness edits is never the person's checkout. A harness
that outlives its `jellymoon run` and still prints is ended by the system at its next line, as before. Because the
harness is in a group of its own, Ctrl-C in the terminal of `jellymoon run` ends `jellymoon run` and not the
harness: that is a cut-off run like any other.

## What a kill leaves in the record

`jellymoon run` writes its record whole each time, to a new file that is synced and then renamed over the old one,
so a kill leaves the record of the step before or of the step after, never half of one. The folder is not synced:
after a power loss the rename itself can be lost, and the record is then the one of the step before. What it names of the repo is
written before anything it describes is removed: `repo` and `start` while the copy's own record is still there,
and `end` after the copy is gone, which is the one step where the two are apart (the commit is made first, and
is on the run's branch). `crates/jellymoon/tests/f48/cut.rs` kills a run while its harness works, and after its
copy is removed and before its end is written; the other rows follow from the order of the steps and are not tested.

| Killed | The record | The repo | `run show` |
|---|---|---|---|
| before the record is written | none; the copy is one nothing names, and is taken back ("A copy nothing names") | the run's branch, until then | `wish <id> has had no run` |
| after the record, before the wish names the run | `repo`, `start`, no end; taken back with the copy | the same | the same |
| while the harness works, or after it and before the commit | `repo`, `start`, no `outcome`, no `end` | the branch at `start`; the copy is kept | `was cut off before it ended`, before a look has written the run down and after |
| after the commit, before the copy is removed | the same | the branch at the run's commit; the copy is kept, with the same work in it | the same |
| after the copy is removed, before the end is written | the same | the branch at the run's commit, and no copy | the same: nothing names that commit as the run's end, so it is not shown as the run's |
| after the end is written, before the wish is told | whole: `outcome`, `why`, `end` | as a finished run leaves it | the work, and the next look gives the wish the record's end |

A look that finds a run cut off writes `failed` and the words of a cut-off run, and never an `end`
("A run that was cut off"). Those words are core's own, in their two forms (the copy kept, or gone), and a
record that holds them for this wish is one Jellymoon wrote: `run show` knows it and says the run was cut off,
in the line it says before any look. An `end` put into such a record by hand changes nothing: the record's
outcome and words are still a cut-off run's, and `run show` refuses it before it reads what it names.

## A copy nothing names

A wish names its run only after the run's copy is made and its lock is claimed, and a run that was never
started has its wish told before its copy is taken back. A `jellymoon run` killed in either gap leaves a copy,
a branch and a folder in `runs` that no wish names, so no look at a wish finds them. They are found by a
second lock (`copy/hold.rs`): every `jellymoon run` holds the system's lock on `runs/<id>/start.lock` from the
moment the run's id is taken, before the copy is made, until it ends, when it removes the file and only then
lets its run's lock go. A plain file there that nothing holds is of a command that is gone. `jellymoon copy
make` writes none, so a copy made by hand is never looked at here.

Every page drawn of a day's made wishes (`jellymoon page`, the app's page) and every `jellymoon run`, before
it counts the runs going, goes over those runs (`run/left.rs`):

| The run | What is done |
|---|---|
| Its start lock or its run's lock is held | nothing: its command lives. The start lock is asked again once the copies' lock is had: a command makes that file and takes its lock in two steps, and one listed between the two holds it by then |
| Something other than a plain file is at `start.lock` | nothing, and it is never opened |
| Its record has an end, other than `not_started` | the start lock is removed: the run is done with |
| Its record is there and cannot be used | nothing: it does not say who names the run |
| Its record names a child | the child is killed, if the pid is still that process ("A run that was cut off"), whichever wish names the run |
| A wish names it: one of the day looked at, or the wish and day its record names | no more than that: it is the look at that wish that writes it down |
| No wish names it, and it has no record, or one that names no child | **taken back**, once the repo has been asked (below): the copy's folder, the branch `jellymoon/run-<id>`, git's registration of the copy, and `runs/<id>`, in that order. Nothing ran there, so the branch is where the checkout was. A link at `copies/<id>` is removed as itself, and what it leads to is not touched |
| No wish names it, and its record names a child and has no end | **kept**, as a cut-off run's copy is: the copy is marked failed and the record says the run was cut off. No wish is changed |

**The repo is asked first.** A copy's record is a file anyone who can write the home can write, so what a look
does in a repo goes by what that repo holds (`record::asked`, `copy/back.rs`):

| The repo the record names | What is done |
|---|---|
| lists the copy as a worktree of its own: the copy's git folder is directly in the `worktrees` folder of the repo's git folder, which is a repo's own, and its `gitdir` file, which git wrote, names `copies/<id>/.git` | the take-back above |
| answers and does not list it: the repo's git folder resolves, and the copy's git folder or its `gitdir` file is not there, is anywhere else than directly in that `worktrees`, or names another place; or the folder named as the repo is a worktree's git folder | nothing in that repo: no git is asked and no branch is removed. What the home holds of the run goes |
| could not be asked: its git folder does not resolve (moved away, on a volume that is not mounted, below a folder that cannot be read), or the copy's git folder or `gitdir` file cannot be read for another reason than not being there, or the record names the repo or the copy's git folder by a path that does not start at a root, which this process and git would read from two different folders | nothing at all. The copy and its record are kept for the next look |

Where the two folders are is asked of them as the system resolves them, never of their names: a record that
reaches the copy's git folder by `..` out of another repo's, or through a link put in another repo's, names a
folder that is not in that repo's `worktrees`, and a record that names its own repo through a link is still
that repo's. "Anywhere inside the repo's git folder" is not enough, and was the rule once
(`tests/f41/shapes.rs`):
- the copy's own git folder is inside itself, and git takes a worktree's git folder named as a repo for one,
  with that worktree's `HEAD`, which is the copy's branch: the branch then looks reached by a checkout. A
  worktree's git folder holds a `commondir` file and a repo's own holds none, and a folder that holds one is
  not taken for a repo;
- a repo whose git folder is inside another repo's, as a submodule's is at `.git/modules/<name>`, has its
  copies' git folders inside the outer repo's too. Named as the copy's repo, the outer one does not list it.

A repo that is itself a worktree of another is recorded by the shared git folder, and a repo with its git
folder inside another's by its own: copies of both are taken back whole.
Jellymoon's git calls switch `worktree.useRelativePaths` off, so the `gitdir` file holds a whole path; one that
holds a path from the copy's git folder, as a git of 2.48 or later writes with that setting on, is read from
that folder.

**The branch goes before the registration.** The registration is the only thing in the repo that says the
branch is Jellymoon's: once it is gone, a branch of the run's name has nothing but the record's word, and no
look removes a branch on that. So every take-back removes the branch first and the registration last. A look
cut off after any git step is finished by the next: cut off before the registration went, the repo still lists
the copy; cut off after, there is no branch left to remove (`tests/f41/asked.rs`). The same order holds for the
command that takes its own never-started copy back, and for a copy that could not be made.

**What is kept.** A left copy's branch that is someone's by now, whoever made it so, stays, and the copy's
folder, the registration and the record go all the same (`back::theirs`). It is someone's when any of these
holds:
- a checkout stands on it, in the repo's own checkout or in any worktree of the repo other than the copy
  (asked through each git folder in `worktrees`; the copy's own `HEAD` is that branch because the copy was
  made so, and is not counted). A checkout stands on a branch when its `HEAD` is that branch (`git
  symbolic-ref HEAD`), and when its `HEAD` is detached by a rebase or a bisect that goes back to the branch:
  git keeps the branch's name in `rebase-merge/head-name`, `rebase-apply/head-name` or `BISECT_START` in that
  checkout's git folder, and itself refuses to delete a branch one of them names. Each is read only as a
  plain file no longer than a path, in a real folder: anything else at those names, and a worktree whose
  `HEAD` git cannot read, is a checkout that could not be asked, and the take-back is put off (`tests/f41/stood.rs`);
- another ref is a name for it, a symbolic ref (`git for-each-ref --format=%(symref)`), which would be left
  naming nothing;
- the run's name is itself a symbolic ref by now (`git symbolic-ref --quiet <name>` answers): it is a name
  for another branch, no longer the run's, and stays as it is (`tests/f41/pointed.rs`);
- it holds a commit that nothing lasting else holds, by the one rule below.

**The one rule: a run's branch is removed only when every commit on it is also on another branch, a tag or a
remote-tracking branch.** The question is `git --no-replace-objects rev-list --max-count=1 <branch> --not
--exclude=<branch> --glob=refs/heads/* --glob=refs/tags/* --glob=refs/remotes/*`, asked after the three
above, and a branch it prints a commit for stays (`tests/f41/held.rs`). A question git cannot answer puts the
take-back off. Nothing else is in it:
- **No `HEAD` counts, wherever it stands.** One that names a branch adds nothing to the branches, and a
  detached one is no ref: git drops its commit at the next switch. So a commit that only this branch and a
  checkout detached at it have keeps the branch, and so does one the person made on a detached `HEAD` before
  the run and has since switched away from.
- **No other ref counts.** `refs/stash` and `refs/bisect/*` go when the stash is taken out or the bisect
  ends, and what a tool keeps under `refs/notes` or a name of its own is not where a person looks for work.
- **No replace ref and no graft counts.** Either can give a commit a parent it does not have and be removed
  tomorrow. Replace refs are off by `--no-replace-objects`. git has no option for grafts, so that one call
  is given `GIT_GRAFT_FILE` as a name in the empty folder it is started in, and `info/grafts` is not read.
- **Nothing a record says counts.** The record once kept the commit the copy was made at (`base`), and a
  branch still at that commit went: that removed the last ref of a commit made on a detached `HEAD`. The
  field is gone. A record that still has it, written by a build of that time, is read as any record, and the
  word is not used.
- Another left copy's branch is another branch. Of two copies made of one commit no other ref has, the first
  asked goes and the last name stays.

**What the rule costs: a name left over.** A copy made of a checkout detached at a commit no ref has, or of
a commit that was amended or reset away afterwards, leaves `jellymoon/run-<id>` in the repo as that commit's
only ref. The record is gone by then, so no later look asks again, and the name stays for good, also once a
branch or a tag holds the commit: `git branch -D jellymoon/run-<id>` removes it. A branch or a tag that
holds the commit before the look lets the branch go with the rest. A name left over is the cheap side: the
other side is a commit with no ref and no warning from git.

A copy an agent may have written in is never removed. Nothing outside Jellymoon's
home is touched but that one branch and git's registration of the copy, as any take-back reaches them
([copy.md](copy.md)). The branch is deleted as the run's own name and never as what a name points at, and
only while it is at the commit the look asked about (`git update-ref --no-deref -d <name> <commit>`): a
commit that lands in between makes git refuse, the take-back is put off, and the next look asks again. The person's checkout, index and other branches are as they were (`tests/f41/left.rs`).

**For thirty days.** A take-back that could not be done (the repo could not be asked, git failed or ran past
its limit, `copies/<id>` could not be removed) puts the time in the copy's record, `put_off`, the first time,
and is tried again at every look. A time there that is later than now, written by hand or before a clock was
set back, would keep the copy for ever: it is taken as now, and written so. A repo that was away and is back
has its copy taken back whole. At the first look more than thirty days after that time the repo is taken to be gone for good: the copy's folder and
`runs/<id>` go, and nothing is done in any repo. The copy's folder goes only as a real folder in a real
`copies`, never through a link. That look says so, once, in one line on stderr of `jellymoon page`, and is no
failure: `jellymoon: the copy of run <id> could not be taken back for thirty days and is given up: what
Jellymoon kept of it is removed, and its branch jellymoon/run-<id> may still be in its repo`. A page has no
place for a run no wish names, so the line is not on it: core hands it back beside the page
(`with_made_said`), and the app, which calls `with_made`, and a `jellymoon run` do not say it. A repo that
comes back later than that keeps the branch and the registration (`git worktree prune` and `git branch -D` remove them). The bound is time, and not the
repo's folder being gone, because a look cannot tell a repo that was removed from one that was moved, or from
a volume that is not mounted.

**A command killed while its copy is made.** The copy's record is written before git is called, naming the
git folder git is about to make, so a command killed while git makes the copy, or before the record is written
again with what git said, leaves a record: the repo lists the copy by then, or does not yet.

**What is not closed.**
- Another folder at the repo's path reads as a repo that answers and does not list the copy. A repo moved
  away, with another repo, or any folder that resolves, put where it was: the record and the copy go at the
  first look, not after thirty days, and the repo moved back keeps its branch and its registration.
- A worktree of the repo whose `HEAD` cannot be read (a git folder in `worktrees` that is damaged), or with
  something other than a plain file where git keeps a rebase's or a bisect's branch, makes the question fail:
  the branch and the registration stay, for thirty days, as for any git step that failed.
- The name the one rule leaves over (above) is never removed by Jellymoon.
- A tag or a remote-tracking branch that holds the branch's commit at the look can itself be removed later
  (`git tag -d`, `git fetch --prune`), and the commit then has no ref. A remote-tracking branch that a
  fetch prunes is the nearest of these to a ref that comes and goes.
- A never-started run's own take-back removes its branch without asking. If that branch was the other
  branch that let a left copy's branch go a moment before, a commit made on a detached `HEAD` loses both
  names: two runs on one such commit, and a switch away, within milliseconds. Reasoned, not run.
- A rebase or a bisect of the run's branch that starts between a look's question and its removal of the
  branch, three git calls later, is not seen: git then ends it with a ref it cannot lock, on a detached
  `HEAD`. Reasoned, not run.
- A copy the person locked (`git worktree lock`): the branch goes, git refuses to drop the registration, and
  the record waits thirty days, then the line. The registration stays for `git worktree unlock` and `remove`.
- At the give-up, a link at `copies/<id>` is not followed and not removed: it stays in `copies` with no
  record.
- A record made by hand that names a second repo, with a git folder for the copy put in that repo's
  `worktrees` and a `gitdir` file copied there: that repo lists the copy by every sign a look has. Its branch
  of the run's name goes if nothing of its own is on it, and the real repo keeps its branch and registration
  with no record. Whoever can do that can write that repo's git folder, and could delete the branch there.
- A commit made in the copy and reset away again is lost with the branch's reflog, as the person left it.
- A branch of the run's name in a repo that answers and does not list the copy is never removed. A look cut
  off no longer leaves that, but the person can: `git worktree prune`, or git's own `gc`, run between a look
  that removed the copy's folder and the look that would have finished, drops the registration and leaves
  the branch. Nothing left in the repo or the home then proves the branch is Jellymoon's, since whoever can
  write the home can write all the home holds: it stays for `git branch -D`.
- git makes the branch and the copy's git folder in one call. A command killed between those two, inside git,
  leaves the branch with no registration, as above.
- A git child killed with its command goes on by itself: one still writing the copy's folder after a look
  took the copy back leaves a folder, which the next `copy clean` removes.
- A record that is there and cannot be read is never taken back and never removed: it holds no time.
- `copies` itself removed, or made a link: its copies cannot be told the repo's own, and are kept for thirty
  days like a repo that cannot be asked.
- On Windows a folder is resolved by a call some drives do not answer (some RAM disks and mapped drives). A
  repo or a home there cannot be asked, so its left copies are kept for thirty days and then leave the home,
  and their branches and registrations stay. Read, not run.
- A run made by a build older than the start lock has none and is not found. The lock a copy is taken back
  behind is waited for 3 seconds, and what could not be done is left for the next look.

## A run started apart

`jellymoon wish --run` keeps the wish and starts its run (F41). The run is never in the asking process: core's
`run_apart` (`run/apart.rs`) starts the `jellymoon` program it is given as `jellymoon run --wish <id> --date <day>`,
a process of its own in a process group of its own, and the asker goes on, or ends, while the run goes. A run
lives as long as the command that holds its lock ("A run that was cut off"), so that command must be able to
outlive whoever asked for it. The app asks the same way for a wish entered in its window (F42,
[app.md](app.md#a-wish-in-the-window-appsrcwishingrs)): it gives the installed command, once core's
`installed_command` takes it, and starts no run itself.

- **The program** is the one given, by its full path, and nothing is looked up: `jellymoon wish --run` gives its
  own (`current_exe`). A name that is no full path is refused.
- **Before anything is started** the wish must be there and name no run, and a harness must be found, as the run
  itself would find it. With none, nothing is started and `why` is core's line with the places looked in.
- **What is waited for.** The asker is answered once the wish names its run and the harness has shown that it
  knows a run's command line, or once the command has ended: at most 120 seconds, after which the command is
  left at work and `why` says no run has been started yet. A wish names its run before the harness is
  started, so the name alone is not a run going: a harness too old for runs, or one that cannot be started,
  takes the name away again within a moment. So the run is said to be going when its record says the stream
  began as a run's (`began`, written at the stream's first line that passes the start check), or when the
  wish has named the run for 3 seconds with the command alive (a harness that is slow to say anything is not
  waited for longer; `JELLYMOON_TEST_APART_QUIET_MS` shortens this in a build with debug assertions), or when
  the wish no longer says `working`, as the run has ended already.
- **A run that was never started is said so.** When the command ends and the wish names no run, `run` is null,
  `why` is the line the wish keeps ("A run that was never started") and the wish is printed as it is kept,
  `needs_you`: the asker's answer, the kept wish and the page agree. A line the wish had before this command
  is an earlier command's, and is used only when this one said nothing on stderr.
- **The arguments** are `run --wish <id> --date <day>`, then `--repo`, `--logs` and `--skip` only as the asker
  was given them: with no name switched off there is no `--skip` at all (an empty `--skip` names nothing).
- **What is read of the command.** Its stdin and stdout are null, so nothing it prints is waited on. Its stderr is
  a pipe read only until the answer: 4 KiB at most, of which the first line, without `jellymoon: `, goes through
  `redact::line` and is cut at 300 characters. That line is the refusal `jellymoon run` gives ("What is
  refused"), and it is `why`. A wish that names a run gives its own `why`, which is core's fixed words.
- **After the answer** nothing the command says is kept: an asker that lives on reads it and drops it, and one
  that has exited leaves the pipe closed. The command writes its record, its star and the wish as it would with a
  terminal, and exits 0 for a completed run: a line it cannot print is not an error to it.
- **A stop** reaches it as it reaches any run: `jellymoon run stop` leaves its request in the run's folder and the
  run's own command finds it. A command started apart that is killed is a cut-off run, written down by the next look.
- **Its group.** On Unix the command is the leader of a new process group, so an interrupt typed at the asker's
  terminal does not reach it. On Windows it is started in a new process group with no window.
- **On Windows the command inherits what its asker has open.** Windows gives a started program every handle the
  starting one holds as inheritable, and the asker's own output is one. So a program that reads what
  `jellymoon wish --run` prints through a pipe, to the pipe's end, waits until the run has ended. Sent to a file
  or a terminal, the command returns as on Unix. Core does not close this: it would take `unsafe` or another
  crate, as the stop's limits on Windows do. On Unix only the three standard streams are replaced: any other
  descriptor the asker was handed open and inheritable reaches the command, and through it the run's agent.
  Rust's own files are opened close-on-exec, so this is what the asker's starter left open in it; closing the
  rest would take a step in the child before it starts (`pre_exec`), which is `unsafe`, so core does not.

What `jellymoon wish --run` prints is the wish's line with `run` and `why` filled in, and it exits 0 whether or
not a run was started: the wish is kept either way ([wish.md](wish.md)).

## Three runs at once

No more than three runs go at once (`run/cap.rs`). A run goes while its `jellymoon run` holds the system's lock on
`runs/<id>/run.lock`. A fourth is refused in one line, with no copy made, and the wish is as it was, so it can
be run when one has ended:

```
nothing was started: 3 runs are going, and no more than 3 go at once; run the wish again when one has ended
```

The count is exact. A run takes its own lock behind the system's lock on `runs/going.lock`, where the held locks
are counted first, so the count and the claim are one step: of six `jellymoon run` started at once exactly three
hold a run (`tests/f41/cap.rs`, where a test's build waits between the count and the claim). The runs are also
counted once before the copy is made, with no lock, only so that a run that is one too many makes no copy; a run
that passes there and is refused at the claim takes its copy back, as any run that was never started does.
**Commands for one wish are not counted against each other.** Behind `going.lock` the wish is asked first
whether it names a run, and a command whose wish does is refused there with "has had its run", before anything
is counted. The run's record is written and the wish is made to name the run behind that lock too, after the
run's lock is taken. So outside `going.lock` a run's lock is held only by a run its wish names, and of four
commands started at once for one wish one runs and three are told of its run; none is told that three runs
are going (`tests/f41/cap.rs`, where three are held with their lock taken and the wish not yet named while a
fourth arrives). Before this, each of the four took its run's lock and only then asked the wish, so three of
them standing there filled the cap for the fourth: possible since the cap was made (F41), with nothing in
between to hold it.

`going.lock` is held for the question, the count, the claim and the naming only, is waited for 3 seconds, and is
not opened when something other than a plain file is at its name. A run's lock that cannot be opened (one whose mode lets nobody read it)
is counted as not held. A cut-off run holds nothing and is not counted, whether or not a look has written it down.

The cap counts commands, and an agent can outlive its command ("A run that was cut off"). So before it
counts, `jellymoon run` ends the agents that killed commands left ("A copy nothing names"): the child each
such run's record names is killed, by pid and start mark, whichever wish names the run. On Linux and macOS no
agent of a dead command is then at work beside the three that may go (`tests/f41/left.rs`). On Windows
nothing is stopped either way, as there is no mark and no signal: three commands killed there leave three
agents at work, and three more runs may start. A run cut off by a build older than the start lock is ended
by the look at its wish, as before, and not here.

## A file of the home that is not a plain file

Anyone who can write Jellymoon's home can put something else where one of its files is expected, and opening a
named pipe waits for whoever is at its other end. So of what a page or a stop opens for a run, only a plain file
is opened: what is at the name is asked first, and then it is opened, as the pages' files are read (`tap::plain`).

| At the name | Not a plain file |
|---|---|
| `runs/<id>/run.lock` | nothing holds the run, so it was cut off: it is written down as failed, and the child it left is killed ("A run that was cut off"). So is a run whose lock is a plain file that cannot be opened: a lock that cannot be asked about is no lock. Made unreadable by hand while its run goes, it ends that run: the next look kills the child and writes the run down as cut off, or, when the run finished by itself meanwhile, with the end its record has by then, waited for one second at most once its copy is gone ("The end is written in the record first") |
| `runs/<id>/copy.json` | the copy's record cannot be read: the copy is left as it is, and the run's end is written all the same |
| `runs/clean.lock` | no copy is ended or cleaned while it is so: a run that finishes then has failed, as its work could not be committed |
| `runs/<id>/run.json`, the day's wishes, the day's page | not read, as before: a run with no record that can be used was cut off, and wishes that cannot be used are none |
| `runs/<id>/stop` | never opened: only its being there is read |

The question and the opening are two steps, so a file changed between them is not covered. `tests/f40/pipe.rs`
puts a named pipe at each of these, and at the other files of the home, and gives `run stop` and `page` ten seconds.

What a look or a stop writes, it writes into a file it makes new and then moves to its name (`hook::put`): the
run's record, the copy's record and the day's wishes. Nothing that is at a name already is opened to be written,
so a pipe or a link left beside a record, at `runs/<id>/copy.new` where the copy's record was once written
through, is neither waited on nor written through.

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
| No `--repo`, and the one folder is not owned by the person (Unix) | that it is not a folder of their own, and to name the folder with `--repo` |
| No `--repo`, and the one folder is a link that leads to a folder of another name | that it leads to a folder of another name, and to name the folder with `--repo` |
| No `--repo`, and the one folder is not there, or leads nowhere | that it is not one a run can copy: it is not there, or cannot be reached |
| Something other than a plain file at `runs/going.lock` | nothing was started: the lock runs are counted behind is not a plain file |
| Three runs going | that three are going and no more go at once ("Three runs at once") |
| A `PATH` with no folder named by its full path | nothing was started, and why |
| No `--repo`, and the one folder is inside a repo, or is no repo | that it is not one a run can copy, and `copy make`'s reason, with fixed words where git's own would be |
| A folder that is not a checkout git can copy | `copy make`'s own line ([copy.md](copy.md)) |

## Not covered

- A `jellymoon run` that is killed leaves its harness editing its copy until the next page or run ("A run that
  was cut off"), and on Windows for as long as that harness goes.
- A folder found in a log is resolved and then copied by its resolved path: a folder on that path swapped
  for a link between the two is still followed by git. Only a `--repo` the person typed is taken unresolved.
- With no harness found at all, a wish made with `--run` is kept as it was made, `working`, with the reason
  in what was printed only: nothing was started for it, and nothing says so on the page.
- A harness too old for runs that takes more than 3 seconds to refuse its command line is said to be going
  to its asker; the wish and the page are right as soon as it has refused.
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
- A stop that finds its request before the child is started ends the run as cancelled with no child. That moment
  lasts from the wish naming its run to the child's start; `tests/f40/request.rs` reaches it by leaving the
  request as soon as the run's folder is there.
- A run whose `jellymoon run` is alive and stuck is not ended by `run stop` over its head: the stop says the run
  has not ended yet. Killing that `jellymoon run` makes it a cut-off run, which the next stop or page ends.
- A cancelled run's `denied` is 0 when the stop came before the result line: what was denied until then is in
  the stream's other lines, which are not counted.
- The app does not stop a run yet (F43). The 30 minutes are one number for every wish.
- A run started apart was tested with an asker that exits. An asker whose terminal is closed (`SIGHUP` to its
  session) was not: the command has a group of its own, not a session of its own.
- On Windows a run started apart is run by CI's tests only, never by a person. Its asker's output read through a
  pipe waits for the run ("A run started apart"): CI's first run there showed it, as two tests that read an
  asker's pipe and waited half a minute. The owner of a folder found in a log is not checked on Windows.
- A person whose home folder cannot be read has no folder of their own: every folder found in a log is refused,
  and `--repo` still names one.
- A `jellymoon run` killed after its wish names the run is counted as going until it is gone, and no longer.
