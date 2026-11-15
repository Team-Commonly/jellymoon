# 0020 · A project Jellymoon knows: a wish on a day nothing was typed

Status: Accepted by the coordinator, 2026-11-14. Nothing here was built or run. What the docs do not say is marked
"not confirmed", and the first feature checks it at the binary.

## Why

A wish needs a project, and a project is offered only when a folder of its name was typed in that day
(`jellymoon_core::wishable`, [wish.md](../development/wish.md), [app.md](../development/app.md)). A run finds its
repo the same way ([run.md](../development/run.md), "The order", 3). The reader leaves out a run's copy
([ADR 0018](0018-a-run-asks-and-is-told-more.md), decision 7), so a run does not count as typing.

So the loop only works for a person who also typed into a harness by hand that day:

| The person | What they meet |
|---|---|
| typed into Claude Code today, in a repo | a chip for that repo. The loop works. |
| made wishes with Jellymoon yesterday and typed nothing by hand today | no chip, and no wish by the entry, the command or the held key |
| has a repo and has never used a harness in it | no chip, ever |
| has no repo at all | nothing to start from |

The owner's direction is "as easy and fun as installing minecraft and building a wooden house with friends". Rows
two to four are the person that sentence is about. [ADR 0019](0019-a-wish-said-aloud.md) left this open as its
first risk: a held key makes it the first thing a new person meets.

## What binds the design

- **A folder's path is the person's.** It is not printed, not in page data and not sent to the model
  ([run.md](../development/run.md), "What is kept").
- **A log is untrusted.** A folder found in one must be the person's own before it is copied.
- **A switched-off folder stays off**, matched by its whole path (`--skip`, `skip.txt`).
- **Two folders of one name start nothing.** The person says which.
- **Logic lives in core**, proved at a `jellymoon` command.

## Decisions

### 1. Jellymoon keeps a list of the projects it knows

One file in Jellymoon's own folder, `projects.json`: for each project its name, its folder, and the day it was last
used. It is written as the tap's files are and read as one (a plain file only the person can write to). One folder
for a name: a second folder under a listed name is refused, in words that say how to let the first go.

A project gets on the list three ways:

1. **A run started in it.** Once a run's repo is found and has passed every check it passes today, core writes it
   down. So a repo typed in once is known from then on, with nothing asked of the person.
2. **The person names it**: `jellymoon project add --repo <folder>`, and in the app a folder dropped on the window
   (decision 4).
3. **The person starts a new one** (decision 3).

It leaves by `jellymoon project forget <name>`, which removes the line and touches nothing else. A listed folder
that is gone, is no repo's top, or is switched off stays listed and is not offered.

Not chosen: reading the folders back from the runs' records. It needs no new file, but it cannot hold a project the
person named or made, and a record from before F48 names no repo.

### 2. A project is offered when it was typed in today, or is on the list

`wishable` offers the names of the day's typed-in folders, as now, and the listed names whose folder passes the
checks a run makes: it is there, it is a repo's top, it is the person's own, it is not switched off. A run finds its
repo by the same one lookup, so what is offered is what will start.

When a name is both typed in today and listed, and the two folders are not one repo, nothing is offered under that
name and a run says so, as it does for two typed-in folders today. `--repo` settles it at the command line, and
`project forget` settles it for the page.

`jellymoon project list` prints names and the day each was last used. No path is printed: the rule that binds a
run's words binds these.

### 3. A new project is a folder Jellymoon makes, and only there

`jellymoon project new <name>` makes `<name>` in one folder of the person's home that Jellymoon owns, makes it a
repo with one empty first commit under the name a run commits under, and lists it. The app's entry offers it as
"Start something new" when no project is on offer.

- It writes inside that one folder and nowhere else. A name that is a path, is taken, or would not pass as a
  project's name is refused, and nothing is made.
- It needs git, as a run does. With none, it says so and makes nothing.
- This is a write to the person's disk, so the feature waits for a gate.

Where the folder is (`~/Jellymoon`, or under Documents) is decided in the feature, with the reason written down.

### 4. The app never asks for a path to be typed

A folder is named by dropping it on the window. The shell hands the path to core, and core answers with the name
or a refusal. The path is not drawn.

### 5. The day's page does not depend on typing

Not confirmed: what the window shows on a day with made wishes and nothing typed. [app.md](../development/app.md)
says only that such a day makes no reading call. The rule this note sets: a day with a made wish, or with a project
on offer, has a page with its wishes and an entry, and makes no call to a model for it.

## Features

Each is a row of `feature_list.json`, proved at its `test` command.

