# 0021 · Living beside a person's own Claude Code

Status: Written by a planner on the coordinator's decisions, 2026-11-15. Nothing here was built or run. Every
"today" below was read in the code at `3fa9354`, and names its file. What could not be read there says "not
confirmed", and the feature that builds it checks it first.

## Why

Jellymoon starts the person's own installed `claude` and reads the files it writes. It holds no login and carries
no traffic: [ADR 0014](0014-a-run-is-started-headless.md) and
[ADR 0018](0018-a-run-asks-and-is-told-more.md), "Read, not measured", stand.

That makes Jellymoon a guest on a machine it does not own. The harness is found in a place the person chose, its
settings file is theirs, its session store is theirs, and what it leaves running is theirs to meet in their next
session. Seven places where a guest goes wrong were read in how another tool survives on real machines, and each
was then checked against this repo's code. Four turned out to hold already, in whole or in part. This note writes
down all seven, says which hold, and cuts the rest into features.

Where a decision takes an idea from Magpie, an open-source tool, it says so in one line with the public file the
idea was read in. The idea was read. No code was copied, and none of it was run.

## What binds the design

- **Logic lives in core**, proved at a `jellymoon` command that prints JSON.
- **`unsafe` is forbidden in every crate** (`Cargo.toml`, `[workspace.lints.rust]`). A new dependency gets a note
  here before it is added.
- **The home is the variable and nothing else** (`find.rs`, `user_home`): a run given a scratch home never reaches
  what is the person's at the machine.
- **A log, a settings file and a shell's output are untrusted input.** Text from one is drawn as text, checked by
  code, and never run.
- **Every system.** A rule that is Unix only says what Windows does instead.

## Decisions

Each has four parts: what main does today, the decision, what is refused, and what it does not cover.

### 1. Finding `claude` when the app is opened from the Finder

**Today.** Core looks in each folder of `PATH`, then in `~/.local/bin`, `~/.claude/local`, and on Unix
`/opt/homebrew/bin` and `/usr/local/bin` (`crates/jellymoon-core/src/find.rs`, `harness`, `USER` and `SYSTEM`).
The binary and the app read `JELLYMOON_CLAUDE` and hand it to core as the harness the person named
(`crates/jellymoon/src/parse.rs`, `app/src/today.rs`; `find.rs`, `named_or_found`). The file's own first lines
say "no shell is asked, nothing is searched". An app opened from the Finder has a `PATH` of four system folders,
so a `claude` installed under a version manager of Node is found only when the person sets `JELLYMOON_CLAUDE`.

**Decision.** When `JELLYMOON_CLAUDE` is not set and `PATH` and the fixed places hold no harness, core asks the
person's login shell once for its `PATH`, and looks in those folders.

- **Which shell.** The one the system's record of the user names, never the `SHELL` variable, which anything can
  set. It must be a full path to a file that can be run, and its last name must be on a closed list of shells
  whose fixed command is known. `JELLYMOON_SHELL` takes the record's place, as `JELLYMOON_SYSTEM_BIN` takes the
  system folders' place; set to nothing, no shell is asked. Every test sets it (decision 7).
- **What it is asked.** One fixed command, a constant in core: print a mark, the `PATH`, the same mark. Nothing is
  put into that text. It is not `sh -c` of anything built.
- **How it is started.** As a login shell, in a session of its own, with no terminal and nothing on its input,
  and with the variables a harness is started with (`call.rs`, `ENVIRONMENT`). Core waits 2 seconds for the
  line. A shell still going at 5 seconds is killed with everything it started.
- **What is read.** The one line between the two marks, up to a stated size. What a profile prints before or
  after is dropped unread. Of that line, only folders named by a full path are kept.
- **Where the harness may be.** A program found in one of those folders is held to what the fixed places are
  held to: one the group or everyone can write to, or in a folder everyone can write to, is passed over. It was
  Jellymoon's choice to ask, so Jellymoon holds the answer to more than a `PATH` hit.
- **What the harness is started with.** As a harness found in a fixed place is: this process's `PATH`, then the
  folder the program is in, then the fixed places. The shell's other folders are not passed on.
- **A shim.** A version manager leaves a small script under the harness's name that starts the real program. It
  is read, up to a stated size, and taken as the program it names only when its text names one program by a path
  that can be worked out from the text and the shim's own folder. It is never run to find out.
