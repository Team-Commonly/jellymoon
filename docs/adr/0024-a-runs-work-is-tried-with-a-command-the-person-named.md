# 0024 · A run's work is tried with a command the person named, and Jellymoon says how it came out

Status: Written by a designer on the owner's rulings, 2026-11-18. Nothing here was built or run. Every "today" was read in the code at `6c238e9`. It
changes [ADR 0017](0017-a-runs-work-is-shown-kept-or-tossed.md), decision 3, and [ADR 0023](0023-a-runs-work-is-told-as-a-result.md), decision 6, and
adds to [ADR 0020](0020-a-project-jellymoon-knows.md), decision 1. A run keeps its five tools and no shell ([ADR 0014](0014-a-run-is-started-headless.md)).

## Why

The owner: "how can user see/understand the work, hows can people understand agent work easily". ADR 0023 found that nothing is ever run to try a
run's work, so its note says "Nothing was run to try this." The first thing a person wants to know of work they did not watch is whether it works.
**The measure.** When a run ends on a project whose person named a way to try it, the person reads, beside the result and before `Keep it`: that
Jellymoon ran that command on the work, whether it passed, and if not the last lines it printed. It is Jellymoon's own sentence, because Jellymoon
did it and saw it. On a project with no command named, everything is as ADR 0023 has it.

## What the code is today

- **A run's end is one function.** `run/end.rs`, `settle`: `word` reads the last word; for `done`, `copy::end_copy` (`copy/clean.rs`) commits the
  copy with `add -A` and removes its folder, both behind `runs/clean.lock`; then `note` writes the record's end and `tell` the wish. `run.rs` (`run`)
  and `run/answer.rs` (`answer_run`) both end there, and a follow-up's run is a `run`. `recorded` believes `completed` only of a copy that is gone.
- **How a child is started and ended.** `call.rs`: `harness_of` clears the environment and passes `ENVIRONMENT` through `passed_on`, with only the
  full folders of `PATH`. `run/child.rs` (`run`, `halt`) gives the child a process group of its own on Unix, asks for a stop every 10 ms, and after
  any end kills the group (`run/dead.rs`); on Windows the child alone is ended. `record::child` writes its pid and mark, by which a look kills
  what a cut-off run left (`end.rs`, `bury`). A run has 30 minutes (`run.rs`, `LIMIT`). No doc and no source file of the repo names a sandbox.
- **The list.** `project.rs`: `Line` is `name`, `folder`, `used`, with `deny_unknown_fields`, and `Line::own` is asked of every line at every read;
  one line that fails leaves the whole list unused. `Known` is what is printed. `crates/jellymoon/src/parse.rs` (`parse_project`) knows four actions.