| Feature | What | In | Waits for |
|---|---|---|---|
| F62 | The list: `project add`, `list`, `forget`; a run's start writes it; `wishable` and a run's lookup read it | core, binary | a gate: it changes where a run may start |
| F63 | A day nothing was typed has a page of its made wishes and an entry | core, binary, app | F62 |
| F64 | A folder dropped on the window is a project | app | F62 |
| F65 | `project new` | core, binary | F62, and a gate: it writes to the person's disk |
| F66 | "Start something new" in the entry | app | F63, F65 |

Voice (F55 to F61) needs none of them to be built, and needs F62 and F63 before it is shown to a new person.

## Risks left open

1. **A listed folder is a path in one more file.** It is in Jellymoon's own folder, beside the runs' records that
   already name it.
2. **A repo moved or renamed** is not offered and is not found again. The person forgets the name with
   `project forget`, then adds the repo again: a second folder under a listed name is refused.
3. **An empty repo and a first wish.** What an agent builds from nothing, with no instructions in the repo, is
   not known. It is the first thing a new person sees, so it is tried by a person before "Start something new" is
   announced.
4. **Friends.** A project made here has no remote. Sharing it is not in this note.

## What changes in earlier notes

- [wish.md](../development/wish.md), [run.md](../development/run.md) and [app.md](../development/app.md): "the one
  folder of that name that something was typed in that day" becomes "typed in that day, or listed", in the feature
  that builds the list.
- [ADR 0019](0019-a-wish-said-aloud.md), "Which project": with none on offer the jellyfish says to make the first
  wish on the page. That stays, and the page then has a way to make one.
