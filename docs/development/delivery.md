# Delivery: `jellymoon rule` and `jellymoon hook`

A ruling made on a page reaches each running Claude Code session once, at its next step (ADR 0011, section 5). Core
holds all of it (`ruling.rs`, `hook.rs`); the commands only parse arguments and print JSON.

```
jellymoon rule --stands <the words that stand> --over <the words they replace>
jellymoon hook                                   (run by Claude Code, with the step on stdin)
jellymoon hook install [--settings <file>]
jellymoon hook uninstall
jellymoon hook status                            (prints whether the entry is in the settings now; writes nothing)
```

## The ruling store

`jellymoon rule` keeps one ruling as one file, `rulings/<id>.json`, in Jellymoon's home (`JELLYMOON_HOME`, or
`.jellymoon` in the home directory) and prints it: `{"id","stands","over"}`. The app calls the same core function
(`jellymoon_core::rule`) when a clash option is tapped. The id is the time it was made, in milliseconds. Both
statements are cleaned before they are stored and again when they are handed over, as the reader cleans a text
([what is printed](reader.md#what-is-printed)): credential shapes replaced, control and invisible characters
removed, cut to 200 characters. Each is also made one line: a line break, a tab and any run of spaces become one
space, and every `"` becomes `'`. So a statement cannot close its own quote in what the hook prints or start a line
that reads as another ruling. A ruling without both statements is refused. The file is written under another name
and then renamed, so a hook never reads half of one. The folders Jellymoon makes are for the person alone (0700) on
a system with such modes.

A ruling waits for 24 hours. After that no session is told, and the next `jellymoon rule` removes its files.

## A tap in the app

The app makes a ruling when the person taps an option of a clash ([app.md](app.md#taps-appsrctapsrs)). This is the
only way text gets from a page into a running session, so what is trusted is stated here exactly.

- **What is sent.** The two statements of one clash, `quote_a` and `quote_b`, of the page data the tap was made on,
  one as what stands and the other as what it replaces. They go through `jellymoon_core::rule` and nothing else:
  one line each, credential shapes replaced, 200 characters, `"` made `'`, and cleaned again when handed over. The
  app adds no text, sends no label, wish, star or project, and one tap makes at most one ruling.
- **Where that page data comes from.** In the window it is the page data on show: the reading the app just made
  and checked, or the day's kept page as it was read at the door when the window opened. For `jellymoon-app tap` it
  is the kept page as it is on disk at that moment.
- **What is trusted.** The kept page, `<home>/pages/<day>.json`, in a folder for the person alone (0700, the file
  0600). It is held to the door's shape and size, and it must be of the day asked for. It is not checked again
  against the session logs, and it carries no mark that it is the page the reading wrote. So whoever can write
  that file as the person can choose the words of the next ruling the person taps, within core's cleaning; the same
  account can write `rulings/` itself, so this gives them nothing they did not have. A kept page that was changed
  after the reading is ruled on as it is drawn: the words in the ruling are the words on the page, and a ruling
  made on the old words no longer marks the clash as settled. A kept file that is not page data is not drawn and
  cannot be tapped.
- **What is not trusted.** The settlement file `<day>.ruled.json` says only which side stands, for the page to
  draw. No ruling is ever made from it. A model's reading reaches a ruling only as the two quotes of a clash. Core
  kept each only as an exact, whole-word piece of something the person typed that day ([page.md](page.md)). The
  model chose which piece, and a piece can mean other than the whole ("delete the database" out of "do not delete
  the database"), so what protects the person is that both statements are on the page, in full, where they tap.
- **Changing one's mind.** A tap on the other side makes a second ruling with the statements the other way round.
  The first is not removed. A session that was told the first is told the second at its next step, as a newer
  decision that replaces what was said before. A session that was told neither is told both in one step, oldest
  first, so the later one is last. Whether a model always follows the last of two opposite rulings is unproved.
- **Delivery is never turned on by a tap on a clash.** Only the "Turn on delivery" tap installs the hook, with
  the same function as `jellymoon hook install`, for the installed `jellymoon` command.

## The hook

Checked against <https://code.claude.com/docs/en/hooks> on 2026-11-10.

- **Event: `PostToolUse`**, with no matcher, so it fires after every tool call that worked, in every session. This is
  the "next step" ADR 0011 measured (a ruling arrived at the next tool call). `UserPromptSubmit` alone would not reach
  a turn that is already running. One event keeps it to one entry; a session that is idle is told after the first
  tool call of its next turn.
- **Input.** Claude Code gives the hook a JSON object on stdin. `session_id`, `hook_event_name` and `agent_id` are
  read, and stdin is read only when a ruling waits, at most 1 MiB of it. A step larger than that is cut, is then not
  JSON, and is told nothing; the ruling waits for the session's next step.
- **A subagent's step is told nothing.** A step that carries `agent_id` is a subagent's. The ruling waits for the
  main thread, which decides what is done; a subagent would use it up for the session.
- **Output.** With nothing to hand over it prints nothing and exits 0, without reading its input. Otherwise it prints
  one object, the documented way to add context "alongside the tool result":

  ```json
  {"hookSpecificOutput":{"hookEventName":"PostToolUse","additionalContext":"Jellymoon: the person has made a newer decision ..."}}
  ```

  The context says the decision is the person's own and newer, then for each ruling what stands and what it replaces,
  oldest first. Plain stdout would not do: for `PostToolUse` it goes to the debug log only. At most 8 rulings go in
  one step, so the text stays far under the 10,000 characters Claude Code keeps; the rest come at the next step.
- **It never stops a session.** Input that is not a hook step, a session id that is not a plain name (letters,
  digits, dash, underscore, at most 64), an unknown event or a store that cannot be read all print nothing and exit 0.
  So does a home that is missing: with `JELLYMOON_HOME` unset and no home directory, or with the folder not there,
  the hook prints nothing, exits 0 and makes no folder. A `JELLYMOON_HOME` that is not a full path is no home for
  the hook either: it says nothing, exits 0 and hands over nothing, where every other command refuses it in one line
  (`reader.md`). An error printed by a hook would show at every step of a session.
  Exit code 2 is never used for a delivery. `PreToolUse` and `UserPromptSubmit` are answered in the same shape, for a
  person who wires the hook to those by hand.

**Exactly once.** Before a ruling is handed to a session the hook creates a mark file,
`rulings/<id>.<session id>.told`, with create-new, which only one process can win on any system. The one that made
the mark prints the ruling; every other hook of that session, racing or later, skips it. No lock is held and nothing
is rewritten. The mark is made before the print, so a hook killed in between loses that ruling for that session:
never twice, and once unless the hook dies.

## Install and uninstall

`jellymoon hook install` adds one entry to Claude Code's settings: `settings.json` under `CLAUDE_CONFIG_DIR`, or under
`.claude` in the home directory, or the file `--settings` names.

```json
{"hooks":[{"type":"command","command":"<this jellymoon program>","args":["hook"]}]}
```

`args` makes it the exec form: Claude Code starts the program itself with no shell, so the same entry works on
Windows and a path with spaces needs no quoting.

The settings file is never parsed and written back. The entry is one piece of text put right after the opening
bracket of the `PostToolUse` list, with `"PostToolUse"` and `"hooks"` around it when the file lacks them. Every other
byte stays: comments, key order, spacing, line ends. The text, the program and the file's path are kept in
`hook.json` in Jellymoon's home, and `jellymoon hook uninstall` removes exactly that text, so the file is as it was,
along with anything the person changed elsewhere in between. Both print `{"installed","changed"}`.

**The file is replaced whole.** The new content is written to a new file beside the settings,
`<name>.jellymoon-<process id>`, opened create-new so a file that is there is never written over, and readable by the
person alone (0600) on Unix until it is given the old file's mode. A settings file install creates has no old
file, so it stays 0600. It is synced and renamed over the old one. A link
is resolved first, so the link stays and the file it points at is replaced; a link that points at nothing is itself
replaced by a regular file. A read-only settings file is replaced too, with its mode kept (on Windows such a
file may refuse the rename; then the command fails and the file is as it was). Hard links to the old file and its extended attributes are not carried over. When a step
fails the new file is removed, the settings are as they were, and the command exits 1; an install that failed leaves
no record, and a removal that failed keeps its record to try again. A run that is killed between making the new file
and renaming it leaves that file there. Nothing sweeps it; the settings themselves are whole either way.

- A missing file is made, and removed again by uninstall. An empty file gets one object and is emptied again.
- A file that is not a JSON object (`//` and `/* */` comments allowed), or whose `hooks` or `PostToolUse` are not an
  object and a list, is refused with exit 1 and left as it was.
- A file that already holds an entry running this program with `["hook"]`, in any layout and under any event, is
  left alone: install
  records it and prints `"changed":false`. So a file another program laid out anew never gets a second entry.
  Installing into a second settings file while the first holds the entry is refused.

**Removal never writes a file that is not a JSON object**, or one that still names the program. In order:

1. With no record, or with the settings file gone, nothing is changed and it exits 0.
2. The recorded text is removed when it is in the file exactly once and what is left is sound. This is the usual
   case, and it works wherever the person moved the text.
3. Otherwise (the file was laid out anew, or something was added beside the entry so that removing the text would
   orphan a comma) uninstall looks for the one `PostToolUse` group that is exactly the entry and cuts it out with
   one comma and the blank space beside it: the comma after it, or the one before it when it is last. A comment is
   never removed. The `"hooks"` and `"PostToolUse"` keys install added stay, empty: nothing says they were not the
   person's.
4. When no handler under any event names the program (the person took the entry out, or `hooks` is no longer an
   object), there is nothing to remove: the record is forgotten and it exits 0 with `"changed":false`.
5. Anything else is left for the person: the file is not touched, the record is kept, and it exits 1 saying to
   remove the entry by hand. That is an entry edited into another shape, one moved under another event and laid out
   anew, a second copy of it, a comment between the entry and its comma, or a file that is not JSON.

`jellymoon uninstall` ([install.md](install.md)) removes the hook the same way before the app and the binary. An
entry the person edited is left for them, the rest is still removed, and the JSON says `"hook_by_hand":true`.

Core's `serde_json` gained the `raw_value` feature for this: it gives the place of a value in the text. No new package.

## The harness-real step

The coordinator ran it on 2026-11-10 and it passed (1 passed in 11.66 s): a real Claude Code turn told to write RED
was sent a ruling once the turn had started and wrote BLUE. It needs a signed-in person, so it is ignored by default. It installs
the hook in a settings file of the test's own, never in the person's.

```
JELLYMOON_HARNESS_REAL=1 cargo test -p jellymoon --test f29 -- --ignored harness_real --nocapture
```

## What is unproved

- **Old Claude Code.** A version from before the exec form would ignore `args` and run the program bare, which fails
  at every tool call. The version that added `args` is not in the documentation.
- **A ruling goes to every session**, whatever project it runs in. Whether it should go only to the clash's project
  is not decided.
- **Windows and Codex.** The tests are written to pass on Windows; no real session there has been tried. Codex has no hook yet.
