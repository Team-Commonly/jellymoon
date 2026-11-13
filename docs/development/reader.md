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
- `--skip`: folders the person switched off. A turn whose working directory contains one of the names contributes
  nothing. A line with no working directory is matched by the name of the folder its session file is in.
- **`skip.txt`.** The names in `<home>/skip.txt`, one a line, are switched off as well, joined with `--skip`: it is
  the list the app keeps, so a folder switched off there is not read by `day` or `page` either. `<home>` is
  `JELLYMOON_HOME`, or `.jellymoon` in the home directory; with neither there is no such file. A `JELLYMOON_HOME`
  that is not a full path is refused in one line by every command and by the app: a relative one would name another
  list in every folder a command starts in. (`jellymoon hook` alone says nothing and does nothing: `delivery.md`.)
- **How a name matches.** White space around a name is dropped, and a name of white space alone is ignored. Any `/`
  or `\` at its end is dropped too, unless the name is nothing else: `/` alone stays as written and switches off
  every folder written with a `/`. A turn is left out when its working directory, as the log wrote it, contains the name
  character for character. The match is exact: case matters (`Private-Notes` does not switch off `private-notes`),
  and a letter written in another Unicode form does not match. It is not a whole folder name either: `notes` also
  switches off `/work/my-notes-old`. In `skip.txt` a line is one name, whole: a comma or a quote is part of it.
  Nothing counts the folders switched off. What was read is named instead: every turn of `day` carries its
  `project`, `page` lists them in `read.projects`, and the app's footer names them, so a name that matched nothing
  shows as its folder still being there.
- **A run's copy is never read** (ADR 0018, decision 7). A line whose working directory is `<home>/copies/<run>`
  or anything deeper is left out, whatever was typed there: it starts no turn and is no reply, and a typed one
  still ends the turn before it, as a line of a switched-off folder does. Lines are judged one by one, so of a
  session with lines in a copy and lines elsewhere only the copy's go. No flag turns this off, and nothing
  counts or names what was left out. It holds for `day` and `page`, and wherever a folder that was typed in is
  looked for: a run's search for its wish's folder, and the projects a wish is offered for.
  - **How the folder is compared** (`session/copies.rs`). The log's working directory is text and is compared
    as text: nothing of it is looked up. `copies` has two spellings, under the home as it is given and under
    the home as the system resolves it when it is there, since a harness logs the folder it stands in resolved
    (on a Mac a home under `/tmp` is logged under `/private`; on Windows a short name may be logged long). A folder is
    left out when it is a full path whose first names are a spelling's, each whole, with at least one more. So
    `copies-old/x`, `copiesx`, a `copies` anywhere else, a folder beside the home and `copies` itself are read.
    A separator that is doubled or ends the path, and a `.`, change nothing. A path with a `..` in it is read.
  - **Letter case and Windows.** On a Mac and on Linux a name is compared exactly; on a Mac the resolved
    spelling is in the case the disk keeps, which is what a harness logs. On Windows `/` and `\` both part
    names, a name matches in any ASCII case (the drive's letter too), and the long spellings `\\?\C:\…` and
    `\\?\UNC\server\share\…`, which is how a path is resolved there, are read as the short ones.
  - **Not closed.** A line with no working directory is read. So is a copy spelled in a way neither spelling
    covers: through a link the home's path does not have, in another case or Unicode form than the disk keeps
    on a Mac, with a `..`, with a letter outside ASCII in another case on Windows, as a device path (`\\.\`)
    or as a shell there writes it (`/c/…`). A home that is gone has only the spelling it is given, and a home
    that was moved leaves its old copies' sessions to be read. Windows' spellings are tested as text, on every
    system, in core; at the binary only the spellings of the system the tests run on are.
- **A list that is not a list stops the command**, with one line, exit 1, before a log is opened and before any
  reading call: a damaged list is never an empty list. Only nothing at all under the name `skip.txt` is no names.
  Refused: a link to nothing, a folder, a pipe, a file that cannot be read, one over 64 KiB or not UTF-8, and a line
  that holds a NUL, a byte order mark (one at the very start of the file is dropped), a carriage return that does
  not end it, as UTF-16 text and old Mac line ends do, or a character that cannot be seen. The line is named by its
  number, never by its text.
- **Characters that cannot be seen** are a closed list, in one function of core (`unseen` in `reader.rs`): every
  control character, and U+00AD, U+061C, U+115F, U+1160, U+180E, U+2028, U+2029, U+3164, U+FEFF, U+FFA0, U+200B to
  U+200F, U+202A to U+202E, U+2060 to U+206F, U+FE00 to U+FE0F, U+E0000 to U+E007F and U+E0100 to U+E01EF. Nothing
  else is looked for. White space around a name is dropped first, so a tab or a line break at a name's end is
  honoured. A name that holds one of them stops the run when it is in `skip.txt`, and in `--skip` is an argument
  error, exit 2, that names the name by its place in the value (`--skip: name 2 holds …`), never by its text. A
  folder whose own name holds such a character is switched off by naming a part of its name without it, and both
  stop lines end by saying so.
- **No session logs.** When the projects directory is not there (Claude Code was never used, or keeps its files
  elsewhere), the command says so in one line that names the folder it looked for, and exits 1. When the folder is
  the one nobody named, the line adds what to do. A directory that is there with no session in it is a day with no
  turns. A `--logs` that is empty or only spaces is an argument error. In the line, a character
  of the folder's name that cannot be seen is shown as a space.

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
dropped. This is the one step that reads a whole prompt, since a block can be any length: four passes, one for each
kind of block, each taking time in step with the prompt's length.

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
redacted before the cut, and spaces around them are dropped.

Only the start of a long text is read: the length it is cut to and 16,384 characters more (17,084 for `said`). That
much is cleaned, redacted and then cut, and the rest is never looked at, so cleaning and redaction take no longer on
a text of 64 MiB than on one of 20 KB. It is safe because a secret under 8,192 characters that starts in what is printed ends inside
what was read, and is replaced whole before the cut. A PEM private key can be longer, so it is replaced from its
BEGIN line on, whether or not its END mark was read.

One case needs more. When cleaning takes out or squeezes so much that fewer than 8,192 characters are left after the
cut (a text that is mostly invisible characters, or spaces in a text printed on one line), the place where reading
stopped is close to what is printed, and the word there may be the first half of a secret. That last word is then left
out and the text ends in `…`; with no space before it, nothing is printed and the prompt makes no turn. The word stays
only when it is `[redacted]` and nothing more, as a PEM key's replacement is.

Cleaning runs first and takes out what nobody sees:

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

Redaction then replaces credential shapes with `[redacted]`, in this order. A shape that can stay unfinished for
any length (a PEM header, a JWT's first part) has a pass of its own, so it cannot make another shape's pass start over.

1. The token shapes of the product this repo used to hold (`jmu_`, `jmc_`, `jmr_`, `cm_`) are shortened to their prefix,
   `jmr_…`.
2. PEM private keys, from the BEGIN line to the END mark, or to the end of what was read when there is none. Then
   JWTs. Then named keys: model provider, GitHub, GitLab, Hugging Face, Google (`AIza`), Stripe live and Slack keys
   and AWS key ids.
3. The password in `scheme://user:password@host`, the password only.
4. Keys of unknown make. The text is split into words at white space. In each word, a run of 40 or more base64
   characters (letters, digits, `+`, `/`, `=`, `_`, `-`) is replaced when it mixes digits with both letter cases.

