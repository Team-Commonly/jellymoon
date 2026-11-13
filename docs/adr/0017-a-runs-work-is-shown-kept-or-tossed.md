# 0017 · A run's work is shown, then kept or tossed, without git

Status: Accepted, by the coordinator's decision, 2026-11-12, on the owner's direction of that day. It adds to
[ADR 0014](0014-a-run-is-started-headless.md), decision 4, and changes no other decision there.

## Why

The owner moved the product's centre on 2026-11-12. Jellymoon is a way to build with agents that needs no terminal
and no editor, whatever harness the person has; the page is the record of it. The loop a person plays:

1. **Wish.** Say or type what should exist.
2. **Watch.** A little one works on it.
3. **Answer.** It asks in plain words when it must, and the person answers.
4. **See.** The person looks at what it made.
5. **Keep or toss.**

Steps 1 and 2 are built (F37 to F43). Today the loop stops there: a finished run leaves one commit on
`jellymoon/run-<id>`, and taking it needs git in a terminal. This note decides steps 4 and 5. Step 3, voice and the
other harnesses each get their own note, in the order under "What comes after".

## Decisions

1. **A finished run's commit is its work, and the work waits.** The wish is `done` with its star, as now, and the
   work waits until the person keeps it or tosses it. That decision is made once and written in the run's record.
   Nothing decides it for them: no clock, no look.
2. **Showing reads the repo's store, never a folder.** `jellymoon run show --wish <id>` prints what the work made,
   changed and removed, from the run's commit and its parent. `--file <n>` prints one of those files as it now is:
   the whole text, never a diff. Core draws nothing it has not checked: a name or a text is cleaned at the door
   every kept text is cleaned at, a text is cut at a stated size, and a file that is not text is named and sized,
   never read into the page.
3. **Nothing the work made is opened or run.** No file of the work is handed to the system's opener, a browser or an
   interpreter, by core or by the shell. A file's default program can run it. The person opens what they kept
   themselves, in their own project.
4. **Keeping lands the work on the branch the person's checkout stands on, as one commit by them.**
   - The checkout still where the run started: the branch is moved forward to the run's commit.
   - The person moved on, and the run's start is still in their history: git merges the work with theirs in the
     store (`merge-tree --write-tree`), core writes one commit on top of theirs, and the branch is moved forward to
     it. Their checkout is touched by one step only, a fast-forward, which git refuses whole when it cannot do it.
   - Anything else is refused with nothing changed, in plain words: the same lines changed on both sides, an unsaved
     change or a file of the person's in the way, no branch stood on, a rebase or the like in progress, a history
     rewritten since the run started, a git too old to merge in the store (before 2.38).
   - git's hooks are off, as for every git step of a run. No remote is asked and nothing is pushed.
5. **Keeping never fetches, and work that names another repo is refused.** A commit that adds or changes a submodule
   entry or `.gitmodules` is not kept by Jellymoon, and the fast-forward is told not to recurse.
6. **Files the person's tools obey need a second yes.** Work that touches a harness's or an editor's own settings or
   standing instructions (a fixed list of names in core, such as `.claude/`, `.codex/`, `CLAUDE.md`, `AGENTS.md`,
   `.vscode/`, `.envrc`, `.github/workflows/`) is marked when shown, and `run keep` refuses it unless asked in so
   many words. The list is a warning, not a wall: kept work is code in the person's project, and they trust it as
   they trust what their agent writes in a session of their own. A name is compared as the disks read it, not
   letter by letter (`run/obeyed.rs`; `show.md`, "Files the person's tools obey"), and `run keep` asks it of
   the name as git keeps it and of the name as it is printed, as `run show` does.
7. **Tossing deletes the run's own name and nothing else.** The record takes `tossed` and the commit's id first, then
   the branch is deleted as the run's own ref at that commit (the delete a look uses, ADR 0014 and `run.md`). The
   wish becomes `let go`. git keeps a commit nothing names for a while, so the record is how a toss is undone by
   hand; the app offers no undo. A toss of a failed or cancelled run removes its kept copy too.
