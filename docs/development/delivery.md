# Delivery: `jellymoon rule` and `jellymoon hook`

A ruling made on a page reaches each running Claude Code session once, at its next step (ADR 0011, section 5). Core
holds all of it (`ruling.rs`, `hook.rs`); the commands only parse arguments and print JSON.

```
jellymoon rule --stands <the words that stand> --over <the words they replace>
jellymoon hook                                   (run by Claude Code, with the step on stdin)
jellymoon hook install [--settings <file>]
jellymoon hook uninstall
```

## The ruling store

`jellymoon rule` keeps one ruling as one file, `rulings/<id>.json`, in Jellymoon's home (`JELLYMOON_HOME`, or
`.jellymoon` in the home directory) and prints it: `{"id","stands","over"}`. The app calls the same core function
(`jellymoon_core::rule`) when a clash option is tapped. The id is the time it was made, in milliseconds. Both
statements pass through the reader's cleaning ([what is printed](reader.md#what-is-printed)) before they are stored
and again when they are handed over: credential shapes replaced, control characters removed, cut to 200 characters. A
ruling without both statements is refused. The file is written under another name and then renamed, so a hook never
reads half of one.

A ruling waits for 24 hours. After that no session is told, and the next `jellymoon rule` removes its files.

## The hook

Checked against <https://code.claude.com/docs/en/hooks> on 2026-11-10.

- **Event: `PostToolUse`**, with no matcher, so it fires after every tool call that worked, in every session. This is
  the "next step" ADR 0011 measured (a ruling arrived at the next tool call). `UserPromptSubmit` alone would not reach
  a turn that is already running. One event keeps it to one entry; a session that is idle is told after the first
  tool call of its next turn.
- **Input.** Claude Code gives the hook a JSON object on stdin. Only `session_id` and `hook_event_name` are read.
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
byte stays: comments, key order, spacing, line ends. The text and the file's path are kept in `hook.json` in
Jellymoon's home, and `jellymoon hook uninstall` removes exactly that text, so the file is as it was, along with
anything the person changed elsewhere in between. Both print `{"installed","changed"}`.

- A missing file is made, and removed again by uninstall. An empty file gets one object and is emptied again.
- A file that is not a JSON object (`//` and `/* */` comments allowed), or whose `hooks` or `PostToolUse` are not an
  object and a list, is refused with exit 1 and left as it was.
- Installing twice changes nothing. Installing into a second settings file while the first holds the entry is refused.
- When the entry is no longer in the file as written (edited or reformatted), uninstall changes nothing and says to
  remove it by hand.

Core's `serde_json` gained the `raw_value` feature for this: it gives the place of a value in the text. No new package.

## What is unproved

- **The harness-real step of F29.** No real Claude Code turn has run against this hook. A person runs it, signed in,
  with the command below; it installs the hook in a settings file of the test's own, never in the person's.

  ```
  JELLYMOON_HARNESS_REAL=1 cargo test -p jellymoon --test f29 -- --ignored harness_real --nocapture
  ```

- **Old Claude Code.** A version from before the exec form would ignore `args` and run the program bare, which fails
  at every tool call. The version that added `args` is not in the documentation.
- **A ruling goes to every session**, whatever project it runs in. Whether it should go only to the clash's project
  is not decided.
- **Windows and Codex.** The tests are written to pass on Windows; no real session there has been tried. Codex has no hook yet.
