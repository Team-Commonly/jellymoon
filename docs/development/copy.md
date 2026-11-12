# A run's copy: `jellymoon copy make` and `jellymoon copy clean`

A run works in its own copy of the person's repo (ADR 0011, section 7; ADR 0014, decision 4). Core holds all of it
(`copy.rs`, and `copy/git.rs` for the git child); the commands only parse arguments and print JSON. Nothing starts a
run yet: F39 will call the same two core functions, `make_copy` and `clean_copy`.

```
jellymoon copy make --repo <folder> [--run <id>]
jellymoon copy clean --run <id> --outcome <completed|failed>
```

Both print one line, `{"run","folder","branch","kept"}`; `kept` says whether the folder is still there. A refusal is
one plain line on stderr and exit 1; a command line that is not one of the two is exit 2.

## Where things are

In Jellymoon's home (`JELLYMOON_HOME`, or `.jellymoon` in the home directory), both folders for the person alone:

- `copies/<run>` is the copy: a git worktree of the repo on the new branch `jellymoon/run-<run>`. `copies` holds
  copies and nothing else.
- `runs/<run>/copy.json` is core's record of the copy, written when it is made: the repo's git folder, the copy's
  own git folder inside it, and when the run failed. It is outside the copy, where a run cannot write.
- `runs/no-hooks` is an empty folder: git is started in it and told to look for hooks in it.
- `runs/clean.lock` is an empty file, made with the first copy: a clean, and a copy being made, holds the system's lock on it.
- `runs/<run>/start.lock` is an empty file only a `jellymoon run` makes, as soon as the run's id is taken and
  before any git is started: the command holds the system's lock on it until it ends, and removes it then.
  `jellymoon copy make` makes none.

A run id is `r` and sixteen of `0-9a-f`. Core makes it; one given with `--run` is taken only in that shape, so a
name with a dot, a separator or a full path never reaches a path. An id is taken by making `runs/<run>`, which only
one caller can do.

## Make

`--repo` is a full path to the top of a checkout. git is stopped from looking above the folder as it was named
(`GIT_CEILING_DIRECTORIES`), and is asked where the folder is in the repo it found: anything but its top is refused.
git goes by a folder's real path, so both are needed. A plain folder that sits inside another repo is refused under
every name: given directly, as `sub/..`, or through a link to it. So is a folder the repo tracks, a bare repo, a
repo with no commit, and a folder that is no repo. A link to a repo's top is that repo, as is any other name whose
real path is the top: the copy is of the repo, not of the name. The copy is of the commit the checkout is on. What the person
has not committed is not in it, and their working tree, index, `HEAD`, stash and branches are not written; the one
new thing in their repo is the run's branch (and git's own registration of the worktree).

## Clean

- **completed.** Core commits what is in the copy onto the run's branch, removes the folder itself, then has git
  drop the worktree's registration. The branch is kept: it is the work. Nothing to commit is not an error, and
  neither is a folder already gone. The commit is one commit, `Jellymoon run <run>`, on top of the branch.
- **failed.** The copy stays for a look. Of all failed copies, of every repo, the newest five stay; older ones are
  removed with their registration, and what was not committed in them is lost. Their branches stay. Newest is the
  order the runs failed in, which the records hold: each is dated later than every failed run before it, so the run
  that just failed always stays, whatever the clock says. `kept` is read after the old ones went. A clock set back
  makes the dates run ahead of it, one nanosecond a run, until it catches up; the order is unchanged.

**Two cleans at once.** One clean works at a time, of any run: it holds the system's lock on `runs/clean.lock`,
which the system drops when the process ends, however it ends. A second `copy clean` of the same run waits, then
finds what the first left: `no copy is kept for run <run>` after a completed one, exit 1, and nothing done twice.

**Copies made at once.** A copy is made behind the same lock. git reads every worktree of a repo while it adds
one, and fails on one that is half made or half removed (`failed to read .../commondir`), so copies are made one at
a time and never while one is cleaned or taken back. A `make` waits for the lock; it is not refused for it.