8. **Intent before effect.** `run keep` writes what it is about to land in the record before the fast-forward, so a
   keep that was cut off is finished or begun again by the next one, and never lands twice.
9. **A tap on a little one opens its wish.** For a made wish whose work waits, the page shows the wish's own words,
   what was made in plain counts and names, and three choices: `Show me`, `Keep it`, `Toss it`. The shell calls
   core for each, off the window's thread, and draws core's words.

## What changes in earlier notes

Each line is changed by the feature that makes it true, not before.

| It says today | It will say |
|---|---|
| ADR 0014, 4: "The branch is kept: it is the work." | The branch is kept until the person keeps the work or tosses it; either removes the run's branch. F45 made the half about keeping true and changed the line so far |
| `wish.md`: a made wish's "state moves only with its one run" | And with a toss, which lets the wish go |
| Glossary, **copy**: "the branch, with the work on it, is kept" | Kept until the work is kept or tossed. F45 changed it to "until the work is kept" |

A run still never touches the person's checkout. Keeping is the person's own command.

## Measured

On git 2.50.1, on a Mac, in a made-up repo, before any of this was built:

| Scene | What git did |
|---|---|
| The checkout at the run's start, clean | `merge --ff-only <run's commit>`: exit 0 |
| The person committed to another file | `merge-tree --write-tree`: exit 0; `commit-tree -p HEAD`; `merge --ff-only`: exit 0, one parent |
| The person committed to the same line | `merge-tree`: exit 1; `HEAD`, index and files unmoved |
| An unsaved change in a file the work changes | `merge --ff-only`: exit 1; `HEAD` and the file unmoved |
| An unsaved change in another file | `merge --ff-only`: exit 0; the unsaved change still there |
| The person's own untracked file where the work makes one | `merge --ff-only`: exit 1; their file unmoved |
| What the work is | every question asked of git, and how far each answer is read, is the table in `docs/development/show.md`, "What is asked of git" (as built by F44: no file name is an argument to git) |

Not measured: git 2.38 and 2.39 (the oldest that can merge in the store, and the one older Macs ship), Windows and
Linux, a repo with filters or LFS, and a keep cut off between its steps. The features below measure them.

Measured as F45 was built (2026-11-12), on git 2.50.1 on a Mac, in made-up repos with hooks of every kind
planted. The whole account is `docs/development/keep.md`, "What was measured" and "What is not closed".

| Scene | What git did |
|---|---|
| `merge --ff-only` with an unsaved or a staged change in a file of the work, an untracked file, a file where a folder would go, a folder where a file would, or `index.lock` | exit 1; files, index, `HEAD` and refs unmoved |
| the same with an ignored file of the person's in the way | written over, unless `--no-overwrite-ignore` is given: then exit 1 and nothing moved |
| `read-tree -m -u` with an ignored file in the way | written over. So it is used only as a trial (`-n`), never as the step |
| `merge --ff-only` on a detached `HEAD` | moves it. So the branch is asked for first |
| `branch.<name>.mergeOptions "--squash --autostash --no-ff"` | overridden by `--ff-only --no-squash --no-autostash` on the command line |
| a hook of every kind git starts on a merge, a checkout, an index write and a ref write, with `core.hooksPath` an empty folder | none started |
| `merge-tree --write-tree` with a merge driver the config names for a file both sides changed | the driver is started; with `-c merge.<name>.driver=` it is not, and git exits 1 |
| a checkout with a `filter` whose command is set to nothing | the file is written as the store has it. So a keep refuses work that would write such a file |
| `merge --ff-only` asked through a linked worktree's git folder | that worktree's files, index and branch alone |
| work that makes `A.TXT` beside a tracked `a.txt`, on a disk that takes them for one name | with an unsaved change in `a.txt`: exit 1, nothing moved. With none: exit 0, and `a.txt` holds the work's text. So a keep asks the disk first |
| `symbolic-ref HEAD` where `HEAD` names a name for a branch | the branch at the end: git follows it |
| a keep ended after each of its steps (a test's build ends itself there) | the next keep finishes it or begins again; one commit on the branch |

Still not measured: git 2.38 and 2.39 themselves (the refusal is tested with a made-up git that says an older
version), Windows and Linux by hand (CI runs the tests there), real LFS, a sparse checkout, and git killed inside
its own checkout, which no test can time.

## Found while building F45

1. **git moves the person's branch, not core.** The step that writes the checkout is `git merge --ff-only`, and
   git moves the branch `HEAD` names under its own check of the value it read. Core does not write that ref with
   an old value of its own: a write of the ref apart from the checkout would leave, at a cut between the two, a
   branch ahead of its files or files ahead of their branch. Core asks first where the checkout stands, and asks
   again afterwards whether the branch is at the commit. The run's branch is deleted by core, as itself, under
   the old value.
2. **What a keep prints has two names more than the feature's row**: `settings`, the files tools obey, and with
   `--ask` `fits`. A reason the work does not fit is printed by `--ask` as data, exit 0.
3. **Stricter than decision 5**: work that removes another repo's entry or `.gitmodules` is refused too.
4. **A file with a `filter` attribute is refused**, since every filter is switched off and the file would be
   written other than as the person's filter makes it.
5. **Work the branch holds already is marked kept and nothing is landed**: the person moved the branch
   themselves, or their last commit holds exactly what a keep would land.
6. **A `HEAD` that names a name for a branch** stands on that branch, as git says, and the work lands there.
7. **When the branch is still at the run's start, the commit that lands is the run's own**, whose author is who
   the copy found the person to be, or Jellymoon's fallback name. Only the one new commit of decision 4's second
   case is made under the person's name of that moment, and it is refused when git names none.

## Found by the gate on F45 (2026-11-13)

The gate on F45 found eleven things, four of them blocking. What was decided, and what was measured for it (git
2.50.1, a Mac; the whole account is `docs/development/keep.md`):

1. **What is switched off is read where the step runs.** A linked worktree's own config (`config.worktree`) is
   read by git only through that worktree's git folder. The keep read the config's names through the shared git
   folder and ran the step through the worktree's, so a filter named there was started. Every git a keep starts
   is now given the names read through both.
2. **Whether a file has a filter is asked as git will read it when it writes.** A file with a `filter` is
   refused, since the filter is off (found while building, 4). git reads `.gitattributes` from the commit it
   checks out, and from the disk only where the commit has none; no one `check-attr` reads that way. Measured:
   with an untracked attributes file that sets the filter and an unsaved edit of a tracked one that unsets it,
   the checkout says `unset`, the commit by `--cached` and by `--source` says `unspecified`, and the step runs
   the filter. So four readings are asked and any `filter` refuses: the commit's index over a copy of the
   checkout's attribute files the commit lacks; the commit's alone; the checkout's files; the checkout's index.
   `--source` is not used: the commit read into an index of Jellymoon's and asked with `--cached` is the same
   question on every git. The sentence "no file is ever written other than as its filter would have made it"
   now stands with three stated gaps (an attributes file changed between check and step, a case-folded spelling
   of `.gitattributes`, and a linked `.gitattributes` on git before 2.32).
3. **A checkout moved just before the step keeps the work where it landed.** Nothing holds a checkout still
   between the checks and `git merge`. When git took the step and the meant branch is not at the commit, `HEAD`
   is asked: at the commit, the work is kept, the record's `onto` is the branch it is on or the one word `HEAD`
   for none, and the words say so; the run's branch is deleted only when a branch holds the commit. On a branch
   that is ahead of the commit and holds it, git had nothing to do, and the keep says the branch held the work
   already. Anywhere else, the late refusal stands.
4. **One rule says which branch names a record holds**, for the keep that writes and the command that reads. A
   branch whose name holds a space of any kind, a line break or a control character, or is longer than 1024
   bytes, is refused before anything is written. Before, such a keep landed and left a record every later
   command called someone else's.
5. **The late refusal says what is true of every cause**: git stopped the step, files of the work may already be
   written, the next keep begins again. Measured: with a folder of the checkout shut, the trial passes, the step
   exits 1 with some files written, `ORIG_HEAD` written and the branch unmoved. `ORIG_HEAD` is no longer among
   what a refusal is said to leave.
6. **A file that was only touched is no unsaved change.** The trial runs on a copy of the checkout's index that
   has been refreshed, so the check reads the file and writes nothing of the person's, and `--ask` takes no lock
   on their index. Measured: `read-tree -n -m -u` refuses a touched file until `update-index --refresh` has read
   it. A filter the repo says is required is switched off as any other, with its `required`, so a
   touched file that has one is compared as it is.
7. **Smaller ones**: a name or an address made only of spaces or of the marks git takes off a name is no name; a
   config with more than 8 KiB of names to switch off, or one with `=` in it, is refused in its own words; a
   keeps' lock that cannot be made is said as that; the hook a test's build ends a keep at is compiled out of a
   release with the read of its variable.

## Found by the second gate on F45 (2026-11-13)

The second gate found ten things, three of them blocking (git 2.50.1, a Mac; `docs/development/keep.md` has the
whole account):

1. **Attributes are asked of the index the step would leave.** The four readings took a `.gitattributes` the
   work removes as still there. Measured: a tracked `sub/deep/.gitattributes` that unsets a filter, removed by
   the work beside a new file there, and an untracked `sub/.gitattributes` of the person's that sets it: git
   removes before it writes and starts the filter. git reads attributes from the index it is leaving, and from
   the disk only where that index has none. So the trial, once passed, is taken again without `-n` and without
   `-u` into the copy of the index, and three of the four readings are asked of that copy; the fourth is the
   commit's own. One the work changes or adds is read from the index, whatever git writes first; one the person
   staged is read as staged. A change that takes a filter off is still refused, by the checkout's own file.
2. **A run's branch is never where work was kept.** A checkout moved onto the run's own branch just before the
   step was said "kept, on another branch", and the next keep deleted that branch. Now `HEAD` on any name
   under `refs/heads/jellymoon/run-` after the step is a refusal in fixed words, the record keeps what was
   meant, the branch is left, and the next keep begins again. A record whose `onto` is such a name is not core's.
3. **Three guards have tests**: a filter named only by `process`, a `HEAD` detached ahead of the work, and a
   checkout moved away and back. One comparison was no guard and is gone: the branch `HEAD` names is where
   `HEAD` is.
4. **Smaller ones.** The words of work kept on no branch are true of a joined commit too. A question that must
   merge writes one tree in the store, and says so when the store takes none. A keep begun again asks git
   whether what is in the way is ignored, so it agrees with a first keep. A name the disk does not take and a
   branch stood on under another spelling have words of their own. Two outcomes of a moved checkout are stated
   and not closed. The list of a change is one function for a look and a keep, and a test's wait is compiled
   out of a release.

## The slices

`feature_list.json` is read top down, and F30 and F31 stand above these. The order is stated here: **F44 to F47 are
built before F30 and F31.**

| Feature | What it adds | Waits on | Waits for `GATE: PASS` |
|---|---|---|---|
| F44 | `jellymoon run show`: what a run's work made, and one file's text | nothing | no |
| F48 | A run's record names its repo, the commit it started from and the commit it ended as | F44 | yes: it changes what a run writes |
| F45 | `jellymoon run keep`: the work lands on the person's branch | F44, F48 | yes: an agent's work reaches the person's checkout |
| F46 | `jellymoon run toss`: the work and its branch go, and the wish is let go | F44, F48 | yes: it deletes in the person's repo |
| F47 | A tap on a little one opens its wish: `Show me`, `Keep it`, `Toss it` | F45, F46 | no |

## Found while building F44

A run's record (`runs/<id>/run.json`) names no repo and no commit, and the copy's record that named the repo is
removed when a completed run's copy is cleaned. So F44 finds the repo as a run finds it, and takes as the work the tip
of `refs/heads/jellymoon/run-<id>` against its one parent, when that tip is a commit with one parent and the message a
copy ends with. A commit made by hand in that shape, under that name, is shown as the run's. Showing it harms nothing;
keeping it would land work no run made.

**F48 closes this before anything is kept or tossed**: a run writes its repo, its start and its end in its record,
and `run show` takes the work from the record and checks the branch against it. `feature_list.json` cannot give F45
and F46 a new edge, so the order is stated here: **F48 is built before F45 and F46.**

As F48 is built: a record from before, which names none of the three, is still shown by the branch's shape, and
whoever can write the home can make a record into one from before. So **`run keep` and `run toss` refuse a record
that names no commits**: only a look may go by shape. The path in the record is compared and never resolved, so a
repo that was moved or copied since the run is refused as another repo. The record names the git folder the repo's
checkouts share, so with linked worktrees it does not say which checkout a run started from, and `run keep` goes by
`start` and by where the person stands.

## What `run keep` and `run toss` may build on (2026-11-12)

Two sentences were said of a run's record more widely than they hold. Each is true with its scope, and F45 and
F46 are built on them as scoped.

1. **Of a finished run, as written:** a record's `end` is its `start`, or the one commit core made whose one
   parent is `start`. That holds of every `end` core writes, and a record whose words and names do not agree is
   refused (`Record::named`). It says nothing of where the branch is. The branch can be moved after the last
   check of a run's end, and a record can be changed by hand. So `run keep` and `run toss` compare the branch
   with `end` again, under their own check of the old value, and never take the record's word for it.
2. **Of a finished run only:** a record names all three of `repo`, `start` and `end`, or, from before F48,
   none. A run that is going, failed, was cancelled or was cut off names `repo` and `start` and no `end`; or,
   when its record was written before F48, or was written down as failed by the released v0.2.0-rc1 (which
   drops the three, as `run.md` says), it names none of the three. A record of two is so by design, and its
   branch may hold core's commit all the same: a run killed after its commit, or one whose copy could not be
   removed. `run toss` acts on failed and cancelled runs, and is built for a record of two and for a record
   of none.

**Decided 2026-11-12, by the coordinator: `run toss` of a record that names none removes its kept copy as `copy
clean` does and leaves the branch, in fixed words. A branch is deleted only as the run's own ref at a commit the
record names; a branch no record names a commit for is left to the copy's own take-back rule (`copy/back.rs`).**
Of the sentence above, that `run keep` and `run toss` refuse a record that names no commits, this is what stands:
neither acts on a branch by such a record. `run keep` refuses it, and `run toss` takes back its copy and no more.

## What comes after

Each gets its own note before it is built.

1. **A run that answers back** ([ADR 0018](0018-a-run-asks-and-is-told-more.md)). A run today is one shot: it
   cannot ask, and it cannot be told more. Which of an agent's questions reach the person, how an answer gets
   back, and "more like this" on work that was shown.
2. **A wish said aloud, from anywhere** (F30): a held key, and a little one at the edge of the screen.
3. **Codex** (F31), then one starter that speaks ACP.

## What this does not do

- It shows no running program and no picture of one. A preview needs the work run, and nothing here runs it.
- It makes no merge commit, resolves no conflict and rebases nothing. Work that no longer fits is said to no longer
  fit, and the person wishes again.
- It does not make the list in decision 6 complete. A `Makefile`, a build script or a test can run what it likes
  when the person next builds.
