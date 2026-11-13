# Tossing a run's work: `jellymoon run toss`

`jellymoon run toss --wish <id>` is the person refusing what a made wish's run left (F46;
[ADR 0017](../adr/0017-a-runs-work-is-shown-kept-or-tossed.md), decision 7 and "Found while building F46"). The
run's record takes `tossed`, the run's branch is deleted, the kept copy of a run that did not finish is removed,
and the wish is let go. It is the one command that deletes in a person's repo, so it is built on three rules:

1. **One name is deleted, and nothing else is written in the repo.** The run's own ref, `refs/heads/jellymoon/run-<id>`,
   as itself and only while it names the commit the run's record names. No working tree, no index, no `HEAD` and
   no other ref is touched, and the commit stays in the store until git tidies it away.
2. **The record says tossed before anything goes**, with the commit the branch was at: that line is how a toss
   is undone by hand (`git branch jellymoon/run-<id> <commit>`). The app offers no undo. A later `run toss`
   of that wish deletes a branch put back so, as it does whatever the record's toss left: it says the record said
   tossed and that it did what was left to do, and claims no cause.
3. **Work that waits is checked whole first.** A refusal of it leaves every ref, the record and the wish as they
   were.

The rules are in `crates/jellymoon-core/src/copy/keep/toss.rs` (the repo), `copy/clean.rs` (`drop_copy`, the
kept copy) and `crates/jellymoon-core/src/run/toss.rs` (the record and every printed word). The tests at the
binary are `crates/jellymoon/tests/f46.rs` and `tests/f46/`.

## What it prints

One line of JSON (`Tossed`; the example `crates/jellymoon-core/tests/fixtures/tossed.json` round-trips):

| Name | What it is |
|---|---|
| `run`, `wish` | the run's id and the wish's |
| `tossed` | whether this command changed anything. `false` only of a run that was tossed before with nothing left to do |
| `commit` | the commit the record names for the toss: a finished run's `end`, the `start` of a run that did not finish. `null` when the record names no commit |
| `why` | what was done, in core's fixed words |

`why` is a head, then what became of the copy and the branch:

| When | Head |
|---|---|
| the run made its one commit | `the work of run <id> was tossed` |
| the run finished and changed nothing | `run <id>, which changed nothing, was tossed` |
| the run failed, was cancelled or was cut off | `run <id>, which did not finish, was tossed` |
| the record said tossed and something was left to do, whatever left it: a toss that was cut off, a branch put back by hand, a toss asked in a folder that is not the repo | `the record of run <id> said it was tossed, and this toss did what was left to do` |
| the record said tossed and nothing was | `run <id> was tossed before, and nothing was changed` |

After the head, when something was changed: `: <copy><branch>, and the wish is let go`. `<copy>` is said only of a
run that did not finish: `its kept copy was removed, ` or `no copy of it was kept by now, `. `<branch>` is one of:

| Branch | Words |
|---|---|
| deleted | `its branch was deleted` |
| not there | `its branch was not in the repo` |
| a checkout stands on it | `its branch is left, as a checkout stands on it` |
| it is a symbolic ref | `its branch is left, as it is a name for another branch by now` |
| another ref of the repo is a symbolic ref that names it | `its branch is left, as a name of the person's points at it` |
| it is at another commit | `its branch is left, as it is not at the commit the record names for it` |
| git refused the delete | `its branch is left, as it was changed while the toss was at work` |
| the repo was not found, is another, or did not answer | `its branch is left, as the repo could not be found or asked` |
| the record names no commit | `its branch is left, as the record names no commit for it` |

A second toss that changes nothing prints the "tossed before" head alone, or with `: its branch is left, as …`
when a branch is still there that it would not delete.

The wish takes the state `let_go` and the line `run <id> was tossed, and the wish was let go`. Its stars are as
they were: a star says a job was reported finished, and that is still so. `jellymoon run stop` on such a wish
prints that line with the outcome the record has.

`--ask`, `--with-settings` and `--file` are not a toss's words, and are refused as usage (exit 2).

## Which run, which repo

The run is found as a stop finds it: the wish's one run, which must have ended. A run nothing holds any more is
given its end first, as a stop and a page give it. The repo is found as a run finds it (`--repo`, whose last name
is the project, or the logs), and the git folder its checkouts share must be the one the record names, compared
and never resolved.

