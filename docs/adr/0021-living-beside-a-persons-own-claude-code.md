# 0021 · Living beside a person's own Claude Code

Status: Written by a planner on the coordinator's decisions, 2026-11-15, and amended on 2026-11-17 after its
review found six faults, and again before that amendment merged ("Amended after review", at the end). Nothing
here was built. Every "today" below was read in the code at `02393d1`, names its file, and was read again at
`bb2a384`, which the amendment sits on. Three things were measured since and say so. What is neither read nor
measured says "not confirmed", and the feature that builds it checks it first.

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

Two words are kept apart throughout. The **shell** is the app's window, as the glossary has it. A **login shell**
is the person's command program (`zsh`, `bash`), which decision 1 asks for its `PATH`.

## What binds the design

- **Logic lives in core**, proved at a `jellymoon` command that prints JSON.
- **`unsafe` is forbidden in every crate** (`Cargo.toml`, `[workspace.lints.rust]`). A new dependency gets a note
  here before it is added.
- **The home is the variable and nothing else** (`find.rs`, `user_home`): a run given a scratch home never reaches
  what is the person's at the machine. `tests/harness.rs` holds it by starting `jellymoon` with no home, an empty
  one and a relative one, on purpose.
- **A refusal writes nothing** (ADR 0018; `delivery.md`, `project.md`, `run.md`). The tests compare the whole
  home before and after (`tests/support/mod.rs`, `tree`).
- **A log, a settings file and a login shell's output are untrusted input.** Text from one is drawn as text,
  checked by code, and never run.
- **Every system.** A rule that is Unix only says what Windows does instead.

## Decisions

Each has four parts: what main does today, the decision, what is refused, and what it does not cover.

### 1. Finding `claude` when the app is opened from the Finder

**Today.** Core looks in each folder of `PATH`, then in `~/.local/bin`, `~/.claude/local`, and on Unix
`/opt/homebrew/bin` and `/usr/local/bin` (`crates/jellymoon-core/src/find.rs`, `harness`, `USER` and `SYSTEM`).
The binary and the app read `JELLYMOON_CLAUDE` and hand it to core as the harness the person named
(`crates/jellymoon/src/parse.rs`, `app/src/today.rs`; `find.rs`, `named_or_found`). The file's own first lines
say "no shell is asked, nothing is searched". A harness in a fixed place is held to a check of modes: the
file's, read after links are followed, and the listed folder's, not their owners and not the folders above
([page.md](../development/page.md)). A `claude` installed under a version manager of Node is in none of these
places, so an app opened from the Finder finds it only when the person sets `JELLYMOON_CLAUDE`.

**Decision.** When `JELLYMOON_CLAUDE` is not set and `PATH` and the fixed places hold no harness, core asks the
person's login shell for its `PATH`, and looks in those folders. Unix only.

- **Which login shell.** The one the system's record of the user names, never the `SHELL` variable.
  `JELLYMOON_SHELL` takes the record's place. It is the person's word, as `JELLYMOON_CLAUDE` is: whoever sets
  the environment of `jellymoon` already chooses the harness. Set to nothing, no login shell is asked.
- **What the named file must be.** A full path, to a file that can be run, that passes the file checks below,
  and whose last name is on a closed list in core. The list does not guard anything: it is how core knows the
  fixed command that kind of login shell takes.
- **What it is asked.** One fixed command, a constant in core: print a mark, the `PATH`, the same mark. Nothing is
  put into that text. It is not `sh -c` of anything built.
- **How it is started.** As an interactive login shell, with no terminal on any of its three streams and
  nothing on its input, with the variables a harness is started with (`call.rs`, `ENVIRONMENT`). Interactive,
  because the file a version manager writes its line to is read only by an interactive one. Measured on one Mac:
  a login shell with no terminal named 14 folders and an interactive one named 16.
- **How long.** Core waits 5 seconds for the line. The login shell's whole group is killed as soon as the line
  is read or the wait ends, and always before the command returns. Measured on one Mac, an interactive login
  shell with no terminal answered in 0.02 to 0.05 seconds; that Mac has no version manager in its profile, and
  the slow case, which is the one the ask is for, is not measured.
- **When it is asked.** Only when every other place gave no harness: a command that finds one on `PATH` or in a
  fixed place starts no login shell.
- **What is read.** The one line between the two marks, up to a stated size; the rest is dropped unread. Output
  that holds more than one pair of marks is refused. Of the line, only folders named by a full path are kept.
- **The file checks.** A program found in one of those folders is held to what a fixed place is held to: a file
  the group or everyone can write to, or a folder everyone can write to, is passed over. One check is added,
  since Jellymoon did not choose these folders: where the program is a link, the folder its link-resolved file
  is in is held to the folder's check too. It was Jellymoon's choice to ask, so Jellymoon holds the answer to
  more than a `PATH` hit.
