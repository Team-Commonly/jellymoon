# 0025 · The wish is shaped before a run

Status: Built as F91, 2026-11-23. It adds to [ADR 0014](0014-a-run-is-started-headless.md) (a run's prompt) and
uses the door of [ADR 0023](0023-a-runs-work-is-told-as-a-result.md) (a model's text is checked, then quoted).

## Why

A person who does not program types a wish in a few words, and a run takes those words as they are. Between Enter
and the run, Jellymoon asks the person's own Claude Code, headless and read-only, to say the same wish more sharply,
and shows the sharper wish with two choices. A model's text reaches the person here, so the check of that text is
the heart of the decision.

## Decisions

1. **One call, before anything is kept.** `jellymoon shape --say <words> --project <name>` checks the words as a
   wish's are checked and finds the project's folder as `wish --run` finds it. Only then is the harness started,
   once. No wish id is used while a shape waits, and no run is started.
2. **The command line is fixed**, one list in core (`run/shape.rs`, `LINE`), found and started as the reading call
   is (`find::named_or_found`, `call::harness_of`: a cleared environment and the short list of variables):

   ```
   claude -p --restricted --strict-mcp-config --permission-prompts none --tools Read Glob Grep --model haiku
     --disable-slash-commands --no-session-persistence --output-format json --system-prompt <fixed words>
     --add-dir <the project's folder>
   ```

   It starts in a new folder under Jellymoon's home (`shaping/call-…`, for the person alone, removed afterwards),
   never in the project. Its input is two lines core builds: `The project is in the folder: <folder>` and
   `The wish: <the typed words>`. It has 20 seconds, and 64 KiB of what it prints are read.
3. **The system words are fixed** (`PROMPT`). They ask for one JSON object and no other word: `clear`, `wish`,
   `done_when`, `not_touching`, `why`, `ask`, each text under 120 characters.
4. **The check.** The harness's reply is refused when it is no reply, exited with an error or says `is_error`.
   The model's text has its fence taken off and is parsed into six fields, an unknown or a missing field refused;
   the whole is under 8 KiB; every text goes through the door a run's result goes through (one line, credentials
   replaced, nothing unseen, not only marks), the wish within 280 characters and each other text within 160.
   No text holds `|`, `“` or `”`, the marks the run's prompt is built with: a model's text could else read, to
   the run, as a label of Jellymoon's or as the person's own words. An answer whose prompt core could not build
   (decision 7) is thrown away too: a sharper wish that no run could be given is never shown.
   **Any failure throws the whole answer away**: the outcome is `as_typed`, and why is one of Jellymoon's own fixed
   sentences. No word of the model's or the harness's is ever in an error.
5. **`shape` exits 0** and prints `outcome`: `clear` (the words are sharp enough; nothing is kept), `shaped`, or
   `as_typed`. A usage error, and a home that cannot be used, fail as they do for every command.
6. **A shape waits in one file**, `pages/<day>.shape.json`: one a day, a new one in the place of the old, with the
   typed words and the project, written for the person alone. It is read back within a size and checked again.
7. **A wish is kept from a shape only for the same words and project.** `jellymoon wish --shaped` keeps the wish
   with `shaped`: the run's prompt, built by core on one line from fixed labels, with the person's own words
   quoted last. With no such shape the wish is kept as typed, and the command says so; the window says so too
   when `Go with this` was tapped and the wish could only be kept as typed (the day's file was busy). `said` is the typed words
   everywhere; a run is given `shaped` when the wish has one. The shape is removed when a wish is kept from it.
8. **In the window** Enter shows at once that the wish is being read. `clear` and `as_typed` go on as a typed
   wish always did. `shaped` keeps nothing and shows the sharper wish, every word of the model's a quote under a
   label of Jellymoon's, with `Go with this` and `Go with what I typed` (`docs/development/app.md`).

## Measured

By the owner, on Claude Code 2.1.330, macOS only; none of it by the tests, which never start Claude Code.

- The line above exits 0 with plain JSON in `result`, no fence, `is_error` false: about 6 seconds with `haiku`,
  in 5 turns.
- Files in the `--add-dir` folder are read from a call started elsewhere.
- A `Read` forced outside both folders is refused by the harness: one permission denial, and no word of the file
  in the answer.
- A project with `SessionStart` and `PreToolUse` hooks in both `.claude/settings.json` and
  `.claude/settings.local.json` ran neither, and an instruction in the project's `CLAUDE.md` was not obeyed.

- The texts ran 125 to 140 characters before the system words named a length, which is why they say "under 120".
- The `why` text is the model's own claim and no check holds it against a file. Twice, of a file of three lines,
  it spoke of an empty fourth line that the file does not have; and it named the file though the system words ask
  it not to. So the page draws it as a quote, last of the rows, and no run is given it.
- Walked without a window in a made-up project, through the app's own calls: Enter to the sharper wish took
  about 9 seconds, `Go with this` started the run, and the run's result came as a typed wish's does.

Not measured: Windows, Linux, an older Claude Code, and a link inside the project that points outside it.

## Open

- The app does not end the shaping call when it quits: the child goes on to its 20 seconds at most, and its
  `shaping/call-…` folder is left. At the limit only the child itself is ended, not what it started.
- The entry takes no key from Enter until the reading's answer, 20 seconds at most.
- A wish refused after a tap (the day has its eight wishes; the project is no longer on offer) leaves a note with
  the start of the typed words, not the whole of them in the entry.
- Enter while a sharper wish stands reads the new words in its place; the first words are gone with no note.
- A shape made by the command for the same words on the same day, while a pair stands in the window, is the one a
  tap keeps: the text is checked, and it is not the one on show.
- A build before this one refuses a day's made wishes when one of them holds `shaped`.
- The model reads what the project holds, so a file's text can steer what the sharper wish says. It can only
  reach the person as a quote that passed the check, and a run only with the person's tap and their own words
  beside it.