- **Once.** The shell is asked once in the life of a process.
- **Said.** `jellymoon harness` prints one line of JSON: whether a harness was found, how (a closed list: named,
  on `PATH`, in a fixed place, by the login shell, through a shim, or not at all), and one fixed sentence. It
  prints no folder of the shell's `PATH`.

**Refused, each in a fixed sentence.** A shell that is not a full path, is not on the list, did not answer in
time, or answered with no line between the marks: the look ends as it ends today, and the error names the shell
step among the places looked in. A shim that names no program by a path: passed over, and said.

**Not covered.** Windows keeps today's rule, and `jellymoon harness` says the shell is not asked there. A shim
that only hands its work to its manager cannot be followed without running the manager: the person names the
program with `JELLYMOON_CLAUDE`.

**Not confirmed.** The record of the user is read through a call core cannot make without `unsafe`. The feature
finds a way that needs none (a file of the system, or a system program started by its full path as `ps` is in
`run/dead.rs`), or adds a note here for a package. The same holds for a session of its own: if the pinned
toolchain's standard library cannot start one in safe code, the shell gets a process group of its own and no
terminal on any of its three streams, as a run's child has today, and the feature says so.

**Also here: the harness's own variables.** `call::harness_of` clears the environment and passes a short list
(`call.rs`, `ENVIRONMENT`), which names no `CLAUDE_CODE_*` variable. So a harness started from inside a Claude
Code session is not told it is that session's child. This already holds. What proves it today is narrower:
`tests/harness.rs` asserts the list for the reading call on Unix, in a world that never set such a variable, and
`tests/f39/safe.rs` asserts four named variables for a run. The feature adds the pin: with `CLAUDECODE` and
several `CLAUDE_CODE_*` variables set on `jellymoon`, none reaches the reading call, a run, an answer or a
follow-up, on every system.

The login-shell look and the shim were read in Magpie's `internal/proc/path_other.go` and
`internal/proc/wrapper.go`; the variables in `internal/claudetui/tui.go`. Read, not copied.

### 2. A log of its own, `app.log`

**Today.** Nothing is written anywhere when Jellymoon misbehaves. The binary prints one line on stderr
(`crates/jellymoon/src/main.rs`) and so does the app (`app/src/main.rs`, `main`). A run started apart has its
stderr read by the command that started it and kept nowhere (`run/apart.rs`, `command`). An app opened from the
Finder has no stderr a person can see, so a person whose app misbehaves has nothing to send.

**Decision.** The app and the binary write to one file, `app.log`, in Jellymoon's folder.

- **What a line is.** A time, a code from a closed list in core, that code's fixed sentence, and counts. Where a
  line names a file or a folder it names its last name only, cleaned at the door every kept text is cleaned at
  (`still.rs`).
- **Never in it.** A word the person typed, a wish, a question, an answer, an agent's text, a path beyond a last
  name, or anything shaped like a credential. A panic is logged as a fixed sentence and the place in Jellymoon's
  own source; its message, which can hold anything, is not written.
- **Bounded.** The file has a size cap. At the cap it becomes the one older file, which replaces the older file
  there was. No third file is ever kept.
- **Private.** For the person alone where the system has modes, as Jellymoon's other files are.
- **Never in the way.** A log that cannot be written fails nothing: the command does what it would have done.
- **Proved by feeding it.** One test hands each of the things above to every logging call core has, and to the
  commands that log, and finds none of them in either file.

**Refused.** A logging call that takes free text. The calls take a code, numbers and at most a last name, so a
caller has no way to hand over a sentence.

**Not covered.** Sending the log anywhere: a person attaches it by hand. What the harness itself prints: a
run's stream and stderr stay unkept, as today.

A log for an app with no terminal was read in Magpie's `internal/applog/applog.go`. Read, not copied.

### 3. Where a session's file is

**Today.** Three parts were decided. One holds, and two have no code at all.

- **The harness's own words are the one signal of a lost session: already holds.** `run/asks.rs` holds the sign
  (`LOST_SIGN`, "No conversation found with session ID") and the fixed words a run then fails in (`LOST`). It is
  read only in a result's `errors` and on stderr, only in a run told to go on with a session, and only with a
  failing exit (`run/stream.rs`, `lost`; `run/child.rs`, the last lines of `run`). F52 and F53 prove it.
- **The store's folder for a project: no code.** Core never works out where the harness keeps a session. A run
  hands the harness an id and the copy's folder, and the harness finds its own file (`run/answer.rs`,
  `run/follow.rs`).