- **What the harness is started with.** As a harness found in a fixed place is (`find.rs`, `harness`): this
  process's `PATH`, then the folder the program was found in, then the fixed places. A `claude` that Node runs
  finds its `node` there, since a version manager keeps both in one folder.
- **What is remembered.** The folders the login shell named, in the process's memory and nowhere on disk. The
  look in them and the file checks are redone at every start of a harness. A failed or timed-out ask is not
  remembered: the next start asks again.
- **Off the window's thread.** In the app the ask is made where a reading call is made, never on the thread
  that draws.
- **Said.** `jellymoon harness` prints one line of JSON: whether a harness was found, how (a closed list: named,
  on `PATH`, in a fixed place, by the login shell, or not at all), and one fixed sentence. It prints no folder
  of the login shell's `PATH`. It is a step of the smoke chain.

**A shim is started, not read.** The first writing had core read a shim and take the program it names. That is
dropped. A grammar for "the program this script names" would have to follow every version manager, and a wrong
guess starts the wrong program. A shim is what the person's terminal runs when they type `claude`, so it is
started as any harness is. What a shim needs is its manager on `PATH`. So, in a row of its own: a harness found
by the login shell is started with this process's `PATH`, then the folders the login shell named that pass the
folder's check, in their order, then the fixed places. A folder that is not there is not passed on: one that
is made later could be made by someone else. This is looked at again at every start, as the file checks are.

**Refused, each in a fixed sentence.** A login shell that is not a full path, fails a file check, is not on the
list, did not answer in time, or answered with no pair of marks or more than one: no harness comes from the
login shell, the look ends as it ends today, and the error names the login shell among the places looked in.

**Not covered.**
- Windows keeps today's rule, and `jellymoon harness` says the login shell is not asked there.
- An interactive login shell runs the person's profile, and a profile may do anything: start an agent for keys,
  attach a terminal multiplexer, `exec` another command program. With no terminal most of these give up. What
  stays in the group is killed when the line is read or the wait ends. What moved itself out of the group is not reached, as in decision 6, and one such
  program can be left by each ask. The remedy is the person's: `JELLYMOON_CLAUDE`, or `JELLYMOON_SHELL` set to
  nothing.
- Owners are not checked, and neither are the folders above the two that are. The checks are a second look at
  folders the person's own terminal already trusts, not a wall.
- No test runs the way a real person takes. The door of decision 7 sets `JELLYMOON_SHELL` in every world, so
  reading the system's record of the user is run by no test, and it is also the part that is not confirmed.
  Before F78 passes a person checks it by hand, once, on a Mac whose `claude` is under a version manager: with
  `JELLYMOON_SHELL` and `JELLYMOON_CLAUDE` not set and a `PATH` of the system's folders alone,
  `jellymoon harness` says the harness was found by the login shell. F78's progress note says who did and when.

**Not confirmed.** The record of the user is read through a call core cannot make without `unsafe`. The feature
finds a way that needs none (a file of the system, or a system program started by its full path as `ps` is in
`run/dead.rs`), or adds a note here for a package. Whether safe code can give the login shell a session of its
own is not known either; a process group of its own, as a run's child has, is what the cap needs.

**As built (F78).** The record of the user is read with no `unsafe` and no package: core starts the system's own
program for it by its full path, with no variable of its own, as `ps` is started in `run/dead.rs`. That is
`/usr/bin/id -P` on a Mac and `/usr/bin/getent passwd <uid>` elsewhere, two seconds and 4,096 bytes at most; the login
shell is the last field of the one line printed, when its third field is this user's number. A file of the system
was not taken: on a Mac `/etc/passwd` does not hold a person's account. The list is `zsh` and `bash`. The mark is
`__jellymoon_path__`, the stated size is 65,536 bytes, and the login shell gets a process group of its own and no
session of its own. The words of each refusal are in [page.md](../development/page.md), "The login shell". A test's
made-up login shell meets the checks as a person's does: it is a program of the test's, under the name `zsh` or
`bash`, in a folder of the door's world that only the test's user can write to. There is no seam for tests.

**Also here: the harness's own variables.** `call::harness_of` clears the environment and passes a short list
(`call.rs`, `ENVIRONMENT`), which names no `CLAUDE_CODE_*` variable. So a harness started from inside a Claude
Code session is not told it is that session's child. This already holds, by the code. What tests hold is
narrower: `tests/harness.rs` asserts the list for the reading call on Unix, in a world that never set such a
variable, and `tests/f39/safe.rs` asserts four named variables for a run. A row of its own adds the pin, on
every system.

The login-shell look and the shim were read in Magpie's `internal/proc/path_other.go` and
`internal/proc/wrapper.go`; the variables in `internal/claudetui/tui.go`. Read, not copied.

### 2. A log of its own, `app.log`

