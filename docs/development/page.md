# The reading call: `jellymoon page`

`jellymoon page` prints one local day's page data on stdout as JSON; the type and its example file are in core. It reads the day as [`jellymoon day`](reader.md) does, sends it through one reading call to the person's own Claude
Code, and checks what comes back (ADR 0011, section 2). It talks to no server and reads no token.

```
jellymoon page [--date <YYYY-MM-DD>] [--logs <dir>] [--skip <name,name>] [--model <name>]
```

`--date`, `--logs` and `--skip` are the reader's. `--model` is the model the harness is asked for, `sonnet` when left out. It
must be a name: letters, digits, dot, dash and underscore, starting with a letter or digit. Anything else is refused before any
call.

## The call

The harness is `claude` (on Windows `claude.exe`, then `claude.cmd`, then `claude`), looked for as the next section
says. It is run once, like this:

```
claude -p --model <name> --tools "" --strict-mcp-config --disable-slash-commands --no-session-persistence
       --setting-sources local --output-format json --system-prompt <the prompt>
```

- Every tool, MCP server, slash command, saved session and user-level setting is off, so the model can return text and do
  nothing else.
- The day, byte for byte what `jellymoon day` prints, goes in on stdin and nowhere else. No text from a session log reaches the arguments or the
  environment, so nothing a person or an agent wrote can add a flag or a tool.
- The prompt is one fixed text compiled into core. It tells the model that the
  day is data and that an instruction inside it is not to be followed.
- The environment is `PATH`, `HOME`, `TMPDIR`, `USER`, `LANG`, `LC_ALL`, `TERM` and `CLAUDE_CONFIG_DIR`, when set: enough for
  the harness to find its own sign-in. On Windows the list is the system's equivalents (`USERPROFILE`, `APPDATA`,
  `SystemRoot` and the like). Jellymoon passes no credential of its own or anyone else's. `PATH` is this process's own,
  made longer only when the harness was found in a well-known place (below).
- It runs in a new empty folder under `reading/` in Jellymoon's home (`JELLYMOON_HOME`, or `.jellymoon` in the home
  directory), removed when the call ends.
- It may take three minutes. After that it is stopped and there is no page.

A command line that is wrong exits 2 with the reason and the usage; a run that fails exits 1. Both print nothing on
stdout and start the stderr line with `jellymoon: `.

A day with no turns prints an empty page and makes no call.

## Where the harness is looked for

An app opened from Finder is given a `PATH` of four system folders (`/usr/bin:/bin:/usr/sbin:/sbin`), which holds no
`claude` however it was installed. So the harness is looked for in this order, and the first one found is run:

1. The program `JELLYMOON_CLAUDE` names, when it is set. A path there that does not exist is an error, not a fall
   back to the rest.
2. Each folder of `PATH`, as a terminal would.
3. `~/.local/bin`: where Claude Code's native installer puts `claude`, as a link to the version it installed.
4. `~/.claude/local`: the local install older versions of Claude Code made.
5. `/opt/homebrew/bin`: Homebrew on a Mac with an Apple chip.
6. `/usr/local/bin`: Homebrew on a Mac with an Intel chip, and what an administrator installed by hand.