- **The note.** `run/waits.rs` (`Waits`, `work_tapped`, which reads every text of the note that asks once more, but the last, as a file's name),
  `run/show.rs` (`Shown`), carried by `app/src/working.rs`.

## Decisions

1. **A person names one try command for a known project, and that is their yes.** `jellymoon project try <name> --with <program> [<argument>...]`
   names it, `project try <name>` prints it, `project try <name> --never` removes it, and `project list` prints it. It is kept as `try` on the
   project's line of `projects.json`, so `project forget` takes it with the line, and a folder listed again has none. Until one is named nothing is
   ever run. Nothing a model wrote is in the command line, its arguments or its environment.
2. **The command is words, split by rules of Jellymoon's own, and no shell reads it.** At most 16 words with one space between, 200 characters in
   all, of ASCII letters, digits and `_ - . / : = + , @`. The first word is a program's bare name, or `./` and a path inside the project with no
   `..`. Core finds a bare name itself in the full folders of `PATH` (on Windows as `<name>.exe`, `.cmd`, `.bat`) and starts the file it found by
   its full path; a `./` path is joined to the copy's folder by core. The copy is never searched for a bare name. It is so on macOS, Linux and
   Windows, where Rust's `Command` hands a `.cmd` or `.bat` to the system's command program and takes these characters whole. **It lets in**
   one program with fixed arguments, and that program (`make`, `npm`, `sh` with a script) does what files of the copy say. **It keeps out** the
   kept text being a language: a shell would give every character of a line a hand can write a meaning (`;`, `$( )`, `>`), differ on Windows,
   and need quoting rules. `a && b`, a pipe, `VAR=x` and an argument with a space go in a script of the project, named `./<script>`.
3. **Core tries the work in `settle`, after the commit and before the wish is told.** `end_copy` is cut in two. First the commit, behind
   `clean.lock`, as today; the lock is let go. Then, when the run made a commit and the wish's project has a line whose folder is the run's repo
   and names a command, the try, in the copy's folder, behind the run's own lock only. Then the folder is removed, with all the try left in it:
   nothing the try wrote is ever committed. Then the record's one end, with `tried` in it, and the wish. A run that changed nothing, asks, failed
   or was cancelled is not tried. An answered run and a follow-up's run end in `settle` too, so each is tried each time it ends with work.
4. **While it is tried the wish is `working`**, and its line is `run <id> has finished, and its work is being tried`. The record has no end yet and
   names the try's child (`pid`, `mark`), so a look kills what a cut-off try left as it kills a harness. A `jellymoon run` killed while it tries
   is a run cut off: its commit is on its branch, and the work can be tossed, not kept.
5. **How it is started.** In the copy's folder. The variables are the harness's list without `CLAUDE_CONFIG_DIR`, through `passed_on`, with the
   `PATH` the run's harness had. Nothing on its input; its output and what it says beside it go into one pipe, read to its end, of which core holds
   the last 16 KiB. No terminal is given it. The limit is 600 seconds; then the group is asked, given 5 seconds and killed, and after any end the
   group is killed, as a harness's is (`child.rs`). A `run stop` asked meanwhile stops the try and not the run: the run has finished.
6. **The outcome is core's fact, kept in the record, and the lines are untrusted text.** `tried`: the command as named, how it ended, how many
   seconds, and the last 12 lines that are not empty, each cut at 160 characters, cleaned before they are written and again at each look as a
   file's text is (`still_text`: credential shapes, characters nobody sees). A line that reads like a note's count is left out. `run show` and
   `run waiting` print `tried` beside the work and the result. It is read only from a record whose end is believed `completed`. No line reaches `app.log`.
7. **On the page the try is a note of its own, `tried-<wish>`, over the result's note.** It is the kit's `Note::Told` (F86): the title and the
   labels are Jellymoon's sentences, the command stands between `“` and `”` as the person's words do, and the lines are `Quote` pieces. It takes no
   tap. With a `tried`, ADR 0023's line "Nothing was run to try this. A run reads and edits files, and no more." is not on the result's note.
8. **A failed try never takes `Keep it` away, and a page then asks once more.** After `failed` or `too_long` the first `Keep it` changes nothing
   and shows a note that says so, with `Keep it anyway`. A tap is small, the failing lines may not have fitted the window, and a keep puts on the
   person's branch what Jellymoon saw fail. It is a note and a tap of its own, asked before the one for files the person's tools obey. After
   `passed`, `not_started` or `stopped` nothing more is asked. `jellymoon run keep` is as it was: there the person read `run show`.
9. **Nothing is started on a failure.** No run goes back to mend it by itself. The follow-up's field is not filled with the last lines: what is
   in that field is kept and sent as the person's words, and these are a program's. Next, not designed: core hands a follow-up's run the last
   lines as data; and a loop that mends until the try passes, started by one tap and bounded.

## What this lets in

The try runs code a model wrote or changed (a test file, a build script, a `conftest`), with the person's rights, before the person has read it.
ADR 0017, decision 3, now reads: nothing the work made is opened or run by the app; core runs it only through a project's try command.

- **What holds it.** It happens only in a project where the person named a command, and they can take it back. It starts in the copy, never in
  their checkout. Its variables are the list. It has 600 seconds. What it leaves in the copy is removed and never committed.
- **What does not.** The copy is no sandbox. The process reads and writes whatever the person can (their keys, their checkout, the repo's store,
  which the copy shares, and Jellymoon's own folder, so the list and a run's record) and reaches the network. A process it moved out of its group
  outlives it, and on Windows everything it started does. So `passed` says the command ended with no error and no more: the work can have changed
  what the command runs, and a pass with no test in it is a pass. A line of the list written by hand names a command as `project try` does, which
  is new: until now no file in Jellymoon's folder could name a program to start.
- **Next: an OS sandbox around the try**, writing only in the copy and a scratch folder, with no network unless the person says so. Read, not
  tried: macOS has `sandbox-exec`, which Apple marks as deprecated; Linux has `bwrap` where it is installed and the kernel lets a user make
  namespaces; Windows has nothing a safe Rust program reaches without a package. Not designed here.

## The shape

The list's line and what `project` prints each gain `try`, left out when there is none. The record gains `tried`; `Shown` and `Waits` print it.
`code` is null when the system ended the program. `last` is kept for every end that printed, and drawn only after `failed` and `too_long`.
```json
{"projects":[{"name":"shop","folder":"/work/shop","used":"2026-11-18","try":"cargo test"}]}
{"with":"cargo test","seconds":41,"end":{"type":"failed","code":101},"last":["test free_from_50 ... FAILED","test result: FAILED. 1 passed; 1 failed"]}
```
```rust
#[serde(deny_unknown_fields)] pub struct Tried { pub with: String, pub seconds: u64, pub end: TryEnd, pub last: Vec<String> }
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum TryEnd { Passed, Failed { code: Option<i32> }, TooLong { limit: u64 }, NotStarted { why: NoStart }, Stopped }
pub enum NoStart { NoProgram, Refused }   // snake_case: no program of that name was found; the system would not start it
```

## Every sentence

`<c>` is the command, `<n>` a number; 1 is `1 second`. With nothing printed, `Its last lines:` is `It printed nothing.`
| Where | The words |
|---|---|
| `project try --with`, on stderr after the project's line | `From now on, when a run ends with work in <name>, Jellymoon runs “<c>” in the run's copy before you have read the work. That runs code the run wrote, as you, with your files and the network in reach.` |
| refused | `no project of that name is on the list, so nothing was changed` (as `forget`); `project try takes a program's name, or ./ and a path inside the project, and its arguments: at most 16 words of letters, digits and _ - . / : = + , @ with one space between and 200 characters in all, with no quote, no $ and nothing a shell reads; nothing was changed. Put anything more in a script of the project and name that` |
| title | `Jellymoon tried this work, and it passed.` `Jellymoon tried this work, and it failed.` `Jellymoon tried this work, and it did not end in time.` `Jellymoon could not try this work.` `Jellymoon began to try this work, and you stopped it.` |
| first label | `Jellymoon ran “<c>” in the run's copy.`; of `not_started`, `Jellymoon was to run “<c>” in the run's copy.` |
| `passed` | `It ended with no error after <n> seconds. That is all a pass says.` |
| `failed` | `It ended with error <n> after <n> seconds. Its last lines:`; with no code, `The system ended it after <n> seconds. Its last lines:` |
| `too_long` | `It was still going after <n> seconds, so Jellymoon stopped it. Its last lines:` |
| `not_started` | `No program of that name was found. Nothing was run to try this.` or `The system would not start it. Nothing was run to try this.` |
| `stopped` | `You stopped it after <n> seconds. It says nothing of the work.` |
| no command, or a run that was not tried | ADR 0023's line, unchanged |
| asked once more | `This work failed when Jellymoon tried it.`, or of `too_long` `This work was not tried to its end.`; then `“<c>” ended with error <n>.`, `The system ended “<c>”.` or `“<c>” was still going after <n> seconds.`; then `Nothing was changed. Kept, the work is in your checkout as it is.`; choices `Show me`, `Keep it anyway`, `Toss it` |

## Two worked examples, typed by hand

ADR 0023's first, in the project `shop`, whose person ran `jellymoon project try shop --with cargo test`. **It passes.** `run show` prints what it
prints in ADR 0023 and `tried`; the page reads this note over ADR 0023's two, which lose the line that nothing was run and keep all three choices.
```json
"tried":{"with":"cargo test","seconds":41,"end":{"type":"passed"},"last":["test free_from_50 ... ok","test result: ok. 2 passed; 0 failed"]}
```
```text
Jellymoon tried this work, and it passed.
  Jellymoon ran “cargo test” in the run's copy.
  It ended with no error after 41 seconds. That is all a pass says.
```
**It fails.** The run wrote `total > 50`, and its own test asks for 50 to ship free. The same, but for `tried` and its note; `▏` is a `Quote` piece:
```json
"tried":{"with":"cargo test","seconds":38,"end":{"type":"failed","code":101},"last":["test free_from_50 ... FAILED","assertion `left == right` failed","  left: 8","  right: 0","test result: FAILED. 1 passed; 1 failed"]}
```
```text
Jellymoon tried this work, and it failed.
  Jellymoon ran “cargo test” in the run's copy.
  It ended with error 101 after 38 seconds. Its last lines:
  ▏test free_from_50 ... FAILED
  ▏assertion `left == right` failed
  ▏left: 8
  ▏right: 0
  ▏test result: FAILED. 1 passed; 1 failed
```
The work's note still has `Show me`, `Keep it` and `Toss it`. A tap on `Keep it` changes nothing and shows:
```text
This work failed when Jellymoon tried it.
  “cargo test” ended with error 101.
  Nothing was changed. Kept, the work is in your checkout as it is.
  [Show me] [Keep it anyway] [Toss it]
```

## Turned down

- **Giving the run a shell tool**: the model would then choose what is run, each time, and ADR 0014, decision 2, would go.
- **A command the model proposes and the person approves each time**: an ask at every run's end, of a line the person must read as code.
- **The app running it**: the app starts no program of the work's (ADR 0017, decision 3) and decides nothing; core does both.
- **Trying only after `Keep it`**: the person would decide before knowing, and the work's code would run in their checkout.
- **A shell (`sh -c`, `cmd /C`)**: decision 2. **The command in a file of the repo**: a run can edit it, and it would be the model's then.

## Where the rulings meet the code

- **Ruling 2, where it sits.** Commit and removal are one step behind one lock (`end_copy`). Tried before the commit, what the try wrote
  (`target/`, a cache) would be committed as work; tried behind `clean.lock`, no other run could start or end for ten minutes. So the cut in
  two, and its cost: a kill during a try leaves a commit and no end (decision 4), where today that moment is one file's write.
- **Ruling 2, the copy.** It holds only what git tracks (no `target/`, no `node_modules`): every try starts cold, and may fail for that alone.
- **Ruling 3, the variables.** No `CARGO_HOME` or `JAVA_HOME`, and an app opened in the Finder has a short `PATH` (`find.rs`): a `cargo`
  under the home's own folder is then `not_started` on every run.
- **Ruling 3, what it leaves.** On Windows nothing it started is ended (`child.rs`, `halt`), and a folder a live process stands in cannot be
  removed: the run then fails as one whose work could not be committed, as today. ADR 0021's job, decision 6, closes it.
- **Ruling 5, core's own fact.** The record is a file: a process the try left can write `tried` after core did, and a look cannot tell.
- **Ruling 1, the list.** `Line` refuses a field it does not know, so an older build finds a list with a `try` unusable. And the record names
  the repo's git folder, not its top folder: a line is matched by asking git of the listed folder, as a copy does.
- **Ruling 6, once more.** `work_tapped` gives that note's texts one meaning, so this ask is a second note: with obeyed files, two asks.

## Not confirmed

- Nothing was run, on macOS, Linux or Windows. Whether 600 seconds suit a cold `cargo test` or `npm test`; how many real commands pass the
  rule of words and need no variable that is not passed; that Rust's `Command` starts a `.cmd` with these words unharmed.
- Whether a child with a group of its own and no input can still open the terminal of a `jellymoon run` typed in one: it has no session of its own.
- The room: three notes over one page at 960x640. The work's note is planned whole first, then `tried-<wish>`, then the result's.
