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

`said` is cut to 700 characters, `reply` to 500 and `project` to 255; a cut text ends in `…`. All three are cleaned and
redacted before the cut.

Cleaning runs first, in one pass, and takes out what nobody sees:

- A whole terminal escape sequence: CSI (ESC `[` or U+009B, then parameter bytes, intermediate bytes and a final byte,
  as in the colour code `ESC[32m`) and OSC (ESC `]` up to BEL or ESC `\`). A sequence that is cut short, or has
  another of these characters inside it, is not taken whole: it loses only those characters. A key written straight
  after a bare `ESC[` or U+009B loses its first characters to the sequence, and the rest of it is shown.
- Every other control character except newline and tab.
- Every format character (Unicode category Cf: the bidi controls, zero-width space, soft hyphen, word joiner,
  byte-order mark, invisible operators, tag characters) and U+E0000 to U+E007F, except the joiners U+200C and U+200D,
  which emoji and several scripts are written with. The cost: a flag emoji spelled with tag characters loses them.

So a key right after a colour code, or split by one of these characters, is still found. A character that is seen
only as part of its neighbour (a variation selector, a combining mark) stays, and a key split by one is missed.

Redaction then replaces credential shapes with `[redacted]`, in this order. Each step is one pass over the text.

1. The token shapes of the product this repo used to hold (`jmu_`, `jmc_`, `jmr_`, `cm_`) are shortened to their prefix,
   `jmr_…`.
2. Named keys: model provider, GitHub, GitLab, Hugging Face, Google (`AIza`), Stripe live and Slack keys, AWS key ids,
   JWTs and PEM private keys.
3. The password in `scheme://user:password@host`, the password only.
4. Keys of unknown make. The text is split into words at white space. In each word, a run of 40 or more base64
   characters (letters, digits, `+`, `/`, `=`, `_`, `-`) is replaced when it mixes digits with both letter cases.

A word is a path or a link only by how it starts: `/`, `./`, `../`, `~/`, a Windows drive (`C:\` or `C:/`) or a URL
scheme (`https://`), after any opening quotes or brackets (`"`, `'`, `` ` ``, `(`, `[`, `{`, `<`). In such a word a `/`
ends a run, so each segment is judged alone: a long folder path survives, and a segment of 40 or more mixed characters
inside it is replaced. Nothing else about a word is read: a slash or a dot inside it does not make it a path.

What that replaces wrongly: a word that is a path but does not start like one, when a run in it reaches 40 characters
and mixes digits with both cases. `feature/JIRA-1234-AddUserProfileSettingsPanel` and
`src/components/UserProfile2/SettingsPanel3/index` both become `[redacted]`, and with `.tsx` after it the second becomes
`[redacted].tsx`.

What happens to a 40-character secret with slashes in it: alone, after `~`, after `notes.txt/`, before `.txt` or inside
quotes it is replaced. Inside a word that starts as a path or a link it is missed, because each piece between its
slashes is under 40 characters: `https://host/<secret>` and a secret that itself starts with `/` both pass.

`project` is the last segment of the working directory. Under `<repo>/.claude/worktrees/<name>` it is the repo folder's
name, so a repo and its worktrees are one project.

A line that is not valid JSON is skipped. A line over 64 MiB is read past without being kept. A session file whose modification time is before the start of the requested local day is not opened, since it can hold no turn of that day; the one consequence is that a file copied or restored with an old modification time is skipped. Reading a busy day on a
computer with gigabytes of logs takes around ten seconds.

## Not covered yet

A session file is listed, then opened. Something that swaps the file for a link or a pipe between those two steps would
have the swapped thing opened; the check is made on the listing only.
