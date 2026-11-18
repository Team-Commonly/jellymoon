# LOCK-NOTES-JOIN: the lock notes joined with F65, and "denied" on Windows told apart

**What changed.** On branch `lock-notes-join`, one commit on `edaf08a` (the lock notes joined with main, which
holds F65). The join broke eight of F65's tests; this mends the join, and takes the second gate's notes 2 and 3.

## The join with F65

Every hold of a lock now leaves `take-over.lock` beside it, and `project new` holds the list's lock.

- **The home is no longer taken back.** F65 removed Jellymoon's folder after a refusal behind the lock when the
  folder had been made only to hold the lock. With `take-over.lock` in it that removal could never succeed, so it
  is taken out of `project/new.rs`: the folder stays, holding `take-over.lock` and what the making already left
  (`runs/no-hooks` after a git step). `project.md` and ADR 0020 say so, and the "Not promised" entry of
  `project.md` about that removal racing another command is gone with the removal.
- **A refusal before the lock still makes nothing**, not even the folder: the name, the list, a link or a file
  where `projects` would be, no git, a home that could not be listed in. Their tests are as they were.
- **Something at the name** is the one refusal said behind the lock, so it leaves `take-over.lock`.
- **A safe way to keep the take-back, not built.** Removing the folder means removing `take-over.lock`, and
  another caller may have it open, or be about to open it: it would then hold the system's lock on a file that
  is at no name while a third makes and locks a new one. It is safe only if every caller, once it has the
  system's lock, looks again that the file it locked is the one at the name still, and asks again when it is
  not. That is a change to the lock's own rule, for a folder with one empty file in it.

**F65's tests changed** (each still compares the whole test folder, with `take-over.lock` named, never left out):

| test | what changed |
|---|---|
| `alone::but_the_lock` (helper) | leaves out `projects.lock` only; it no longer leaves out `take-over.lock` |
| `alone::and_the_take_over` (new helper) | what was there before, and `take-over.lock`, empty, and Jellymoon's folder when it was not there; nothing else |
| `refused::new_refused` (helper) | for "something is there" the folder after is the folder before and `take-over.lock`; for every other refusal it is the folder before, as it was |
| `alone::a_making_that_is_killed_is_waited_for_…` | after the take-over the whole folder is compared: the lock gone, `take-over.lock` there |
| `alone::something_at_the_name_is_said_before_git_is_looked_for` | "something is there" leaves `take-over.lock`; "no git", said before the lock, leaves the folder as it then is |
| `alone::a_home_a_project_could_not_be_listed_in_…` | the home made to hold the lock stays, holding exactly `take-over.lock`, empty; the two refusals before the lock are as they were |
| `mark::a_making_whose_file_a_hand_removed_…` | the folder after is the folder before and `take-over.lock` |
| `half::a_git_step_that_fails_…` | a home that was not there holds `runs`, `runs/no-hooks` and `take-over.lock`, each empty, compared with their bytes |
| `half::a_making_that_is_killed_leaves_a_folder_…` | after the take-over the whole folder is compared, with `take-over.lock` |

`alone::a_folder_put_in_place_…` and `alone::a_folder_removed_while_…` pass again with no line of their own
changed: `take-over.lock` is there before and after, and is now compared.

**The take-over tests still pass for the reason in their names.** The three that kill a making and age
`projects.lock` (`alone`, `half`, `mark`): a caller that waits never reads a mark, so a lock that holds a dead
maker's mark and is 40 seconds old is left behind by its time and taken over behind the system's lock. With the
lock not aged each of them fails (mutations j3a to j3c).

**A making that outlives 30 seconds.** F65's argument holds as written: its lock is taken over by the next
command, and the making's file, not the lock, tells it whose the folder is. With a marked lock such a making
could know at its end that the lock is no longer its own. That would allow it to refuse to list or to take
back anything once its lock is gone, which closes the window where it writes the list with no lock. Not built
here: it belongs to the F65 notes.

## "Denied" on Windows (the second gate's note 2)

- **The look.** A `take-over.lock` with the read-only attribute is not to be had at all, on Windows, at the look
  already made before the open (`lockable`).
- **Every pass.** "Denied" is now an answer of its own, apart from "another caller holds it". A wait ends in the
  "remove both" refusal when every one of its passes was denied, or when its last pass found what can never be
  locked, as before. A sharing violation or a lock violation on any pass, a take-over that was done, or a lock
  that is not left behind, and the answer is the usual one. The rule is a value (`Wait` in `lock.rs`), tested on
  every system.
- A holder letting go that is denied waits its 3 seconds, as before.

## The let-go on Windows (note 3)

`delivery.md` said 6 seconds at most; it is 9 (the system's lock, the read of the mark, the removal). The
sentence is corrected. No limit changed.

## Red first

F65's tests on `edaf08a`: 8 of 33 failed, the eight the gate named. Now 33 of 33.

## Mutations

| | mutation | killed by |
|---|---|---|
| j1a | the home's take-back put back as it was | **lives**: it can remove nothing, since the folder holds `take-over.lock`; that is why it was dead code |
| j1b | the take-back put back so that it works (the take-over's file removed first) | 12 of F65's tests |
| j2 | `take-over.lock` left out of every comparison of the whole folder | 8 of F65's tests |
| j3a | `mark`'s take-over test, lock not aged | that test |
| j3b | `half`'s take-over test, lock not aged | that test |
| j3c | `alone`'s take-over test, lock not aged | that test |
| j4 | a refusal before the lock makes Jellymoon's folder | `refused::with_no_git_…`, `alone::a_home_…`, and the first test of `f65.rs` |
| j5 | the lock is held before the first look | `refused::with_no_git_…` and three more |
| b1 | the read-only attribute is not looked at | `on_windows_a_read_only_file_at_the_take_over_s_name_…` |
| b2 | one denied pass is enough | `a_wait_ends_on_a_lock_nobody_can_take_over_…` |
| b3 | a sharing violation counted as denied | the mapping's test |
| b4 | every pass denied still read as busy | `a_wait_ends_on_a_lock_nobody_can_take_over_…` |
| b5 | a first pass that was denied is not counted | the same |
| b6 | the wait is not asked at its end | the three tests of a lock nobody can take over |

## What's open

Windows and Linux: nothing was run there. The Windows-only test of a read-only `take-over.lock` was not even
compiled; that the open there says "denied" of a read-only file, and every Windows error code, are read, not
run. The app's tests were not run.

## Deleted

The take-back of Jellymoon's folder in `project/new.rs`, and the "Not promised" entry of `project.md` about it.
