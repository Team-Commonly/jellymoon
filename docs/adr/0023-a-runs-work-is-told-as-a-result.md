# 0023 · A run's work is told as a result the model composes from pieces Jellymoon checks

Status: Written by a designer on the owner's rulings, 2026-11-17. Nothing here was built or run. Every "today" was read in the code at `303ece4`. It
adds to [ADR 0017](0017-a-runs-work-is-shown-kept-or-tossed.md), decisions 2 and 9, and [ADR 0018](0018-a-run-asks-and-is-told-more.md), decision 2.

## Why

The owner: "how can user see/understand the work", "I thought we want more of like generative UI", and earlier "the real bottleneck is human, human
don't have context". A finished run's note says the wish's words, core's counts, the files' names and three choices. To know more a person reads files.

**The measure.** A person who made a wish and walked away understands in about ten seconds, reading no file: what was done, in their words; whether it
works and how that is known; what the run decided alone or is not sure of; and what `Keep it` would put in their repo. Then they keep or toss.

## What the code is today

- **The work's note is core's.** `run/waits.rs`: `Waits` holds the wish's words, the counts, six names (`NAMED`), `obeyed` and `plain`; `title`,
  `lines`, `choices` and `asks` word the note; `WorkTap::of` reads a tap from what was drawn. `run/show.rs` (`whole`, `show_file`) reads the work
  through `copy/work.rs` (`work_of`): `diff-tree` and `cat-file`, each file as it now is, 64 KiB of it. No file is read as it was before the work.
- **The app carries it.** `app/src/working.rs`: `waiting` puts `Waits` in a `Note::Text` (`work-<wish>`), `notes_of` answers a tap, `with_work`
  prints the notes for `layout`.
- **What a note can draw.** `sticker-kit/src/scene.rs`, `Note::Text`: a title, `body` texts each from its own line, options, one `quoted`, a field.
  `layout/text_note.rs` draws the texts that fit and counts the rest (`+N more`) and places options only under a title and a body drawn whole.
  `layout/notes.rs` (`height_needed`) plans a text with options at its whole body, and one without at two lines and the room later notes leave.
- **How a model's words are told from Jellymoon's.** Jellymoon's sentences are a note's title and body. The person's words stand in the body between
  `“` and `”`. An agent's words (a run's question, `answering.rs`) are `quoted`: one `Quote` piece on a ground of its own, in `Role::Quoted`
  (`layout/typed.rs`). A note has one, and it is cut at eight lines with `…`, not counted.
- **How page data from the model is checked.** `reading.rs` ([page.md](../development/page.md), "What code checks"): a quote is kept only when found
  in what the person typed, ids and counts are code's, the model's few words are cleaned and cut, and a failing piece is dropped alone.
- **What core keeps of a run.** `run/stream.rs` keeps counts, the first result line's `subtype`, `is_error` and `permission_denials`, and its last
  word: no tool line and no prose. The last word is `structured_output` under `--json-schema` (`run/asks.rs`, `SCHEMA`, `last`): exactly
  `{"state":"done"}`, or `asks` with one question; anything else is `Unfit` and the run has failed. A run has five tools, `Read`, `Edit`, `Write`,
  `Glob`, `Grep` (`run.rs`, `ARGS`), and its `PROMPT` says "You cannot run commands". `run/record.rs` keeps `runs/<id>/run.json`.

## Decisions

1. **A run ends with its result in its last word (ruling 3, a).** The last word reaches core today, as data under a schema, so no second call is
   made: the schema's `done` takes one more part, `result`, and `PROMPT` asks for it. A reading call over the work (b) is a second call and knows
   the change and not why. **A result never fails a run:** `asks::last` reads `done` alone, or with a `result` of any shape, as done. A `result`
   that is not the shape below, or over 8 KiB as written, is no result. `asks` with one is `Unfit`, as now.
2. **Core keeps what the run said and checks it at every look.** The run's end writes the shape it read, at most 12 pieces, to `runs/<id>/result.json`
   before the record's end. A file there is anyone's to write, so every piece is checked against the work at each look, by `run show` and by `waits`,
   and never when it is kept. No word of it reaches `app.log`.
3. **The result is a note of its own, over the work's note.** `Keep it`, `Toss it` and `Show me` stay on `work-<wish>` with every word and every rule
   of the yes: its lines, `sure-<wish>`, `OVER` and `WorkTap::of` are not touched, and no file name moves off it. The result's note has no option,
   so the kit plans the work's note whole first and gives the result the room that is left. On the work's own note a long result would take `Keep it` off the page.