One keep or toss of a home runs at a time, behind the lock a keep holds (`runs/keep.lock`), taken before the
record is read; the copy is removed behind the lock every clean holds (`runs/clean.lock`), always in that order.
A lock file left by a command that was killed is nobody's lock.

## Work that waits

A finished run whose record names its repo, start and end, with no `work` yet. In this order, and each a refusal
below: a keep that was cut off has not landed the work; the run's branch is the run's own name at `end` (and `end`
is `start`, or a commit whose one parent is `start`); no checkout of the repo stands on it, under any case of the
letters of its name (a disk that folds case takes `Jellymoon/run-<id>` for the branch itself). A checkout on
another spelling of the run's branch is refused as stood on on every disk, one that keeps case apart too, where
that spelling is a different branch: the toss is the safe side, and `git checkout` of another branch frees it. A
branch that is not in the repo at all is no refusal: there is nothing to delete, and the toss is written.

**A name of the person's that points at the branch** (a symbolic ref, `refs/heads/alias` naming
`refs/heads/jellymoon/run-<id>`, in either case of its letters) would be left naming nothing by the delete. It is
no refusal either: the record takes `tossed`, the wish is let go, and the branch is left in the words above. It
is the question a take-back asks (`copy/back.rs`, `pointed_at`: `for-each-ref --format=%(symref)`), asked once
for both, and a keep asks it before it deletes the run's branch ([keep.md](keep.md), step 5).

