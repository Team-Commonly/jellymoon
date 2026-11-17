# The projects Jellymoon knows: `jellymoon project`

A project is a repo's top folder, called by its last name. Until F62 a wish could be made for one, and a run could
find it, only on a day something was typed in it. Jellymoon now keeps a list of the projects it knows
([ADR 0020](../adr/0020-a-project-jellymoon-knows.md), decisions 1 to 3), so a wish can be made on a day
nothing was typed, and makes a new project when asked (F65). The rules are core's, in
`crates/jellymoon-core/src/project.rs`, `project/offer.rs` and `project/new.rs`.

```
jellymoon project add --repo <folder> [--date <YYYY-MM-DD>]
jellymoon project new <name> [--date <YYYY-MM-DD>]
jellymoon project list
jellymoon project forget <name>
```

Each prints one line of JSON and exits 0. No line any of them prints, and no refusal, names a folder. A command
line that is wrong is answered in one fixed line with the usage, and the argument that was not expected is never
said back: it may be a folder with `--repo` forgotten, or a credential.

```json
{"name":"alpha","used":"2026-03-10"}
```

`add`, `new` and `forget` print the project (`project.json` in core's examples). `list` prints them all, in the order
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
- A folder directly in Jellymoon's `projects` that a `project new` that was cut off left is added as any
  other, and then loses that making's file: each plain, empty file in it named `.jellymoon-making-` and three
  numbers with a dash before each. A link, a folder, a file that holds something or has any other name is
  not touched, and no folder anywhere else is looked into.
- A project added again keeps its line and its place. Its day moves on to a later day and never back.
- A name has one folder. A second folder under a listed name is refused, in words that say how to let the first
  go. The list holds 100 projects.
- Nothing in the folder is written, git is not started, and no run is.

## A new project

`project new <name>` makes a project where there was none: the folder `projects/<name>` in Jellymoon's own
folder, a git repo with one first commit that holds no file, listed under `<name>`, used on `--date` (today
when none is named). It prints the project as `add` does, and no folder. A wish for the name then starts its
run there, with no `--repo` and no session log.

- **Where.** Only in `projects` in Jellymoon's folder, which is made with the first project, for the person
  alone (0700), as the new folder is. Nothing is written outside it but what any run writes in Jellymoon's
  folder: the list, its lock, and the empty folder git is started in (`runs/no-hooks`).
- **The name** is one a folder can have on macOS, Linux and Windows: a project's name as `add` takes one
  (which has no space at either end and no two together, and nothing that draws nothing or draws as a space:
  U+034F, the Hangul fillers U+115F, U+1160, U+3164 and U+FFA0, the empty Braille pattern U+2800 and the
  selectors U+FE00 to U+FE0F, beside what cleaning takes out), with no dot, dash or tilde at its start, no dot at
  its end, none of `: * ? | < > "` in it, no joiner (U+200C, U+200D: not seen between two letters), and not a
  name a system keeps for a device: `CON`, `PRN`, `AUX`, `NUL`, `CONIN$`, `CONOUT$`, and `COM` or `LPT` with
  one of `0` to `9`, `¹`, `²`, `³` after it, in any case, with or without an ending. So no name is a path, an
  option, a hidden folder, the person's home or a drive. `COMA` and `COM10` are names.
- **Taken.** A name on the list, whatever folder it is listed for, and anything that is at the name in
  `projects`: a folder, a file, or a link, which is never followed and never replaced. Both are asked with the
  case of letters set aside, on every system: with `alpha` listed or there, `Alpha` is taken. Two Unicode
  forms of one name are left to the file system.
- **`projects` itself** must be a real folder. A link or a file at its name is refused.
- **The repo.** git is run as a run's copy runs it ([copy.md](copy.md), "What is switched off"): no hook,
  signing program or monitor of the person's config is started. `init` copies no template, so the repo holds no
  hook. The person's config chooses who its commit is by, and not the format, the branch or who may read it
  (a setting such as `core.splitIndex` still shapes a file inside `.git`): `init` is told git's own
  object format (`sha1`) and ref format (`files`) and to open no file to others (`core.sharedRepository =
  umask`, which the new repo's own config then holds, as every later git reads the person's again). The
  branch is `main`. The commit has no file and no parent, the message `Jellymoon project <name>`,
  and is by the person as the new repo's config reads them, or by `Jellymoon <jellymoon@localhost>` when
  the name or the address is missing: as a run's commit is.
- **The order.** The name, the list, what is in `projects`, git on `PATH` and whether Jellymoon's folder is
  one a project can be listed in (the person's own, by a path that is text and short enough) are looked at
  first; a refusal there has written nothing, not even Jellymoon's folder. Something at the name is the one
  refusal that is asked again behind the lock, as it may be a making under way. Jellymoon's folder, when it
  was not there, is made to hold the lock, and a refusal behind the lock leaves it there, holding
  `take-over.lock`, empty, which no command removes ([delivery.md](delivery.md), "The lock"), and what the
  making had already left (`runs/no-hooks` after a git step): "nothing was made" is said of the project, the
  list and every other file.
- **One making at a time.** The whole of a making is behind the list's lock (`projects.lock`): every look
  again, the folder, made by a call that fails when anything is at its name, the repo, and the line on the
  list. So no `project new`, `add` or `forget` meets a making under way: it waits its 3 seconds and is told
  one has not finished. Of two `project new` of one name at once, one makes the project and the other is told
  the name is listed, or that something is there.
- **A step that fails after the folder is made** (git, a list that was filled or written over by hand
  meanwhile, a folder that turns out not to be the person's own) takes back the folder this command made, and
  lists nothing. The folder is not known by its name. It is known by two things, both asked just before it
  is listed and just before it is taken back: what the system says of it (where it keeps it, on Unix, and
  when it was made), and a file of this making's own in it, `.jellymoon-making-` with the process, the clock
  and a count after it, written as the first thing in the folder. A folder another hand put at the name
  meanwhile does not hold that file: it is left, with what is in it, and is not listed (`the new project's
  folder was removed, replaced or changed…`, which is also what a folder that is simply gone is told, and one
  that is the made folder still but whose making's file a hand took out, by `git clean` say). The file is the
  person's alone (0600 on Unix) and empty. It is removed once the line is on the list, when it can be: a
  folder a hand closed to writing meanwhile is listed and keeps it. Git does not commit it. A folder a
  line of the list names is never removed: it loses the making's file and stays. `projects` goes only when
  this command made it and it is empty. A refusal that came after the folder was made, and whose own words
  do not say so, ends `; the folder made for it meanwhile was taken back`; when the folder was this command's to take back and
  could not be removed, it ends `; the folder made for it meanwhile could not be taken back and was left`, and what is left may have lost part of
  what was in it.
- **A command that is killed** between making the folder and writing the list leaves the folder, unlisted,
  with the making's file in it beside the repo, and the lock. For half a minute every change to the list is told one has not finished; then the lock is taken
  over, and the next `project new` of that name says something is there. Mend it by `project add --repo` with
  the folder, when the repo in it is whole, which also takes the making's file out; or by removing the folder
  and asking again.
- **`project forget`** of a made project takes its line off the list and leaves the folder. The name is then
  taken by the folder, and `project add --repo` with the folder lists it again.

## `project forget`

Takes the line of that name off the list and touches nothing else: not the folder, not a wish, not a run or its
branch. The name is as `project list` prints it, letter for letter. It is any name a line can have, so a line
an earlier build wrote under a name that is refused today can be taken off.

## A name that comes in, and a name that was kept

The characters nobody sees that cleaning leaves (U+034F, the Hangul fillers U+115F, U+1160, U+3164 and U+FFA0,
the empty Braille pattern U+2800, the selectors U+FE00 to U+FE0F) were a project's name once: an earlier build
listed `app` followed by a heart and U+FE0F, and kept wishes for it.

| where the name is | such a name |
|---|---|
| `project new <name>` | refused |
| `project add --repo`, the folder's last name | refused |
| `wish --project` for a name the list does not hold | refused: `--project takes one folder's name…` |
| `wish --project` for a name the list holds, letter for letter | kept, and its run starts in the listed folder |
| the offers: a chip on a page, a wish said aloud, a wish made from a page | never offered |
| a line of the list, read back | used: listed, found by a run, forgotten by `project forget` |
| a kept wish, read back | used: the day's other wishes are, and its own run starts when its folder is listed, typed in that day or named with `--repo` |

So nothing an earlier build wrote is refused, and no new name of that kind comes in. A typed-in folder of such
a name that is on no list gets no new wish, and `project add` of it is refused: rename the
folder. A run in a listed folder of such a name does not move the line's day on, as that is `add`'s look.

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

`project` refusals are one line on stderr, exit 1, and nothing is written, or, for those of `new` that
say so, what was written is taken back. A `--repo` sentence from `new` is said of the folder it made, after
that folder was taken back, and so is any other sentence that ends `; the folder made for it meanwhile was taken back`: a
list that filled or was written over meanwhile, and `jellymoon's own files: …` for a list that could not be
written. One that ends `; the folder made for it meanwhile could not be taken back and was left` is said of a
folder this command made and could not remove. The list's own sentences (full, the lock, a list that cannot be used)
are `new`'s too. A command line that is no command
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
| the command line is wrong | ``project is `project add --repo <folder>`, `project new <name>`, `project list` or `project forget <name>` ``, exit 2, with the usage |
| `new` is given no folder's name | `project new takes a name for the new project: one folder's name of at most 100 characters, with no dot, dash, tilde or space at its start, no dot or space at its end, no two spaces together, none of : * ? | < > " in it, no name a system keeps for a device, nothing that cannot be seen and nothing shaped like a credential; nothing was made` |
| `new`: the name is listed | ``a project of that name is on the list already, so nothing was made; pick another name, or let the listed one go with `jellymoon project forget` `` |
| `new`: something is at the name | ``something is already there under that name in the folder Jellymoon makes projects in (projects in Jellymoon's folder), so nothing was made; pick another name, list what is there with `jellymoon project add --repo` and its folder, or, if a `project new` that was cut off left it, remove it and ask again`` |
| `new`: `projects` is a link or a file | `the folder Jellymoon makes projects in (projects in Jellymoon's folder) is not a folder Jellymoon made: a link or a file is at its name; nothing was made` |
| `new`: no git | ``no `git` program was found on PATH, and a new project is a git repo, so nothing was made`` |
| `new`: git fails | `git could not make the new project's repo (what git said is not repeated, as it names the folder), so what was made was taken back and nothing was listed` |
| `new`: the empty folder for git holds something | `the folder Jellymoon keeps empty for git (runs/no-hooks) holds something, so what was made was taken back and nothing was listed` |
| `new`: Jellymoon's folder is someone else's, or the person has no home folder | `Jellymoon's folder is not a folder of your own, and a project is listed only in a folder of your own, so nothing was made` |
| `new`: the path of Jellymoon's folder is not text, or too long | `the full path of Jellymoon's folder is not text, or a project's in it would be over 4096 bytes, so a new project could not be listed and nothing was made` |
| `new`: no folder, another folder, or the folder without the making's file is at the name once the repo is made | `the new project's folder was removed, replaced or changed while it was being made, so nothing was listed, and what is there now was left as it is` |
| `new`: the folder is kept under another name than was asked | `this file system does not keep the folder under the name it was given, so what was made was taken back and nothing was listed; pick a name of plain letters` |
| `forget` is given no project's name | ``project forget takes a project's name, as `jellymoon project list` prints it`` |
| the name is not listed | `no project of that name is on the list, so nothing was changed` |
| the lock stays for 3 seconds | `another change to the list of projects, or the making of a new project, has not finished, so nothing was changed; if it was cut off, ask again in half a minute` |
| the lock was left behind and cannot be taken over ([delivery.md](delivery.md), "The lock") | `jellymoon's own files: projects.lock is a lock that was left behind, and it could not be taken over, as take-over.lock beside it could not be locked; remove both, then try again` |
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
  for 3 seconds, and taken over when it was left behind; a change leaves `take-over.lock` beside it (the tap's
  lock, [delivery.md](delivery.md), "The lock"). A change
  is tried on the list before the lock is asked for, so a refusal makes no folder and no file, and again
  behind the lock. A making (`project new`) holds the lock from its second look to its line on the list. Reading takes no lock: a reader sees the old file or the new one.
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
- **`project add` compares a name letter for letter, and `project new` sets the case of letters aside.**
  `Alpha` can be added beside a made `alpha`, from another folder; `Alpha` cannot be made beside a listed `alpha`.
- **A making is not named.** A command that waits for the lock is told that a change to the list or a making
  has not finished, not which, and not of which name.
- **A making that takes over 30 seconds** (a git that hangs has 5 minutes) loses its lock to the next command,
  as any change to the list does. Its own folder is still the only one it takes back or lists.
- **A hand, not a command, can still meet a making.** Whoever can write Jellymoon's folder can remove or put a
  folder at the name while the repo is made. The folder is asked for the making's file just before it is
  listed and just before it is taken back; between that look and the act there is no lock a hand obeys, on
  any system. And a hand that copies the making's file into the folder it puts there has made that folder
  the making's own: on Windows nothing else tells the two apart, on Unix the system's number still does.
- **A killed `project new` is not cleaned up** by the next one. It may have left a folder with no repo, a repo
  with no commit, or a whole repo: the sentence names both ways out, and does not say which fits. Its lock is
  in the way of every change to the list for half a minute.
- **A name switched off that is in the path of Jellymoon's own folder switches off every made project.** The
  names of `--skip` and `skip.txt` are matched against a folder's whole path, and a made project's path goes
  through Jellymoon's folder and `projects`: `--skip projects`, or a skip of part of the home's name, and a
  wish for a made project is told `it is switched off`.
- **`uninstall` leaves made projects**, as it leaves all of Jellymoon's folder ([install.md](install.md)).
  Clearing that folder as journal data removes them.
- **A failed `project new` leaves `runs/no-hooks`, and Jellymoon's folder around it,** when git was reached
  and they were not there before, as a run's copy that was refused does. Jellymoon's folder is removed only
  when it is empty; a folder above it that had to be made for it is left; and so is Jellymoon's folder when
  the lock itself could not be taken in it.
- **A made project is inside a folder the Finder does not show** (`.jellymoon`), has no remote, no first
  file and no instructions for an agent (ADR 0020, "Risks left open", 3 and 4).
- **What tells the made folder from another on Windows** is the making's file alone. Windows has no number
  for a folder that stable Rust reads, and it hands a folder made again under a name within a few seconds
  its old time of making (CI on Windows showed it: a folder removed and made again had the mark of the first),
  so the time says nothing there. The folder is not held open against a hand for the length of a making: a
  hand's `remove` goes through, and the making then says the folder was removed, replaced or changed.
- **The making's file is in the folder while git works.** Git is told nothing of it and its commit has no
  file, so the file is never in the repo's history; a killed making leaves it, with the folder.
- **A typed-in folder whose last name holds an unseen character** (an emoji written with U+FE0F, say) is not
  offered, and a new wish for it is refused as no project's name, unless the list holds the name already
  ("A name that comes in, and a name that was kept").
- **A sentence that says "what was made was taken back"** (git failed, the name was kept in other letters)
  and the folder could not be removed: the words that it was left are added after it, and the two stand
  together.
- **`project add` takes the making's file out after the lock is let go**, and only then. A making whose lock
  was taken over after 30 seconds, and whose folder is added meanwhile, finds its file gone and lists
  nothing; the folder is listed by the `add`.
- **Unicode forms.** Two forms of one name are two projects where the file system keeps them apart.
- **A file system that stores another spelling** of the name is refused after the folder was made and taken
  back. That sentence has no test, and a change that never compares the two names passes every test: no disk
  at hand keeps a folder under other letters than it was given.
- **`project new` was run on a Mac, and by CI on Linux and Windows.** The device names, the characters Windows
  takes in no name and the cut dots and spaces are refused on every system; that Windows refuses them was read,
  not seen.
- A folder dropped on the window is F64 and F66. The page of a day nothing was typed on is built (F63):
  [page.md](page.md) and [app.md](app.md) say what it holds.

## Tests

`crates/jellymoon/tests/f62.rs` and `f62/`: `add`, `list` and `forget`; every refusal with the home's tree
compared; a stray folder or credential after each command, never said back; a run refused at its copy, which
lists nothing; a wish said aloud with two projects on offer; eight changes at once and a lock that stays; the list a run's start writes; a listed project found by
`run`, `wish --run` and `hear`; a folder that is gone, no repo's top, a link, switched off or someone else's; a
name typed in and listed; and lists written by hand. `crates/jellymoon/tests/f65.rs` and `f65/`: what `new`
makes, with the person's git config obeyed in nothing but who they are (a template, hooks, a first branch,
signing, an object format, a ref format, a shared repo); every name that is refused, a name that is taken by
the list or by what is there, links, no git, each with the whole test folder compared; a git step that fails
and a list written over meanwhile, each taken back; a lock that stays, with nothing made; a command killed
half way, its lock, and both ways to mend it; six at once; `f65/alone.rs`: a folder put in place of the one
being made, left and never listed, and one removed; a line a hand wrote for the folder being made; a list
that cannot be written; a home made for the lock; `f65/mark.rs`: the making's file, its mode, a making whose
file a hand removed, a cut-off making's folder added, and a folder that could not be taken back;
`f62/earlier.rs`: a list and a day's wishes an earlier build kept under a name refused today; a second `new`, an `add` and a `forget` that meet a making; the looks taken
again behind the lock; a home that is nobody's or whose path is not text; a branch that is there before the
first commit; a wish, its run and its keep in a made project; and a made project forgotten. The examples round-trip in core's `tests/shapes.rs` and
`project.rs`.
