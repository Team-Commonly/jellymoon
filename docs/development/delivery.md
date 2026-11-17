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
that reads as another ruling. A statement is cleaned again until a cleaning changes nothing, and cut only then
(`still.rs` in core; why is in [wish.md](wish.md), "What is kept"): the words a ruling carries are the words the
next tap compares. Up to 8 cleanings may change it and the one after them must change nothing; the cut is not
counted, and what it leaves is cleaned again within the same 8. That is the whole rule: how many keys in a row it
allows depends on their shapes and on where the cut falls (wish.md has the example of keys of letters alone).
Because the cut comes last, a statement can hold words from past the first 200 characters of what was typed, when
a later cleaning made the text before them shorter; those words are ones the cleaning leaves as they are. A statement a ninth cleaning still changes, or
of over 4096 bytes (4096 are cleaned), cannot be ruled on: `jellymoon rule` and a tap refuse it in one line, exit 1,
with nothing kept, since its ruling would tell a session only that `[redacted]` stands. So is a statement of which
cleaning leaves nothing but `[redacted]`, in the sentence a wish of only marks is refused with
([wish.md](wish.md), "Only marks is nothing"). A page still draws such a
statement, as `[redacted]` and nothing else, and so does the hook for a ruling file that holds one; a tap on it in
the app says "That tap did nothing." and why. A ruling without both statements is refused. The file is written under another name
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
  and is then refused in one line with nothing changed. A folder at the
  lock's name is never removed: every tap on that day is refused after the same 3 seconds until it is taken away.
  What the lock promises, when one is taken over and what a crash leaves is "The lock" below.
  `pages` is made the person's alone (0700) only once the lock is held. The settlement is written to a
  file of the process's own, made with create-new and 0600, and moved into place; a link waiting at either name is
  replaced, never followed, and a failed write removes its temporary file.
- **On Windows** a file another process is using at that moment cannot be made, opened, removed or renamed over:
  the step fails with "Access is denied" or a sharing violation, where unix lets it through. Between two changes
  to one day that happens at four steps: making the lock while the last holder's is being removed, removing the
  lock while a waiter looks at it, reading a kept file while it is replaced, and moving a new file over one being
  read. So on Windows only, each of the four is tried again every 10 ms for the same 3 seconds (`busy.rs`), and a
  lock that still cannot be made is "another tap on this day has not finished", as on unix. The cost: a folder
  nobody may write in gives the same error, so there a refusal takes 3 seconds, and then says the system's own
  words. Seen once in three CI runs before this, in eight wishes made at once; that test now runs 12 rounds and the
  taps' 24. A run's `runs/clean.lock` is not of this kind: it is the system's lock on a file that is never
  removed or renamed, and a copy's record is read only behind it.
