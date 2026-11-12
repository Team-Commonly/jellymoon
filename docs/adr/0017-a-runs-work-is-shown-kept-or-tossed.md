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
   they trust what their agent writes in a session of their own.
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
| ADR 0014, 4: "The branch is kept: it is the work." | The branch is kept until the person keeps the work or tosses it; either removes the run's branch |
| `wish.md`: a made wish's "state moves only with its one run" | And with a toss, which lets the wish go |
| Glossary, **copy**: "the branch, with the work on it, is kept" | Kept until the work is kept or tossed |

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
| What the work is | `diff-tree -r --no-renames --name-status -z <parent> <commit>`, `cat-file -s`, `cat-file blob` |

Not measured: git 2.38 and 2.39 (the oldest that can merge in the store, and the one older Macs ship), Windows and
Linux, a repo with filters or LFS, and a keep cut off between its steps. The features below measure them.

## The slices

`feature_list.json` is read top down, and F30 and F31 stand above these. The order is stated here: **F44 to F47 are
built before F30 and F31.**

| Feature | What it adds | Waits on | Waits for `GATE: PASS` |
|---|---|---|---|
| F44 | `jellymoon run show`: what a run's work made, and one file's text | nothing | no |
| F45 | `jellymoon run keep`: the work lands on the person's branch | F44 | yes: an agent's work reaches the person's checkout |
| F46 | `jellymoon run toss`: the work and its branch go, and the wish is let go | F44 | yes: it deletes in the person's repo |
| F47 | A tap on a little one opens its wish: `Show me`, `Keep it`, `Toss it` | F45, F46 | no |

## What comes after

Each gets its own note before it is built.

1. **A run that answers back.** A run today is one shot: it cannot ask, and it cannot be told more. Which of an
   agent's questions reach the person, how an answer gets back, and "more like this" on work that was shown.
2. **A wish said aloud, from anywhere** (F30): a held key, and a little one at the edge of the screen.
3. **Codex** (F31), then one starter that speaks ACP.

## What this does not do

- It shows no running program and no picture of one. A preview needs the work run, and nothing here runs it.
- It makes no merge commit, resolves no conflict and rebases nothing. Work that no longer fits is said to no longer
  fit, and the person wishes again.
- It does not make the list in decision 6 complete. A `Makefile`, a build script or a test can run what it likes
  when the person next builds.