4. **A piece that fails its check is dropped whole**, and the note then ends with Jellymoon's line "Something the run said could not be checked, so it
   is not here." With no piece left there is no result note: today's note alone.
5. **The run's words are drawn as quoted words, and only there.** The kit gets one note, `Note::Told`: a title and rows, each a label in the note's
   own role and texts each a `Quote` piece. A row is drawn to its last character or left out and counted (`+N more`), as `Note::Whole` does it. Labels
   are core's fixed words and core's cleaned file names. Nothing the run says is a title, a label, a body text or an option, so it cannot read as
   Jellymoon's sentence and no tap finds it. The note takes no tap and has no field.
6. **Nothing was run, and the note says so.** A run cannot run a command and core keeps no line of what it did, so no piece says something was
   checked. Every result note carries Jellymoon's line "Nothing was run to try this. A run reads and edits files, and no more."
7. **F85 to F87 build it. Next after them (ruling 7):** a run that asks (F52, F54) carries its question's context in the same shape; not designed here.

## The catalogue

Order on the note is fixed: `did`, `change`, `decided`, `unsure`, whatever order the run wrote. A `text` passes the door a question passes (`still.rs`),
and a quoted line is cleaned as a file's text is (`still_text`, which leaves a `"` as it is). A text or a line that is empty, over its length, holds a
character that draws as nothing or reads like a note's count (`obeyed.rs`, `plain`) is dropped, never cut.

| Kind | Fields | Check | Bounds | Label |
|---|---|---|---|---|
| `did` | `text` | the door: a credential shape becomes `[redacted]`, a `"` becomes `'`, and a text that is only marks is dropped | the first one; 280 characters | none: it is the first row |
| `change` | `file`, `before`, `after` | `file` is, byte for byte, the path of a listed file of the work that is text, and its cleaned name is its name as git keeps it. Each `before` line is a whole line of the file as it was and each `after` line of the file as it is, spaces and tabs at both ends aside. The file had a `before` line more times before the work than after, and has an `after` line more times after. A made file takes no `before`. A line the door would change fails, and one line that fails drops the piece | the first two; three lines a side, one at least in all; 100 characters a line | `<name>, before`, `<name>, after`; of a made file `<name>, new` |
| `decided` | `text` | as `did` | the first three; 160 characters | `It decided this alone` |
| `unsure` | `text` | as `did` | the first three; 160 characters | `It is not sure of this` |

- **Counts and the files are core's**, never a piece: they stay on the work's note, as today. **Cut from the owner's list:** *what the run did, in
  order* (core keeps nothing of the order, so a step could not be checked, and `did` says it) and *what was checked and how it came out* (decision 6).
- **Limits, not built:** a picture of the result (a rendered page or window needs the work run or rendered safely, a design of its own); a table or a
  chart of data the work made; sound; and `checked`, which comes back when a run can run a command and core keeps its exit from the stream.
- **One new question of git:** `cat-file blob` of the id `diff-tree` gives for a file as it was, for two files at most, read to 64 KiB
  ([show.md](../development/show.md), "What is asked of git", gains the row with F85).

## The shape

There is no `schema/` folder: a wire shape is a serde type in core (`shapes.rs`, `run/show.rs`) with `deny_unknown_fields`, and a union is an enum
tagged by `type`, as the kit's `Note` is. What the harness is given (`asks::SCHEMA`, from F87) has no `oneOf`: `type` is an enum, and which fields go
with which `type` is core's check. The `x-variants` line says that check here and is not sent.

```json
{"type":"object","required":["state"],"additionalProperties":false,"properties":{
  "state":{"type":"string","enum":["done","asks"]},"question":{"type":"string"},
  "result":{"type":"object","required":["pieces"],"additionalProperties":false,"properties":{
    "pieces":{"type":"array","items":{"type":"object","required":["type"],"additionalProperties":false,
      "properties":{"type":{"type":"string","enum":["did","change","decided","unsure"]},"text":{"type":"string"},"file":{"type":"string"},
        "before":{"type":"array","items":{"type":"string"}},"after":{"type":"array","items":{"type":"string"}}},
      "x-variants":{"did":["text"],"decided":["text"],"unsure":["text"],"change":["file","before","after"]}}}}}}}
```

What core prints, checked: `Shown` and `Waits` each gain `result`, left out when there is none. `n` is the file's number in the list and `name` core's
cleaned name of it, never the run's spelling; `dropped` is how many pieces failed a check.

```rust
pub struct Told { pub pieces: Vec<Piece>, pub dropped: usize }
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Piece {
    Did { text: String }, Decided { text: String }, Unsure { text: String },
    Change { n: usize, name: String, made: bool, before: Vec<String>, after: Vec<String> },
}
```