**Today.** Nothing is written anywhere when Jellymoon misbehaves. The binary prints one line on stderr
(`crates/jellymoon/src/main.rs`) and so does the app (`app/src/main.rs`, `main`). A run started apart has its
stderr read by the command that started it and kept nowhere (`run/apart.rs`, `command`). An app opened from the
Finder has no stderr a person can see, so a person whose app misbehaves has nothing to send.

**Decision.** The app and the binary write to one file, `app.log`, in Jellymoon's folder.

- **What logs.** Three things, and no other: a run's end; a failure in something Jellymoon tried (a reading
  call or a harness that was started and failed, a file of Jellymoon's own that could not be written); and a
  panic in Jellymoon's own crates.
- **What never logs.** A refusal, a usage error, and the hook, at any step. "A refusal writes nothing" stands as
  it is. With no Jellymoon's folder there is no log.
- **What a line is.** A time, a code from a closed list in core, that code's fixed sentence, numbers, and at
  most one kind from a closed list ("the settings file", "a session log", "a copy", "a run's record", "the list
  of projects", "a page"). No name of any file, folder or project is in it: a project's name is a word the
  person typed.
- **A panic.** A fixed sentence. Its message, which can hold anything, is not written. Its place is written
  only when the file is in Jellymoon's own crates; a panic inside a dependency names no place, since that place
  is a path of the machine that built it.
- **One writer rule.** A line is one write, appended. When the line would take the file over its cap, the
  writer renames `app.log` to the one older file, with one rename, and writes to a new one. There is no lock
  file: nothing is left in Jellymoon's folder but the two logs. A rename that fails loses the line and never
  the command. There are never three files.
- **No flood.** The app looks once a second (`app/src/main.rs`). Of that look, one line is logged for a code,
  and no more until the look's code changes.
- **Private.** For the person alone where the system has modes, as Jellymoon's other files are.
- **Never in the way.** A log that cannot be written fails nothing: the command does what it would have done.
- **How it is proved.** At the commands: a run's end and a tried failure each leave their line, and a refusal
  leaves the tree as it was. `tree` keeps listing the log, so a refusal that logged would fail the tests there
  are. That no call can carry the person's text is held by the calls' types, and by a core test that rides in
  the row: it is a test of functions, not of a command.
- **Two more tests of functions.** No command can be made to panic from outside, and no way to panic on purpose
  is shipped. So the panic's line is proved by a test of the function that writes it, which rides in F79 for
  core and the binary and in F80 for the app. The rule of one line for a code is memory of the open app, and
  `layout` is a new process each time: it is proved by a test of the function that holds that memory, which
  rides in F80. The steps of F79 and F80 name only what a command shows.

**Refused.** A logging call that takes text. The calls take a code, numbers and a kind.

**Not covered.** Sending the log anywhere: a person attaches it by hand. What the harness itself prints: a
run's stream and stderr stay unkept, as today. Two writers at the cap at once: each renames, and the older file
may be lost. Two writers just under the cap at once: each sees room and appends, so the file can end one line
over its cap for each of them, and the next line renames it. No command fails for either.

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
- **Looking by id across the store: no code.** An answer and a stop never open the session store: they "read
  the home alone, and take no repo and no logs" (`crates/jellymoon/src/parse.rs`, above `parse_run`). A first
  start with no `--repo` does read it: it reads the day through the reader to find the wish's folder
  (`run/place.rs`, `found`). The reader needs no folder's name: it lists every folder of the store and reads
  each line's own working directory (`reader.rs`, `session_files`; `session.rs`, `read`).
- **Two rules for the store's place, in three places.** The binary works it out (`crates/jellymoon/src/main.rs`,
  `harness_folder`) and so does the app (`app/src/today.rs`, `from_env`); both take a `CLAUDE_CONFIG_DIR` that
  is not a full path. Core drops such a one before a harness is started (`call.rs`, `passed_on`). So Jellymoon
  and the harness it starts can look in two places.

**Decision.**
- **One rule for the store's place, in core.** One function says where the session store is, for every caller:
  `CLAUDE_CONFIG_DIR` when it is a full path, else the home's `.claude`. One that is not a full path is as none,
  which is what the started harness is given. The binary's own working-out goes, and the app's with it. The
  rule's pin rides in F69, as a test at `jellymoon day` and one at `jellymoon-app layout`: with a
  `CLAUDE_CONFIG_DIR` that is not a full path, both read the home's store. The second is a test of the app,
  run beside the row's `test` command, which is the binary's.
- **A look by id before "lost".** When the harness says it keeps no session of that id, core looks for a file
  named for the id in every folder of the store. It reads names only. It opens no file and works out no
  folder's name. `run answer` learns the store's place from that one function and takes no flag for it.
- **Two fixed sentences.** A session that is nowhere is lost, in today's words. A session that is there is not
  called lost: the run fails in other fixed words, which say Claude Code keeps the session and did not find it
  from the copy.

**Refused.** Core writes nothing in the store and moves nothing in it (ADR 0018, decision 6). A link in the
store is not followed.

