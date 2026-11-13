# A look at a run's work: `jellymoon run show`

What a finished run made, read from the repo's store and printed as data (F44; ADR 0017, decisions 1, 2, 3 and 6).
The rules are in `crates/jellymoon-core/src/run/show.rs` (which run, what is refused, what is printed) and
`crates/jellymoon-core/src/copy/work.rs` (every question asked of git). The test is `crates/jellymoon/tests/f44.rs`.

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

The record does not name the repo, the commit the run started from or the commit it ended as, and the copy's
record, which named the repo, is removed when the copy is cleaned. So the repo is found as a run finds it
(`run.md`, "The order"): `--repo`, or the one folder of the wish's project typed in on the day, from `--logs`. It
must be the top of a checkout. Of a folder read from a log nothing git says is repeated; here nothing git says is
repeated of any folder.

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
shown as the run's. This is all that can be checked, since the record keeps neither commit (see "What it does not
do").

## Cleaning

- **A name** passes `still::statement`, the door a clash's statement passes: one line, no control or invisible
  character, credential shapes replaced, `"` made `'`, cut at 200 characters.
- **A text** passes `still::still_text`: the reader's cleaning of a turn (`redact::shown`), which keeps lines and
  quotes, made again until a making changes nothing. Eight makings may change it; a text a ninth changes is
  refused, never printed half cleaned.
- **`obeyed`** is asked of the name as git keeps it and of the cleaned name, so an invisible character inside
  `.claude` does not hide it.

## Files the person's tools obey

One list, `OBEYED` in `run/show.rs`. A path is obeyed when any of its parts is, in any case of ASCII letters, one
of: `.claude`, `.codex`, `.cursor`, `.vscode`, `.idea`, `.husky`, `.devcontainer`, `CLAUDE.md`, `AGENTS.md`,
`.mcp.json`, `.envrc`, `.gitmodules`, `.gitattributes`; or when `.github` is followed by `workflows`. It is a
warning, not a wall: a tool the list does not know obeys its file all the same, and kept work is code in the
person's project either way.

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
| a tip that is not a run's commit | `the branch of run <run> is not as the run left it, ...` |
| a list of files over 16 MiB | `the work of run <run> is more files than Jellymoon lists` |
| `--file` with no such number, or one past the 200 listed | `file <n> of run <run> names no file: ...` |
| `--file` of a removed file | `file <n> of run <run> was removed by the work, ...` |
| `--file` of a link | `file <n> of run <run> is a link, and where it leads is not shown` |
| `--file` of another repo's entry | `file <n> of run <run> is another repo's entry, ...` |
| `--file` of what is not text | `file <n> of run <run> is not text, so it is not shown` |
| `--file` of a text cleaning goes on changing | `file <n> of run <run> cannot be cleaned, so it is not shown` |

`--file` with anything but digits is a usage error, exit 2. No refusal holds a file's name, a byte of a file, a
word of the agent's or a word of git's.

## What it does not do

- **It does not know the run's commits.** The record keeps neither where the run started nor what it ended as. A
  commit with the run's message and one parent, put on the run's branch by anyone who can write the repo, is shown
  as the run's work. F45 needs both ids in the record before it lands anything; writing them changes what a run
  writes at its end.
- **It does not look past 200 files or 64 KiB of one.** There is no way to ask for the rest.
- **It writes no `work` to the record.** Nothing does yet: `kept` and `tossed` are F45's and F46's to write.
- **It does not bury a run that was cut off.** A look is not a page: it refuses, and writes nothing down.
- **Measured on macOS only**, with git 2.50; the test lists work of ten thousand files and allows it a minute. Not
  measured: Windows, Linux, a git older than 2.31, a repo of the SHA-256 format, a partial clone.