**Whether a cut-off keep landed** is asked of the repo, and only for a record that holds a keep's `kept`. The
record names the branch the keep meant, and git moves whatever `HEAD` names at the moment of the step
([keep.md](keep.md), "A checkout moved just before the step"), so the commit `kept` is looked for in three
places. The work is landed, and the toss refused, when the commit is held by the branch `onto` names; or by
any local branch of the person's (`rev-list --max-count=1 <kept> --not --exclude=<a run's name> --branches`: the
commit with every such branch's history taken away, which is nothing when one holds it; no name is printed); or
by the `HEAD` of the checkout the toss is asked in, when that names no branch (`merge-base --is-ancestor <kept>
HEAD`). A branch named as a run's is, under `refs/heads/jellymoon/run-` in either case of those letters, is none
of the person's ([keep.md](keep.md): a keep lands nothing on one), so the run's own branch, another run's, and
`Jellymoon/run-zz` are left out of the question, and none of them is touched. An id no commit has is in no
history. `run keep` then finishes the keep where the work is; from a `HEAD` that names no branch it does so once
the checkout stands on a branch again. A record without `kept` asks none of this.

| Step | What is done | Cut off after it, the next `run toss` |
|---|---|---|
| 1 | every check | begins again: nothing was written |
| 2 | the record takes `work: tossed` and `tossed`, the commit; a keep's unfinished `kept` and `onto` go | says the record said tossed and it did what was left, deletes the branch if it is the run's own at that commit and leaves it if not, and lets the wish go |
| 3 | (a finished run has no kept copy) | |
| 4 | the branch is deleted (`git update-ref --no-deref -d <name> <commit>`) | lets the wish go, and says the same |
| 5 | the wish is let go | says the run was tossed before |

Once the record says tossed, nothing is a refusal of the toss any more: a branch that is not the run's own at
the record's commit by then is left, in the words above, and the wish is let go all the same. A run that finished
and changed nothing is tossed the same way: its branch, at `start`, is deleted, and its star stays.

## A run that did not finish

A failed, cancelled or cut-off run keeps its copy, and its record names `repo` and `start` and no `end` (ADR
0017, "What `run keep` and `run toss` may build on"). A toss of one:

| Step | What is done | Cut off after it, the next `run toss` |
|---|---|---|
| 1 | the copy's place is asked about, and nothing removed: a real folder under `copies/` with the copy's record, or nothing there at all | begins again: nothing was written |
| 2 | the record takes `work: tossed` and `tossed`, the `start` | removes the copy and the branch, lets the wish go |
| 3 | the kept copy goes as a failed copy past the newest five goes: its folder, the repo's listing of it, and `runs/<id>/copy.json` | says no copy was kept by now, deletes the branch, lets the wish go |
| 4 | the branch is deleted if it is the run's own name at `start` with no checkout on it, and is left if not | lets the wish go |
| 5 | the wish is let go | says the run was tossed before |

The copy is removed by its own record, which names its repo: so it goes even when the toss was asked of a folder
that is not the run's repo. The branch is then left ("the repo could not be found or asked"), and a later toss
asked in the repo deletes it and says the record said tossed and it did what was left to do.

**A run that asks** (F51) is tossed by these steps too: it has a copy, a `start` and no `end`, and until an
answer can be given a toss is the one thing to do with it. Its copy is not one of the failed copies, and is
removed all the same, edits and all; its branch is at `start`, since nothing was committed, and goes; the wish is
let go and loses its `question`. The record keeps its outcome `asks` and its question beside `work: tossed`, so
`run stop` afterwards prints the outcome `asks` with the line of a tossed run and no question, and `run show`
says the run did not finish. Nothing holds a run that asks, so a toss, which holds the lock a keep and a toss share
(`runs/keep.lock`) as every toss does, then takes the run's own lock,
`runs/<id>/run.lock`, before it reads the record again and holds it until it is done, then removes the file:
no other command can take the run up meanwhile. When another command holds that lock the toss is refused as of a
run that is going. A toss of any other run takes no lock of the run's. No
keep writes in the record of a run that asks: `run keep` refuses it before the record is touched, and a record
that asks and holds a keep's `kept` all the same is not one Jellymoon wrote. While a toss holds it, the run counts as
one of the three that may go at once.

**A branch that is not at `start` is left.** A run killed after core's commit, or one whose copy could not be
removed, has a branch that holds more than the start, and no record names that commit. It is not deleted as the
run's: the copy goes, and the branch is left in the words "not at the commit the record names for it".

**A record that names no commit** (from before F48, or written down as failed by v0.2.0-rc1): the kept copy, if
any, is removed, the record takes `work: tossed` with no `tossed`, and the branch is left, "as the record names
no commit for it". This holds for a finished run of such a record too.

## What is refused

Each ends the command with exit 1, one line on stderr, and nothing changed, but for the last three.

| When | Words |
|---|---|
| the wish is not there | the words of a stop |
| the wish has had no run | `wish <id> has had no run, so there is nothing to toss` |
| the record is not there or not JSON | `the record of run <id> cannot be read, so nothing was tossed` |
| the run is another wish's | `wish <id> names run <run>, which is another wish's run: nothing was tossed` |
| the run is going, or it asks and another command holds its lock | `run <id> is going: it is stopped first, and nothing was tossed` |
| the wish is working or the record has no outcome | `run <id> has no end written down yet: nothing was tossed` |
| a keep or a toss holds the lock for ten seconds | `another keep or toss is at work in this home: nothing of run <id> was tossed` |
| the lock's file could not be made or asked about | `the lock a keep or a toss holds (runs/keep.lock) could not be made or taken in this home: nothing of run <id> was tossed` |
| the record's words and names do not agree, it says tossed at another commit than it names, it says tossed and still holds a keep's `kept` and `onto` (a toss takes them out as it writes `tossed`), or it holds half a keep that a run that did not finish cannot have | `the record of run <id> is not one Jellymoon wrote for this wish` |
| the record's `work` says kept, whatever branch, or none, it names beside | `the work of run <id> was kept, so it is not tossed: nothing was changed` |
| a keep that was cut off had landed the work | ``a keep of run <id> that was cut off had landed its work, so it is kept and not tossed: `run keep` finishes that keep, and nothing was changed`` |
| a checkout stands on the branch | `a checkout stands on the branch of run <id>: its work was not tossed, and nothing was changed` |
| the branch is a symbolic ref, is not at `end`, or `end` is not a commit on top of `start`; the repo is another, is not found, or git did not answer | the words of a look ([show.md](show.md), "Every refusal") with `toss` and `tossed` for `show` and `shown` |
| the record could not be written | `the record of run <id> could not be written: nothing was tossed` |
| something is at the copy's name that is no folder Jellymoon made (a link, or a folder with no `copy.json`), or the clean lock was not had | `the kept copy of run <id> could not be removed: its place is not a folder Jellymoon made, or it could not be asked about, and nothing was tossed` |
| the copy could not be removed after the record was written | ``the kept copy of run <id> could not be removed: the record says tossed, and the next `run toss` goes on from there`` |
| the wish's file could not be written after the toss | ``run <id> was tossed and its record says so, but its wish could not be told: the next `run toss` lets the wish go`` |

