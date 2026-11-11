# Delivery: `jellymoon rule`, `jellymoon settle` and `jellymoon hook`

A ruling made on a page reaches each running Claude Code session once, at its next step (ADR 0011, section 5). Core
holds all of it (`ruling.rs`, `tap.rs`, `command.rs`, `hook.rs`); the commands only parse arguments and print JSON.

```
jellymoon rule --stands <the words that stand> --over <the words they replace>
jellymoon settle --date <YYYY-MM-DD> --clash <n> --option <1|2>     (a tap on a clash of that day's kept page)
jellymoon hook                                   (run by Claude Code, with the step on stdin)
jellymoon hook install [--settings <file>]
jellymoon hook install --installed               (for the installed command, checked: "Turn on delivery")
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
and then renamed, so a hook never reads half of one. The `rulings` folder is set to the person alone (0700) each
time a ruling is made, also when it was there before and open to others, on a system with such modes.

A ruling waits for 24 hours. After that no session is told, and the next `jellymoon rule` removes its files.

## A tap on a clash

`jellymoon settle --date <day> --clash <n> --option <1|2>` is the tap (`jellymoon_core::settle`): clash `n` of the
day's kept page, counted from 1 among the first eight, and its `quote_a` (1) or `quote_b` (2) as the one that
stands. The app's window calls the same function with the page data on show. It prints one line and exits 0:

```json
{"ruling":{"id":"…","stands":"…","over":"…"},"stands":"…","replaces":"…","unkept":null}
```

`stands` and `replaces` are the exact words of the ruling. `ruling` is null when that statement stands already.
This is the only way text gets from a page into a running session, so what is checked is stated here exactly.

- **What is sent.** The two statements of one clash, one as what stands and the other as what it replaces. They go
  through `jellymoon_core::rule` and nothing else: one line each, credential shapes replaced, 200 characters, `"`
  made `'`, and cleaned again when handed over. No label, wish, star or project is sent, and one tap makes at most
  one ruling.