**Not covered.** Making the harness find a session it did not find. That is under "Open". That no file is
opened cannot be seen at the binary: the store being byte for byte as it was can, and the rest is a core test.

The look by id was read in Magpie's `internal/sessions/relocate_claude.go`. Read, not copied.

### 4. Shapes the reader does not know

**Today.** Two of the four hold.

- **A compaction summary: already holds.** A `user` line with `isCompactSummary` starts no turn and ends none
  (`session.rs`, `prompt`). `tests/f26.rs` proves it, also for a summary marked as typed.
- **The copies a fork or a resume leaves: already holds.** A typed line whose `uuid` was already read is dropped
  with its replies, across every file of the read (`session.rs`, `read`; [reader.md](../development/reader.md)).
- **A session marked as moved: not known.** Nothing reads a line of type `relocated`. By reading, nothing is
  counted twice: such a line is no `user` line and the lines before it keep their own working directory. A line
  with none is judged two ways (`session.rs`, `read`): for the switched-off rule by its file's folder, and for
  the copy rule as the last decoded line that had one was. No test pins a moved session.
- **A session another program started headless: read as typed.** ADR 0018 measured that the words handed to a
  headless harness are plain `user` lines the reader's rule keeps. Only a run's own sessions are left out, by
  their copy's folder. So a headless session any other program started in the person's repo is on their page as
  their words, and goes to the reading call. reader.md says prompts "another program sent (no `origin`)" are
  left out; that is true of a line with no `origin` and not of these.

**Measured, 2026-11-17.** Labels and counts only, on one machine, over its 300 newest session logs: every typed
line carried `entrypoint`; its values were `cli` and `sdk-cli`; no file held both. No line carried `relocated`.

**Decision.**

- A compaction summary by `isCompactSummary`: kept out, as today.
- A fork's or a resume's copy by its `uuid`: counted once, as today.
- **A headless line by its `entrypoint`.** A line whose `entrypoint` is `sdk-cli` is no turn and no reply. It is
  decided for each line, never for a file: a resume writes into the same file (ADR 0018, "Measured"), so one
  file can hold both kinds. A typed-looking line that is left out still ends the turn before it. Its `uuid` is
  not held, as a copy's is not: a line that is left out hides no other. A line with no `entrypoint`, or with
  another value, is read as today.
- **A session marked as moved by its `relocated` line.** The line is no turn, and the turns around it are each
  counted once, under the folder their own line names. This is cut from the headless rule and waits for a
  sample.

Fixtures are typed by hand from what was measured. None is copied from a real log.

**What the headless rule does and does not change.**
- It never keeps more than today. Another program's words under another value of `entrypoint` are kept as they
  are now.
- It can leave out a person's own words, with nothing said: see "Not covered".
- A project is found through the same reader. A first start with no `--repo` (`run/place.rs`, `found`) and the
  projects on offer read the day's turns, so a project that is not on the list of projects and whose sessions
  that day are all headless is no longer found and no longer offered. A listed project is found and offered as
  before.
- Jellymoon's own lines change nothing. The reading call keeps no session, and a run's lines are in a copy,
  which the reader already leaves out.

**Refused.** Guessing from a session's id or from its words who wrote it.

**Not covered.**
- A value of `entrypoint` other than the two seen: read as typed until one is measured.
- A person's own headless typing: `claude -p` typed by hand, and any app the person types into that drives a
  headless harness. Its words are left out as another program's are.
- A program that drives the harness through a terminal. It is under "Open".

**Not confirmed.** That a line of type `relocated` exists, and its shape: it was read in the study and seen in
no log here. Whether a line a resume copies keeps its first `entrypoint`: no file with both kinds was seen, so
F76's fixture of such a file is typed from the rule, not from a sample.

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
  the target, synced, renamed over it, through a link, with the old file's mode. Core's unit tests hold it.
- **Line endings: already hold.** No byte but the inserted text changes. `tests/f29.rs` installs into a file
  with Windows line ends.
- **A byte-order mark: refused today.** By reading, a settings file that starts with one does not parse as a
  JSON object, so `install_hook` answers that it is not settings and leaves it as it was. This was not run.

**Decision.** The four that hold stay as they are, and are the rule. A settings file that starts with a
byte-order mark is taken: the entry goes in after the mark, the mark stays the file's first bytes, and removal
leaves the file byte for byte as it was. A file that is the mark and nothing else is taken as an empty file is
(`hook.rs`, `splice`): it becomes the mark and one object. A file install makes is given no mark.

**Refused.** As today: a file that is not a JSON object, a `hooks` of another shape, an entry the person
edited, a second settings file while the first holds the entry.

**Not covered.** A settings file in another encoding than UTF-8 stays refused. No copy of the person's file is
kept. No lock is taken on it, as none is today. A write that is killed in between still leaves its new file
beside the person's (`hook.rs`, above `replace`); this note does not change that. That a failed write leaves
the old file whole is seen at the binary on Unix only, and is held on every system by core's unit test.

