# Keeping a run's work: `jellymoon run keep`

`jellymoon run keep --wish <id>` lands the work of a made wish's finished run on the branch the person's checkout
stands on (F45; [ADR 0017](../adr/0017-a-runs-work-is-shown-kept-or-tossed.md), decisions 4 to 6 and 8). It is
the first command that writes to a person's branch and working tree. The work is an agent's, the repo's config
may be anyone's, and the command can be cut off at any moment, so it is built on three rules:

1. **Nothing is written before every check has passed.** A refusal before the step leaves the working tree,
   `HEAD`, the index, every branch and the record as they were.
2. **One step touches the checkout.** git checks every file before it writes one, and stops whole for everything
   it can check. What it cannot check stops it part way ("A step git stops"), and then the keep says exactly that.
3. **The record says what is about to land before it lands**, so the next keep can tell a keep that landed from
   one that did not.

The rules are in `crates/jellymoon-core/src/copy/keep.rs` and `copy/keep/` (what is done in the repo: `fits.rs`
and `attrs.rs` the checks, `made.rs` the one new commit, `step.rs` the step and what is found after it) and
`crates/jellymoon-core/src/run/keep.rs` (the record and every printed word). The tests at the binary are
`crates/jellymoon/tests/f45.rs` and `tests/f45/`.

## What it prints

One line of JSON (`Kept`; the examples `crates/jellymoon-core/tests/fixtures/kept.json`, `kept-asked.json`,
`kept-loose.json` and `kept-stayed.json` round-trip):

| Name | What it is |
|---|---|
| `run`, `wish` | the run's id and the wish's |
| `kept` | the commit on the person's branch that holds the work. `null` of work that would not be kept, and, asked first, of a commit that would have to be made |
| `onto` | the branch, by its short name, cleaned at the door every kept text is cleaned at, on one line. `null` of work that would not be kept, and of work that was kept on no branch a record names |
| `why` | what was done, or with `--ask` what would be, in core's fixed words |
| `settings` | the files of the work that the person's tools obey ([show.md](show.md), "Files the person's tools obey"), cleaned, at most 200. Left out when there are none |
| `stayed` | each `.gitattributes` the work removes that was still at its path in the checkout after the step, cleaned as `settings` is, at most 200 ("A file with a filter"). Left out when there are none, which is every keep but that one |
| `fits` | with `--ask` only: whether a keep would land the work |

Two more words on the command line:

- `--ask` prints what a keep would do, or why it would be refused, and changes nothing: no ref, no file of the
  checkout, no word of the record. A reason the work does not fit is printed as data with `fits: false` and exit 0,
  since the question was answered. A wish with no work to keep is still a refusal. "Asked first" says what a
  question does write.
- `--with-settings` is the second yes of decision 6: without it, work that touches a file the person's tools obey
  is refused.

What a keep did, in `why`:

| When | Words |
|---|---|
| the branch was at the run's start | `run <id> was kept: the branch was moved forward to the run's commit` |
| the person had commits of their own since | `run <id> was kept: one new commit on top of the branch's own holds its work and the run's` |
| the branch held the work already | `run <id> was kept: the branch held the run's work already, and nothing was landed` |
| a keep that was cut off had landed it | `run <id> was kept: a keep that was cut off had landed its work, and this one finished it` |
| the checkout was moved to another branch just before the step | ``run <id> was kept, on another branch than the keep began on: the checkout was moved to the branch that `onto` names while the keep was at work, and git landed the work there`` |
| the checkout's `HEAD` was detached just before the step, and what landed is the run's own commit | ``run <id> was kept, on no branch that is named here: while the keep was at work the checkout's HEAD was detached, or moved to a branch whose name Jellymoon does not write down or the repo does not list under that spelling, and git landed the work there; the run's branch is left and still holds the run's own commit, which is the commit git landed: `kept` is its id`` |
| the same, and what landed is the one new commit | the same words to `the run's own commit,`, then ``and the new commit the keep made to join the work with the person's commits is named by nothing of Jellymoon's: `kept` is its id`` |
| it was kept before | `the work of run <id> was kept before: nothing was changed` |

When `stayed` names a file, `why` goes on after any of these with: ``; not every `.gitattributes` the work removes
is gone: each one `stayed` names is still at that path in the checkout, and a file of the work it sets a filter
for was written as the store holds it, not as that filter would have made it``, and, when more are there than the
200 it names, with ``, and more of them are still there than `stayed` names``. The work is kept all the same (git
landed the commit) and the command ends with 0.

With `--ask`, in the same order: `a keep of run <id> would move the branch forward to the run's commit`; `a keep of
run <id> would land one new commit on top of the branch's own, holding its work and the run's`; `a keep of run <id>
would land nothing: the branch holds the run's work already`; `a keep of run <id> would finish one that was cut off
after it had landed the work`; and the words of "kept before", with `fits: false`.

## Which run, which repo, which checkout

The run is found as a look finds it ([show.md](show.md), "Which run, and which repo"), and the record must name
the run's repo, start and end: a record from before F48 is shown and never kept. The repo is found as a run finds
it, the folder must be the top of a checkout, and the git folder its checkouts share must be the one the record
names.

The work lands on the branch of the checkout the command is asked in. In a repo with linked worktrees that is the
worktree named with `--repo`, and no other checkout's files, index or `HEAD` is touched. git follows a `HEAD` that
names a name for a branch to the branch itself, as it does when it says where a checkout stands, and that branch
is the one the work lands on.

One keep of a home runs at a time: a keep holds the system's lock on `runs/keep.lock` from before it reads the
run's record until it ends, and one that waits ten seconds for it is refused. So of two keeps started together,
the second reads what the first wrote and says the work was kept before. A lock that cannot be made or taken
for any other reason is said as that, and not as another keep.