A word is a path only by how it starts: `/`, `./`, `../`, `~/` or a Windows drive (`C:\` or `C:/`), after any opening
quotes or brackets (`"`, `'`, `` ` ``, `(`, `[`, `{`, `<`). A link starts at its URL scheme (`https://`) wherever that
is in the word, so a markdown link, `[the notes](https://…)`, and a JSON value, `"url":"https://…"`, are links from
the scheme on. A path or a link ends at the next closing quote, closing bracket or comma (`"`, `'`, `` ` ``, `)`, `]`,
`}`, `>`, `,`), or with its word. Inside it a `/` ends a run, so each segment is judged alone: a long folder path
survives, and a segment of 40 or more mixed characters inside it is replaced. A run outside every path and link is
judged whole, so the next value of a JSON object is. A word can hold any number of links, each from its own scheme to
its own end, so both links of `{"html":"https://…","diff":"https://…"}` are kept. It holds at most one path, the one
it starts with: after a path or a link has ended, a `/`, `./`, `../`, `~/` or a drive starts nothing, so the second
path of `["/srv/a","/srv/b"]` is judged whole. Nothing else about a word is read: a slash or a dot inside it does not
make it a path.

What that replaces wrongly: a word that is a path but does not start like one, when a run in it reaches 40 characters
and mixes digits with both cases. `feature/JIRA-1234-AddUserProfileSettingsPanel` and
`src/components/UserProfile2/SettingsPanel3/index` both become `[redacted]`, and with `.tsx` after it the second becomes
`[redacted].tsx`.

What happens to a 40-character secret with slashes in it: alone, after `~`, after `notes.txt/`, before `.txt` or inside
quotes it is replaced. Inside a path or a link it is missed, because each piece between its slashes is under 40
characters: `https://host/<secret>` and a secret that itself starts with `/` both pass.

`project` is the last segment of the working directory. Under `<repo>/.claude/worktrees/<name>` it is the repo folder's
name, so a repo and its worktrees are one project.

A line that is not valid JSON is skipped. A line over 64 MiB is read past without being kept. A log is read no
further than its length when it was opened: what a session writes after that is in the next reading, and a line
that was half written at that length is cut off there, is not valid JSON, and is skipped, as the unfinished last
line of a log always was. So a log that is written to as fast as it is read still ends. When the system
will not say a log's length, the length is found by seeking to the log's end and back; only a log that can be
neither asked nor sought is read to its end, with no bound, and never taken for an empty one. A byte-order mark
at a log's very start is not part of its first line: one mark, taken off once, and never off a later line.

In a test's build only, the reader can be held between asking a log's length and reading its first byte
(`JELLYMOON_TEST_OPEN_WAIT_MS` with `JELLYMOON_TEST_GO`, core's `busy::pause`): `crates/jellymoon/tests/growing_log.rs`
adds lines to the log there, and the day must hold none of them. A release has no such place; in every build
core's own test reads a log that never ends and returns having asked it for exactly its length in bytes.

Decoding a line takes
memory many times its size (about 27 times for a line that is one long list), so the largest line kept can take
well over a gigabyte while it is read. A session file whose modification time is before the start of the requested local day is not opened, since it can hold no turn of that day; the one consequence is that a file copied or restored with an old modification time is skipped. Reading a busy day on a
computer with gigabytes of logs takes around ten seconds.

## Not covered yet

A session file is listed, then opened. Something that swaps the file for a link or a pipe between those two steps would
have the swapped thing opened; the check is made on the listing only. `skip.txt` is the same: its type is looked at,
then it is opened, and a pipe put there between the two would make the open wait. That needs write access to
Jellymoon's own folder, and nothing is read or sent while it waits.
