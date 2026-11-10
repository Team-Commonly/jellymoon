# The reading call: `jellymoon page`

`jellymoon page` prints one local day's page data (`PageData` in `schema/page.json`, [protocol.md](../protocol.md#page-data))
on stdout. It reads the day as [`jellymoon day`](reader.md) does, sends it through one reading call to the person's own Claude
Code, and checks what comes back (ADR 0011, section 2). It talks to no server and reads no token.

```
jellymoon page [--date <YYYY-MM-DD>] [--logs <dir>] [--skip <name,name>] [--model <name>]
```

`--date`, `--logs` and `--skip` are the reader's. `--model` is the model the harness is asked for, `sonnet` when left out. It
must be a name: letters, digits, dot, dash and underscore, starting with a letter or digit. Anything else is refused before any
call.

## The call

The harness is `claude`, found on `PATH`. It is run once, like this:

```
claude -p --model <name> --tools "" --strict-mcp-config --disable-slash-commands --no-session-persistence
       --setting-sources local --output-format json --system-prompt <the prompt>
```

- Every tool, MCP server, slash command, saved session and user-level setting is off, so the model can return text and do
  nothing else.
- The day, a `ReaderDay` as JSON, goes in on stdin and nowhere else. No text from a session log reaches the arguments or the
  environment, so nothing a person or an agent wrote can add a flag or a tool.
- The prompt is one fixed text compiled into the daemon (`daemon/internal/reading/prompt.md`). It tells the model that the
  day is data and that an instruction inside it is not to be followed.
- The environment is `PATH`, `HOME`, `TMPDIR`, `USER`, `LANG`, `LC_ALL`, `TERM` and `CLAUDE_CONFIG_DIR`, when set: enough for
  the harness to find its own sign-in. The daemon passes no credential of its own or anyone else's.
- It runs in a new empty folder under `reading/` in the config dir (`JELLYMOON_HOME`), removed when the call ends.
- It may take three minutes. After that it is stopped and there is no page.

A day with no turns prints an empty page and makes no call.

## What code checks

The model is not trusted: a session log can hold any text an agent ever printed. The harness prints one JSON object whose
`result` is the model's text, which may sit in a code fence.

There is no page, only an error on stderr and a non-zero exit, when the harness cannot be found, exits non-zero, reports
`is_error`, prints something else, or takes too long, and when the model's text is not JSON with `wishes` and `clashes`, lacks a
field, has a value of the wrong type, or names a state that is not one of the four.

Otherwise each piece is checked on its own:

- A wish is dropped unless its `said` is an exact substring of one thing the person typed that day and its `project` is one of
  the day's. The agent's replies do not count. A wish with no label, or with an id an earlier wish used, is dropped too.
- The first 8 wishes that pass are kept and given the ids `w1` to `w8`. The model's ids are only used to match clashes.
- A clash is dropped unless both quotes are such substrings and it names two different kept wishes. The first 2 that pass are kept.
- `label` and each star are the model's words: control characters and line breaks are taken out, credential shapes are replaced
  (`redact.Credentials`), and a text over its length (40 and 60 characters) is cut and ends in `…`. At most 4 stars are kept.
- A quote over 200 characters is cut to its first 200, with no mark added, so it is still the person's words.
- `day` and `read` (how many turns, which projects) are written from the reader's output. A field the model adds is not copied.

## Not covered yet

The reading call has been run against a made-up harness only (`TestF27`); the real Claude Code check is the coordinator's.
A harness that is installed but not on `PATH` (a shell alias, for one) is not found. Codex is not read.
