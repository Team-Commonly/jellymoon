# The reader: `jellymoon day`

The reader is the part of core that collects one local day of what a person typed to Claude Code (ADR 0011, section 1).
`jellymoon day` prints it on stdout as one line of JSON, `{"day","turns":[{"at","tool","project","said","reply"}]}`; the
type and its example file are in core. It talks to no server, reads no token and writes nothing.

```
jellymoon day [--date <YYYY-MM-DD>] [--logs <dir>] [--skip <name,name>]
```

- `--date`: the local calendar day; today when left out. The time zone is the process's (`TZ`).
- `--logs`: a Claude Code projects directory, laid out `<dir>/<project folder>/<session>.jsonl`. Left out, it is `projects`
  under `CLAUDE_CONFIG_DIR`, or under `.claude` in the home directory.
- `--skip`: folders the person switched off. A turn whose working directory contains one of the names contributes nothing.
  Spaces around a name are dropped, and an empty name is ignored. A line with no working directory is matched by the name of
  the folder its session file is in.

Only a regular `.jsonl` file in a real folder of the projects directory is opened. A link is never followed, to a file or to a
folder, and a pipe or anything else that is not a file is left alone.

## What counts as typed by a person

Measured on Claude Code 2.1 logs. A line starts a turn only when all of these hold: `type` is `user`, `origin.kind` is `human`,
`message.content` is a string or a list with no `tool_result` part, and `isMeta`, `isSidechain` and `isCompactSummary` are not true. That leaves out tool results
(a list, no origin), task notices (`origin.kind` `task-notification`), messages from other sessions (`peer`), compaction
summaries and prompts another program sent (no `origin`).

Of what is left, a slash command (text starting `<command-name>`, `<command-message>` or `<local-command`) is dropped. The
blocks the harness adds inside a prompt are cut out: `<system-reminder>`, `<artifact-view-context>`, the
`<artifact-content-authored-by-others/>` mark and the paragraph starting `(The JSON line above`. A prompt with nothing left is
dropped.

The reply is the text parts of the last assistant line before the person's next prompt, on any day; thinking and tool calls are
not text. A dropped prompt still ends the turn before it. So does a prompt nobody typed: a message from another session
(`peer`) or a prompt another program sent. What the agent says next answers that, not the person. A task notice does not end
the turn, and neither does a line the harness wrote itself (`isMeta`, `isCompactSummary`).

A resumed or forked session repeats earlier lines in a new file under the same `uuid`. A typed line whose `uuid` was already
read is dropped with its replies, so the prompt is printed once, from the file read first (files are read in name order).

A prompt typed with a pasted image is a list: the person's text is its `text` parts joined, treated as a string prompt is.
An image alone leaves no text, so it is dropped, and it still ends the turn before it.

## What is printed

`said` is cut to 700 characters and `reply` to 500; a cut text ends in `…`. Before the cut, redaction replaces
credential shapes with `[redacted]`: model provider, GitHub, GitLab, Hugging Face, Google (`AIza`), Stripe live and Slack keys,
AWS key ids, JWTs, PEM private keys, the password in `scheme://user:password@host` (the password only), and any run of
40 or more base64 characters that mixes digits with both letter cases. A run with a `/` and no `+` or `=` is a path: each of
its segments is judged alone, so a long folder path survives and a long mixed segment inside one does not. The token shapes of the product this repo used to hold (`jmr_` and its
siblings) are shortened to their prefix, `jmr_…`. `project` is the last segment of the working directory. Under `<repo>/.claude/worktrees/<name>` it is the repo folder's
name, so a repo and its worktrees are one project.

A line that is not valid JSON is skipped. A line over 64 MiB is read past without being kept. A session file whose modification time is before the start of the requested local day is not opened, since it can hold no turn of that day; the one consequence is that a file copied or restored with an old modification time is skipped. Reading a busy day on a
computer with gigabytes of logs takes around ten seconds.