**How long the lock is waited for.** `copy make`, `copy clean` and `run` wait with no limit of their own. The wait
ends because whoever holds the lock ends: each git child has its five minutes (below), a `make` starts about six of
them behind the lock and a completed clean about a dozen, so one holder is done within an hour at the very worst,
and in well under a second on a repo git answers for. Commands waiting take the lock one after another. A page
is different: it never waits behind another command for more than 3 seconds ([run.md](run.md), "A run that was cut
off"), and leaves the cleanup to the next look.

**A git that never returns.** Each git child has five minutes. One still running then is killed by its process id,
and the command exits 1 with `git did not finish in 300 seconds and was stopped`. A `make` stopped so takes its
half-made copy back: the folder, git's registration of it, its new branch and its record. A `clean` stopped so
keeps the copy and its record, and can be run again. Only the git child is killed, not the programs it started:
the standard library has no call for a whole process group, and core has no unsafe code. One of those can write
into `copies/<run>` after the folder was taken back. So the folder is removed once more after the calls that take
the rest back, and every clean, under its lock, removes each real folder in `copies` under a run id that has no
folder in `runs` (a copy being made has one before its folder is there; a link is left). What git itself still
lists of such a folder is left to git's own `worktree prune`. A test shortens the limit with `JELLYMOON_TEST_GIT_LIMIT_MS`,
which only a build with debug assertions reads: a release does not. With `JELLYMOON_TEST_GIT_LIMIT_FROM` the
short limit is counted from when the file it names is there, which the made-up git leaves once it has stalled:
no git call that is not meant to stall has to end within it.

**A copy a killed `jellymoon run` left.** One whose `start.lock` nothing holds, and that no wish names, is
taken back by the next page or run ([run.md](run.md), "A copy nothing names"), behind the same lock, waited
for 3 seconds: the folder, its branch, git's registration of it and its record go, in that order (`copy/back.rs`).
The order is every take-back's: the registration is how a look knows the copy as the repo's own, so it goes
last, and a take-back cut off before it is finished by the next look. Its branch is removed only when no
checkout of the repo but the copy stands on it (by its `HEAD`, or by a rebase or a bisect stopped there), no
other ref is a symbolic ref to it, it is not one itself, and `git --no-replace-objects rev-list --max-count=1 <branch> --not --exclude=<branch>
--glob=refs/heads/* --glob=refs/tags/* --glob=refs/remotes/*` prints nothing, which is to say every commit on
it is also on another branch, a tag or a remote-tracking branch. That is the one rule: no `HEAD` is asked,
since a detached one is no ref, no other ref is (`refs/stash` and `refs/bisect/*` come and go), and nothing
the record says is. A branch that is someone's in one of those ways is kept, so a copy made of a checkout
detached at a commit no ref has leaves its name behind ([run.md](run.md), "What the rule costs"). A take-back by the
command that made the copy removes the branch without asking, as nothing but that command has had the copy.
Three more things hold only for a copy that was found: nothing is done when its start lock is held once the
copies' lock is had; a link or a plain file at `copies/<run>` is removed as itself, and what a link leads to is
not touched; and the repo its record names is asked first whether it lists the copy as its own worktree (the
copy's git folder directly in the `worktrees` of the repo's own git folder, its `gitdir` file naming
`copies/<run>/.git`, both named in the record by paths from the root). A repo that answers and does not list it has nothing done in it, and only Jellymoon's
own files of the run go. A repo that could not be asked (it is not where the record says, or cannot be read)
leaves the copy and its record for the next look. A git step that failed or ran past its limit leaves the
record, and whatever of the branch and the registration it had not reached; the copy's folder is gone by
then, since it is removed before any git step. Either is tried again for thirty days ([run.md](run.md), "A
copy nothing names").

**The record is written before the copy is made.** `runs/<run>/copy.json` first names the git folder git is
about to make for the copy, `worktrees/<run>` in the repo's shared git folder, and is written again once git
has said where it put it. A command killed while git makes the copy so leaves a record a look takes the copy
back by.

The commit is by the person: `user.name` and `user.email` as git resolves them for that repo. When either is
missing it is by `Jellymoon <jellymoon@localhost>`.

## What is not looked at

An agent could have rewritten every file in its copy, the `.git` file too, or left a link where the copy was.

- Core reads no file of the copy to decide anything. The folder and the branch come from the run id; the repo
  comes from the record. git is always told `--git-dir` and, when it must read files, `--work-tree`, and is started
  in the empty folder, so it never finds a repo from the copy. The copy's `.git` file is not read by anyone.
- Only `copies/<run>` is ever removed, and only when it and `copies` are real folders. A link under that name is
  refused and left: nothing is committed through it and nothing is removed behind it.
- The variables git takes a repo from (`GIT_DIR`, `GIT_WORK_TREE`, `GIT_INDEX_FILE`, `GIT_CONFIG_COUNT` and the
  rest) do not reach git. It gets `PATH`, the home, the temporary folder, `XDG_CONFIG_HOME`, `GIT_CONFIG_GLOBAL`
  and `GIT_CONFIG_NOSYSTEM` (which config files are read is the person's choice), and what a Windows program needs.
  Of these only full paths are passed on, by the function a run's harness gets its variables from
  ([run.md](run.md)): the folders of `PATH` named by a full path, and a home, temporary folder or config path only
  when it is one.
- For a run whose repo was found in a session log, nothing git says is put in an error, at `make` or at `clean`:
  git's words name the folder. The copy's record keeps that as `found`.
- The one time git is asked from inside a copy is in `make`, before anything has run there: where the copy's own
  git folder is. The answer goes in the record.
- **Submodule paths are left out of the commit.** A repo inside the copy is never opened (why is under "What is
  switched off"), so a run's change to a submodule is not on its branch.
- **Filters are not run, so a new large file in a Git LFS repo is committed whole**, as an ordinary file, and the
  LFS files a run sees in its copy are their pointers.
- **Run branches are never looked at again and never deleted by Jellymoon.** A person removes one with
  `git branch -D jellymoon/run-<run>` in their repo, and lists them with `git branch --list 'jellymoon/*'`. An id
  whose branch is still there is not given to a new copy.

## What is switched off

On the command line of every git call, where it wins over every config file:

| Setting | What it stops |
|---|---|
| `core.hooksPath=<runs/no-hooks>` | every hook: `post-checkout`, `reference-transaction` and the rest. A folder there that holds anything is refused |
| `core.fsmonitor=false` | the file-system monitor program |
| `filter.<name>.clean`, `.smudge` and `.process` set to nothing, `.required=false` | every filter the repo's config names (Git LFS is one). They are read from the config first. A filter whose name holds `=` cannot be named on a command line, and then git is not run |
| `commit.gpgSign=false` | the signing program |
| `submodule.recurse=false` | git commands run inside submodules on checkout |
| `protocol.allow=never`, each `protocol.<name>.allow` the config names set to `never` (git lets one win over the first; they are read from the config like the filters, and a name that holds `=` is refused the same way), `GIT_NO_LAZY_FETCH=1`, `GIT_TERMINAL_PROMPT=0` | every transport, so a missing object of a partial clone is an error and never a fetch through ssh, a credential helper or a remote helper |
| `--no-pager` | the pager |

Two more things are done by how git is called:

- The commit is made with `write-tree`, `commit-tree` and `update-ref`, which run no commit hook and no editor,
  and never start git's own maintenance.
- **A repo inside the copy is left out.** Where the repo has a submodule, every copy has a folder for it, and
  `git add` asks for the status of whatever repo it finds there by starting a second git inside that folder, under
  that repo's own config: one the agent could have written, with a filter of its own. Core reads the submodule
  paths from the index and leaves each out of the add. So a change to a submodule in a copy is not committed.

## What could not be switched off

- **`git` itself.** It is the first `git` in a folder of `PATH` that is a full path. A wrapper there is trusted.
- **Settings only a command core never runs would use**: `core.editor`, `diff.external`, a diff's `textconv`, merge
  drivers, aliases, `core.sshCommand`, `credential.helper`, `core.alternateRefsCommand`. They are not overridden one
  by one; core runs no diff, merge, fetch or push, and transports are off.
- **`include.path` and `includeIf`** files are read. Every setting above still wins from the command line.
- **What the copy's own files tell git while it adds them.** `.gitignore` there can keep a file out of the commit.
  `.gitattributes` there can change line endings or name a filter, though only one the person's config defines, and
  those are off. A new folder that holds a repo is recorded as a link to that repo's commit, and its files are not
  committed. None of this starts a program.
- **An older git** may not know `GIT_NO_LAZY_FETCH`; `protocol.allow=never` still holds. One older than 2.31 is
  refused: it cannot print the paths core asks for.

## Limits

- A sparse checkout's copy is sparse too. Its commit holds only what changed in the files that are there.
- The folder is checked once, just before the commit. An agent still running could put a link there afterwards:
  whoever calls `clean_copy` ends the run first.
- Only the git child itself is killed at the limit. A program it started would go on, though every setting that
  starts one is off; what it writes into a copy that was taken back goes at the next clean. A git killed while it writes can leave its own lock file (`index.lock`, a ref's `.lock`) in
  the repo; the next call then fails in git's words until a person removes it.
- A `make` whose every git call is stopped waits three limits: the one that failed and two that take it back.
- A clean of one run waits for a clean of another. The newest-five rule is applied under the same lock.
- A run's record goes when its copy does.
- Proved on macOS; on Linux and Windows by CI. Where a system cannot start a planted program at all (a hook that is
  a program file, a link to a folder), the test says so on stderr and checks the rest.

## The tests

`crates/jellymoon/tests/f38.rs` and `f38/hostile.rs` run the built `jellymoon` on repos they make with the real git,
with an empty global config and scratch homes. The planted programs are copies of the made-up `fake-hook`, which
leaves a `.ran` file beside itself; each test first shows that plain git does start them. `f38/limits.rs` holds
the time limit, with the made-up `fake-git` first on `PATH` (it runs the real git, then sleeps holding a lock the
test can take only once it is gone), and two cleans started together. The tests' folder is
inside this repo's own checkout, so the test for a plain folder is also what keeps a test from ever making a
worktree of this repo.