## Two worked examples, typed by hand

**One function and its test.** The wish: "Free shipping should start at 50, not 100". The last word:

```json
{"state":"done","result":{"pieces":[{"type":"did","text":"Shipping is now free from 50, not 100. The test tries 49 and 50."},
 {"type":"change","file":"src/shipping.rs","before":["    if total >= 100 { 0 } else { 8 }"],"after":["    if total >= 50 { 0 } else { 8 }"]},
 {"type":"decided","text":"An order of exactly 50 ships free."}, {"type":"unsure","text":"I could not run the test."},
 {"type":"change","file":"README.md","before":[],"after":["Free shipping from 50."]}]}}
```

`README.md` is no file of the work, so the last piece is dropped. A person reads two notes; a first line is a title, `▏` the ground of a `Quote` piece:
```text
The run says this of its work.
  ▏Shipping is now free from 50, not 100. The test tries 49 and 50.
  src/shipping.rs, before
  ▏if total >= 100 { 0 } else { 8 }
  src/shipping.rs, after
  ▏if total >= 50 { 0 } else { 8 }
  It decided this alone
  ▏An order of exactly 50 ships free.
  It is not sure of this
  ▏I could not run the test.
  Nothing was run to try this. A run reads and edits files, and no more.
  Something the run said could not be checked, so it is not here.

Work waits for this wish.
  “Free shipping should start at 50, not 100”
  the work of run r0123456789abcdef waits: files made 0, changed 2, removed 0.
  src/shipping.rs, tests/shipping.rs
  [Show me] [Keep it] [Toss it]
```

**A small web page.** The wish: "Make a page that says we bake on Saturdays". The last word:

```json
{"state":"done","result":{"pieces":[{"type":"did","text":"There is a new page, index.html, with a heading and one line about Saturdays."},
 {"type":"change","file":"index.html","before":[],"after":["<h1>We bake on Saturdays</h1>","<p>Come by from 8 in the morning.</p>"]},
 {"type":"decided","text":"I made up the opening time, 8 in the morning."}, {"type":"decided","text":"The page has no colours or pictures yet."}]}}
```

```text
The run says this of its work.
  ▏There is a new page, index.html, with a heading and one line about Saturdays.
  index.html, new
  ▏<h1>We bake on Saturdays</h1>
  ▏<p>Come by from 8 in the morning.</p>
  It decided this alone
  ▏I made up the opening time, 8 in the morning.
  ▏The page has no colours or pictures yet.
  Nothing was run to try this. A run reads and edits files, and no more.
```

Under it is the work's note as above, with `files made 1, changed 0, removed 0` and `index.html`. The tags are text. The page itself is not drawn.

## Turned down

- **Free-form text or markdown from the model**: no part of it can be checked, and it could read as Jellymoon's.
- **HTML or a web view**, and **the app running or rendering the work**: an agent's code run in the app, which ADR 0017, decision 3, forbids.
- **A second call for every run**: it spends the person's subscription with no tap (ruling 3).
- **A2UI's message as the result's shape** (the open format for a UI an agent composes, v1.0, `https://a2ui.org/specification/v1.0-a2ui/`): the one such format a native window can draw with no web view, and it was read for this. Its basic catalogue is layout and form controls, with nothing for quoted lines before and after; a piece would be a box of text whose kind the check cannot see; and its bindings, functions, callbacks and media by URL would each be refused. The result stays a short ordered list of typed pieces, each a claim core can test. No kind takes a second meaning for one of A2UI's words, so a result can be written out as one `createSurface` if another host ever matters.

## Where the rulings meet the code

- **Ruling 2, "run and passed":** not reachable. A run has no tool that runs anything and core keeps no tool line (decision 6).
- **Ruling 2, "every piece is grounded":** true of `change` only. `did`, `decided` and `unsure` are the run's say-so: core holds them to the door and
  to the quoted ground, and cannot hold them to the work.
- **Ruling 3:** today a last word with any part beside `state` fails the run. Decision 1 loosens that for `done`.
- **Rulings 4 and 5, room:** at 960x640 the work's note with six names and three choices takes the column (`waits.rs`, `NAMED`), and the result is
  then its first rows and a count. The measure is met where there is room.
- **Ruling 6:** the kit's one `quoted` is cut with `…`, not counted, so the kit gains `Note::Told` (F86).

## Not confirmed

- Whether Claude Code holds a nested schema with arrays under `--json-schema`, and how large a last word it writes: only the two-part schema was
  measured (ADR 0018, "Measured", 2.1.323). And how often a model quotes a line to the character: a `change` that fails is dropped, so a low rate
  shows as results with no before and after. F87 measures both on real runs before its row closes.
