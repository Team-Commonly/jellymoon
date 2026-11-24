# 0028 · A finished change is kept by itself, and can be undone

Status: Built, 2026-11-24 (F92). Proved at the API by `crates/jellymoon/tests/f92.rs` and, for the page, by
`app/tests/kept_by_itself.rs`. Run on macOS with git 2.50.1 (Apple Git-155). Windows and Linux were not run. Not
seen in a real window. It changes [ADR 0017](0017-a-runs-work-is-shown-kept-or-tossed.md) in one place: a keep
no longer always waits for a yes.

## Why

A run's work waited in a copy until the person said `Keep it`. For most changes that yes carried no decision: the
change fits, nothing of the person's is in its way, and the person asked for it a minute ago. What a person wants
after a wish is to see the change in their folder, and a way back when it is not what they meant. So the yes
moves to the cases where it decides something, and every other change gets a way back in its place.

## Decisions

1. **A finished change is kept by itself.** When a run's end is noted, and before its wish is told, core does the
   keep of `run keep` with no yes given (`run/end.rs`, `run/keep/itself.rs`). It is that keep and no other: the
   same lock, the same checks, the same one step. `jellymoon run` prints `kept_itself: true` when the change is in
   the folder, and leaves the word out when it is not.
2. **It waits wherever a yes decides something.** A keep by itself lands only work that is moved forward or
   joined. It lands no file the person's tools obey and no change that touches a `.gitattributes`, and it is
   refused for everything `run keep --ask` refuses. A refusal changes nothing and says nothing: the work waits
   as it did before, and its note says in its first line that it waits for a yes.
3. **What the branch named before is written with the intent.** The record's `before` is the commit the person's
   branch named, written in the same write as `kept` and `onto`, before the checkout is touched. A keep that
   landed anywhere but as meant (the checkout was moved just before the step) clears it. Only a record with
   `before` can be undone: a change kept by a tap has none, and its run's branch is deleted as before.
4. **The run's branch stays after a keep by itself**, since the undo's work waits on it afterwards. To bound how
   many stay: once a change was kept by itself, under the same lock, every earlier day's change of that repo that
   was kept by itself loses its run's branch and its `before`. So the branches kept for an undo are one day's.
5. **`jellymoon run undo --wish <id> [--ask]`** takes a change that was kept by itself out of the folder again.
   It is not a revert: no commit is made. The branch names the commit it named before, the files and the index
   are as that commit has them, and the record says neither kept nor tossed, so the work waits and `run keep`
   or `run toss` takes it from there. The wish stays done.
6. **The undo's step names the branch, never `HEAD`.** Two commands, in this order, with hooks off as for every
   git core starts:
   `git -c gc.auto=0 read-tree -m -u <kept> <before>` in the folder, then
   `git update-ref --no-deref refs/heads/<onto> <before> <kept>`.
   The first moves the files and the index and refuses whole for an unsaved or staged change in a file of the
   change, and for an untracked file where one would come back. The second writes one ref by its whole name and
   only while it still names the kept commit. A command that follows `HEAD` (`git reset --keep` was measured)
   rewinds whatever branch the folder stands on by then; these two cannot. `--no-deref` is added so that a
   branch name that is a symbolic ref is not followed either. The word `gc.auto=0` switches off nothing this
   command would do: it is how a test finds the command.
7. **Asked first, and asked again.** Under the keeps' lock an undo asks: the record says kept and has `before`;
   `HEAD` names `onto` and its tip is `kept`; no merge, rebase or other half-done act; no other name of the repo
   holds `kept` (`refs/remotes` included; a run's own branch is not such a name); nothing is on the disk where a
   file would come back, ignored or not (the keep's own rule with the two commits swapped); and git's trial of
   the move passes on a copy of the index. The check of `HEAD`, the tip and the disk is made a second time just
   before the step, with nothing but the writing of `back` between it and `read-tree`.
8. **`back` is written before the step**, and the record is cleared after the branch was seen at `before`. An
   undo cut off at any point is finished by the next `run undo`; until then `run keep` is refused and says so.
   An undo that goes on and finds something in its way is refused, and forgets `back`.
9. **On the page** a change kept by itself has the note `kept-<wish>`: "Your folder has this change now.", the
   count of its files, and one choice, `Undo it`. The answer to the tap is a note, and under it the work's own
   note is back with `Keep it` and `Toss it`, or the kept note again when core refused. Every word is core's.

## When it is kept by itself, and when it waits

| The change | |
|---|---|
| The folder is where the run started, clean in the change's files | kept by itself, moved forward |
| The person committed meanwhile, and the two join without a conflict | kept by itself, one new commit on top |
| It touches a file the person's tools obey | waits |
| It touches a `.gitattributes` | waits |
| A file is in the way, or a file of the change has an unsaved change | waits |
| The folder stands on no branch, or history no longer holds the run's start | waits |
| The folder or the repo is not found | waits |
| It follows work that still waits | waits |
| Another keep, toss or undo holds the lock | waits |
| The branch holds the work already | waits |

## Every refusal of an undo

Each is one fixed sentence with no word of git's and no run's id, and with nothing changed:

- nothing of the wish is kept;
- the change was kept by a tap, or was kept by itself before a later day's change was;
- the branch has a commit since the change was kept;
- a later change was kept on top of this one (the newest is undone first);
- a file of the change has unsaved or staged edits;
- a file is in the way where one the change removed would come back;
- the folder stands on another branch, or on none;
- a merge, a cherry-pick, a revert, a rebase or a bisect is going on;
- another name in the repo holds the change: a change already sent elsewhere is not undone here;
- the lock is busy, or the repo or the folder could not be read.

Two more say that something was changed: git stopped the step part way, and the record could not be written.
Both say that the next `run undo` goes on from there.

## Measured

git 2.50.1 (Apple Git-155), in scratch repos with no hook and empty configs:

- `read-tree -m -u <kept> <before>` exits 128 and writes nothing for an unsaved or a staged change in a file of
  the change, and for an untracked file at a path that comes back. It leaves an unsaved change in any other file.
- It writes over an **ignored** file at a path that comes back, and exits 0. So that is asked before, by core.
- `update-ref <ref> <before> <kept>` exits 0 while the ref names `kept`, and 128 with the ref unmoved otherwise.
- With `HEAD` on another branch at `kept`, the two commands move `refs/heads/<onto>` and no other ref.
  `git reset --keep <before>` there rewinds the branch `HEAD` is on.
- Run again after a cut between the two commands, `read-tree` exits 0 and the step ends as an uncut one.

## Open

- **The window between the second check and the step.** A file made at a returning path, or a checkout moved,
  after the second check and before `read-tree` is not seen: an ignored file made in that moment is written
  over, and a folder switched to another branch in that moment has its files moved while only the named branch
  is. What was measured of it is above; the window is one write of the record wide and is accepted.
- **A change pushed elsewhere** is found only by a name of this repo that holds it. A push that left no
  remote-tracking ref here (a push by URL, or from another clone) is not seen, and the undo then rewinds a
  branch whose commit another repo has.
- **A joined commit is named by nothing after an undo**: the run's branch names the run's own commit, and the
  one new commit a joined keep made is left to git's own tidying. The next keep makes it again.
- **How long a run's branch stays.** Until the change is undone and then kept by a tap or tossed, or until a
  later day's change is kept by itself in that repo. A repo no later change is kept in keeps its branches.
- **A later change that waits on top** (a follow-up of a kept change, not yet kept) does not refuse the undo of
  the change it follows.
- **Windows and Linux** were not run. The smoke chain was not run with this change.