## Asked first

`--ask` makes every check below and takes no step. It writes no file of the checkout, no index of the person's,
no ref and no word of the record, and takes no lock on the checkout's index: git's trial runs on a copy of the
index that is Jellymoon's. When the branch is where the run started it writes nothing in the checkout's git
folder at all (`tests/f45/late.rs` asks in a checkout whose git folder takes no new file). One time is changed
there: of an index kept in two parts, git sets the shared part's time of change to now whenever it reads the
index, the copy too (measured on 2.50.1; no setting stops it, and the person's own `git status` does the same).
What a question does write:

- `runs/keep.lock`, the empty file the keeps' lock is taken on, when it is not there yet. A question waits for a
  keep as a keep does, so that it answers of what that keep left, and the file is never removed.
- `runs/<run>/keep.index`, `keep.attrs/` and `keep.trial`, Jellymoon's own, each removed before the answer.
- With commits of the person's own, one tree in the repo's store: git merges the two there
  (`merge-tree --write-tree`), and nothing names what it wrote. In a store that takes no new object git cannot,
  and the question is answered `fits: false` in the words of "git could not merge" (`tests/f45/late.rs`).

## The steps

Every check comes first, in this order. Each is a refusal below.

1. The run's branch names the commit the record calls `end`, and that commit's one parent is the record's `start`.
2. No checkout of the repo stands on the run's branch.
3. The checkout stands on a branch of the person's whose name a record holds ("What the record takes"), nothing
   is going on in it, and the branch names a commit.
4. If that commit already holds `end` in its history, nothing is landed.
5. The files the work touches are judged: another repo's entry and `.gitmodules` never, a file tools obey only
   with `--with-settings`.
6. If the branch is not at `start`: `start` is in its history, git is 2.38 or later, git merges the two in the
   store, the files the merged tree changes are judged again (the person's own rename can carry the work into a
   file their tools obey), and git names an author.
7. The index is not locked, every name of a file that would be written is one the disk takes, git's own trial
   of the move passes, no file that would be written has a filter set ("A file with a filter"), and nothing is
   on the disk where a file the work makes would go: at its path, or a file or a link where a folder of its path
   would be. What the work itself removes from there (a file it swaps for a folder, a folder it swaps for a
   file) is not in the way, and git's trial has asked whether that is unsaved.

A name the disk does not take is one with a part longer than 255 (no disk of the three systems takes it), or one
the disk itself calls no name when asked what is there. git would stop at such a file part way; asked first, the
keep refuses in words of its own.

The trial is `read-tree -n -m -u` on a copy of the checkout's index (`runs/<run>/keep.trial`), which is first told
which files only look changed (`update-index -q --refresh`): a tracked file whose time changed and whose content
did not is no unsaved change, to the trial as to `git merge`. The check reads such a file and writes nothing of
the person's. A trial that passed is taken once more without `-n` and without `-u`, which writes the copy and
nothing else: the copy is then the index the step would leave, and it is what the attributes are asked of.
What stops only that second take is in the way as well, and is said in the same words. It was measured in a
checkout that is sparse by patterns, with a file of the person's at a path the checkout leaves out, which the
work removes: the trial passes, the second take stops (`not uptodate`), and plain `git merge` refuses
(`tests/f45/attrs.rs`).
Of an index kept in two parts (`core.splitIndex`) the copy is the part that is the checkout's own, and it reads
the shared part where it is: git 2.50.1 writes the copy and no new shared part into the checkout's git folder,
with every file of the checkout touched, and the second take is told to write the copy whole
(`-c core.splitIndex=false`), since without that git put a new shared part there (`tests/f45/late.rs`). The
trial does not see a lock on the checkout's own index: that is asked before it, and found by the step itself,
which then says the index is locked (`tests/f45/raced.rs`).

A keep begun again after one that was cut off asks the last check of step 7 in another way. git may have been cut
after it wrote the work's files and its index and before it moved the branch: those files are then where the
work would go, and they are the work. So what is on the disk where a made file would go is in the way only when
git says it ignores it (`check-ignore`, which says that of no tracked file); an untracked file there that is
not ignored has failed the trial already, and the step itself stops for whatever else is there. So an ignored
file of the person's where the work makes one is refused in the same words by a first keep, by a keep begun
again and by a question (`tests/f45/cut.rs`).

`--ask` ends here. Then:

| Step | What is done | Cut off after it, the next `run keep` |
|---|---|---|
| 1 | With commits of the person's own: `git commit-tree` makes the one new commit in the store. Nothing names it | begins again and makes another; git tidies the first away in time |
| 2 | The record takes `kept` (the commit about to land) and `onto` (the branch's whole name). `work` is not yet written | begins again: the commit the record names is not in the branch's history, so every check is made anew, for the checkout as it is then |
| 3 | `git merge --ff-only` moves the checkout and the branch to the commit. Then core asks git where the commit is ("A checkout moved just before the step"), and the disk whether a `.gitattributes` the work removes is still there ("A file with a filter") | finds the record's commit in the history of the record's branch and finishes: nothing more is landed |
| 4 | The record takes `work: kept`, with the branch the commit is on | says the work was kept before, and deletes the run's branch if it is still there |
| 5 | The run's branch is deleted, as itself and only while it names `end` (`git update-ref --no-deref -d <name> <end>`) and no checkout stands on it. It is left when the work is on no branch | (nothing is left to do) |

Whether a cut-off keep landed is asked of the record and the refs alone, before the run's branch is looked at:
a keep that landed is finished whatever became of that branch since, and wherever the person stands by then. The
work never lands twice, because a second landing is only begun when the commit the record names is in no history
of the branch the record names.

A test's build ends itself after the step `JELLYMOON_TEST_KEEP_CUT` numbers (1 to 4), as a power cut would. A
release holds no read of that variable: the read itself is compiled only into a build with debug assertions
(`copy/keep/step.rs`). `tests/f45/cut.rs` cuts a keep after each step, forward and with the person's commits,
and runs the next one.

## A checkout moved just before the step

Every check is made for the branch the checkout stood on when the keep looked. git moves whatever `HEAD` names
at the moment the step runs, and nothing holds a checkout still between the two. So when git says it took the
step and the branch the keep meant is not at the commit, core asks where `HEAD` is:

| `HEAD` after the step | What the keep says and does |
|---|---|
| on a run's branch (any name under `refs/heads/jellymoon/run-`, those letters in either case), whether git took the step or not | not kept, in the words of "moved onto a run's branch" below: the record keeps what was meant, the run's branch is left, and the next keep begins again |
| on a branch of the person's whose name a record holds and the repo lists, byte for byte, with its tip where `HEAD` is, at the commit | kept, in the words of "another branch": the record's `onto` is that branch, and the run's branch is deleted |
| detached at the commit, or at the commit on a branch whose name no record holds or the repo does not list under that spelling | kept, in the words of "no branch": the record's `onto` is the one word `HEAD`, what is printed has `onto: null`, and the run's branch is left. No ref is written. Moved forward, the run's branch is the one name of Jellymoon's the commit has; joined with the person's commits, the run's branch names the run's own commit and the new commit has no name but `kept` and the detached `HEAD` (or the branch of the odd name) |
| on such a listed branch, ahead of the commit with the commit in its history | kept, in the words of work the branch held already: git had nothing to do there. The record names that branch, and the run's branch is deleted |
| anywhere else: detached somewhere else than at the commit (ahead of it too), or on a branch that does not hold it | the late refusal ("A step git stops") |

**A run's branch is never where work was kept.** A checkout moved onto the run's own branch just before the step
is already at the run's commit, so git says it has nothing to do; said as "kept there", the next keep would find
the work "landed" on that branch and delete it, with the commit on no branch at all. So nothing was kept:

``while the keep was at work the checkout was moved onto a branch named as a run's is, where Jellymoon keeps no
work: git may have moved that branch forward and written the work's files, and nothing is recorded as kept; the
branch of run <id> is left, and the next `run keep` begins again``

It is said whenever `HEAD` names a run's branch after the step, whether git took the step or stopped it (with a
commit of the person's own to land, git stops: the run's branch is not behind that commit). On the run's own
branch git had nothing to move; on a branch someone named like a run's (`jellymoon/run-mine`), behind the work,
git moved it and wrote the files, which is why the words say "may have". The same rule reads a record: one whose
`onto` is a run's branch is not core's ("What the record takes").

**Only a name the repo lists is where work was kept.** On a disk that folds case, `git checkout Jellymoon/run-<id>`
stands the checkout on the run's branch under a name the repo does not list, and `git checkout Feature` on the
person's `feature`. Found by the third gate: said as "kept, on another branch" with `onto: Jellymoon/run-<id>`,
the same keep deleted the run's branch from under the checkout, `HEAD` named a branch with no commit, and
nothing reached the run's commit. So a name is written as `onto`, and lets the run's branch be deleted, only
when `for-each-ref` lists that very name with its tip where `HEAD` is. A name it does not list is a run's when
it is under `refs/heads/jellymoon/run-` in either case of those letters (nothing kept, above; the checks before
the step and the reading of a record go by the same rule); any other, with `HEAD` at the commit, is work kept on
no branch that is named, and the run's branch is left. So is a name it lists at another commit: found by the
fourth gate, with `Feature` in the packed list at a commit of the person's own and `HEAD` aimed at it, git moved
`feature` through its file and the keep said the work was on `Feature`. Some systems re-spell the branch when git
moves it (on Windows the file `feature` becomes `Feature`, on a Mac it keeps its spelling), and the keep goes by
what the repo lists afterwards: there the work is kept on `Feature`, which the repo lists at the commit.

Kept, the work is the person's checkout's: its files are written and its `HEAD` is at the commit. A later
`run keep` says the work was kept before, with the same `onto`, and `run show` says `kept`.
`tests/f45/raced.rs` moves the checkout to a new branch, to an older branch, to a branch that holds the work, to a
branch whose name has a no-break space, off every branch (forward and joined), off every branch to a commit ahead
of the work, onto the run's own branch (forward and joined; and, on a disk that folds case, as
`Jellymoon/run-<id>` and `JELLYMOON/RUN-<id>`), onto the person's `feature` as `Feature` there, and away and back
again around the step, with a stand-in git; and moves a linked worktree, where `HEAD` is asked through that worktree's own git folder.