`run keep` of tossed work says `the work of run <id> was tossed, so nothing of it was kept`.

## A toss that is cut off

A test's build ends itself at the step `JELLYMOON_TEST_TOSS_CUT` numbers, as a power cut would; a release never
reads the variable (`busy::cut`, a keep's reader too, is compiled only into a build with debug assertions): 1
before the record is written, 2 after it, 3 after the kept copy went, 4 before the wish is told.
`tests/f46/cut.rs` cuts a toss at each, of a run that finished and of one that did not, and runs the next toss;
it also kills a toss that holds the home, and runs a keep and a toss of one run in every order and at once.

A keep and a toss never both win. Each reads the record behind the same lock. A keep that wrote its intent and
did not land is tossed, and its `kept` and `onto` go from the record. A keep that landed, whose record does not
say kept yet, is not tossed: the work is on a branch of the person's or on their `HEAD`, wherever git put it
(`tests/f46/landed.rs`).

## The delete

`git update-ref --no-deref -d refs/heads/jellymoon/run-<id> <commit>`: git deletes that name only while it holds
that commit, and never follows a symbolic ref through it. It is asked with a keep's git: no hook, no config of
the machine's, an answer bounded in bytes and in time. When git refuses, the branch is asked for once more: gone
is said as gone, and anything else is left as "changed while the toss was at work". On a disk that folds case,
with the run's branch packed and a loose other spelling of it at another commit, git reads the loose one for the
name and refuses the delete: the toss has written the record and lets the wish go, and the branch is left in
those words. With both spellings at one commit, the one delete removes both.

Between the last question and the delete a checkout can come to stand on the branch. After the delete the
checkouts are asked again, and if one stands on the name it is put back at the same commit, as a new name only
(`update-ref --no-deref <name> <commit> <zeros>`), and said to be left. `tests/f46/races.rs` moves the branch,
makes it a symbolic ref, removes it and checks it out just before the delete, with a made-up git first on `PATH`.

## What is not closed

- **A cut between the delete and the putting back** leaves a checkout standing on a branch that is not there.
  git shows it as a branch with no commit yet; the record names the commit.
- **`run keep` deletes the run's branch with the same window** and does not ask again afterwards.
- **A keep killed inside `git merge`**, whose git child lands the work after a toss that follows at once, leaves
  work both landed and tossed. The lock is dropped with the keep, and git is not asked whether a child of it
  still runs.
- **Work a cut-off keep landed on a detached `HEAD` of another checkout** than the one the toss is asked in, or
  on a `HEAD` the person has since moved off, is held by no branch of the person's: a toss finds no landing
  and tosses. Only the checkout the toss is asked in has its `HEAD` asked.
- **A branch a toss left** for a run that did not finish is no longer reached by the copy's own take-back
  (`copy/back.rs`), which goes by `copy.json`: that file went with the copy. It is the person's to delete.
- **A copy whose `copy.json` was taken out of the home by hand**, with its folder, is no copy to a toss, and
  nothing says whose repo lists it. The repo lists it still, as a checkout that stands on the run's branch, so
  the branch is left ("as a checkout stands on it") until the person runs `git worktree prune`.
- **A wish's stored line** for a failed run still says `its copy is kept` after the copy went with a later run's
  pruning. `run stop` prints the second form (`its copy is no longer kept, and no star was added`) when the
  folder is not standing; the page prints the stored line.
- **On Windows the agent of a run that was cut off is not stopped** (`run/dead.rs`: there is no signal there),
  and a folder a live process stands in cannot be removed. A toss of such a run writes its word, says the copy
  could not be removed, and the next `run toss`, once the agent has ended, goes on from there (CI saw it).
- **Whoever can write the home can write a record**, here as everywhere: a record made to name a commit can
  have a toss delete the run's branch at that commit, when that commit is the record's start or a one-parent
  commit on it, and can never have it delete another name.

## What was not measured

Windows and Linux by hand (CI runs the tests there), git older than 2.36 for the warning test (it is skipped
there), and a repo on a disk that is full.