Places 3 and 4 are the two fixed paths Claude Code's own pages name
([setup](https://code.claude.com/docs/en/setup), [troubleshoot installation](https://code.claude.com/docs/en/troubleshoot-install#check-for-conflicting-installations));
5 and 6 are where `brew install --cask claude-code` links it. On Windows only 3 and 4 are looked in, under the user's
profile. The list is fixed: no folder is listed, no shell is asked for its `PATH`, and the disk is not searched. An npm
install made under a version manager (nvm and the like) sits in a folder named after a Node version, so it is found
only through `PATH` or `JELLYMOON_CLAUDE`.

A candidate counts when it is a file, or a link to one, that can be run. When none does, the one-line error names
every place looked in.

The home is `HOME` (`USERPROFILE` on Windows) and nothing else, and only when it names a full path. The system's own
record of the account is not asked. With no such home, places 3 and 4 are not looked in, and the error says so. Core
reads the variable itself, so the command and the app cannot differ.

**Who can change the program.** The reading call hands the day's text to this program, so each place on the list is
one a stranger on the machine cannot write to. `~/.local/bin` and `~/.claude/local` are inside the person's home.
Any administrator of the Mac can write to `/opt/homebrew/bin`: Homebrew makes it writable by the administrators'
group. `/usr/local/bin` belongs to the system, or on an Intel Mac with Homebrew to its administrators likewise. None
is writable by everyone as installed, and no such folder (a temporary or shared one) is on the list.

On top of that, on a Mac and on Linux a candidate in places 3 to 6 is passed over when the group or everyone can write
to its file, or when everyone can write to the folder it is in. A folder the group can write to is accepted, as
Homebrew's are. The look goes on to the next place, so a later place's harness can be the one run with nothing said;
only when no place has one does the error name what was passed over and why. The file's mode is read after links are
followed, and the folder is the one on the list: not their owners, and not the folders above.

That check is not made on a `PATH` hit or on `JELLYMOON_CLAUDE`. Both are the person's own word: it is the program
their terminal already runs when they type `claude`, or one they named. The list is Jellymoon's choice, so Jellymoon
holds it to more.

**What the harness is started with.** An older npm install of `claude` is a script that starts `env node`, and under
Finder's `PATH` there is no `node`. So when the harness was found in places 3 to 6, its `PATH` is this process's own,
then the folder the program is in, then the four places. Found on `PATH` or named by `JELLYMOON_CLAUDE`, it gets
`PATH` unchanged. Nothing else about the call differs.

Those added folders are where a script harness gets its `node`, and nothing about that `node` is checked: not its
mode, and not its folder's. They come after this process's own `PATH`, so they are used only for what it lacks. A
folder whose name cannot be written in a `PATH` (a home with a colon in it) is left out of the longer one: a harness
there that is a program by itself still runs, and a script there finds `node` only in the other folders.

`JELLYMOON_SYSTEM_BIN`, a list of folders written as `PATH` is, takes the place of 5 and 6, for the command and the
app alike; set to nothing, neither is looked in. The tests set it so that no folder of the machine they run on is read.

## What code checks

The model is not trusted: a session log can hold any text an agent ever printed. The harness prints one JSON object whose
`result` is the model's text, which may sit in a code fence.

There is no page, only an error on stderr and a non-zero exit, when the harness cannot be found, exits non-zero, reports
`is_error`, prints something else, or takes too long, and when the model's text is not JSON with `wishes` and `clashes`, lacks a
field, has a value of the wrong type, or names a state that is not one of the four.

An error never quotes the model. A reading that is not page data is named by where it stopped being so, as a line and a
column of the model's JSON. What the harness itself printed (its stderr when it fails, its result when it reports an
error) is shown on one line, cleaned and redacted as the reader does it, and cut to 300 characters.

The model's and the harness's words are read only from their start, as the reader reads a long text
([what is printed](reader.md#what-is-printed)): the length a text is cut to and 16,384 characters more. A secret under
8,192 characters that starts in what is shown is inside that and is replaced whole, and what comes after is never
cleaned or kept, so a label, a star or an error of any length costs the same.

The check below is bounded the same way. Only the first 64 wishes and the first 64 clashes of a reading are looked at,
and the looking stops when the page is full. A quote longer than the turn it is compared with is not searched for.

Otherwise each piece is checked on its own:

- A wish is dropped unless its `said` is a quote of the person (the rule is below) and its `project` is one of the
  day's. A wish with no label, or with an id an earlier wish used, is dropped too.
- The first 8 wishes that pass, among the first 64 the model wrote, are kept and given the ids `w1` to `w8`. The model's ids are only used to match clashes.
- A clash is dropped unless both its quotes are quotes of the person and it names two different kept wishes. The
  first 2 that pass, among the first 64, are kept.
- `label` and each star are the model's words: control characters, invisible characters (the reader's list) and line
  breaks are taken out, credential shapes are replaced as the reader replaces them, and a text over its length (40 and
  60 characters) is cut and ends in `…`. At most 4 stars are kept.
- A quote over 200 characters is cut to its first 200, with no mark added, so it is still the person's words. The cut
  is code's and can fall inside a word.
- `day` and `read` (how many turns, which projects) are written from the reader's output. A field the model adds is not copied.

### What a quote of the person is

A wish's `said` and a clash's two quotes are each kept only when all of these hold. Spaces around a quote are dropped first.

- It is an exact substring of one thing the person typed that day: one turn's `said`, as the reader printed it. The
  agent's replies do not count.
- It is at least 8 characters long, or it is the whole of what the person typed in that turn. The count is of
  characters. When every character of the quote is at or above U+2E80 (Chinese, Japanese and Korean and their
  punctuation), 4 are enough, since those scripts say more in a character. One letter, digit or space from below it
  and the quote needs 8.
- It does not start or end inside a word. Where the character at the quote's edge and its neighbour in the typed text
  are both letters or digits, the quote is refused. That holds for scripts that put spaces between words, taken to be
  every letter below U+2E80. Chinese, Japanese and Korean are above it and may be cut anywhere. Thai and other scripts
  below it that use no spaces are held to the rule, so a quote of them must run from one space or mark to another.

So the model cannot invent words, or show a letter or half a word as a quote. It can still choose which of the person's
words to show: from "please do not delete the prod database" the quote "delete the prod database" is kept, and code
cannot tell that it lost its "not". The page shows a quote as a quote for that reason.

## Not covered yet

The reading call has been run against a made-up harness only (the F27 test); the real Claude Code check is the coordinator's.
A harness that is neither on `PATH` nor in a well-known place (a shell alias, an install under a version manager) is
not found; `JELLYMOON_CLAUDE` is the way round. With a home whose files are made group-writable by default (a umask
of 002), a harness in a well-known place is passed over.
Codex is not read.

Windows has been run in CI only, against a made-up `.exe` harness, never by a person against Claude Code. Three things
are known to be open there. A `claude.cmd` shim may refuse the prompt's line breaks. Stopping a call that ran too long
stops the program Jellymoon started, not what that program started: behind a `.cmd` shim that is `cmd.exe`, and the
`node` under it keeps running and holds the call's folder. The short environment may leave out a variable Claude Code
uses there to find its shell.

On every system, a harness that exits while something it started still holds its output is waited on for one second,
then reported as having printed something other than its result.