- **The person sees every word that is sent.** The page draws the statements in those same cleaned words, whole
  ([app.md](app.md#taps-appsrctapsrs)), and a tap sends the statement whose drawn words were tapped.
- **The kept page**, `<home>/pages/<day>.json`, is used only when it is a plain file, not a link, that neither its
  group nor others can write to (no modes are checked on Windows), at most 1 MiB, and page data whose `day` is the
  day asked for with clashes of core's shape. Anything else is refused in one line and nothing is written. That is
  all that is checked: the file is not compared with the session logs again and carries no mark that the reading
  wrote it, and the modes of the folders above it are not looked at. Whoever can write that file as the person can
  choose the words of the next ruling the person taps, within core's cleaning; the same account can write
  `rulings/` itself. A kept page changed after the reading is ruled on in the words it is drawn in.
- **The settlement**, `<home>/pages/<day>.ruled.json`, says which side of each clash stands: the day, and for each
  clash ruled on its two cleaned statements and `"stands": "a"` or `"b"`. No ruling is ever made from it. It is
  believed only when it is a plain file the group and others cannot write to, at most 64 KiB, of exactly this shape
  with no other field, of this day, with at most eight clashes; anything else is no settlement. A clash is known by
  its two statements, so a reading that words it differently shows it open again. The file is never sent anywhere.
- **What stands** is asked of the ruling store first: the newest waiting ruling of these two statements, either way
  round, from any day and whether a tap or `jellymoon rule` made it. So a ruling made on the day before's page with
  the same two statements draws today's clash as chosen. Only the newest 256 waiting rulings are looked at, each up
  to 8 KiB. Only when none of them is of this clash is the settlement asked. A tap on the statement that stands
  makes no ruling; a tap on the other makes a new one with the two the other way round. A clash the reading found
  settled has no ruling of the person's, so a tap on either side makes one.
- **The order of writes** is the lock, the ruling, the settlement. A lock file, `<day>.ruled.lock`, made with
  create-new, is held from reading what stands to the last write, so two taps at once on two clashes both keep
  theirs and two on one statement make one ruling. A tap waits 3 seconds by the clock for it, trying every 10 ms,
  and is then refused in one line with nothing changed. A lock is one a crash left, and is removed and taken, when
  it is older than 30 seconds, when it is dated ahead of now, or when its age cannot be read. A folder at the
  lock's name is never removed: every tap on that day is refused after the same 3 seconds until it is taken away.
  Two taps that both find a stale lock can each remove it and take it in turn; this was not seen in 80 rounds and
  is not ruled out. `pages` is made the person's alone (0700) only once the lock is held. The settlement is written to a
  file of the process's own, made with create-new and 0600, and moved into place; a link waiting at either name is
  replaced, never followed, and a failed write removes its temporary file.
- **When the settlement cannot be written** the ruling stands: exit 0, the ruling in the JSON, the reason in
  `unkept`, and one line on stderr, "the ruling was made, but the page could not keep it". The same holds after a
  crash between the two writes. For the 24 hours the ruling waits it says what stands, so the same tap again makes
  no second ruling and writes the settlement if it now can, as long as the ruling is among the newest 256 that
  wait: behind more than that it is no longer seen, and the same tap makes a second ruling. After those 24 hours a
  settlement that was never written is forgotten: the clash is drawn open, and a tap makes a ruling again.
- **A refusal writes nothing**: no kept page, a clash that is not on it, an option that is not 1 or 2, a statement
  that is empty once cleaned, two statements that are the same once cleaned (they differ only in spacing or quote
  marks, or each is a credential shape that was replaced: a ruling of a statement over itself says nothing, and the
  same tap again could not be told from the first), a lock that was not freed. One line on stderr, exit 1.
  `--clash` and `--option` take digits only (`+1` is an argument error, exit 2).
- **What is not trusted.** A model's reading reaches a ruling only as the two quotes of a clash. Core kept each only
  as an exact, whole-word piece of something the person typed that day ([page.md](page.md)). The model chose which
  piece, and a piece can mean other than the whole ("delete the database" out of "do not delete the database"), so
  what protects the person is that both statements are on the page, in full, where they tap.
- **Changing one's mind.** The first ruling is not removed. A session that was told the first is told the second at
  its next step, as a newer decision that replaces what was said before. A session that was told neither is told
  both in one step, oldest first, so the later one is last. Whether a model always follows the last of two opposite
  rulings is unproved.

### Turning delivery on from the app

A tap on a clash never changes Claude Code's settings. `jellymoon hook install --installed`, and the app's "Turn on
delivery" (`jellymoon_core::turn_on_delivery`), install the hook for the `jellymoon` command in the folder
`install.sh` uses (`JELLYMOON_BIN_DIR` or `~/.local/bin`). Claude Code runs that file at every step of every
session, so it is named only when (`jellymoon_core::installed_command`):

- the folder is an absolute path, and so is the settings file's (`CLAUDE_CONFIG_DIR`; a relative one is refused by
  the plain `jellymoon hook install` as well, in one line);
- the file is a plain file named `jellymoon`, not empty, and not a link: `install.sh` copies a file, so a link at
  that name is not what it put there, wherever it leads;
- it can be run, neither its group nor others can write to it, and it is not setuid or setgid;
- its owner is the owner of the person's home folder (`HOME`). That is how "the person's own file" is told with no
  call of the system's own: the two owner ids are read from the two files and compared;
- no folder on the way to it can be written to by its group or by everyone, unless it is sticky (as `/tmp` is:
  there only a file's owner can replace it). Every folder is looked at, from the root down to the install folder.
  A folder that is a link is allowed; the link is read and the path it gives is walked the same way, at most 32
  links, so a folder that only a link passes through is looked at too. `~/.local/bin` under an ordinary home passes.

What is not looked at: who owns the folders (a folder owned by another account that only that account can write to
passes); access lists and file flags beyond the mode bits; whether the file is a `jellymoon` at all, beyond its
name. The check is made once, just before the entry is written, and the file and folders are not held open
between the two: whoever can change them in that gap, or later, is not seen. The check narrows who that can be to
the person and the system's administrator. On Windows there are no such modes and none of this is looked at: the
file must be a plain, non-empty `jellymoon.exe`, not a link, in an absolute folder, and who may write there is the
system's rule for that folder. Every refusal is one line, exit 1, and writes nothing.

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