- **Looking by id across the store: no code.** In a run, core never opens or lists the session store. Only the
  reader does, and it needs no folder's name: it lists every folder of the store and reads each line's own
  working directory (`reader.rs`, `session_files`; `session.rs`, `read`). The reader already knows a run's copy
  under its link-resolved and its unresolved spelling (`session/copies.rs`, `Copies::of`).

**Decision.** When the harness says it keeps no session of that id, core looks before it calls the session
lost. It looks for a file named for the id in the store's folder for the copy, worked out from the copy's
link-resolved path, then from the unresolved one, then in every folder of the store. It reads names only and
opens no session file. A session that is nowhere is lost, in today's words. A session that is there is not
called lost: the run has failed in other fixed words, which say Claude Code keeps the session and did not find
it from the copy.

**Refused.** Core writes nothing in the store and moves nothing in it (ADR 0018, decision 6). A link in the
store is not followed.

**Not covered.** Making the harness find a session it did not find. That is under "Open".

**Not confirmed.** How the harness names a project's folder in its store. It was read in the study, not
measured here. The feature measures it on a real harness before a fixture is typed.

The two names and the look by id were read in Magpie's `internal/sessions/relocate_claude.go`. Read, not copied.

### 4. Shapes the reader does not know

**Today.** Two of the four hold.

- **A compaction summary: already holds.** A `user` line with `isCompactSummary` starts no turn and ends none
  (`session.rs`, `prompt`). `tests/f26.rs` proves it, also for a summary marked as typed.
- **The copies a fork or a resume leaves: already holds.** A typed line whose `uuid` was already read is dropped
  with its replies, across every file of the read (`session.rs`, `read`; [reader.md](../development/reader.md)).
- **A session marked as moved: not known.** Nothing reads a line of type `relocated`. By reading, nothing is
  counted twice: such a line is no `user` line, the lines before it keep their own working directory, and a line
  with none is judged by its file's folder. No test pins it.
- **A session another tool wrote through the harness: read as typed.** ADR 0018 measured that the words handed
  to a headless harness are plain `user` lines the reader's rule keeps. Only a run's own sessions are left out,
  by their copy's folder. So a headless session any other program started in the person's repo is on their page
  as their words.

**Decision.** Each shape is recognised by its own field, and is kept out or counted once.

- A compaction summary by `isCompactSummary`: kept out, as today.
- A fork's or a resume's copy by its `uuid`: counted once, as today.
- A session marked as moved by its `relocated` line: the line is no turn, and the turns around it are each
  counted once, under the folder their own line names.
- A session the harness was started headless for, by the field its lines carry for how the harness was started:
  no line of it is a turn or a reply, and a typed-looking line of it still ends the turn before it.

Fixtures are typed by hand from what a feature measured. None is copied from a real log.

**Refused.** Guessing from a session's id or from its words who wrote it.

**Not covered.** A program that drives the harness through a terminal writes sessions no field tells apart from
a person's. And a person who types `claude -p` at their own terminal is left out with every other headless
session. Both are under "Open".

**Not confirmed.** The field's name and its values. It was read as `entrypoint`, not measured here; the feature
measures it first. If no field tells a headless session apart, the last rule is not built and this note says so.

The shapes were read in Magpie's `internal/sessions/claude.go` and `internal/sessions/cc_copies.go`. Read, not
copied.

### 5. Leaving the person's settings as found

**Today.** All of it holds but one case.

- **The hook is removed only if it is still what Jellymoon wrote: already holds.** `remove_hook` takes out the
  recorded text when it is in the file exactly once and what is left is sound. Else it cuts the one group that
  equals the entry as a value. Anything else is left for the person, and `jellymoon uninstall` says
  `"hook_by_hand":true` (`hook.rs`, `remove_hook` and `cut`; `uninstall.rs`).
- **One key is edited in the text: already holds.** The file is never parsed and written back. One piece of
  text goes in at the place a parse found (`hook.rs`, `splice`).
- **A temporary file beside it, flushed, renamed: already holds.** `hook.rs`, `put` and `fill`: made new beside
  the target, synced, renamed over it, through a link, with the old file's mode.
- **Line endings: already hold.** No byte but the inserted text changes. `tests/f29.rs` installs into a file
  with Windows line ends.
- **A byte-order mark: refused today.** By reading, a settings file that starts with one does not parse as a
  JSON object, so `install_hook` answers that it is not settings and leaves it as it was. This was not run.

**Decision.** The four that hold stay as they are, and are the rule. A settings file that starts with a
byte-order mark is taken: the entry goes in after the mark, the mark stays the file's first bytes, and removal
leaves the file byte for byte as it was. A file install makes is given no mark.

