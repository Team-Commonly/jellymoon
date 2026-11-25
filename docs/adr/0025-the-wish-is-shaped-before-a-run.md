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
   Amended 2026-11-24: `why` is read and checked as before, and it is not shown (decision 8): a text of the
   answer that fails the check still throws the whole away, the reason too. The prompt that must be buildable
   for a sharper wish to be shown is the one with no answer.
5. **`shape` exits 0** and prints `outcome`: `clear` (the words are sharp enough; nothing is kept), `shaped`, or
   `as_typed`. A usage error, and a home that cannot be used, fail as they do for every command.
6. **A shape waits in one file**, `pages/<day>.shape.json`: one a day, a new one in the place of the old, with the
   typed words and the project, written for the person alone. It is read back within a size and checked again.
7. **A wish is kept from a shape only for the same words and project.** `jellymoon wish --shaped` keeps the wish
   with `shaped`: the run's prompt, built by core on one line from fixed labels, with the person's own words
   quoted last. With no such shape the wish is kept as typed, and the command says so. The wish and its prompt
   are kept in one write, so a wish is never kept without the shape it was to go with: a wish that is refused (a
   busy day, a day with its eight) keeps nothing, and its shape still waits. `said` is the typed words everywhere;
   a run is given `shaped` when the wish has one. The shape is removed when a wish is kept from it.
   Amended 2026-11-24: **the question can be answered.** A page keeps a wish from the shape that waits for its
   words in one of three ways (`wish_of_shape`, `ShapeGo`): as typed, with the shape, or with the shape and the
   person's answer to its question. With an answer the prompt has two more parts, before the person's own words:

   ```
   The wish: … | Done when: … | Not touching: … | Asked of the person: <the question> | The person's answer:
   “<the answer>” | The person's own words: “<the typed words>”
   ```

   The question is the model's text and stands bare under its label, as the other texts of the model do; the
   answer is the person's and stands in `“ ”`, as their own words do. The answer is cleaned as a wish's words are
   (the same door: one line, nothing unseen, credentials replaced, each `"` made `'`) and is 200 characters at
   most, a wish's own length. An answer is refused with a fixed sentence of core's, nothing written and the shape
   still waiting, when the shape asks nothing, when nothing is left of the answer once cleaned, when it is
   longer, and when no prompt can be built with it. A prompt is 1300 characters at most (it was 1000): the
   longest of everything comes to 1280 (a wish of 280, three texts of 160, the words and the answer of 200 each,
   and 120 of labels and marks). The door looks at no text of more than 4096 bytes, so the longest of everything
   builds in characters of up to three bytes each, which is every script in common use; written wholly in
   characters of four bytes it is 4198 bytes and is refused with that sentence, and a shorter answer builds.
   Amended 2026-11-25, after a verifier: **a page keeps a wish only from the shape it showed.** A page hands
   core the shape it has on show, with its words and project (`ShapeSeen`), and core keeps a wish from the
   shape that waits only while the two are the same, text for text, in all three ways. A shape made since for
   the same words (the command, the same day) is another shape: the tap or the answer is refused with the
   sentence that the sharper wish on show waits no more, nothing is written, and the newer shape still waits.
   So an answer never goes with a question the person did not see, and a wish is never kept for another project
   than the page's. `wish --shaped` is as it was: it shows no shape and keeps the one that waits.
8. **In the window** Enter shows at once that the wish is being read. `clear` and `as_typed` go on as a typed
   wish always did. `shaped` keeps nothing and shows the sharper wish, every word of the model's a quote under a
   label of Jellymoon's, with `Go with this` and `Go with what I typed` (`docs/development/app.md`).
   Amended 2026-11-24: the rows are "The wish", "Done when" and "Not touching". The model's reason is not
   drawn: it is its unchecked claim, it was wrong twice ("Measured"), and no run is given it. When there is a
   question, the note of the two choices is the question's: it is titled "Only you know", the question is its
   quote, and under the choices is a field, named "Answer". Enter in it with words keeps the wish with the
   sharper wish and the answer and starts its run as `Go with this` does. The person's typed words are not
   drawn then; the window holds them, and core still checks that the shape waits for them. With no question
   the note is titled "What you typed", quotes the typed words, and has no field. (The same day, after a walk
   with a real harness: the question first stood as a last row of the sharper wish, a whole note and a quote
   of the typed words away from the field it is answered in, under a sentence for an offer.)

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
- The entry took no key from Enter until the reading's answer, 20 seconds at most. Amended 2026-11-24: it takes
  every key now, and what is typed meanwhile is the next words (the kit's `README.md`, "The entry is then owed an
  answer"). Enter gives nothing until the answer has come. Amended 2026-11-25: one Enter pressed meanwhile is
  remembered and given when the answer has come and kept the first words, so a second wish entered while the
  first is read is read next. It is not given over a sharper wish that waits (the next point but one), nor when
  the first words were refused: the words stay in the entry and Enter is the person's to press.
- A wish refused after a tap (the day has its eight wishes; the project is no longer on offer) leaves a note with
  the start of the typed words, not the whole of them in the entry.
- Enter while a sharper wish stands reads the new words in its place; the first words are gone with no note.
  That is an Enter pressed in sight of the pair. One pressed before the pair stood is never given over it
  (`docs/development/app.md`, "An Enter pressed while core is asked is not lost").
- A sharper wish that stands when the app is quit is not shown at the next open: its shape waits in its file, and
  neither it nor the typed words are on the page (found on a walk of the one-column page).
- A shape made by the command for the same words on the same day, while a pair stands in the window, was the one
  a tap kept. Closed 2026-11-25 (decision 7): a tap or an answer on the older pair is refused and keeps nothing.
  The older pair then says so and goes; the newer shape waits in its file and is on no page.
- A build before this one refuses a day's made wishes when one of them holds `shaped`. A build before the
  answer (2026-11-24) refuses them when a prompt is longer than 1000 characters.
- `wish --shaped` has no answer: the question is answered in the window only.
- The person's own words may hold `|`, `“` and `”`, and so may their answer: cleaning leaves them. A person can
  so write a part that reads, to their own run, as a label of Jellymoon's. Only a model's text is refused for
  those marks; the person is the one the run works for.
- The answer is in the run's prompt and nowhere else: the wish's bubble shows the typed words (with a question
  the pair draws them nowhere), a wish made again is made of the typed words alone, and a follow-up's run is
  given the follow-up's words.
- An answer that core refuses leaves the pair on the page with core's reason over it. When the reason is that
  the shape waits no more, the pair still stands, and the next tap on it says so and takes it away.
- The model reads what the project holds, so a file's text can steer what the sharper wish says. It can only
  reach the person as a quote that passed the check, and a run only with the person's tap and their own words
  beside it.