Two outcomes of such a move are not what a person would expect, and are stated here and not closed:

- **Moved away just before the step and back just after it.** git landed the work on the branch the checkout
  stood on for that moment. By the time the keep asks, `HEAD` names a branch that does not hold the commit, so
  the keep says "git stopped the step", which is not what happened: the other branch has the work. The next keep
  begins again and lands the work on the branch stood on then, too. Nothing is lost and nothing is written
  over; the work is on two branches (`tests/f45/raced.rs`).
- **`HEAD` pointed at another linked worktree's branch.** `git symbolic-ref HEAD refs/heads/<theirs>` in the
  checkout just before the step (git's own `checkout` refuses a branch another worktree stands on) makes the
  step move that branch, and the keep says "kept, on another branch". The other worktree's files and index stay
  as they were, so there `git status` shows the work as staged reversals. "No other checkout's files, index or
  `HEAD` is touched" holds; that no other checkout's *branch* moves holds unless the checkout is pointed at
  one while the keep works. It was measured by the gate and is not run by a test here.

A keep that is cut off after such a step has written the branch it meant, and the next keep goes by the checkout
as it is then: on a branch that holds the work it lands nothing and marks it kept there; on a detached `HEAD` it
is refused as standing on no branch, with nothing changed, until the person stands on a branch.

## A step git stops

git checks, before it writes one file, everything it can ask of its index and of the disk's names. Four things
it cannot ask stop it after it has written some files (measured on git 2.50.1 for the first; the others are from
git's own account of the step and were not run):

- a folder of the checkout git may not write to;
- a file name of the work that the disk does not take (too long, or with a character the system refuses);
- a checkout that is sparse by patterns that are not folders (not cone mode), where git's trial and its step
  can differ;
- a link of the person's that git ignores, where a folder of the work would go, in a keep begun again after one
  that was cut off.

git then exits with an error: the branch and `HEAD` are where they were, the index is as it was, files of the
work may be on the disk, and `ORIG_HEAD` may name the branch's commit. git writes `ORIG_HEAD` whenever it tries
the step, so it is not among the refs a refusal is said to leave. The keep says:

``git would not move the branch forward to the work of run <id>: git stopped the step, files of the work may
already be written to the checkout, and the next `run keep` begins again``

Those words are said of every step git ends with an error while the index is not locked, and of a step it took
with `HEAD` left somewhere else than at the commit. When git stopped whole, as it does for a file that came to be
in the way after the checks, no file was written. The files a stopped step wrote are in the way of the next keep
until the person clears them (`git status` shows them as untracked). `tests/f45/late.rs` shuts a folder and
`tests/f45/raced.rs` stages a file of the person's just before the step.

## What the record takes

Three names in `runs/<run>/run.json`, after everything a run wrote (example
`crates/jellymoon-core/tests/fixtures/run-record-kept.json`): `kept`, the commit on the person's branch that holds
the work; `onto`, that branch by its whole name (`refs/heads/...`); and `work`, which is `kept` once the commit has
landed. `kept` and `onto` without `work` are a keep's intent: written at step 2, and all the next keep goes by.
They are read back strictly: an id of 40 or 64 hex digits, and a name a record holds. That is one rule
(`copy::nameable`), asked by the keep before it writes a name and by the command that reads one back: a name
under `refs/heads/` of at most 1024 bytes, with no space of any kind (a no-break space, an ideographic space and
a line separator among them), no line break, no control character (the C1 controls among them) and nothing git
reads as a pattern. And it is no run's branch: a name under `refs/heads/jellymoon/run-`, those letters in either case, is never where work was
kept, so a keep refuses a checkout that stands on one and a record that names one is not core's. A checkout on a branch of any other name is refused before anything is written, so no keep
leaves a record the next one cannot read (`tests/f45/names.rs`). Names of other alphabets, `@`, a name of 300
bytes and a name with a character that turns the line around (U+202E) are held, and are printed cleaned. Beside
a name, `onto` may be the one word `HEAD`, only with `work: kept`: the work landed on no branch a record names
("A checkout moved just before the step"). Anything else, one without the other, or `work: kept` with neither,
is not core's record, and nothing is done for it. Whoever can write the home can write a record, here as everywhere ([show.md](show.md),
"What the record names, and how far it is trusted"): a record made to say a commit landed can end a keep with
nothing landed and the run's branch deleted, and cannot make a keep write to a checkout.

## How the working tree and the index are written

Core writes no file of the checkout and no index itself. One command does both, and moves the branch:

```
git --no-pager -c core.hooksPath=<an empty folder of Jellymoon's> -c core.fsmonitor=false -c commit.gpgSign=false
    -c submodule.recurse=false -c protocol.allow=never -c worktree.useRelativePaths=false
    -c <every filter, merge driver and transport the repo's config names, set to nothing>
    --git-dir=<the checkout's git folder> --work-tree=<the checkout>
    -c gc.auto=0 -c maintenance.auto=false
    merge --ff-only --no-squash --no-autostash --no-verify-signatures --no-overwrite-ignore --no-stat --no-edit
          --no-progress --quiet <commit>
```

It is started in an empty folder of Jellymoon's, with a short list of this process's variables and no `GIT_*`
variable that names a repo, an index or a setting ([copy.md](copy.md), "What is switched off").

What the config names is read twice: through the git folder the repo's checkouts share, and through the git
folder of the checkout the keep is asked in. A linked worktree can have a config of its own (`config.worktree`,
with `extensions.worktreeConfig`), which git reads only through that worktree's git folder, and the step runs
through it. Every git a keep starts is given both lists. A config that names more than 8 KiB of such settings,
or one with `=` in its name, cannot be switched off on a command line, and the keep is refused with nothing
changed.

What it is given is always a commit whose history holds the branch's last commit, so the merge is a fast-forward:
git checks every file it would write or remove against the index and the disk first, and if one check fails it
exits 1 having written nothing. Measured (git 2.50.1): it stops whole for an unsaved change, a staged change, an
untracked file, an ignored file (that is `--no-overwrite-ignore`, whether the file is ignored by `.gitignore`, by
`info/exclude` or by the person's `core.excludesFile`), a file where a folder would go and a folder where a file
would, and a locked index. An unsaved change in a file the work does not touch stays as it is. What it cannot
check first is "A step git stops".

## A file with a filter

Every filter is switched off, so a file with a `filter` attribute would be written as the store has it and not as
the person's filter would make it. Work that writes such a file is refused. git reads attributes differently when
it writes a checkout than when it is asked about one. The step first removes every file its index held and no
longer holds, then writes; and for each folder it takes `.gitattributes` from the index it is leaving, and the
file on the disk only where that index has none. That index is the commit's entries and, beside them, whatever
the person has staged that the work does not touch. So, when a file of the work is written:

- the person's unsaved edit to a tracked `.gitattributes` is not read, and what they staged of it is;
- an untracked `.gitattributes` is read;
- a `.gitattributes` the work **removes** is gone, from the index and from the disk, before the first file is
  written: what it said holds for no file of the work;
- one the work **changes** or **adds** is read as the work has it, for every file, also for a file git writes
  before it writes the `.gitattributes` itself (git reads it from the index, not from the disk).

No one question of `git check-attr` reads that way, so four are asked, and a `filter` in any answer refuses
(`copy/keep/attrs.rs`). The first three are asked of the index the step would leave, which the trial made
(`keep.trial`, "The steps"):

1. as the step will read them: that index, over a folder of Jellymoon's (`keep.attrs/`) that holds a copy
   of each `.gitattributes` of the checkout which the person's index does not hold (one it holds, the step
   reads from its own index or removes);
2. by that index alone, with `--cached`: the commit's own and what the person has staged;
3. by the checkout's files as they are now, and by that index where the checkout has no file;
4. by the commit's own alone, in an index of Jellymoon's (`keep.index`) with `--cached`. It is what
   `check-attr --source=<commit>` asks on git 2.40 and later, and holds on every git a keep runs on.

Only the first is how git 2.50.1 reads when it writes. The other three refuse work that git would not have
filtered, and are asked so that no one reading has to be right: `tests/f45/started.rs` has a scene in which only
the commit's own says `filter`, one for the checkout's files and one for what is staged, and
`tests/f45/attrs.rs` has the work remove, change and add a `.gitattributes`, and a scene in which only the
index the step would leave, asked alone, says `filter`, each beside a control with plain git. Two refusals
there are more than git needs: where the work changes a `.gitattributes` so that it takes a filter off, the
checkout's own file still sets it; and where what the person staged sets a filter that an untracked file of
theirs below takes off again. git would start nothing in either, and the keep is refused.

That the first and the third are asked of the index the step would leave, and not of the commit's own, is held
by one test with two scenes (`tests/f45/attrs.rs`). What the person staged at the top gives a filter to a word
of its own (`[attr]big filter=planted`), which is in that index and in no commit. With an untracked file of
theirs below that gives `*.dat` the word, and the top file on the disk edited so that it no longer has it, only
the first reading says `filter`, and git does start the filter. With the word given by an unsaved edit of a
tracked file below, and the top file gone from the disk, only the third says it; git would start nothing, and
the keep is refused: a third refusal that is more than git needs.

A reading git does not answer (it ends with anything but 0) is no answer that there is no filter: the keep is
refused as one whose repo could not be read (`tests/f45/attrs.rs`).

The repo's `info/attributes` and the config's `core.attributesFile` are read in all four. With these, no file is
written other than as its filter would have made it, except: where the person changes an attributes file between
the check and the step; where a disk that folds case holds the file under another spelling than `.gitattributes`;
on a git before 2.32, which follows a `.gitattributes` that is a link, where a keep does not; and where git
cannot remove a `.gitattributes` the work removes.

**A `.gitattributes` git cannot remove.** git warns (`unable to unlink`), goes on, ends with 0, and reads the
file it left for the files it writes beside and below it. Measured: tracked `sub/.gitattributes` sets a filter,
tracked `sub/deep/.gitattributes` takes it off, the work removes both and makes `sub/deep/new.dat`, and `sub` is
shut (mode 555): plain git filters the new file; all four readings say no filter, `--ask` says `fits: true`, and
the keep writes the file as stored. No check before the step can know every reason a file cannot be removed, and
`--ask` cannot see this one. So the keep looks after the step: each `.gitattributes` the work removes that is
still at its path is named in `stayed`, and `why` says so ("What it prints"). The work is kept, with exit 0:
git landed the commit, and the file it left is untracked from then on. It is looked for after every step git
took, but one that found the work already on the branch `HEAD` was moved to. `tests/f45/attrs.rs` runs the
scene beside plain git, forward and joined, and is skipped where a folder cannot be shut.

Still at its path means a plain file there, with every folder above it in the checkout a folder (found by the
fourth gate). Where the work puts a folder at the path (`sub/.gitattributes/x`), or makes `sub` a link to a
folder that holds a `.gitattributes`, git did remove the file and nothing is named. A file the person makes
again between the step and the look is named: the words say where a file is, and not what left it there.

A filter is also what git starts when it looks whether a touched file changed, of any tracked file, in the work
or beside it. Switched off, git compares the file as it is. A filter the repo says is `required` is
no other: `required` is switched off with the filter, for every git a keep starts.

Why nothing of the repo's or the person's config can run:

| What could start a program | Why it does not |
|---|---|
| a hook (`post-merge`, `post-checkout`, `post-index-change`, `reference-transaction`, and the commit hooks) | `core.hooksPath` is an empty folder on every command line; the one new commit is made by `commit-tree`, which has no hook |
| a filter (`filter.<name>.smudge`, `.clean`, `.process`), LFS among them | each one the config names is set to nothing, in the repo's config, the person's and a linked worktree's own; and a keep refuses work that would write a file with a `filter` attribute ("A file with a filter") |
| a merge driver (`merge.<name>.driver`) | each one the config names is set to nothing; git then takes the file as one that clashes, and the keep is refused |
| `branch.<name>.mergeOptions`, `merge.autoStash`, `merge.ff`, `merge.verifySignatures` (which starts gpg) | each has its opposite on the command line, where it wins |
| a signing program | the one new commit is made by `commit-tree`, which signs only when its own command line says so, and a fast-forward makes no commit. `commit.gpgSign=false` is on every command line all the same, and is what keeps a run's own commit unsigned ([copy.md](copy.md)): a keep is the same with it and without it (measured on git 2.50.1) |
| a submodule's checkout | `submodule.recurse=false`, and work with another repo's entry is never kept |
| a fetch, a credential helper, ssh | `protocol.allow=never`, each named transport set to `never`, `GIT_NO_LAZY_FETCH=1`, `GIT_TERMINAL_PROMPT=0` |
| a file-system monitor, a pager, an editor, a tidy of the store | `core.fsmonitor=false`, `--no-pager`, `--no-edit`, `gc.auto=0` and `maintenance.auto=false` |

`tests/f45/settings.rs` and `tests/f45/started.rs` plant a program as each hook, as a filter, as a merge driver
and as a signer, and fail if any was started. Each has a control beside it: the same program is started when the
person does the same thing with plain git. `started.rs` names the filter in a linked worktree's own config, hides
it behind the person's unsaved attributes, and has a scene where switching the filter off is all that stops it (a
touched file beside the work).

