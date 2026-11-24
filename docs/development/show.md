# A look at a run's work: `jellymoon run show`

What a finished run made, read from the repo's store and printed as data (F44 and F48; ADR 0017, decisions 1, 2, 3
and 6). The rules are in `crates/jellymoon-core/src/run/show.rs` (which run, what is refused, what is printed) and
`crates/jellymoon-core/src/copy/work.rs` (every question asked of git). The tests are
`crates/jellymoon/tests/f44.rs` and, for what the record names, `crates/jellymoon/tests/f48.rs`.

```
jellymoon run show --wish <id> [--date <YYYY-MM-DD>] [--repo <folder>] [--logs <dir>] [--skip <names>]
jellymoon run show --wish <id> ... --file <n>
```

Each prints one line of JSON and exits 0, or one line on stderr and exits 1. It writes nothing in the repo, no
file of the work anywhere, and nothing of the day or of a run. It starts no program but `git`.

**What a look makes in the home.** One thing, and only when git is asked: the folder

```
runs/no-hooks
```

when it is not there. It is the empty folder every git of Jellymoon's is started in and told to look for hooks in
(`copy.md`, "What is switched off"), a run makes it too, and a look that finds it gone makes it again, empty.
Nothing else in the home is made, changed or removed by a look.

## What it prints

The list (`Shown`, example `crates/jellymoon-core/tests/fixtures/shown.json`):

| Field | What it is |
|---|---|
| `run`, `wish` | the run's id and the made wish's id |
| `work` | `waits`, `kept` or `tossed`: the record's `work`, and `waits` when the record does not say |
| `files` | the first `most` files of the work, in git's own order of paths |
| `more` | how many files there are after those |
| `most` | 200 |
| `result` | what the run said of its work, checked ("What the run says of its work"); left out when there is none |

Each file: `n` (its number, from 1), `name` (the path, cleaned, on one line, at most 200 characters), `change`
(`made`, `changed`, `removed`), `size` in bytes, `kind` and `obeyed`.

| `kind` | It is | `size` is |
|---|---|---|
| `text` | a file whose first 64 KiB are whole characters with no zero byte | the file's |
| `other` | any other file | the file's |
| `link` | a symbolic link (mode 120000) | the length of where it leads |
| `repo` | another repo's entry, a submodule (mode 160000) | 0: nothing of it is in this repo |

A removed file is described as it was. A file whose mode alone changed is `changed`. A rename is a file removed
and a file made: nothing guesses that two names are one file. Git's plumbing finds a rename only when it is told
to, and it is not told; `diff.renames` in the repo's config or the person's changes nothing
(`f44/shapes.rs`).

**What a printed name can hold.** A name is cleaned (below), so two files can print the same name, and only `n`
tells two files apart:

- A name that is only spaces, or only characters cleaning takes out (U+200B, say), prints as `""`. `README.md`
  with a U+200B after it prints exactly as `README.md`.
- A name over 4,096 bytes is not cleaned at all and prints as `[redacted]`, as every such name does.
- The joiners U+200C and U+200D are kept in a printed name: emoji and several scripts are written with them. A
  name can hold them and show nothing of it.
- A text keeps those two and also U+2028, U+2029, U+034F, U+FE0F and U+3164, which are not format or control
  characters to the cleaning and draw as nothing or as a line's end.

