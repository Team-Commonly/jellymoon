# The reader: `jellymoon day`

The reader is the part of the daemon that collects one local day of what a person typed to Claude Code (ADR 0011, section 1).
It prints a `ReaderDay` (`schema/journal.json`, [protocol.md](../protocol.md#journal)) on stdout. It talks to no server, reads
no token and writes nothing.

```
jellymoon day [--date <YYYY-MM-DD>] [--logs <dir>] [--skip <name,name>]
```

- `--date`: the local calendar day; today when left out. The time zone is the process's (`TZ`).
- `--logs`: a Claude Code projects directory, laid out `<dir>/<project folder>/<session>.jsonl`. Left out, it is `projects`
  under `CLAUDE_CONFIG_DIR`, or under `.claude` in the home directory.
- `--skip`: folders the person switched off. A turn whose working directory contains one of the names contributes nothing.

## What counts as typed by a person

Measured on Claude Code 2.1 logs. A line starts a turn only when all of these hold: `type` is `user`, `origin.kind` is `human`,
`message.content` is a string, and `isMeta`, `isSidechain` and `isCompactSummary` are not true. That leaves out tool results
(a list, no origin), task notices (`origin.kind` `task-notification`), messages from other sessions (`peer`), compaction
summaries and prompts another program sent (no `origin`).

Of what is left, a slash command (text starting `<command-name>`, `<command-message>` or `<local-command`) is dropped. The
blocks the harness adds inside a prompt are cut out: `<system-reminder>`, `<artifact-view-context>`, the
`<artifact-content-authored-by-others/>` mark and the paragraph starting `(The JSON line above`. A prompt with nothing left is
dropped.

The reply is the text parts of the last assistant line before the person's next prompt, on any day; thinking and tool calls are
not text. A dropped prompt still ends the turn before it.

Known gap: a prompt typed with a pasted image is a list, not a string, so the reader drops it. In the logs measured that was
under 1% of typed prompts.

## What is printed

`said` is cut to 700 characters and `reply` to 500; a cut text ends in `…`. Before the cut, `redact.Credentials` replaces
credential shapes with `[redacted]`: model provider, GitHub and Slack keys, AWS key ids, JWTs, PEM private keys, and any run of
40 or more base64 characters that mixes digits with both letter cases. Jellymoon's own tokens are shortened to their prefix, as
everywhere in the CLI. `project` is the last segment of the working directory, or the worktree's name under
`.claude/worktrees/`.

A line that is not valid JSON is skipped. A file last written before the day began is not opened. Reading a busy day on a
computer with gigabytes of logs takes around ten seconds.