**Not confirmed.** Whether Claude Code itself reads a settings file that starts with a byte-order mark. F71
measures it first. If it does not, the hook would go into a file the harness ignores: F71 is then not built,
and a progress note says so.

Removal of only what was written, and the mark, were read in Magpie's `internal/library/hooks.go` and
`internal/edit/file.go`. Read, not copied.

### 6. Stopping a run

**Today.** On a Mac and on Linux the harness is started in a process group of its own (`run/child.rs`, `run`).
A stop the person asks for sends that group an interrupt, waits 5 seconds, sends it the polite signal, waits 5
seconds, and kills it. The time limit and the stream's cap send the polite signal, wait 5 seconds, and kill
(`run/dead.rs`, `stop`; `run/child.rs`, `halt` and `GRACE`). After any end the group is killed, so nothing the
harness started in its group outlives the run. A program the harness moved to another group or session is not
reached by any kill and goes on: [run.md](../development/run.md), "Not covered", says so, and
`tests/f40/deaf.rs` asserts it.

On Windows there is no asking and no group. The harness alone is ended at once, and what it started goes on
until it ends by itself (`run/child.rs`, `halt`; `run/dead.rs`, the last two functions, which do nothing there).

**Decision.** A stop asks first, waits a short fixed time, then kills: on a Mac and on Linux, as today. On
Windows the harness is started in a job, so that when the run ends, however it ends, everything the harness
started ends with it. That is stronger than what a Mac and Linux have, where a group can be left.

**Refused.** On a Mac and on Linux, leaving a program in the harness's group going after a run's end. On
Windows, leaving anything the harness started.

**Not covered.**
- On a Mac and on Linux, a program the harness moved out of its group. It goes on, as today.
- Windows has no polite signal for a program with no window, so a stop there is still at once; only its reach
  changes.
- Three kills that ask nothing stay as they are and are under "Open".
- The Windows steps can be proved only by CI's Windows job, never by a maintainer's own run on a Mac.

**Not confirmed.** A job is made through the system's own interface, which core cannot call without `unsafe`.
The feature names a package with a safe surface, and with no gap between the harness's start and its joining
the job, and adds its note here, or this decision waits.

Asking before a kill, and a job on Windows, were read in Magpie's `internal/proc/tree.go`. Read, not copied.

### 7. Tests cannot touch a real home

**Today.** Nothing refuses it. What keeps tests off the real home is habit and one rule of core.

- The binary's test support starts `jellymoon` with only the variables a test gives
  (`crates/jellymoon/tests/support/mod.rs`, `started`), and core reads the home from the variable alone
  (`find.rs`, `user_home`). A test that gives no home reaches none.
- Some tests give no home, or one that is not a full path, on purpose: `tests/harness.rs` (unset, empty, `user`
  and `.`), `tests/f29/edges.rs` (no variable at all), `tests/f39/safe.rs` (a relative `HOME`), and the app's
  layout tests, which run it "with nothing in its environment" (`app/tests/support/mod.rs`, `run`).
- A test's world puts `HOME`, `JELLYMOON_HOME` and `CLAUDE_CONFIG_DIR` under a folder of its own
  (`tests/f39/world.rs`, `world_in`; `app/tests/support/day.rs`). That folder is under Cargo's own temporary
  folder for tests (`support/mod.rs`, `scratch`) or under the system's.
- The built programs are started from more than one place: `support/mod.rs`, `f39/world.rs`, `f52.rs`,
  `f55/started.rs`, and five files under `app/tests`. A few tests start `jellymoon` through another program:
  the hook's entry (`f29.rs`), the installer (`f33.rs`), a made-up asker (`f41/apart.rs`).
- `cargo xtask check` has four rules and none is about tests (`crates/xtask/src/rules.rs`, `check`).

**Decision.**
- **One door.** The test support is the one door a test starts `jellymoon` or `jellymoon-app` through. The
  binary's tests and the app's tests each have their support, so each has its door, with one rule.
- **What it refuses.** A full path outside the temporary folders, given as the home, as Jellymoon's folder, as
  `CLAUDE_CONFIG_DIR`, or as the value of `--logs`, `--settings` or `--repo`: one fixed sentence, and nothing is
  started. With no home, a full-path `CLAUDE_CONFIG_DIR` is where `jellymoon day` reads and
  `jellymoon hook install` writes (`crates/jellymoon/src/main.rs`, `harness_folder`).
- **Links are resolved first.** The door compares link-resolved paths on both sides, resolved as far as the
  path exists (a refused `--repo` or `--logs` may name a folder that is not there): a home under a temporary
  folder that is a link out of it is refused, and on a Mac the system's temporary folder is itself reached
  through a link.
- **What it lets through.** No home, an empty one, and one that is not a full path. Core reaches nothing with
  any of them, and the tests that pin that stay as they are.