Settings that start no program are the person's and are obeyed: how line ends are written (`core.autocrlf`), the
attributes and ignore files, whether git looks for renames when it merges.

## What is asked of git

Every answer is read to a size and git's words are never kept: a failure is one of core's sentences.

| Question | Command | It writes |
|---|---|---|
| where the folder is | `rev-parse --is-bare-repository --is-inside-work-tree --show-prefix --git-dir --git-common-dir`, in the person's folder | nothing |
| what the config names | `config -z --name-only --get-regexp`, through the shared git folder and through the checkout's own | nothing |
| where a branch is | `for-each-ref --format=... <whole name>` | nothing |
| the run's commit and its parent | `cat-file commit <end>` | nothing |
| where a checkout stands | `symbolic-ref --quiet HEAD`, and the names git keeps in a checkout's git folder; of a branch the repo does not list, `rev-parse --verify --quiet HEAD^{commit}` | nothing |
| whether a commit is in a history | `merge-base --is-ancestor` | nothing |
| what a change touches | `diff-tree -r --no-abbrev -z` | nothing |
| how old git is | `version` | nothing |
| the two merged | `merge-tree --write-tree --no-messages <branch's commit> <end>` | objects in the store, which nothing names |
| who the person is | `config --get user.name`, `config --get user.email`, through the checkout's own git folder | nothing |
| whether it fits | on a copy of the checkout's index (`runs/<run>/keep.trial`, removed before the answer): `update-index -q --refresh`, then `read-tree -n -m -u <branch's commit> <target>`, and when that passed `-c core.splitIndex=false read-tree -m <branch's commit> <target>`, which makes the copy the index the step would leave | Jellymoon's own file: `-n` is a trial, and without `-u` no file of the checkout is written |
| whether a filter is set | `ls-files -z -- ':(glob)**/.gitattributes'` of the checkout's index, `read-tree <target>` into an index file of Jellymoon's own (`runs/<run>/keep.index`), then `check-attr -z --stdin filter` four times ("A file with a filter"), the names on git's input. The file, the folder beside it and a lock a cut keep left are removed at once | Jellymoon's own files |
| whether what is in the way is ignored, in a keep begun again | `check-ignore -z --stdin`, the names on git's input | nothing |
| the one new commit | `commit-tree -p <branch's commit> -m "Jellymoon run <id>" <tree>`, with the person's name and address as author and committer | one object in the store |
| the landing | `merge --ff-only ...`, above | the checkout's files, its index, the branch, `ORIG_HEAD` and the reflog |
| where the commit is after the step | `for-each-ref` of the branch; if it is not there, `symbolic-ref --quiet HEAD`, `rev-parse --verify --quiet HEAD^{commit}`, `for-each-ref` of the name `HEAD` gave and, with `HEAD` elsewhere than at the commit, `merge-base --is-ancestor`, the first two through the git folder of the checkout the keep was asked in. After a step git stopped, `symbolic-ref --quiet HEAD` alone | nothing |
| the run's branch | `update-ref --no-deref -d <name> <end>` | that one ref |