- **The lock** is the day's, and `projects.lock` of the list of projects is the same lock
  (`crates/jellymoon-core/src/lock.rs`): a file only one caller can make, removed by that caller when its change is
  done. Its maker writes one line into it, its mark: the process, the moment and a count, which no other lock
  holds.
  - **Promised.** While a caller holds the lock, and its change takes under 30 seconds, no other caller holds
    it and nothing removes it. A lock is removed behind the system's lock on `take-over.lock` beside it (in
    `pages`; in the home for the list), by what a look made there found: so what is removed is the lock that was
    looked at, and no two callers act on two readings of one lock (but for a holder that could not have the
    system's lock, under "Not promised"). A caller that waits removes only a lock that
    was *left behind*: something at the name that is no folder and was last written more than 30 seconds from
    now, before or after, since a clock that was set back dates a lock ahead. Nothing at the name, a lock that
    cannot be looked at (on Windows, one that is being removed), a lock whose time cannot be read and a lock dated
    up to 30 seconds ahead are waited for, never removed. A holder removes only the lock that holds its own mark.
  - **A take-over.** A lock left behind is removed only behind the system's lock, and only when a second look,
    made there, finds it left behind still. So of all the callers that found one lock left behind, one removes
    it, and none removes the lock the next holder made. Each then asks for the name again, as any caller does,
    and one of them gets it. `take-over.lock` is opened without following a link: on unix a link at its name is
    refused by the open itself, and on Windows it is the link that is opened, never what it leads to.
  - **A lock nobody can take over.** Where the system's lock is not to be had at all (its file cannot be made
    or opened, something that is no plain file is at its name, the disk gives no such locks) nothing is taken
    over. A lock a crash left there stays for ever, and every change to that day, or to the list, is refused
    after its 3 seconds in words of its own: `<the lock's name> is a lock that was left behind, and it could not
    be taken over, as take-over.lock beside it could not be locked; remove both, then try again`. The way out is
    what it says: remove the two files (or whatever stands at `take-over.lock`) from `pages`, or from the home for
    `projects.lock`. While another caller holds the system's lock, the refusal is the usual one: that caller
    is at work. On Windows "denied", a sharing violation and a lock violation are what `take-over.lock` gives
    while another process opens or locks it, so there they too are a caller at work and get the usual refusal,
    never this one; a `take-over.lock` that may truly not be written says "denied" as well, so on Windows a lock
    left behind beside one is refused in the usual words, and letting go there takes its 3 seconds each time. A
    process with no file handle left is the same on every system: that passes, and says nothing of the file.
    On unix the open of `take-over.lock` never waits, so a pipe at its name is refused as a folder is.
  - **Letting go.** A holder asks for the system's lock, waits up to 3 seconds for it while another caller
    holds it, and behind it removes the lock at the name when that lock holds its own mark, however long it held
    it. A lock that was taken over is gone, and what is at the name then is another caller's: it stays. Where the
    system's lock is not had, after those 3 seconds or because it cannot be had at all, the holder still removes
    a lock of its own that is not left behind: left there, nothing would ever remove it, which is worse than the
    one case this leaves open (a clock set during the change, below). A lock of its own that is left behind by
    then stays, for a take-over.
  - **How long a caller waits.** For the lock, 3 seconds, and it is then refused; a made wish asks again up to
    its own wait ([wish.md](wish.md)). A take-over never waits: it asks for the system's lock once. Letting go
    waits at most 3 seconds for the system's lock and, on Windows only, 3 more for a lock in use at that moment.
    Nothing waits for ever.
  - **What a crash leaves.** A command killed while it holds the lock leaves the lock's file. For 30 seconds
    every change to that day, or to the list, is refused after its 3 seconds; then the next caller takes the lock
    over. `take-over.lock` is an empty file for the person alone, made by the first change in its folder and
    never removed. A command killed in the middle of a take-over, or of letting go, leaves nothing more: the
    system drops its lock. A lock whose mark could not be written, as on a full disk, is removed at once, and
    its caller gets the system's error.
  - **Not promised.** A change that takes over 30 seconds, as when the computer sleeps in the middle of one, is
    not protected: its lock can be taken over while it is still at work, and two changes then write the day. A
    folder on a disk whose clock is more than 30 seconds from the computer's gives no lock at all, as every lock
    made there counts as left behind; where that disk gives no file locks either, a caller that waits is told
    to remove a lock that is held, in the words of a lock nobody can take over. A clock set during a change is
    not protected either: the lock can count as left behind while its holder is at work; and where the holder
    cannot have the system's lock as well, it can find its lock in time while a caller that waits, behind the
    system's lock, finds it left behind. Then either removes what it did not look at, in one of two orders: the
    caller that waits removes the holder's lock, a third makes its own, and the holder removes the third's; or
    the holder removes its own, a third makes its own, and the caller that waits removes the third's. A mark is
    one lock's alone on one machine: it names the process, the moment to the nanosecond and a count, with nothing
    random in it, so two machines or containers that share the folder could write one mark, if their processes
    had one number and made their locks in the same nanosecond, and one then took the other's lock over. A lock
    a crash left with a time that cannot be read, or dated less than 30 seconds ahead by a clock set back, stays
    until its time is 30 seconds past or the person removes the file. A `jellymoon` built before these rules
    removes a lock as described next, and takes part in neither the system's lock nor the marks. On Windows
    nothing of this was run by its builder: it rests on the tests there.
  - **Before these rules** a caller that waited removed whatever was at the name whenever its look at the lock
    failed, for any reason. One reason is that the holder had just let go: by the removal the name could be the
    next holder's, and two callers then wrote behind one lock. In one CI run on Windows that lost a made wish:
    eight wishes were made at once, each command exited 0 with its line, and seven were kept. Two callers that
    both found a lock left behind could also each remove it, the second removing the first's new lock. After
    that was mended a holder still let go by its own clock and outside the system's lock: one that let go at
    its thirtieth second, one whose lock was dated ahead, and one whose lock had been taken over could each
    remove the lock of the next caller. All are shown by `crates/jellymoon-core/src/lock/tests.rs` and
    `lock/tests/corners.rs`, which hold each of many threads between its look and what it does about it;
    unslowed, the first showed in no take of 3600 on a Mac.
- **When the settlement cannot be written** the ruling stands: exit 0, the ruling in the JSON, the reason in
  `unkept`, and one line on stderr, "the ruling was made, but the page could not keep it". The same holds after a
  crash between the two writes. For the 24 hours the ruling waits it says what stands, so the same tap again makes
  no second ruling and writes the settlement if it now can, as long as the ruling is among the newest 256 that
  wait: behind more than that it is no longer seen, and the same tap makes a second ruling. After those 24 hours a
  settlement that was never written is forgotten: the clash is drawn open, and a tap makes a ruling again.
- **A refusal writes nothing**: no kept page, a clash that is not on it, an option that is not 1 or 2, a statement
  that is empty once cleaned, a statement that cannot be cleaned (above), two statements that are the same once cleaned (they differ only in spacing or quote
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

`install.sh` makes the folders it has to make for the person alone to write (0755), whatever the umask, so the
command it places passes. A bin folder that was already there keeps its mode; when its group or everyone can write
to it the installer's closing lines say that the app will not offer the tap, and why. The plain
`jellymoon hook install` from a terminal names the program that is running and makes none of these checks. On a
Mac the system itself follows 32 links and no more, so there a longer chain ends as "it was not found".

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
