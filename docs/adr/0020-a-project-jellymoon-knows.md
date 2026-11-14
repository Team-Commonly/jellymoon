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
2. **A repo moved or renamed** is not offered and is not found again. The person adds it again.
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