- **The login shell.** The door sets `JELLYMOON_SHELL` to nothing in every world, itself, so no test can forget
  it. A test may name a made-up login shell by a full path under a temporary folder; any other value is refused.
  Until decision 1 is built nothing reads the variable, so the door can land first.
- **Held by a check.** `cargo xtask check` refuses a test file that starts either program, or builds a home,
  outside the support. No test under either program's tests can see an xtask rule, so the rule's own test rides
  in the row as a test of xtask, beside the row's `test` command. That every earlier test still asserts what it
  asserted is no step of any test: it is what the tests passing in preflight says.
- **Inside core.** Core reads its variables inside itself (`find.rs`, `harness`, `user_home`, `own_folder`).
  Once decision 1 is built, a unit test that called the look with the process's own environment would ask the
  builder's real login shell. No test in core or in the app calls it so: a test hands the look its variables.
  `cargo xtask check` holds that too, in F74 and F75.

**A temporary folder is the system's, or the one Cargo gives tests.** The decision was first worded as the
system's alone. Most worlds today are under Cargo's (`support/mod.rs`, `scratch`), and a rule that refused them
would refuse nearly every test there is. So both count, and nothing else does.

**Not covered.** Core's unit tests, which start no built program. The smoke chain, which builds its own world
(`crates/xtask/src/smoke.rs`): it sets `JELLYMOON_SHELL` to nothing in the door's row. A test that starts
`jellymoon` through another program gives that program a world the door has passed. By reading there are four:
the hook's entry (`f29.rs`), the installer (`f33.rs`), a copy of the program under another name
(`f35/delivery.rs`) and a made-up asker (`f41/apart.rs`). F74 names them and any other its builder finds.

A test world that refuses a real home was read in Magpie's `internal/testenv/testenv.go`. Read, not copied.

## The features

Each is a row of `feature_list.json`, proved at its `test` command, built by one builder and read by one gate.
A gated feature waits for `GATE: PASS` on its exact head before it merges.

**Four of the first seven rows are never built.** A row is never reworded, so F67, F68, F70 and F73 stay in
the list byte for byte, with `passes: false` for good, and new rows stand in for them. The repo did this once
for one step (F49's last step replaces F32's step 4). Each new row's last step names the row it stands in for.

**The list itself says so.** `feature_list.json` has a key `dead` beside `features`: each dead row's id, with
the ids that stand in for it. The scoreboard `init.sh` prints shows `dead` for them, and the start-of-session
step in `AGENTS.md` takes the first row with `passes: false`, no `blockedBy` and not dead. Nothing but
`init.sh` reads the list, so the new key breaks no other reader.

The order to build in is the order of this table, not of the list. `AGENTS.md` says so in the same step.

| Order | Feature | What | Systems | Waits on | Gated | Stands in for |
|---|---|---|---|---|---|---|
| 1 | F74 | The door for `jellymoon`'s tests: it refuses a real home, sets `JELLYMOON_SHELL` to nothing, and `cargo xtask check` holds it | every | nothing | no | F73 |
| 2 | F75 | The same door for `jellymoon-app`'s tests | every | F74 | no | F73 |
| 3 | F76 | A headless line (`entrypoint` `sdk-cli`) is no turn; the summary and the copies stay pinned | every | nothing | yes: the reader | F70 |
| 4 | F77 | No `CLAUDECODE` or `CLAUDE_CODE_*` variable reaches a started harness | every | nothing | yes: what a run is started with | F67 |
| 5 | F78 | The harness is found through the login shell's `PATH`; `jellymoon harness` says how | Unix; Windows says it is not asked | F74, F75 | yes: it chooses the program a run starts, and starts a login shell | F67 |
| 6 | F79 | `app.log` from core and the binary: a run's end, a tried failure, a panic | every | nothing | yes: it is what a person sends off the machine | F68 |
| 7 | F80 | `app.log` from the app, one line for a code until it changes | every | F79 | yes: the same | F68 |
| 8 | F81 | A shim is started with the login shell's folders on its `PATH` | Unix | F78 | yes: what a run is started with | F67 |
| 9 | F82 | A moved session's turns are counted once | every | a sample of a `relocated` line | yes: the reader | F70 |
| 10 | F69 | A session the harness did not find is looked for in the store before it is called lost; one rule for the store's place | every | nothing | yes: a run, and the reader's default place | |
| 11 | F71 | The hook goes into and out of a settings file that starts with a byte-order mark | every | nothing | yes: the person's settings | |
| 12 | F72 | On Windows everything a run's harness started ends with the run | Windows, in CI only | nothing | yes: a run | |

F74 to F77 go in before the next release candidate: the door because F78 is not safe without it, and F76 because
it is the one fault of these a person meets today.

Three rows are built as written, with what this note adds to them. F69's three places in its second step are
three folders of any name, since no folder's name is worked out, and the one rule for the store's place rides
in it. F71 is measured first (decision 5), and its last step is seen at the binary on Unix. F72's Windows steps
are CI's.