**Refused.** As today: a file that is not a JSON object, a `hooks` of another shape, an entry the person
edited, a second settings file while the first holds the entry.

**Not covered.** A settings file in another encoding than UTF-8 stays refused. No copy of the person's file is
kept. No lock is taken on it, as none is today.

Removal of only what was written, and the mark, were read in Magpie's `internal/library/hooks.go` and
`internal/edit/file.go`. Read, not copied.

### 6. Stopping a run

**Today.** On a Mac and on Linux it holds. The harness is started in a process group of its own
(`run/child.rs`, `run`). A stop the person asks for sends that group an interrupt, waits 5 seconds, sends it
the polite signal, waits 5 seconds, and kills it. The time limit and the stream's cap send the polite signal,
wait 5 seconds, and kill (`run/dead.rs`, `stop`; `run/child.rs`, `halt` and `GRACE`). After any end the group is
killed, so nothing the harness started outlives the run. F39 and F40 prove it.

On Windows there is no asking and no group. The harness alone is ended at once, and what it started goes on
until it ends by itself (`run/child.rs`, `halt`; `run/dead.rs`, the last two functions, which do nothing there).

**Decision.** A stop asks first, waits a short fixed time, then kills: on a Mac and on Linux, as today. On
Windows the harness is started in a job, so that when the run ends, however it ends, everything the harness
started ends with it.

**Refused.** Leaving a child of the harness going after a run's end, on any system.

**Not covered.** Windows has no polite signal for a program with no window, so a stop there is still at once;
only its reach changes. Three kills that ask nothing stay as they are and are under "Open": the reading call at
its limit, the child a cut-off run left, and a stream that does not start as a run's.

**Not confirmed.** A job is made through the system's own interface, which core cannot call without `unsafe`.
The feature names a package with a safe surface and adds its note here, or this decision waits.

Asking before a kill, and a job on Windows, were read in Magpie's `internal/proc/tree.go`. Read, not copied.

### 7. Tests cannot touch a real home

**Today.** Nothing refuses it. What keeps tests off the real home is habit and one rule of core.

- The binary's test support starts `jellymoon` with only the variables a test gives
  (`crates/jellymoon/tests/support/mod.rs`, `started`), and core reads the home from the variable alone
  (`find.rs`, `user_home`). A test that gives no home reaches none.
- A test's world puts `HOME`, `JELLYMOON_HOME` and `CLAUDE_CONFIG_DIR` under a folder of its own
  (`tests/f39/world.rs`, `world_in`; `app/tests/support/day.rs`). That folder is under Cargo's own temporary
  folder for tests (`support/mod.rs`, `scratch`) or under the system's.
- The built programs are started from more than one place: `support/mod.rs`, `f39/world.rs`, `f52.rs`,
  `f55/started.rs`, and five files under `app/tests`. Any of them can be handed any home. A few tests start
  `jellymoon` through another program: the hook's entry (`f29.rs`), the installer (`f33.rs`), a made-up asker
  (`f41/apart.rs`).
- `cargo xtask check` has four rules and none is about tests (`crates/xtask/src/rules.rs`, `check`).

**Decision.** The test support is the one door a test starts `jellymoon` or `jellymoon-app` through. It refuses
to start either unless the home it is given and Jellymoon's folder are both set and both under a temporary
folder, with one fixed sentence. `cargo xtask check` holds that no test file starts those programs, or builds a
home, another way. Once decision 1 is built, the door also refuses a world in which a real shell could be
asked: `JELLYMOON_SHELL` must be set.

**A temporary folder is the system's, or the one Cargo gives tests.** The decision was first worded as the
system's alone. Most worlds today are under Cargo's (`support/mod.rs`, `scratch`), and a rule that refused them
would refuse nearly every test there is. So both count, and nothing else does.

**Refused.** A test that starts the binary with no home, with a home that is not a full path, or with one
outside those two folders.

**Not covered.** Core's unit tests, which start no built program. The smoke chain, which builds its own world
(`crates/xtask/src/smoke.rs`); it is held to the same two folders in the same feature or says why not. A test
that starts `jellymoon` through another program gives that program a world the door has passed, and the feature
lists each one.

A test world that refuses a real home was read in Magpie's `internal/testenv/testenv.go`. Read, not copied.

## The features

Each is a row of `feature_list.json`, proved at its `test` command. F67 and F68 go in before the next release
candidate. A gated feature waits for `GATE: PASS` on its exact head before it merges.

