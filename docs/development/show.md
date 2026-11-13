# A look at a run's work: `jellymoon run show`

What a finished run made, read from the repo's store and printed as data (F44 and F48; ADR 0017, decisions 1, 2, 3
and 6). The rules are in `crates/jellymoon-core/src/run/show.rs` (which run, what is refused, what is printed) and
`crates/jellymoon-core/src/copy/work.rs` (every question asked of git). The tests are
`crates/jellymoon/tests/f44.rs` and, for what the record names, `crates/jellymoon/tests/f48.rs`.

```
jellymoon run show --wish <id> [--date <YYYY-MM-DD>] [--repo <folder>] [--logs <dir>] [--skip <names>]
jellymoon run show --wish <id> ... --file <n>
```

Each prints one line of JSON and exits 0, or one line on stderr and exits 1. It writes nothing: not Jellymoon's
home, not the repo, and no file of the work anywhere. It starts no program but `git`.

## What it prints

The list (`Shown`, example `crates/jellymoon-core/tests/fixtures/shown.json`):

| Field | What it is |
|---|---|
| `run`, `wish` | the run's id and the made wish's id |
| `work` | `waits`, `kept` or `tossed`: the record's `work`, and `waits` when the record does not say |
| `files` | the first `most` files of the work, in git's own order of paths |
| `more` | how many files there are after those |
| `most` | 200 |

Each file: `n` (its number, from 1), `name` (the path, cleaned, on one line, at most 200 characters), `change`
(`made`, `changed`, `removed`), `size` in bytes, `kind` and `obeyed`.

| `kind` | It is | `size` is |
|---|---|---|
| `text` | a file whose first 64 KiB are whole characters with no zero byte | the file's |
| `other` | any other file | the file's |
| `link` | a symbolic link (mode 120000) | the length of where it leads |
| `repo` | another repo's entry, a submodule (mode 160000) | 0: nothing of it is in this repo |

A removed file is described as it was. A file whose mode alone changed is `changed`. A rename is a file removed
and a file made: nothing guesses that two names are one file.

Work that is `kept` or `tossed`, and a run that finished and changed nothing, print no files, and git is not asked.

One file (`ShownFile`, example `shown-file.json`): `run`, `wish`, `work`, `n`, `name`, `size`, `text`, `cut` and
`most` (65536). `text` is the file as it is in the work, whole, and never a diff. Of a file over 64 KiB only the
first 64 KiB are read, `cut` is true, and the text ends before the word the cut fell in: that word can be the head
of a key whose rest was never read.

## Which run, and which repo

The wish's run is read as a stop reads it (`run.md`, "A stop the person asks for"): the day's made wishes, then
`runs/<id>/run.json`. A finished run is believed only as `run/end.rs` believes one: its record is for this wish and
day, its words are core's, and its copy is gone. Whether it made a commit is read from those words.

The repo is found as a run finds it (`run.md`, "The order"): `--repo`, or the one folder of the wish's project
typed in on the day, from `--logs`. It must be the top of a checkout. Of a folder read from a log nothing git says
is repeated; here nothing git says is repeated of any folder.

## What the record names, and how far it is trusted

The record names the run's repo, the commit the run started from and the commit it ended as (F48; `run.md`, "What
is kept"). The record is a file, and whoever can write the home can write it, so none of the three is trusted:

- **Checked first, before git is asked anything** (`named` in `run/show.rs`). A record names all three or none.
  `start` and `end` must each be a whole object id, 40 or 64 hex digits and nothing else, and `repo` a path from a
  root. Anything else (a branch's name, `HEAD~1`, a short id, an option, two of the three) is refused as a record
  Jellymoon did not write.
- **The repo's path chooses nothing, and nothing is looked up at it.** The repo is found as above, whatever the
  record says. Git then names the git folder that repo's checkouts share, and that folder must be the record's.
  One folder has many spellings, on a Mac by a link or by case, on Windows by case, a short name, either
  separator or a `\\?\` prefix, and the system gives one resolved path for all of them (`fs::canonicalize`, every
  link and `..` followed). A run writes that resolved path in its record. A look resolves the folder it found the
  same way and compares the two part by part, with no letter's case folded. The record's own path is never
  resolved, opened or asked about: it is a file's word, and could name a place on another machine. So a record
  that spells the same folder another way is refused, as naming another repo.
- **No id from the record is given to git.** The branch is asked for by its name, and git prints the commit it
  names; that id must be the record's `end`. Git is asked for that commit by the id git printed, and its one
  parent must be the record's `start`. The ids `diff-tree` is given are those two, which git printed.

So the work shown is the change from the recorded start to the recorded end, in the recorded repo, and only while
the run's branch still names that end.

**A record from before F48** names none of the three, and is shown by the shape of its branch alone, as F44 built
it: the tip must be a run's commit (below), and the work is its change from its one parent.

## What is asked of git

All through `copy/git.rs`, so with every program of the repo's config switched off, nothing fetched, and git never
finding a repo by the folder it is in. Every `cat-file` and the `diff-tree` read an object as it is
(`--no-replace-objects`). A graft changes nothing here: the parent is read from the commit's own bytes, and the two
commits are compared as trees, with no history walked.

| Asked | For | Read up to |
|---|---|---|
| `for-each-ref --format='%(refname) %(objectname) %(objecttype) %(symref)' refs/heads/jellymoon/run-<id>` | the branch, by its whole name: only the line of exactly that name is taken | every ref under that name: not bounded |
| `cat-file commit <id>` | the commit's parents and message | 64 KiB |
| `diff-tree -r --no-renames --no-abbrev -z <parent> <commit>` | the files, as modes, ids and a letter, each path ended by a zero byte | 16 MiB |
| `cat-file --batch-check`, ids on its input | each listed file's size, and that it is a blob | 128 bytes a file |
| `cat-file --batch`, ids on its input | every listed file of at most 64 KiB | their sizes |
| `cat-file blob <id>` | the start of a longer file | 64 KiB |

No file name is ever an argument: git is given ids, which are checked to be 40 or 64 hex digits. A name is bytes
git printed between two zero bytes. So a name that is `-rf`, `--output=x` or `HEAD:a.txt`, or holds a line break,
is a name. Reading stops at the size given: the pipe is closed there and git ends on its next write
(`call::run_most`). Each git has the copy's time limit (`copy.md`, "Limits").

**The work is the tip's change from its one parent.** The tip must be a commit with exactly one parent and the
message `Jellymoon run <id>`, which is what a copy ends as (`copy/clean.rs`). A commit put on the branch afterwards,
a merge, a commit with no parent and a branch made a name for another branch are refused: what is there is not
shown as the run's. For a record that names its commits the tip must also be that end and its parent that start,
so a commit of the right shape made by hand is refused too. For a record from before, the shape is all that can
be checked (see "What it does not do").

## Cleaning

- **A name** passes `still::statement`, the door a clash's statement passes: one line, no control or invisible
  character, credential shapes replaced, `"` made `'`, cut at 200 characters.
- **A text** passes `still::still_text`: the reader's cleaning of a turn (`redact::shown`), which keeps lines and
  quotes, made again until a making changes nothing. Eight makings may change it; a text a ninth changes is
  refused, never printed half cleaned.
- **`obeyed`** is asked of the name as git keeps it, read as the disks read it (below), and of the cleaned name
  too. Cleaning takes out most characters nobody sees but keeps the joiners U+200C and U+200D, which HFS+ leaves
  out of a name: those two are dropped by the rule itself. The name as git keeps it is asked whole, so a name
  the tools obey past the 200 characters that are printed is marked.

## Files the person's tools obey

One list and one rule, in `crates/jellymoon-core/src/run/obeyed.rs`. A path is obeyed when any of its parts is one
of: `.claude`, `.codex`, `.cursor`, `.cursorrules`, `.vscode`, `.idea`, `.husky`, `.pre-commit-config.yaml`,
`.devcontainer`, `.cargo`, `CLAUDE.md`, `AGENTS.md`, `GEMINI.md`, `.mcp.json`, `.envrc`, `.gitmodules`,
`.gitattributes`; or when `.github` is followed by `workflows` or by `copilot-instructions.md`. It is a warning,
not a wall: a tool the list does not know obeys its file all the same, and kept work is code in the person's
project either way.

**A part is read as the disks read it before it is compared.** A run's agent needs only to write a file to make
a name, and a disk opens more spellings as `.claude` than a comparison of letters finds. The rule is generous: a
spelling marked that no disk folds costs the person one more yes, and one missed is the fault.

| Step | What is done | Whose fold it is |
|---|---|---|
| 1 | a path is parted at `/` and at `\` | Windows |
| 2 | a part is cut at its first `:` | Windows: after it is a stream of the same file |
| 3 | U+200C to U+200F, U+202A to U+202E, U+206A to U+206F and U+FEFF are dropped | HFS+ |
| 4 | `ſ` U+017F is `s`, `K` U+212A is `k`, `ß` U+00DF and `ẞ` U+1E9E are `ss`, U+FB00 to U+FB06 are `ff`, `fi`, `fl`, `ffi`, `ffl`, `st`, `st` | APFS |
| 4 | `ı` U+0131 is `i` | NTFS, whose table of capitals makes it `I` |
| 5 | dots and spaces at the end of a part are dropped | Windows |
| 6 | the part is compared in any case of ASCII letters | all three |

And a part that holds `~` and then a digit is marked whatever else it holds: it has the shape of a short name
(`CLAUDE~1`, `GITATT~1`) that a Windows disk gives a file beside its own, and which name it is short for cannot
be read from it.

Not obeyed, because no disk measured or reasoned about takes one for the other: a longer name (`.claudex/`,
`xAGENTS.md`, `.cargo.toml`), a letter of another script that looks the same (Cyrillic `с` in `.сlaude`, a
fullwidth `ｃ`), a letter with an accent, a dot or space that is not at the end of a part, and the second name
of a pair without the first (`copilot-instructions.md` outside `.github`).

### What was measured

On a Mac (macOS 26, arm64), with a script that made files and asked the disk, in a fresh folder: once on the
system's APFS volume (not case-sensitive) and once on a 100 MB HFS+ image (not case-sensitive) mounted for the
purpose. Asked of every code point from U+0001 to U+FFFF but `/` and the surrogates, and of U+E0000 to U+E01EF:
63,982 in all.

- **One character for one ASCII character**: the file named by the character was made, then each of `a` to `z`,
  `0` to `9`, `-` and `_` was asked for (and `.` between two letters). APFS: U+017F for `s` and U+212A for `k`,
  and no other. HFS+: none.
- **One character for two or three**: every name of two and of three of those 39 characters was made, then each
  character was asked for, and each one found was asked again the first way. APFS: U+00DF and U+1E9E for `ss`,
  U+FB00 `ff`, U+FB01 `fi`, U+FB02 `fl`, U+FB03 `ffi`, U+FB04 `ffl`, U+FB05 and U+FB06 `st`. HFS+: none.
- **A character left out**: `x<c>y`, `<c>xy` and `xy<c>` were made, and `xy` asked for. APFS: none. HFS+: the
  sixteen of step 3, in all three places.
- **Ends**: neither disk takes `xy.`, `xy `, a `\` or a `:` for anything but itself.
- APFS refused to make a name of 1,529 of the code points (the first is U+0378), so they were not asked; a name
  the disk will not make cannot be opened as another. HFS+ refused none.

`run/obeyed/tests.rs` asks the disk again on every `cargo test` on a Mac, in the system's temporary folder:
every obeyed name in every spelling the table knows (where the disk takes the spelling for the name, the rule
must say obeyed), and every character above against every name of one and two ASCII characters and against
`xy` (the disk may take none the table does not know). On a disk that folds nothing they assert nothing.

### What is not closed

- **Windows was not run.** Steps 1, 2 and 5, `ı`, and the short names are reasoned from how Win32 and NTFS are
  written up. Other characters NTFS's table of capitals may take for an ASCII letter were not looked for.
- **A short name is marked by its shape, not found.** `~` and a digit anywhere in a part is marked, so
  `notes~1.txt` is too; a short name made of a hash, such as `CL1A2B~1`, is marked the same way.
- **HFS+ was measured on one image**, and the test run on every `cargo test` asks only the disk the temporary
  folder is on. Not measured: a case-sensitive volume of either kind, a name of four or more characters taken
  for one, a character above U+FFFF other than U+E0000 to U+E01EF, ext4 with case folding, exFAT, SMB and NFS
  mounts.
- **A name that is not UTF-8** is asked with U+FFFD where its bad bytes are.
- **The list is closed.** A tool it does not name is not marked.

## Every refusal

| It finds | It prints (exit 1) |
|---|---|
| no made wish of that id on the day | `the page of <day> has no made wish of that id; ...` |
| a wish with no run | `wish <id> has had no run, so there is no work to show` |
| no record that can be read | `the record of run <run> cannot be read, so its work is not shown` |
| a run that is another wish's | `wish <id> names run <run>, which is another wish's run: its work is not shown` |
| a run that is going | `run <run> is going: its work is shown once it has finished` |
| a run cut off, not yet written down | `run <run> was cut off before it ended: there is no work of it to show` |
| a record that is not core's for this wish, or a finished run whose copy is still there | `the record of run <run> is not one Jellymoon wrote for this wish` |
| a cancelled run | `run <run> was cancelled by a stop: there is no work of it to show` |
| a run that failed or was never started | `run <run> did not finish: there is no work of it to show` |
| no one folder for the wish | the refusals of `run.md`, "What is refused", for the folder |
| a folder that is not the top of a checkout, a git that failed or ran out of time, an object that is missing | `the work of run <run> could not be read from the repo: ...` |
| no branch of the run's name | `the branch of run <run> is not in the repo, so there is no work of it to show` |
| the branch is a name for another | `the branch of run <run> is a name for another branch by now, ...` |
| a finished run's record that names a commit by anything but a whole id, a repo by a path not from a root, or only some of the three | `the record of run <run> is not one Jellymoon wrote for this wish` |
| a repo that is not the one the record names | `the repo found for run <run> is not the one its record names, so its work is not shown` |
| a tip that is not a run's commit, or not the end the record names | `the branch of run <run> is not as the run left it, ...` |
| a tip whose one parent is not the start the record names | `the work of run <run> does not start where its record says the run started, so it is not shown as the run's` |
| a list of files over 16 MiB | `the work of run <run> is more files than Jellymoon lists` |
| `--file` with no such number, or one past the 200 listed | `file <n> of run <run> names no file: ...` |
| `--file` of a removed file | `file <n> of run <run> was removed by the work, ...` |
| `--file` of a link | `file <n> of run <run> is a link, and where it leads is not shown` |
| `--file` of another repo's entry | `file <n> of run <run> is another repo's entry, ...` |
| `--file` of what is not text | `file <n> of run <run> is not text, so it is not shown` |
| `--file` of a text cleaning goes on changing | `file <n> of run <run> cannot be cleaned, so it is not shown` |

`--file` with anything but digits is a usage error, exit 2. No refusal holds a file's name, a byte of a file, a
word of the agent's or a word of git's, and neither a refusal nor an answer holds the repo's path or a commit's id.

## What it does not do

- **It does not know the commits of a run from before F48.** Such a record names none, and a commit with the run's
  message and one parent, put on the run's branch by anyone who can write the repo, is shown as that run's work.
  Whoever can write the home can make any record one from before, by taking the three names out. So the record
  guards against a change to the repo, not against a change to the home: F45 and F46, which act on the work,
  should refuse a record that names no commits and not fall back to the shape.
- **It does not follow a repo that was moved.** The record names the git folder where it was when the run was
  made. A repo moved or copied elsewhere since is another repo, and its work is not shown. The same holds when
  the system resolves the folder to another path than it did then (a volume mounted under another name or
  letter): it is refused, never guessed.
- **It does not show a commit no record names.** A run killed after its commit and before its end was written
  leaves that commit on its branch (`run.md`, "What a kill leaves in the record"). It is not shown.
- **It does not look past 200 files or 64 KiB of one.** There is no way to ask for the rest.
- **It writes no `work` to the record.** Nothing does yet: `kept` and `tossed` are F45's and F46's to write.
- **It does not bury a run that was cut off.** A look is not a page: it refuses, and writes nothing down.
- **Measured on macOS only**, with git 2.50; the test lists work of ten thousand files and allows it a minute. Not
  measured: Windows, Linux, a git older than 2.31, a repo of the SHA-256 format, a partial clone.