What already holds gets no feature of its own. Its pins ride in the row beside it: the harness's own words in
F69, the summary and the copies in F76, the exact removal and the line ends in F71, the Unix stop in F72.

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
- **Owners, and the folders above, in the file checks.** Not checked for a fixed place today, and not added for
  the login shell's folders: it would be a new rule for every place, which is page.md's to make.

## Open

1. **A session that is there and not found.** Decision 3 lets core say so, and nothing more. Core cannot make
   the harness find it, and it writes nothing in the store. By reading, the harness and not core chooses where it
   looks, so F69 changes a failed run's words and not its end. Whether a follow-up whose fork was not found
   should still start with a new session, as it does today (`run/follow.rs`, `Go::started`), is not decided.
2. **A person's own `claude -p`.** Decision 4 leaves out every `sdk-cli` line, and a person who types
   `claude -p` at their terminal typed it. Whether such a line carries `sdk-cli` was not measured, and no field
   was found that would tell the two apart.
3. **A program that types into the harness through a terminal.** Its sessions look typed in every field read
   so far. No rule is invented for them.
4. **Three kills that ask nothing.** The reading call at its limit ends its child alone, with no group and no
   polite signal (`call.rs`, `run_most`). The child a cut-off run left is killed at once (`run/dead.rs`, `kill`).
   A stream that does not start as a run's is killed at once (`run/child.rs`). Whether any should ask first is
   not decided: the last two are kills of something already taken for hostile or orphaned.
5. **The login shell's record against the home's rule.** Core does not ask the system's record for the home, on
   purpose. Decision 1 reads the login shell's name from that record. The door's `JELLYMOON_SHELL` is what keeps
   a test from starting a real person's login shell. A run given a scratch home by hand still asks the real one
   unless the variable is set.
6. **How much of decision 7 is moving tests.** At least nine files start a built program today, and three
   more start one through another program. Bringing them through the doors is most of F74 and F75, and it adds
   no rule a person meets.
7. **A command a second each.** `jellymoon` is a new process for each command, so with no harness elsewhere each
   command that needs one asks the login shell again. Whether the answer should be kept on disk is not decided:
   a kept `PATH` is a file that names the person's folders.

## What changes in earlier notes

- [page.md](../development/page.md), "Where the harness is looked for": "no shell is asked for its `PATH`"
  becomes the order with the login shell after the fixed places, in F78; the started `PATH`, in F81. The line
  that the system's record of the account is not asked stays true of the home.
- [install.md](../development/install.md), "Opening the app": the app started through LaunchServices, "which
  gives it no shell `PATH`", then asks the login shell when the listed places hold no harness, in F78.
- [run.md](../development/run.md), "The session": "Jellymoon never reads, writes or removes anything there"
  becomes: it lists the names of the store's files when the harness says a session is not kept, and still opens,
  writes and removes nothing, in F69. "The time limit" and "A stop the person asks for": what Windows does, in
  F72.
- [reader.md](../development/reader.md), "What counts as typed by a person": the `sdk-cli` line, and the
  sentence on prompts another program sent set right, in F76; the moved session, in F82. Its "writes nothing"
  stands but for a panic's line, in F79. Where the store is by default follows core's one rule, in F69.
- [delivery.md](../development/delivery.md), "Install and uninstall": a file that starts with a byte-order mark,
  in F71. Its "a refusal writes nothing" stands unchanged, and so do ADR 0018's and project.md's.
- [workflow.md](../development/workflow.md), "What a worker's own runs taught": the lesson that `HOME` is never
  unset in a probe becomes the door, which lets a test do it on purpose, in F74.
- [smoke.md](../development/smoke.md): `jellymoon harness` is a step, in F78.
- `docs/GLOSSARY.md` gained **session store**, **shim** and **Jellymoon's log** with this note, and gains
  **login shell** with its amendment.

## New dependencies

None is decided. Two decisions may need one, and each then adds its note here before the package is added:
reading the user's record for the login shell (decision 1), and a job on Windows (decision 6).

## Amended after review

The after-merge review failed this note with six faults. Each line is one change and why.

1. **Decision 6, today.** "Nothing the harness started outlives the run" was false: the kill reaches the group
   only, and F40 asserts that a program moved away goes on. Reworded; the moved program is under "Not covered";
   the Windows job is said to be stronger.
2. **Decision 3, today.** "In a run, core never opens or lists the session store" was false: a first start with
   no `--repo` reads the day. Reworded to an answer and a stop. One rule in core for the store's place; the look
   is by id in file names only, and the two worked-out folder names are gone.
3. **Decision 2 against "a refusal writes nothing".** The log would have broken it. The older rule stands: a
   refusal, a usage error and the hook log nothing.
4. **Decision 7.** The door would have refused the tests that hold "the home is the variable and nothing
   else". It now refuses only a full path outside the temporary folders.
