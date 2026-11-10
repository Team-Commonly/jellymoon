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

The harness is `claude`, found on `PATH` (on Windows `claude.exe`, then `claude.cmd`, then `claude`), or the program
`JELLYMOON_CLAUDE` names; a path there that does not exist is an error, not a fall back to `PATH`. It is run once, like this:

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
  `SystemRoot` and the like). Jellymoon passes no credential of its own or anyone else's.
- It runs in a new empty folder under `reading/` in Jellymoon's home (`JELLYMOON_HOME`, or `.jellymoon` in the home
  directory), removed when the call ends.
- It may take three minutes. After that it is stopped and there is no page.

A command line that is wrong exits 2 with the reason and the usage; a run that fails exits 1. Both print nothing on
stdout and start the stderr line with `jellymoon: `.

A day with no turns prints an empty page and makes no call.

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
A harness that is installed but not on `PATH` (a shell alias, for one) is not found; `JELLYMOON_CLAUDE` is the way round.
Codex is not read.

Windows has been run in CI only, against a made-up `.exe` harness, never by a person against Claude Code. Three things
are known to be open there. A `claude.cmd` shim may refuse the prompt's line breaks. Stopping a call that ran too long
stops the program Jellymoon started, not what that program started: behind a `.cmd` shim that is `cmd.exe`, and the
`node` under it keeps running and holds the call's folder. The short environment may leave out a variable Claude Code
uses there to find its shell.

On every system, a harness that exits while something it started still holds its output is waited on for one second,
then reported as having printed something other than its result.