Work that is `kept` or `tossed`, and a run that finished and changed nothing, print no files, and git is not asked.
A run that finished and changed nothing still prints what it says it did as `result` ("What the run says of its
work", below).

One file (`ShownFile`, example `shown-file.json`): `run`, `wish`, `work`, `n`, `name`, `size`, `text`, `cut` and
`most` (65536). `text` is the file as it is in the work, whole, and never a diff. Of a file over 64 KiB only the
first 64 KiB are read, `cut` is true, and the text ends before the word the cut fell in: that word can be the head
of a key whose rest was never read. With no whole word before the cut (no space or line break in what was read,
as in a minified file) the file is refused in fixed words, not printed as an empty text.

**`cut` is true whenever the file was not read whole or the cleaned text was cut at `most`.** Cleaning
shortens a text at its start and at its end, by the spaces and line breaks there, and by characters it takes out
(a zero-width space, for one): none of that is `cut`. `most` counts the bytes that are read of a file, and it
is no count of what is printed. Cleaning can make a text longer: a token gains an ellipsis and a password in a
link becomes `[redacted]`. A text is kept to `most` characters, so a file of 64 KiB or less that cleaning makes
longer than that is cut there, ends in `…`, and says `cut`. A character can be several bytes, so `text` can be
more than `most` bytes long.

## Which run, and which repo

The wish's run is read as a stop reads it (`run.md`, "A stop the person asks for"): the day's made wishes, then
`runs/<id>/run.json`. A finished run is believed only as `run/end.rs` believes one: its record is for this wish and
day, its words are core's, and its copy is gone. Whether it made a commit is read from those words.

The repo is found as a run finds it (`run.md`, "The order"): `--repo`, or the one folder of the wish's project
typed in on the day, from `--logs`, or else the folder listed under the project's name, which the run's start
wrote down (`project.md`). So a run's work is shown, kept and tossed on a later day with no `--repo`. It must be the top of a checkout. Of a folder read from a log nothing git says
is repeated; here nothing git says is repeated of any folder.

**A follow-up's work** ([run.md](run.md), "A follow-up's run") is shown as any run's: its record's `start` is
the commit of the work it follows while that work waited, so the list and each file's `change` are what the
follow-up adds to that work, and a file only the followed work touched is not listed. A file's `text` is the
file whole, as always, with the followed work in it. Nothing is asked of the followed run.

## What the record names, and how far it is trusted

The record names the run's repo, the commit the run started from and the commit it ended as (F48; `run.md`, "What
is kept"). The record is a file, and whoever can write the home can write it, so none of the three is trusted:

- **Checked first, before git is asked anything and before anything of the run is printed** (`Record::named` in
  `run/record.rs`, the one place the three are read), for every finished run: one whose work waits, one that
  changed nothing, and one whose work is `kept` or `tossed`. A finished run's record names all three or none.
  `start` and `end` must each be a whole object id, 40 or 64 hex digits and nothing else, and `repo` a path from a
  root. Anything else (a branch's name, `HEAD~1`, a short id, an option, one or two of the three) is refused as a
  record Jellymoon did not write.
- **The record's words and its names must agree.** A run whose `why` says its work is one commit ended as
  another commit than it started from, and a run whose `why` says it changed nothing ended as its start. A
  record with `one commit` and `end` equal to `start`, or with `changed nothing` and two commits, is refused the
  same way, at the same place.
- **The repo's path chooses nothing, and nothing is looked up at it.** The repo is found as above, whatever the
  record says. Git then names the git folder that repo's checkouts share, and that folder must be the record's.
  One folder has many spellings, on a Mac by a link or by case, on Windows by case, a short name, either
  separator or a `\\?\` prefix, and the system gives one resolved path for all of them (`fs::canonicalize`, every
  link and `..` followed). A run writes that resolved path in its record. A look resolves the folder it found the
  same way and compares the two part by part, with no letter's case folded. The record's own path is never
  resolved, opened or asked about: it is a file's word, and could name a place on another machine. The two are
  compared as paths, so a record that spells the same folder with a separator at its end, a `.` part or a
  doubled separator compares equal. One that spells it in another case, through a link or with `..`, or that
  names where the repo was before it was moved, does not, and is refused as naming another repo.
- **No id from the record is given to git.** The branch is asked for by its name, and git prints the commit it
  names; that id must be the record's `end`. Git is asked for that commit by the id git printed, and its one
  parent must be the record's `start`. The ids `diff-tree` is given are those two, which git printed.

So the work shown is the change from the recorded start to the recorded end, in the recorded repo, and only while
the run's branch still names that end.

**A record from before F48** names none of the three, and is shown by the shape of its branch alone, as F44 built
it: the tip must be a run's commit (below), and the work is its change from its one parent.

## What the run says of its work

A run may end its last word with a `result` beside `done` ([run.md](run.md), "A run that asks"). It is a model's
text. It is kept as it was read, and nothing of it is printed until it has been checked against the work, at
every look (F85; ADR 0023; `crates/jellymoon-core/src/run/told.rs`; tests `crates/jellymoon/tests/f85.rs`).
The list of work that waits prints what passed (example `crates/jellymoon-core/tests/fixtures/shown-told.json`):

```json
"result":{"pieces":[{"type":"did","text":"Shipping is now free from 50, not 100. The test tries 49 and 50."},{"type":"change","n":1,"name":"src/shipping.rs","made":false,"before":["if total >= 100 { 0 } else { 8 }"],"after":["if total >= 50 { 0 } else { 8 }"]},{"type":"decided","text":"An order of exactly 50 ships free."},{"type":"unsure","text":"I could not run the test."}],"dropped":1}
```

**Where it is read from.** `runs/<id>/result.json` in the home, as a plain file of at most 8,192 bytes
(`tap::plain`, so a link, a pipe or a folder at the name, and on Unix a file its group or others can write, is
no result). Its shape is
`{"pieces":[...]}`, each piece one of

```json
{"type":"did","text":"..."}
{"type":"change","file":"<path>","before":["..."],"after":["..."]}
{"type":"decided","text":"..."}
{"type":"unsure","text":"..."}
```

with `before` and `after` each optional. A file that is longer, that is not this shape, that names another type
or one part more, is no result, and the list prints as it did before F85. Only the first twelve pieces are read.
A file written by hand is checked exactly as one a run left: the file is not believed, the work is.

**When there is a `result`.** Only in the list (`run show` without `--file`, and `run waiting`), only of work
that waits or of a run that changed nothing (below), and only when a piece passed or one was dropped. Work that was kept or tossed, and `--file <n>`,
print what they printed before.

**Told alone: a run that changed nothing.** A run that finished and changed nothing (its record's line says so,
and it was not tossed) has no work to wait, so `run waiting` lists none of it, but `run show` prints its
`result` beside `"files":[]`. There is no file to check a `change` against, so every `change` is dropped and
counted; `did`, `decided` and `unsure` pass the door and their bounds as they do with work. No repo is asked.
The app reads it through `jellymoon_core::ended_alone`, which says how a made wish's run ended when it left no
work: `Unchanged`, with that `result`, or `Unfinished`, a run whose record says it failed. Core keeps no result
of a run that failed, so none is read of it, though a file be at the name (`app/tests/told_alone.rs`,
`crates/jellymoon/tests/f85/kept.rs`).

**The order and the counts.** Pieces are printed in the catalogue's order, whatever order the run wrote them
in: the first `did`, the first two `change`, the first three `decided`, the first three `unsure`. A piece past
its count is left out and is not counted as dropped.

**A text** (`did`, `decided`, `unsure`). It passes the door a question passes (`still.rs`) as the run wrote it,
its line breaks with it: it is made one line, each line break and run of spaces one space, a credential's shape
is printed as `[redacted]`, one that a line break splits in two as well, and a `"` as a `'`. The piece is
dropped, never cut, when the text

- is over its length, counted in characters after the door: 280 for `did`, 160 for `decided` and `unsure`;
- holds a character that draws as nothing or as a blank (the list under `plain`, below, in "Work that waits");
- is empty, or is only marks once it is cleaned;
- reads like a note's count, a digit and `more` (`3 more`, `+5 more`).

**A `change`.** It is printed as `n` and `name`, the number and the name the list itself gives the file, with
`made` (whether the work made the file) and the quoted lines. Nothing the run wrote names the file in what is
printed. The piece is dropped unless all of this holds:

- `file` is, byte for byte, the path of one of the listed files, and that file is `text`;
- the file's printed name is the path as git keeps it (`plain`): a name cleaning changed is not quoted under;
- the file is at most 64 KiB after the work, and for a changed file at most 64 KiB before it too;
- it quotes one line at least, and at most three `before` and three `after`;
- every quoted line, with the spaces and tabs at its two ends taken off, is not empty, is at most 100
  characters, holds no tab, no line break and no character that draws as nothing, does not read like a note's
  count, and is a line the cleaning of a file's text leaves exactly as it is (`still_text`): a line with a
  credential's shape in it is never printed, with the mark or without;
- every quoted line is also a whole line of its side of the file cleaned whole, as `--file <n>` cleans the
  file: a shape that spans lines, a private key block from its first line to its last, is the mark there, so no
  line of it is quoted, however short the line and whether the work made the file, changed it or removed it;
- every `before` line is a whole line of the file, ends taken off, that the file has more often before the
  work than after it; every `after` line is one it has more often after. A line that is there as often as it
  was is neither, and a part of a line is not a line. So a made file passes no `before` line and a removed
  file no `after` line.

One line that fails drops the whole piece. A quoted line is printed with its ends taken off, as the line is in
the file otherwise.

**`dropped`** is how many pieces of those read failed their check. Nothing says which, or why.

**At every look.** The check is made against the work as git gives it at that look, not when the result was
kept. A run's branch that names another commit than the record does is refused before anything is read, as
before; a result is checked against the files of the commits the record names.

**What holds a model's text.** These are the only places a run's own words reach what is printed, and each is
held as said above: the `text` of a `did`, `decided` or `unsure` (the door, the length, the checks), and the
`before` and `after` lines of a `change` (a whole line of the work's own file, unchanged by the cleaning).
`n`, `name`, `made`, `type` and `dropped` are Jellymoon's. Nothing of a result is in `run`'s own line, in a
record, in a wish, in a refusal or in `app.log`.

## What is asked of git

All through `copy/git.rs`, so with every program of the repo's config switched off, nothing fetched, and git never
finding a repo by the folder it is in. Every `cat-file` and the `diff-tree` read an object as it is
(`--no-replace-objects`). A graft changes nothing here: the parent is read from the commit's own bytes, and the two
commits are compared as trees, with no history walked.

| Asked | For | Read up to |
|---|---|---|
| `for-each-ref --format='%(refname) %(objectname) %(objecttype) %(symref)' refs/heads/jellymoon/run-<id>` | the branch, by its whole name: only the line of exactly that name is taken | 16 KiB; an answer of that size or more is no answer, and is refused as a repo that could not be read |
| `cat-file commit <id>` | the commit's parents and message | 64 KiB; a commit of that size or more is not a run's, whatever its first 64 KiB say |
| `diff-tree -r --no-abbrev -z <parent> <commit>` | the files, as modes, ids and a letter, each path ended by a zero byte | 16 MiB; a list of that size or more is refused |
| `cat-file --batch-check`, ids on its input | each listed file's size, and that it is a blob | 128 bytes a file |
| `cat-file --batch`, ids on its input | for the list, every listed file of at most 64 KiB; for `--file <n>`, file `n` when it is that short | their sizes |
| `cat-file blob <id>` | the start of a longer file: of each, for the list; of file `n` alone, for `--file <n>` | 64 KiB |
| `cat-file blob <id of the file as it was>` | for the list of work that waits, when a result is kept: a changed file as it was before the work, for the first two files the result's changes name that are listed and were files before | 64 KiB and one byte: a file that long is not quoted from |

The last is asked with the id `diff-tree` printed for the file's old side. A path or a line a result holds is
compared with what git printed and is never an argument to git (`f85.rs` plays a git that would be caught by one).

The list says of every file whether it is text, so it reads the start of every file. `--file <n>` reads the
sizes of all and the bytes of file `n` only: a file git cannot be made to give keeps the list from being shown,
and no other file.

No file name is ever an argument: git is given ids, which are checked to be 40 or 64 hex digits. A name is bytes
git printed between two zero bytes. So a name that is `-rf`, `--output=x` or `HEAD:a.txt`, or holds a line break,
is a name. Reading stops at the size given: the pipe is closed there and git ends on its next write
(`call::run_most`). What git says beside its answer (stderr) is not read that way: it is read to its end, so git
is never ended for what it says, and no more of it is held than the size given, or 128 KiB when that is less;
none of it is used. A repo whose config makes git warn at every command is shown as any other
(`f44/bounds.rs`). Each git has the copy's time limit (`copy.md`, "Limits").

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
  refused, never printed half cleaned. A making that leaves more than 65,536 characters is cut there, and the
  answer's `cut` says so.
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
| a run that was cut off, before a look has written it down and after | `run <run> was cut off before it ended: there is no work of it to show` |
| a record that is not core's for this wish, or a finished run whose copy is still there | `the record of run <run> is not one Jellymoon wrote for this wish` |
| a cancelled run | `run <run> was cancelled by a stop: there is no work of it to show` |
| a run that failed or was never started | `run <run> did not finish: there is no work of it to show` |
| a run that asks ([run.md](run.md), "A run that asks") | `run <run> asks a question: there is no work of it to show` |
| no one folder for the wish | the refusals of `run.md`, "What is refused", for the folder |
| no `git` on `PATH`, or something in the folder of the home git is started in | `the work of run <run> could not be read: no `git` program was found on PATH, or the folder Jellymoon keeps empty for git (runs/no-hooks) is not an empty folder` |
| a folder that is not the top of a checkout, a git that failed, ran out of time or answered about the branch at more length than is read, an object that is missing | `the work of run <run> could not be read from the repo: ...` |
| no branch of the run's name | `the branch of run <run> is not in the repo, so there is no work of it to show` |
| the branch is a name for another | `the branch of run <run> is a name for another branch by now, ...` |
| a finished run's record that names a commit by anything but a whole id, a repo by a path not from a root, only some of the three, or a start and an end that do not fit its words | `the record of run <run> is not one Jellymoon wrote for this wish` |
| a repo that is not the one the record names | `the repo found for run <run> is not the one its record names, so its work is not shown` |
| a tip that is not a run's commit, is a commit longer than is read, or is not the end the record names | `the branch of run <run> is not as the run left it, ...` |
| a tip whose one parent is not the start the record names | `the work of run <run> does not start where its record says the run started, so it is not shown as the run's` |
| a list of files over 16 MiB | `the work of run <run> is more files than Jellymoon lists` |
| `--file` with no such number, or one past the 200 listed | `file <n> of run <run> names no file: ...` |
| `--file` of a removed file | `file <n> of run <run> was removed by the work, ...` |
| `--file` of a link | `file <n> of run <run> is a link, and where it leads is not shown` |
| `--file` of another repo's entry | `file <n> of run <run> is another repo's entry, ...` |
| `--file` of what is not text | `file <n> of run <run> is not text, so it is not shown` |
| `--file` of a text cleaning goes on changing | `file <n> of run <run> cannot be cleaned, so it is not shown` |
| `--file` of a text over 64 KiB with no space or line break in what is read | `file <n> of run <run> has no whole word in what is read of it, so it is not shown` |

`--file` with anything but digits is a usage error, exit 2. No refusal holds a file's name, a byte of a file, a
word of the agent's or a word of git's, and neither a refusal nor an answer holds the repo's path or a commit's id.

## Work that waits: `jellymoon run waiting`

A page asks which made wishes have work for the person to look at, and what to say of each (F47,
`crates/jellymoon-core/src/run/waits.rs`). `jellymoon run waiting [--date] [--repo] [--logs] [--skip]` prints it,
and writes nothing:

```json
{"day":"2026-03-10","waits":[{"wish":"m1","run":"r0123456789abcdef","said":"make the uploader retry twice","read":true,"made":1,"changed":1,"removed":1,"names":["CLAUDE.md","a.txt","b.txt"],"more":0,"obeyed":["CLAUDE.md"],"plain":true,"why":"the work of run r0123456789abcdef waits: files made 1, changed 1, removed 1"},{"wish":"m3","run":"rfedcba9876543210","said":"tidy the notes","read":false,"made":0,"changed":0,"removed":0,"names":[],"more":0,"obeyed":[],"plain":true,"why":"no folder of the name the wish gives was typed in on 2026-03-10; name the folder with --repo"}],"follows":["m1","m2"]}
```

- **`follows`.** The ids of the day's made wishes a follow-up may be made of now, in the day's order (F54,
  `run/follow.rs`, `followable`): the wishes `jellymoon wish --after` would not refuse for the wish they name
  ([wish.md](wish.md), "A follow-up") that have work: work that waits and work that was kept. A wish whose run
  changed nothing may be followed too, and is not listed here. It is read from the
  home alone, whatever `--repo` and `--logs` say, and the app offers `More like this` under these wishes and
  no other ([app.md](app.md#a-question-answered-and-more-like-this-appsrcansweringrs)). The day's eight are
  not counted here: a ninth wish is refused when it is made.
- **`result`.** What the run said of its work, checked at this look as `run show` checks it and printed in the
  same shape ("What the run says of its work"). It is left out of a wish whose run said nothing that could be
  read, and of one whose list could not be read.
- **Which wishes.** A made wish of the day whose run finished with its one commit, whose record names that
  commit, and whose work is neither kept nor tossed. That is asked of the home alone. A wish with no run, a run
  that changed nothing, failed, was cancelled or asks, and a record from before F48 are not listed.
- **What is said of each.** The list `run show` gives, counted: files made, changed and removed, the first six
  names as `run show` cleans them, and how many more there are. When the list stopped at 200 files, `why` ends
  ` of its first 200`. `obeyed` is not of the list: it names every file of the whole work the person's tools
  obey, the ones past the 200 too, by the rule a look and a keep judge a file by (`obeyed.rs`,
  `named`). Of the files past the list only the paths are read. `plain` says whether each of those names is one
  a person reads as git keeps it (`obeyed.rs`, `plain`). It is `false` when:
  - cleaning changed a name, as it prints one that looks like a credential as `[redacted]`;
  - a name holds a letter that draws as nothing or as a blank: a code point Unicode calls ignorable by default
    (the joiners U+200C and U+200D, the Hangul fillers U+115F, U+1160, U+3164 and U+FFA0, the variation
    selectors), any space but the plain one, the Braille blank U+2800, U+16FE4, U+1D159, the object
    replacement character U+FFFC, or any code point that is private or not assigned (a system font draws
    `.claude/settings`, U+F860, `.json` as `.claude/settings.json`). "Not assigned" is by the tables of the
    `regex` crate, Unicode 16, so a name with a letter Unicode gave out later has no yes on a page: it fails
    closed. It is the one list core has of what is
    not read in a printed text (`redact.rs`, `holds_unseen`), the same a refused `voice.toml` value is held to;
  - a plain space stands where none is seen: at an end of the name or of a folder's, or after another;
  - a name reads like the line a note counts with, a digit and then `more` (`+5 more`, `and 12 more`).

  The names are printed whole all the same. The list of letters is a list and not a proof: a font can draw a
  letter it does not know as nothing. Not closed:
  - the shape of a count is a decimal digit, then spaces or none, then `more`. A name is plain when its number
    is no decimal digit (`+⑤ more`, `+Ⅴ more`), when it is in words (`+five more`), and when what stands
    between the number and `more` is not a space (`+5·more`);
  - a letter Unicode gave out after version 16 is refused as one that is not assigned, until the `regex`
    crate's tables know it.
- **A repo that is not found** (or any other refusal of `run show`) still lists the wish, with `read: false`, no
  counts, no name, and `why` holding that refusal. A page then offers no choice.
- **The choices and their words** are core's too: `Waits::title`, `lines`, `choices` and `asks`, `ShownFile::title`
  and `lines`, `WorkTap::doing`, and `stands` for a wish with no work waiting. A shell words nothing. What a
  page's notes say first, for a person who has never seen Jellymoon, is `run/waits/plain.rs`: the change is in a
  copy and the folder not changed yet, the files as a person counts them, and after a keep or a toss where the
  change is now, true of every way each ends (a keep that found the work there says nothing was changed). Those
  words are a page's alone and name no run: `why` and the lines `run waiting`, `keep` and `toss` print are as above.
- **A tap on a choice** is `work_tapped`. It does nothing unless the work waits now and was read from its repo
  now, so a toss never begins where the repo is not found: `run toss` there would remove the copy and leave the
  branch. `Show me` gives the next file after the one on show that is text `run show --file` prints, from the
  first again past the last, asking for at most eight (a ninth is never asked for: the tap then says why the
  eighth was not shown); a file the work removed is not asked for.
- **A keep from a page.** The rule: a page keeps a file the person's tools obey only when that file's name was
  drawn, whole, on the note the yes was given under. A count is not a name.
  - `Keep it` of work with such a file, anywhere in the whole work, changes nothing and gives the names back:
    `Waits::asks` gives each name as a text of its own, and counts none.
  - Work with more than six of them (`NAMED`: what a note has room for in the smallest window, a line a name) has
    no yes on a page. Nor has work with one whose name is not read as it is (`plain` is `false`): the word
    drawn would not be the file's name, two files can be printed as one word, a blank is no name to say yes to,
    and a name that reads `+5 more` reads as the kit's own count of names it did not draw. `Waits::lines`, `asks` and
    `choices` say so in one sentence and offer `Show me` and `Toss it`; any keep tapped all the same is answered
    with that note. The sentence says only that such work is kept from the command line, with `jellymoon run
    keep`.
  - A tap is made of what was drawn (`WorkTap::of`: the option's words and its note's texts), and the tap on
    `Keep it, with those files` carries that note's texts: the names, and the note's last sentence. Core gives
    the keep those names, and nothing it read at the tap stands in for them: there is one check of them, the
    keep's own, under its lock.
  - The keep is `run keep` with one difference (`keep.rs`, `keep_named`, `judge`): it is given the names the tap
    carried, held as a set, and under the keep's lock the work's files the person's tools obey must be those
    names, all of them and no other; what one new commit would land must be among them. A keep that was not
    asked once more carries none. So a yes to a note that names other files than the work's (one fewer, one
    more, another in one's place) is refused there, and so is work whose merge would bring such a file that no
    note named: whole, in a page's words and not `--with-settings`. One yes to other names is not refused:
    where the person's branch holds the run's commit already, in its history, nothing is landed whatever
    the note named, the keep compares no name, and the run is written down as kept (`run keep` says the same
    there).
  - The line of the work's own note still counts (`a.txt, b.txt, and 4 more`), and is at most 320 characters of
    names: nothing is kept on the word of it. `Toss it` is `run toss`, with its own checks.

- **A keep a tap could not do.** A refused keep is told on a page under "The change was not kept.", with
  what the person can do next and then the command line's sentence, which names the run. Two of core's reasons
  (`copy/keep/why.rs`) are said in the window's own sentences instead, with no run's id and no word of git's
  (`run/waits/plain.rs`, `unkept`):
  - `Clash`, the person's branch and the work changed the same place: "Your folder changed in the same place
    after this change was made, so the two do not fit together." and "Nothing in your folder was changed.";
  - `InWay`, a change or a file of the person's own is where the work would go: "Your folder has changes of your
    own that are not saved to the project yet, where this change would go." and "Nothing in your folder was
    changed. Save or put away your own changes, then tap Keep it again."

  Every other reason is told as before. The reason reaches the page typed: the page's keep (`run/keep/tapped.rs`,
  `keep_named`) gives back core's `Unkept` beside the sentence `run keep` prints, and no sentence is matched.
  `run keep` and `run keep --ask` print what they printed.
- **`Make it again`.** No later `Keep it` keeps a change that no longer fits, so the note of a `Clash`, and of
  no other reason, has one choice, after one more sentence: "Make it again throws this one away and makes the
  wish again on your folder as it is now." The waiting note offers it for one kind of work ("Work that did not
  fit when its run ended", below). A tap on it (`WorkTap::Again`) is
  answered, while the work waits and was read, with the wish's own words and its project (`WorkDone::Again`),
  and nothing is changed by core there. The shell makes the wish as it makes a typed one, only then asks for
  the toss, as a tap on `Toss it` does, and starts the new wish's run last ([app.md](app.md), "A change that no
  longer fits"; ADR 0030, decision 10). The words said of each end are core's (`run/waits/again.rs`): a wish
  that was not made is told as a typed wish is, and then "The change that does not fit was not thrown away.";
  after the toss, done or refused, "Your wish was made again, and it is being worked on.", or what a typed
  wish that was kept and has no run is told.
- **Work that did not fit when its run ended** (`run/waits/unfit.rs`). When a run ends and the keep by itself
  keeps nothing, core writes its typed reason with the run's record ([keep.md](keep.md), "Kept by itself":
  `unkept`). `Waits` carries it, and `run waiting` does not print it. For `Clash` and no other reason, the
  note of work that was read says first, in the place of the line that it waits for a yes, the refusal's two
  sentences: "Your folder changed in the same place after this change was made, so the two do not fit
  together." and "Make it again throws this one away and makes the wish again on your folder as it is now."
  Its choices are `Show me`, `Make it again`, `Toss it`, whatever files the work holds: there is no `Keep it`,
  which core has just refused, and a tap on `Make it again` is the tap above. Every other reason, a record
  with none and a record from before this was written leave the lines and the choices as they were. The
  reason is the run's end's: no look asks git whether the change fits, so a folder that changed back since
  still has the note say what was true then.
  The window draws such work with the kit's `unfit` picture: the arrow to the folder barred, and "does not fit
  your folder" under it, so the closed note says it too.

`work_tapped` has no command of its own: it is proved at the app's boundary (`app/tests/f47.rs`,
`app/tests/a_change_that_no_longer_fits.rs`), and what it
calls is proved at `run show`, `run keep` and `run toss`. `run waiting` is `crates/jellymoon/tests/f47.rs`, and
the smoke chain's `show` step asks it.

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
- **It does not say a result is true.** A `did`, a `decided` and an `unsure` are the run's own sentences: they
  are cleaned and bounded, and nothing compares them with the work. A `change` is held to the lines, not to
  their meaning or their place: a quoted line is one the file gained or lost, wherever in the file it is.
- **It does not quote from a file past the list.** A change that names a file after the first 200, or a third
  changed file, is dropped or left out.
- **It does not look past 200 files or 64 KiB of one.** There is no way to ask for the rest.
- **It does not tell two files apart by name.** Two files can print one name ("What a printed name can hold");
  `n` is the file.
- **It does not find a run's branch that a disk keeps under another case.** On a disk that folds case, with a
  branch `Jellymoon/other` already there, git stores the run's branch under `refs/heads/Jellymoon/` and lists
  it by that name. The look asks for `refs/heads/jellymoon/run-<id>` to the letter and says the branch is not
  in the repo. Nothing is shown that is not the run's; the run's own work is not shown either.
- **It writes no `work` to the record.** `run keep` writes `kept` ([keep.md](keep.md)) and `run toss` writes
  `tossed` ([toss.md](toss.md)).
- **It does not bury a run that was cut off.** A look is not a page: it refuses, and writes nothing down.
- **Measured on macOS only**, with git 2.50; the test lists work of ten thousand files and allows it a minute. Not
  measured: Windows, Linux, a git older than 2.31, a repo of the SHA-256 format, a partial clone.