| Feature | What | Decision | Waits on | Gated |
|---|---|---|---|---|
| F67 | The harness is found through the login shell's `PATH` and through a shim; `jellymoon harness` says how; no `CLAUDE_CODE_*` variable reaches a started harness | 1 | nothing | yes: it chooses the program a run starts, and starts a shell |
| F68 | `app.log`: fixed sentences, codes, counts and times, bounded, with nothing of the person's in it | 2 | nothing | yes: it is what a person sends off the machine |
| F69 | A session the harness did not find is looked for in the store before it is called lost | 3 | nothing | yes: a run |
| F70 | The reader knows a moved session and a headless session another program started | 4 | nothing | yes: the reader |
| F71 | The hook goes into and out of a settings file that starts with a byte-order mark | 5 | nothing | yes: the person's settings |
| F72 | On Windows everything a run's harness started ends with the run | 6 | nothing | yes: a run |
| F73 | The built programs start in a test only through one door, which refuses a real home | 7 | F67 | no |

What already holds gets no feature of its own. Its pins ride in the row beside it: the variables in F67, the
harness's own words in F69, the summary and the copies in F70, the exact removal and the line ends in F71, the
Unix stop in F72.

## Not decided here

- **A run started with a settings file of its own, and which checkout the harness's trust is keyed on.** Open
  with the owner: a run under the restricted line ignores the repo's own instructions file, and what a run
  should be told of a repo is a question about the product before it is one about a flag.
- **Windows Subsystem for Linux.** No rule is set for a harness that lives inside it. It needs a person on such a
  machine first.
- **An update that checks a signed list of sums.** It changes what the machine runs, so it gets its own ADR with
  the release work.
- **Codex's files.** They belong to the Codex feature's ADR (F31). No such doc exists yet, so nothing points
  here.
- **Whether a headless run costs more of a subscription's allowance than a typed one.** It is reported and not
  confirmed. The owner measures it; this note asserts nothing.

## Open

1. **A session that is there and not found.** Decision 3 lets core say so, and nothing more. Core cannot make
   the harness find it, and it writes nothing in the store. By reading, the harness and not core chooses where it
   looks, so F69 changes a failed run's words and not its end. Whether a follow-up whose fork was not found
   should still start with a new session, as it does today (`run/follow.rs`, `Go::started`), is not decided.
2. **A person's own `claude -p`.** Decision 4 leaves out every headless session, and a person who types one at
   their terminal typed it. No field was found that tells the two apart.
3. **A program that types into the harness through a terminal.** Its sessions look typed in every field read
   so far. No rule is invented for them.
4. **Three kills that ask nothing.** The reading call at its limit ends its child alone, with no group and no
   polite signal (`call.rs`, `run_most`). The child a cut-off run left is killed at once (`run/dead.rs`, `kill`).
   A stream that does not start as a run's is killed at once (`run/child.rs`). Whether any should ask first is
   not decided: the last two are kills of something already taken for hostile or orphaned.
5. **The shell's record against the home's rule.** Core does not ask the system's record for the home, on
   purpose. Decision 1 reads the shell's name from that record. `JELLYMOON_SHELL` is what keeps a test, or a
   run given a scratch home, from starting a real person's shell; F73 holds tests to it.
6. **How much of decision 7 is moving tests.** At least nine files start a built program today, and three
   more start one through another program. Bringing them through one door is most of F73's work, and it adds no
   rule a person meets.

## What changes in earlier notes

- [page.md](../development/page.md), "Where the harness is looked for": "no shell is asked for its `PATH`"
  becomes the order with the shell after the fixed places, in F67. The line that the system's record of the
  account is not asked stays true of the home.
- [run.md](../development/run.md), "The time limit" and "A stop the person asks for": what Windows does, in F72.
- [reader.md](../development/reader.md), "What counts as typed by a person": the two new shapes, in F70.
- [delivery.md](../development/delivery.md), "Install and uninstall": a file that starts with a byte-order mark,
  in F71.
- [workflow.md](../development/workflow.md), "What a worker's own runs taught": the lesson that `HOME` is never
  unset in a test becomes a check, in F73.
- `docs/GLOSSARY.md` gains **session store**, **shim** and **Jellymoon's log**, in this note's PR.

## New dependencies

None is decided. Two decisions may need one, and each then adds its note here before the package is added:
reading the user's record for the shell (decision 1), and a job on Windows (decision 6).
