# The projects Jellymoon knows: `jellymoon project`

A project is a repo's top folder, called by its last name. Until F62 a wish could be made for one, and a run could
find it, only on a day something was typed in it. Jellymoon now keeps a list of the projects it knows
([ADR 0020](../adr/0020-a-project-jellymoon-knows.md), decisions 1 and 2), so a wish can be made on a day
nothing was typed. The rules are core's, in `crates/jellymoon-core/src/project.rs` and `project/offer.rs`.

```
jellymoon project add --repo <folder> [--date <YYYY-MM-DD>]
jellymoon project list
jellymoon project forget <name>
```

Each prints one line of JSON and exits 0. No line any of them prints, and no refusal, names a folder. A command
line that is wrong is answered in one fixed line with the usage, and the argument that was not expected is never
said back: it may be a folder with `--repo` forgotten, or a credential.

```json
{"name":"alpha","used":"2026-03-10"}
```

`add` and `forget` print the project (`project.json` in core's examples). `list` prints them all, in the order
they were listed (`projects.json`):

```json
{"projects":[{"name":"alpha","used":"2026-03-10"},{"name":"beta","used":"2026-03-11"}]}
```

## `project add`

Lists the folder under its last name, used on `--date` (today when none is named).

- `--repo` is a full path. Every link and every `..` in it is followed once, and the project is listed by where
  that leads: its real path, under the real folder's last name. A link named `nick` that leads to a repo named
  `alpha` lists `alpha`.
- The folder must be there, hold a `.git` entry of its own (so a folder inside a repo is refused, and the repo
  above it is not listed in its place), and on Unix be owned by the owner of the person's home folder.
- Its last name must be a name `jellymoon wish --project` takes: one folder's name of at most 100 characters,
  with nothing unseen and nothing shaped like a credential. A name that is not is never said back.
- A project added again keeps its line and its place. Its day moves on to a later day and never back.
- A name has one folder. A second folder under a listed name is refused, in words that say how to let the first
  go. The list holds 100 projects.
- Nothing in the folder is written, git is not started, and no run is.

## `project forget`

Takes the line of that name off the list and touches nothing else: not the folder, not a wish, not a run or its
branch. The name is as `project list` prints it, letter for letter.

## A run's start writes the list

Once `jellymoon run` (and so `wish --run`, the app and a heard wish) has found its repo, made its copy and taken
its lock, the repo is on the list, used on the wish's day ([run.md](run.md), "The order", step 4). So a repo
typed in once, or named once with `--repo`, is known from then on. This write never fails a run. It is left out
when the folder's real last name is not the wish's project, when the folder is not the person's own, when
another folder is listed under the name, when the list is full, when the list cannot be used, and when the
list's lock is held: the run goes on after the 3 seconds and lists nothing.

## What is offered, and where a run starts

One lookup answers both (`project/offer.rs`), so what is offered is what starts. `offered` (the chips of the
page's entry), `wishable` (a wish entered there, and a wish said aloud) and a run with no `--repo` ask it for a
name:

| The name has | It leads to | Offered |
|---|---|---|
| one folder typed in that day, and no line | that folder, as before | yes |
| one folder typed in that day, and a line for the same folder | that folder | yes |
| one folder typed in that day, and a line for another folder | nothing: the run says so | no |
| two or more folders typed in that day | nothing: the run says how many, as before | yes, as before |
| no folder typed in, and a line whose folder stands | the listed folder | yes |
| no folder typed in, and a line whose folder does not stand | nothing: the run says why | no |
| neither | nothing: the run's line is the one it was | no |

"Typed in that day" is what the reader says, with the folders switched off left out; no session logs at all is
a day nothing was typed. Two folders are one when the typed-in folder's real path is the listed path.

**A listed folder stands** when, at that moment, all four hold. They are asked each time the name is looked
up, because the list is a file and a file can be changed:

1. It is there, and the listed path is still its real path, link for link: `it is not there, or is reached
   through a link now`.
2. It holds a `.git` entry: `it is not the top of a git repo`.
3. On Unix it is owned by the owner of the person's home folder: `it is not a folder of your own`.
4. No name given with `--skip` or kept in `skip.txt` is in its path: `it is switched off`. A `skip.txt` that
   cannot be used offers nothing at all, as before.

A folder that does not stand stays listed. The run's copy then asks git itself, as it does of a folder found
in a log, and nothing git says of a listed folder is repeated.

## Every sentence

`project` refusals are one line on stderr, exit 1, and nothing is written. A command line that is no command
is exit 2 with the usage.

| When | The line |
|---|---|
| `--repo` is no full path | `--repo must be a full path, from the root` |
| nothing, or no folder, is there | `--repo is not a folder that is there` |
| the last name is no project's name | `--repo is not a folder a project can be named after: its last name must be one folder's name of at most 100 characters, with nothing that cannot be seen and nothing shaped like a credential` |
| the real path is not text, or over 4096 bytes | `--repo is not a folder Jellymoon can list: its full path is not text, or is over 4096 bytes` |
| it holds no `.git` | `--repo is not the top of a git repo` |
| it is someone else's | `--repo is not a folder of your own` |
| another folder has the name | ``another folder is listed under the name <name>, and a name has one folder, so nothing was changed; let the listed one go with `jellymoon project forget <name>`, then add this one`` |
| the list is full | ``the list has its 100 projects already, so this one was not added; let one go with `jellymoon project forget` `` |
| the command line is wrong | ``project is `project add --repo <folder>`, `project list` or `project forget <name>` ``, exit 2, with the usage |
| `forget` is given no project's name | ``project forget takes a project's name, as `jellymoon project list` prints it`` |
| the name is not listed | `no project of that name is on the list, so nothing was changed` |
| the lock stays for 3 seconds | `another change to the list of projects has not finished, so nothing was changed` |
| the list cannot be used | `the list of projects (projects.json in Jellymoon's folder) is not used: <why>; move the file away and add your projects again` |

What a run says, where `<way out>` is ``name one with --repo, or let it go with `jellymoon project forget` ``:

| When | The line |
|---|---|
| a typed-in folder and another listed one | `a folder of the name the wish gives was typed in on <day> and another folder is listed under that name; <way out>` |
| the listed folder does not stand | `the folder listed under the name the wish gives is not one a run can start in: <one of the four>; <way out>` |
| git cannot copy the listed folder | `the folder listed under the name the wish gives is not one a run can copy: <copy's reason, in fixed words>` |
| nothing typed in, and the list cannot be used | `no folder of the name the wish gives was typed in on <day>, and the list of projects (projects.json in Jellymoon's folder) is not used: <why>; move the file away and add your projects again; or name the folder with --repo` |

## The file

`projects.json` in Jellymoon's own folder, one line (`projects-kept.json` in core's examples):

```json
{"projects":[{"name":"alpha","folder":"/work/alpha","used":"2026-03-10"},{"name":"beta","folder":"/work/deep/beta","used":"2026-03-11"}]}
```

- **The folder.** Jellymoon's home is the person's alone (0700) only when Jellymoon makes it. A home that was
  there is left as it is; the file is 0600 either way.
- **The write.** Whole, into a new file beside its place that is then moved there, 0600, behind
  `projects.lock` in the same folder: a file only one command can make, removed when the change is done, waited
  for 3 seconds, and taken over when it was left behind, which can leave `take-over.lock` beside it (the tap's
  lock, [delivery.md](delivery.md), "The lock"). A change
  is tried on the list before the lock is asked for, so a refusal makes no folder and no file, and again
  behind the lock. Reading takes no lock: a reader sees the old file or the new one.
- **The read.** The file is untrusted. It is used only when it is a plain file, never a link, that its group
  and others cannot write to, of at most 512 KiB, that is text, of this exact shape with no other field, with at
  most 100 projects, each of which `project add` could have written: a name that is one folder's name; a folder
  that is a full path of at most 4096 bytes with no `.`, no `..`, nothing doubled and no separator at its end,
  whose last name is the name; a day; and no name twice. Otherwise the whole list is not used: `list`, `add`
  and `forget` refuse, nothing listed is offered, a run's start does not write it, and it is never written
  over. The reason names a line by its place, never by its text: `it is not a plain file`, `others can write to
  it`, `it is too large`, `it is not text`, `it is not a list as Jellymoon writes one`, `project <n> of it is
  not one Jellymoon wrote`, `project <n> of it has the name of one before it`. A folder typed in that day is
  still found.

## What a line written by hand can do

Whoever can write Jellymoon's folder can write a line, as they can write a ruling or a made wish.

- **It can** list a repo of the person's own under that repo's own last name, exactly as `project add` would.
  A wish for that name then starts its run in that repo.
- **It cannot** start a run in a folder under another name than the folder's own; reach a folder through a
  link, a `..` or a relative path (the line is refused, or the folder does not stand); start one in a folder
  that is someone else's, is switched off or holds no `.git`; make any command print a path or the line's
  text; or make a typed-in folder of the same name run somewhere else: the two clash and nothing starts.

## Not covered

- **`.git` is looked for, git is not asked**, by `add` and by the look that offers a project. A folder with a
  `.git` that is no repo is listed and offered, and is refused when the run's copy asks git. Asking git needs
  the empty folder a run keeps for it, and a look writes nothing.
  A `.git` that is a link, or a file that holds `gitdir:`, is accepted too. So a worktree of a repo is listed,
  and its run's branch is in the main repo, as it is for a worktree typed in or named with `--repo`.
- **A run that was never started** has listed its repo by then: the checks were passed. That is so for a
  harness too old for runs and for one that cannot be started at all.
- **A run named `--repo` in a folder that is switched off is listed.** `--repo` is the person's own word, and
  the run goes. The next run with no `--repo` is refused: `it is switched off`.
- **A repo that was moved and is typed in at its new place is refused** until `project forget`: the name has
  a typed-in folder and another one listed. The line of the old place is never dropped by itself.
- **One folder under two spellings.** A name is compared letter for letter. Where the file system takes
  `Alpha` and `alpha` for one folder, a folder typed in as `Alpha` and listed as `alpha` are two names.
- **A wish said aloud picks a project less often.** A hearing picks the project of the day's last made wish
  when it is offered, else the only one offered, else none ([hear.md](hear.md)). Every listed project that
  stands is offered every day, so with two listed there is often no only one, and the jellyfish says to make
  the wish on the page. The smoke chain's `hear` step forgets what the chain's runs listed for this reason.
- **Two typed-in folders of one name** are still offered and still refused by the run, as before the list.
- **The app's refusal** of a wish for a project that is not offered still says `no folder of that project that
  is switched on was typed in on <day>`.
- **Git's words for a listed folder** are replaced by the fixed words written for a folder read from a session
  log, which name a session log.
- **Windows** has no owner to read, as for a typed-in folder. Nothing here was run on Windows or Linux.
- `project new` and a folder dropped on the window are F64 to F66. The page of a day nothing was typed on is
  built (F63): [page.md](page.md) and [app.md](app.md) say what it holds.

## Tests

`crates/jellymoon/tests/f62.rs` and `f62/`: `add`, `list` and `forget`; every refusal with the home's tree
compared; a stray folder or credential after each command, never said back; a run refused at its copy, which
lists nothing; a wish said aloud with two projects on offer; eight changes at once and a lock that stays; the list a run's start writes; a listed project found by
`run`, `wish --run` and `hear`; a folder that is gone, no repo's top, a link, switched off or someone else's; a
name typed in and listed; and lists written by hand. The examples round-trip in core's `tests/shapes.rs` and
`project.rs`.