- `docs/GLOSSARY.md` gains **project** (a repo's top folder, called by its last name) and **known project** (one on
  Jellymoon's list), in this note's PR.
- `docs/INDEX.md`: the row of ADR 0019 said "Proposed" after the note was accepted. The word is removed.

## Found while building F62

What the build settled, and where it is not this note's words. The detail is
[project.md](../development/project.md).

- **A repo's top is a folder that holds `.git`, when a project is listed and when it is offered.** Decision 2
  says a listed folder passes "the checks a run makes". The run's own check of a repo's top asks git, and git is
  started in an empty folder a run makes in Jellymoon's home. `project add` and the look that offers a project
  write nothing, so they look for `.git` and do not start git. The run's copy still asks git before anything
  is made, and refuses in fixed words. So a folder with a `.git` that is no repo is offered and then refused,
  where the decision reads as not offered.
- **The list is all or nothing.** A file with one line `project add` could not have written is not used at all,
  as a day's made wishes are. Passing over the one line would leave the person a list that silently lost a
  project.
- **A listed path is used only as the real path it was listed by.** It is resolved when it is listed, and each
  time it is used it must still resolve to itself. A link put where a listed folder was, or a line written by
  hand that names a link or holds a `..`, leads nowhere.
- **A run's start writes the list after the copy is made and the run's lock is taken**, which is after every
  check. A harness too old for runs is found out later, so that repo is listed though no run went.
- **A run named a folder with `--repo` under a name another folder is listed for goes, and the list stays.**
  Decision 1 refuses the second folder at `project add`. At a run's start there is nobody to refuse to.
- **`run show`, `run keep` and `run toss` find the repo by the same lookup**, so the work of a run found by the
  list is shown, kept and tossed with no `--repo`.
- **Two typed-in folders of one name are offered and then refused by the run, as they were.** Decision 2 keeps
  the typed-in names "as now". Only the new case, a name typed in and listed, is taken off offer.
- **A wish said aloud picks a project less often.** ADR 0019 picks the only project offered. With a list,
  every listed project that stands is offered every day, so there is less often an only one. The rule is
  ADR 0019's and is unchanged; what it should be with a list is F63's or voice's to settle.
- **A repo's path is in a second file of the home**, as "Risks left open", 1, says: the test that held it to
  the run's record alone (`tests/f48.rs`) now names the two.
- **Not built here:** the app's refusal still says "typed in", and git's words for a listed folder are replaced
  by the words written for a folder read from a session log.

## Found while building F65

What the build settled for decision 3, and where it is not this note's words. The detail is
[project.md](../development/project.md), "A new project".

- **The folder is `projects` in Jellymoon's own folder**, so a made project is `projects/<name>` there, and not
  `~/Jellymoon` or a folder under Documents. That folder is already the one place Jellymoon writes; it is the
  person's alone; `JELLYMOON_HOME` moves it for a test with no second variable; and on a Mac a folder under
  Documents is behind a permission the binary would have to ask for. The cost: it is inside a folder whose
  name starts with a dot, which the Finder does not show. The app's entry (F66) is where a person meets a made
  project, and "Friends" (risk 4) is where its place may have to change.
- **It is made only when the first project is**, for the person alone (0700), as `copies`, `runs` and `pages`
  are. It must be a real folder: a link or a file at its name is refused, and is never followed or replaced.
- **A name is one folder's name on macOS, Linux and Windows.** It is F62's check of a project's name, and on
  top of it: no dot, dash or space at its start (a hidden folder, an option), no dot or space at its end
  (Windows cuts them), no colon (a drive, a stream) and no name a system keeps for a device (`NUL`, `COM1`);
  and, since the gate, what is listed below. Refused in one sentence that never says the name back.
- **Taken is a name on the list, whatever its folder, or anything at the name in `projects`**: a folder, a file
  or a link, which is never followed. Both are asked with the case of letters set aside, on every system, so a
  home copied to a disk that folds case holds no two projects in one folder. Two Unicode forms of one name are
  left to the file system: where it takes them for one, the second is something that is there; where it keeps
  them apart, they are two projects. A file system that stores a name in other letters than it was given is
  refused after the fact, and what was made is taken back.
- **The order.** The name, the list, the folder and git on `PATH` are looked at first, and a refusal there has
  written nothing at all. Then, behind the list's own lock (see what the gate found, below), every look is
  taken again, `projects/<name>` is made by a call that fails when anything is at the name, then the repo,
  then the line on the list.
- **A step that fails after the folder is made takes the folder back and lists nothing**: git, a list that
  filled or was written over by hand meanwhile. `projects` goes too when this command made it. What stays is Jellymoon's folder and the empty folder git is started in
  (`runs/no-hooks`), as after a run's copy that was refused.
- **A command killed between its steps leaves the folder, unlisted.** Once its lock is nobody's, the next
  `project new` of that name says that something is there, and names the two
  ways out: list it with `project add --repo` when the repo was whole, or remove it and ask again. A folder
  with a `.git` and no commit is listed by `project add` and refused by a run's copy, as F62 found.
- **git is run as a run's copy runs it**: the program found in the full folders of `PATH`, started in the empty
  folder, with the copy's own short list of variables and its settings, so no hook, signing program or monitor
  of the person's config is started. `git init` is given an empty template, so the repo holds no hook and no
  sample. The branch is `main`, set after `init`, whatever the config names for a new repo. The commit is made
  with git's plumbing, as a run's is: no file, no parent, the message `Jellymoon project <name>`, by the
  person as the new repo's config reads them, or by `Jellymoon <jellymoon@localhost>` when either the name or
  the address is missing. No step of it is one git does upkeep after.
- **What git says is never repeated**, as for a folder read from a session log: it names the folder.
- **`project forget` of a made project forgets the line and keeps the folder**, as for any project. The name
  is then taken by the folder: `project add --repo` lists it again.
- **What F65's gate found, and what was decided.**
  - *One making at a time.* The whole of a making, from its second look to its line on the list, is behind
    the list's lock. Before, a making held between its folder and the list could be met: its name's folder
    removed and made again by another command, and then taken back by name with the person's file in it. No
    `project new`, `add` or `forget` meets a making now; each waits 3 seconds and is told one has not finished.
  - *What is taken back is the folder this command made*, known by the system's number for it and its time of
    making, compared before it is removed and before it is listed; never a folder the list names; and
    `projects` only when this command made it and it is empty. A folder put at the name by another hand is
    left and not listed.
  - *Something at the name is asked again behind the lock*, so a second `project new` of a name under making
    waits, and is not told to remove or list a folder that is being made.
  - *A killed making leaves its lock* for the 30 seconds any lock of the list is left; the sentence of the
    lock says to ask again in half a minute.
  - *The name's rule is wider*: none of `* ? | < > "`, no tilde at the start, no joiner, and the device forms
    `COM¹`, `CONIN$` and `CONOUT$`. It is a name a folder can have on macOS, Linux and Windows.
  - *The sentence of a file system that changed the name is said only when the kept name differs.* Whose
    Jellymoon's folder is, and whether its path is text, are asked before anything is made, in their own
    sentences; what `project add` says of a folder is passed on when it is said of the made one.
  - *The person's config chooses nothing of the repo but who its commit is by*: `init` is told the object
    format, the ref format and that no file is opened to others, and the last is written to the repo's config.
- **Not built here:** the app's "Start something new" (F66); a remote; a `.gitignore` or any first file; a
  way to make a project anywhere else; and `project new` on Windows and Linux, which nobody has run.