No file name is an argument to git: names go on git's input, ended by a zero byte.

## What is never kept, and what needs a second yes

- **Another repo's entry and `.gitmodules`**: work that adds, changes or removes one is refused, whatever was
  asked. Removal is refused too: it changes which other repos the person's checkout holds.
- **Files the person's tools obey**: refused without `--with-settings`. `--ask` names them in `settings` either
  way, and a keep that lands them names them too. The name is asked as git keeps it and as it is printed.
- **A file with a filter**: refused, above.

## Whose commit it is

With commits of the person's own, the one new commit is made with the name and address git gives for the checkout
at that moment (`user.name`, `user.email`), as author and committer. They are read through the checkout's own git
folder, so a linked worktree's own name is the one used there. Jellymoon never makes one up: with either missing,
empty, or made only of spaces and the marks git takes off a name's ends (`,:;<>"\'`), the keep is refused. Its message is `Jellymoon run <id>`, and its one parent is the person's last
commit: there is no merge commit, and the run's own commit is in nobody's history.

When the branch is still where the run started, what lands is the run's own commit, and no commit is made. Its
author is who the run's copy found the person to be when the run ended, or Jellymoon's fallback name when the
repo named nobody then ([copy.md](copy.md)).

## Every refusal

Each ends the command with exit 1 and one line on stderr. With `--ask`, the ones marked are printed in `why`
instead. `<unchanged>` stands for `the work of run <id> was not kept, and nothing was changed`.