5. **Rows a test cannot reach.** Each app half has its own row with a `-p jellymoon-app` test (F75, F80). The
   call-level proof of the log is a core test riding in F79. "Every test asserts what it asserted" is preflight's
   to say, not a step.
6. **A name in the log.** A last name is the person's word. No name at all: a code, numbers and a kind.
7. **Order.** The door lands first and sets `JELLYMOON_SHELL` to nothing; the login shell waits on it. The
   headless rule moves ahead of the release candidate.
8. **Decision 1.** An interactive login shell, and what its profile may do; `JELLYMOON_SHELL` as the person's
   word, held to the file checks; more than one pair of marks refused; the link-resolved file's folder checked;
   what is remembered, and what is not; never on the window's thread. Reading a shim is dropped for starting it.
9. **Decision 2.** One writer rule, a panic's place only in Jellymoon's own crates, rows for the panic line,
   and no flood from the once-a-second look.
10. **Decision 4.** `entrypoint` measured and decided per line; a left-out line's `uuid` is not held; the
    sentence on a line with no folder set right; reader.md's sentence reconciled; `relocated` is what stays not
    confirmed.
11. **Rows.** F67 is cut in three (F77, F78, F81), F70 in two (F76, F82), F68 in two (F79, F80), F73 in two
    (F74, F75). Each new row says its systems.
12. **The glossary.** **login shell** is a term, and the word shell alone means the app's window.
13. **Earlier notes.** The sentences of run.md and install.md are listed.
14. **Decision 5.** Whether the harness reads such a file is measured first; a file that is the mark alone; the
    new file a killed write leaves is listed.
15. **Order of the rest.** F69, F71 and F72 come last.
16. **The gated column** was right and is kept; F79 and F80 stay gated as the safe reading.

### Amended again, before the amendment merged

The amendment was read before it merged and failed on one fault, with nine notes.

1. **Steps no command can show.** F79's panic step and F80's last two steps are reworded to what a command
   shows. The panic's line and the rule of one line for a code are tests of functions riding in their rows. No
   way to panic on purpose is shipped.
2. **An xtask rule's proof.** Its own test rides in the row as a test of xtask.
3. **The store's place.** The app's copy of the rule is named, and the one rule is pinned at `jellymoon day`
   and at `layout`, in F69.
4. **The door.** It also refuses a full-path `CLAUDE_CONFIG_DIR`, `--logs`, `--settings` and `--repo` outside
   the temporary folders, and compares link-resolved paths. The tests that start a program through another
   program are listed. No test calls the look with the process's own environment.
5. **The way a real person takes** is run by no test: said, with what a person checks by hand.
6. **The wait.** 5 seconds, not 2; the group is killed as soon as the line is read or the wait ends, and always
   before the command returns. The fast case was measured, the slow one was not.
7. **A folder that is not there** is not passed on a started `PATH`.
8. **Dead rows.** The list names them under `dead`, the scoreboard prints it, and the start-of-session step
   skips them and takes this note's table for the order.
9. **The headless rule's effects** are said: it never keeps more, it can leave out a person's own headless
   typing, and a project with headless sessions only that is not on the list of projects is not found.
10. **Small text.** The commit this was read at; the glossary's **login shell**; no lock file for the log, with
    the race at the cap under "Not covered".

### Amended after the door's review

The door for `jellymoon`'s tests (F74) was reviewed after it merged and failed on two faults, with ten notes. All
twelve are taken in F75, which gives the app's tests the same door: the two share one rule.

1. **Decision 7, what it refuses.** `JELLYMOON_APP_DIR` and `JELLYMOON_BIN_DIR` join the folders the door
   checks: `jellymoon uninstall` removes under them. A `--logs` or `CLAUDE_CONFIG_DIR` that is no full path is
   refused when it climbs with `..` or is rooted with no drive, since both are read as given from the folder the
   program is started in. On Windows a variable's name is the same name in any case.
2. **Decision 7, the temporary folders.** "Both count, and nothing else does" was short by one: on Unix `/tmp`
   counts for the binary's tests, as the one folder a test can name that is another owner's. The app's door has
   the two and no third. A temporary folder that is a file system's root, or that is or holds the home of the
   person running the tests, is taken for none, and the door then starts nothing. A path through more links than
   the door reads is refused.
3. **Decision 7, the login shell.** The door also sets `JELLYMOON_SYSTEM_BIN` to nothing unless a test names
   folders of its own, so no world looks for a harness in a folder of the machine.
4. **Decision 7, inside core.** "A test hands the look its variables" was wrong: core's look takes none and
   reads the process's. What holds is that no test of core or of the app has the look made with the process's
   own environment, and `cargo xtask check` refuses one that does.
5. **Held by a check.** In each program's tests `support/door.rs` alone may name a built program; the rest of
   `support/` may make a temporary folder and nothing more.