- That the `done` of a run that asked and was answered (`answer.rs`) carries a result the same way; the room at each window size; Windows and Linux.

## Found while building F86

- **The words are the app's, not core's** (`app/src/told.rs`), while core is left to F87: the title "What it says it did", which is
  also the label of what the run did; "Changed in" and "New file"; "Decided" and "Not sure about"; the small words "before" and "after"
  beside a change's two sides; and the two sentences of decisions 4 and 6, as written there.
- **A file's name is the run's word.** It shares its label's line ("Changed in src/shipping.rs") only when the two lay out as that one
  line and the name holds no blank, unseen or control character; else the label stands alone and the name is the row's first quote. So no
  name reads as a line of Jellymoon's own, such as a name ending in " Decided" or holding a sentence.
- **A change is one block**, drawn whole or not at all; rows are drawn in order up to the first that does not fit, and counted from there.
- **A result costs no other note anything, by construction.** With a result on the page the kit lays the notes out twice, with and
  without the result's notes; where a note drawn in both differs but for its place, or the note a drawn result is over is drawn only
  without it, the layout without the result is the one shown. A later note may give way whole, counted. While a result's note is on the page,
  the count of the notes left out is one plain line, not a sheet, so a typical result (a did, one change, a decision) is read whole at
  1440x900.

## Found while building told alone (2026-11-18)

- **Decision 3 is widened to a run with no work.** A run that finished and changed nothing has no work's note, so
  its result's note stands alone over the wish's own note ("This wish is done."), above it, with no option and no
  field. `run show` prints its `result` beside `"files":[]`; with no file to check one against, every `change` is
  dropped and counted, and `did`, `decided` and `unsure` pass as they do with work. The app asks core through
  `ended_alone` ([show.md](../development/show.md), "Told alone"). The note's sentences start with the app's
  "Nothing in your folder was changed."; decision 6's sentence stays, as it is still true.
- A run that did not finish keeps no result (decision 2), so none is shown of it. Its wish's note says, in core's
  fixed words, that it did not finish, that nothing in the folder was changed, and what the person
  can do; no word of core's reason, which names the run and the harness's last word.

## Found after told alone: done with a question (2026-11-18)

- **Decision 1 is widened: `done` takes a `question` beside it, unread.** ADR 0018 ("Found while building F51") read
  `done` with a question as a failed run because "a harness that enforces the schema never sends them". It does: the
  schema lets `question` stand beside any state. A person typed a plain question as a wish, and the model answered it
  in `result` and copied the person's question into `question`; the run failed and the answer was lost, while the same
  wish on another run ended `done` alone and was shown. The state is the run's own claim, so beside `done` a
  `question` of any shape, written any number of times, is not kept, shown, asked or counted, and the `result` beside
  it is read as beside any `done`. `asks` is unchanged. The schema's `state` and `question` now say when a question
  is written ([run.md](../development/run.md), "A run that asks").

## Found while building live work: what a run is doing (2026-11-19)

- **The context said core keeps "no tool line and no prose".** That stays the rule for the end and for the record:
  `run.json` and how a run's end is read are unchanged. Beside it, the reader now keeps a run's steps while it works,
  so the page can say what the run is doing: of a tool use with one of the five tools only the tool, and for `Edit`,
  `Read` and `Write` the cleaned name of a file inside the run's copy; every step counted and the last 4 kept, in
  `runs/<id>/steps.json`. Still no prose, no tool input but that name and no tool result
  ([run.md](../development/run.md), "What is read of the stream").
- **A name on the pill.** A file's name is the run's word here too: the pill beside a working wish draws a name
  after its fixed word only when the name holds no blank, as a result's note draws a blank-free name beside its
  label; a name with a blank is left out of the pill and stands, quoted, in the note.
- **2026-11-19, later: no name on the page.** The pill and the note now say the stage ("looking around", "making
  the change") and the count, in fixed words, and draw no file's name at all, so the rule above has nothing left to
  rule: a person who asked for something does not read each file. Core still keeps the names.
- **2026-11-20: no name and no changed line on the result's first view.** The same holds at the end of a run as
  while it works: the note of what the run says it did, and the work's note under it, first come onto the page
  with no file's name and no changed line, because following and trusting a wish must not take reading a file. One
  fixed sentence of the app's stands for every change that passed core's check, and the work's note keeps core's
  count. The rows of the changes and the names come with a tap on `Show me`, drawn as decision 4 and "Found while
  building F86" say. What core checks, cleans and prints is unchanged: the app chooses when to draw it
  ([app.md](../development/app.md), "The result in plain words").