| When | Words | With `--ask` |
|---|---|---|
| the wish is not there, has no run, names another's, or its run is going, was cut off, was cancelled or failed | the words of a look ([show.md](show.md), "Every refusal") with `keep` and `kept` for `show` and `shown`; a run that is going: `run <id> is going: its work can be kept once it has finished` | refused |
| the work was tossed | `the work of run <id> was tossed, so nothing of it was kept` | refused |
| the run made no commit | `run <id> changed nothing: there is no work of it to keep` | refused |
| the record names no commits | `the record of run <id> is from before a run's record named its commits: its work is shown, and is not kept by Jellymoon` | refused |
| the record's `kept` and `onto` are not an id and a branch's whole name, or `work` says kept with neither | `the record of run <id> is not one Jellymoon wrote for this wish` | refused |
| another keep holds the lock | `another keep is at work in this home: <unchanged>` | printed |
| the lock could not be made or taken | `the keeps' lock (runs/keep.lock) could not be made or taken in this home: <unchanged>` | printed |
| the config names what cannot be switched off | ``the repo's git config names a filter, a merge driver or a transport that cannot be switched off on git's command line: there are too many of them, or one has `=` in its name: <unchanged>`` | printed |
| no git, or the empty folder is not empty | `the work of run <id> could not be read: no `git` program was found on PATH, or the folder Jellymoon keeps empty for git (runs/no-hooks) is not an empty folder` | printed |
| the folder is not a checkout's top, or git did not answer | `the work of run <id> could not be read from the repo: the folder is not the top of a checkout git opens, or git did not answer` | printed |
| the repo is another | `the repo found for run <id> is not the one its record names, so its work is not kept` | printed |
| the run's branch is gone | `the branch of run <id> is not in the repo, so there is no work of it to keep` | printed |
| it is a name for another branch | `the branch of run <id> is a name for another branch by now, so its work is not kept` | printed |
| it names another commit, or one not shaped as a run's | `the branch of run <id> is not as the run left it, so what is on it is not kept as the run's` | printed |
| its commit does not stand on the record's start | `the work of run <id> does not start where its record says the run started, so it is not kept as the run's` | printed |
| the work is more files than are listed | `the work of run <id> is more files than Jellymoon lists` | printed |
| a checkout stands on the run's branch | `a checkout stands on the branch of run <id>: <unchanged>` | printed |
| no branch is stood on | `the checkout stands on no branch of the person's: its HEAD is detached, or names a branch named as a run's is: <unchanged>` | printed |
| the branch's name is one no record holds | `the checkout stands on a branch whose name Jellymoon does not write in a run's record: the name holds a space of some kind, a line break or a control character, or is longer than 1024 bytes: <unchanged>` | printed |
| something is going on | `a merge, a cherry-pick, a revert, a rebase or a bisect is going on in the checkout: <unchanged>` | printed |
| the history does not hold the start, or the branch has no commit | `the branch's history does not hold the commit run <id> started from: <unchanged>` | printed |
| `HEAD` names a branch under a spelling the repo does not list (`git checkout Main` for `main`, on a disk that takes both) | ``the checkout's HEAD names a branch the repo does not list under that name: on a disk that takes two spellings for one name it is another spelling of a branch's name, and `git checkout` with the name as `git branch` prints it stands on the branch itself: <unchanged>`` | printed |
| git is too old | `the branch has commits since run <id> started, and this git is too old to merge them with the work in the store (it is from before 2.38): <unchanged>` | printed |
| both changed the same | `the branch's commits since run <id> started and the run's work changed the same lines, or the same file in a way git does not merge: <unchanged>` | printed |
| git ended the merge in the store with an error | `git could not merge the work of run <id> with the branch's commits since in the repo's store: the store takes no new object (the merge writes one tree there, asked or not), or git did not answer: <unchanged>` | printed |
| git names no author | `git has no name and address for this repo's commits (user.name and user.email), and Jellymoon makes none up: <unchanged>` | printed |
| another repo's entry | ``the work of run <id> adds, changes or removes another repo's entry or `.gitmodules`, and Jellymoon never keeps such work: nothing was changed`` | printed |
| files tools obey | ``the work of run <id> touches files the person's tools obey, which `run keep --ask` names: it is kept only with --with-settings, and nothing was changed`` | printed |
| a filter | `a file of the work would be passed through a filter this repo sets for it, and Jellymoon starts no filter: <unchanged>` | printed |
| a name the disk does not take | `a file of the work has a name this disk does not take: a part of it is too long, or is no name on this system: <unchanged>` | printed |
| something in the way, or git's trial of the move stopped for any other reason | `an unsaved change, or a file that is not in the branch, is where the work would go: <unchanged>` | printed |
| the index is locked | `the checkout's index is locked (index.lock): another git is at work in it, or one was cut off and left the file: the work of run <id> was not kept` | printed |
| git stopped the step | ``git would not move the branch forward to the work of run <id>: git stopped the step, files of the work may already be written to the checkout, and the next `run keep` begins again`` | (a keep's alone) |
| the checkout was moved onto a run's branch just before the step | ``while the keep was at work the checkout was moved onto a branch named as a run's is, where Jellymoon keeps no work: git may have moved that branch forward and written the work's files, and nothing is recorded as kept; the branch of run <id> is left, and the next `run keep` begins again`` | (a keep's alone) |
| the record could not be written | ``the record of run <id> could not be written: the next `run keep` goes on from where this one stopped`` | (a keep's alone) |

"Git is too old" is said only when a merge is needed: moving a branch forward asks nothing of 2.38.

The last three are the only refusals after which something is changed: the record names what was meant to land
(step 2); after "git stopped the step" files of the work may be on the disk and `ORIG_HEAD` may be written ("A
step git stops"); after "moved onto a run's branch" the checkout is where it was moved to, and when that branch
was one git could move forward (a branch someone named like a run's, behind the work) git moved it; and after
"could not be written" at step 4 the work has landed. The next keep goes on from any of them. "The index is locked" says nothing of files: said after the step, git had stopped before it wrote one.

## Work the branch holds already

If the person moved their branch to the run's commit themselves, or their last commit holds exactly what a keep
would land (they picked the work onto their own), a keep lands nothing, marks the work kept with the commit that
holds it, and deletes the run's branch.

## What was measured

On git 2.50.1 on a Mac, in made-up repos (2026-11-12): every row of the fast-forward's refusals above; that
`branch.<name>.mergeOptions "--squash --autostash --no-ff"` is overridden; that a planted hook of every kind
git starts on a merge, a checkout, an index write and a ref write is not started; that a merge driver is started
by `merge-tree` unless its command is set to nothing, and is not started then; that `read-tree -m -u` writes over
an ignored file in the way, which is why it is the trial and never the step; that `merge --ff-only` on a detached
`HEAD` moves it, which is why the branch is asked for first; that the step works in a linked worktree and moves
that worktree's branch alone; and that on a disk that takes `A.TXT` and `a.txt` for one name, work that makes
`A.TXT` beside the person's tracked `a.txt` is refused by git when `a.txt` has an unsaved change and is written
through `a.txt` when it has none, leaving the checkout changed. A keep refuses both: the file is on the disk
where a made file would go. The tests at the binary repeat each on every system CI runs.

Measured for the gate's findings (2026-11-13, git 2.50.1, a Mac): a filter named in a linked worktree's
`config.worktree` is not listed by `git config` through the shared git folder and is listed through the
worktree's own; with an untracked `sub/.gitattributes` that sets a filter and an unsaved edit of a tracked
`sub/deep/.gitattributes` that takes it off, `check-attr` in the checkout says `unset`, `check-attr --cached` of
the commit and `check-attr --source=<commit>` say `unspecified`, and `merge --ff-only` runs the filter: only the
first reading of "A file with a filter" says `filter`; `read-tree -n -m -u` refuses a tracked file that was only
touched until `update-index --refresh` has read it, and with `GIT_INDEX_FILE` at a copy it does not see a lock on
the checkout's own index; `merge --ff-only` runs the clean filter of a touched file beside the work, and with the
filter set to nothing and `required` set to false it starts nothing and exits 0; `update-ref -d` without `--no-deref` on
a run's branch made a name for `main` deletes `main`; with a folder of the checkout shut, the trial passes and
the step exits 1 with some files written, `ORIG_HEAD` written and the branch unmoved; and git refuses a name
made only of spaces or of `<>`, and takes an address of either.

## What is not closed

- **git cut off inside its own checkout.** A kill or a power cut while git writes the checkout's files (inside
  step 3) can leave some files written, the branch unmoved and `index.lock` in place. The next keep is refused
  for the lock; with the lock removed by hand, the files git wrote are in the way of a keep begun again, and are
  the person's to clear (`git status` shows them as changed or untracked; `git checkout -- .` and removing the
  untracked ones restores the branch's commit). Nothing of the person's is written over in any of this, and the
  work is not landed twice. git has no step that writes a checkout and a ref as one.
- **Between the trial and the step** the person can change a file. The step is git's own check again, so nothing
  is written over; the keep is then refused as "git stopped the step". They can change an attributes file then
  too, which the step obeys and the check did not see.
- **A branch moved in that moment.** git reads `HEAD`, checks that the commit is ahead of it and moves it under
  its own check of what it read. Core does not write the person's branch with an old value of its own. A branch
  set back to an older commit between core's reading and git's is moved forward all the same, which loses
  nothing. A checkout moved to another branch or off every branch gets the work where it stands by then, and the
  keep says so ("A checkout moved just before the step"): nothing can make git land on the branch the checks
  were made for and no other.
- **A step git stops part way** leaves files of the work for the person to clear ("A step git stops").
- **A file staged as new with exactly the work's content** is in the way, though git would take it.
- **Two files of the work whose names a disk takes for one** are both written by git, the second through the
  first: nothing of the person's is lost, and the checkout is left changed against the commit.
- **A keep begun again after one that was cut off** takes what is on the disk where a made file would go as in
  the way only when git ignores it, since the files there may be the ones the cut keep wrote ("The steps"). A
  folder there, or a file where a folder would be, is left to git's own step, which stops for it; and the step
  writes through a tracked file of another spelling that has no unsaved change.
- **A checkout moved away for the step and back after it**, and **a `HEAD` pointed at another worktree's
  branch**, end in ways a person would not expect ("A checkout moved just before the step").
- **A name the disk does not take** is found by its length and by what the disk says of the name. A name that is
  one only to some program (a name Windows keeps for a device, on a git that writes it) is left to git's step.
- **Who the person is** is read from git's config alone (`user.name`, `user.email`). A name given only in
  `GIT_AUTHOR_NAME` or under `author.name` is not read, and the keep is refused as having no name.
- **A sparse checkout** was run in one scene (a file of the person's at a path it leaves out, "The steps"). **A
  partial clone and a repo whose objects are borrowed** were not run.
- **A `.gitattributes` git cannot remove** is found after the step and said, not prevented ("A file with a
  filter"). Any other file git cannot remove stays as well, untracked, and is not named.
- **A name of only dots** (`user.name` set to `.`) is a name to git 2.50.1 and to a keep. An older git takes the
  dot off too and makes no commit: the keep is then refused as "git did not answer", with nothing changed.
- **Old git.** 2.38 and 2.39 were not run: the version is read and compared, and the refusal was tested with a
  made-up git that says an older one. A git before 2.32 follows a `.gitattributes` that is a link. **Windows and Linux** run in CI and were not run by hand. **Real LFS** was
  not run: a made-up filter was.

## What it does not do

It resolves no conflict, makes no merge commit, rebases nothing, fetches nothing and pushes nothing. It does not
change the wish, whose state and star are its run's. It offers no undo: the commit it lands is the person's to
revert, and the branch's reflog and `ORIG_HEAD` say where the branch was.
