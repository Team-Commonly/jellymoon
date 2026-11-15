# A wish said aloud: `jellymoon hear`

A hearing turns a held key and what the ear returned into a heard wish, shows it for four seconds, and then makes
it a wish (F55, [ADR 0019](../adr/0019-a-wish-said-aloud.md), decisions 4 to 6). The rules are core's, in
`crates/jellymoon-core/src/hear.rs` and `hear/`. Core registers no key and opens no ear: it is given events, and
says what the jellyfish shows. `jellymoon hear` is that as a command, so every rule is proved with no key, no
microphone and no window. The app's shell carries the same events to the same type, `Hearing` (F58).

```
jellymoon hear [--date <YYYY-MM-DD>] [--logs <dir>] [--skip <names>] [--run]
jellymoon hear --settings
```

`hear` reads events on its input, one JSON object a line, until the input ends. It prints what the jellyfish
shows, one line of JSON: once as it begins, and then once for each event that changes it. Each line is flushed
as it is printed. It exits 0 when the input ends. With no `--date` the day is today in the computer's time zone,
read once as the hearing begins; after that **no clock is read**. `--logs` and `--skip` are the reader's, and
say which projects a wish can be made for. `--run` starts a made wish's run, as `jellymoon wish --run` does
([wish.md](wish.md), [run.md](run.md), "A run started apart").

## Events in

| Line | What it says |
|---|---|
| `{"key":"down"}` | the key was pressed |
| `{"key":"up"}` | the key was let go |
| `{"said":"partial","words":"make the up"}` | the ear's words so far in this hold, whole each time |
| `{"said":"final","words":"make the uploader retry twice"}` | the ear's words for the hold that is over |
| `{"said":"cannot","why":"microphone_refused"}` | the ear cannot hear, and which of its eight reasons |
| `{"tick":1200}` | the time: milliseconds on any clock that does not go back |

The first `tick` starts the clock, whatever its number: nothing has passed at it. Until one comes no time passes,
so every hold is a slip and no heard wish is made. A tick earlier than the last changes nothing.

**A span that an event begins is counted from the first tick at or after that event**, never from the tick
before it: the four seconds a heard wish is shown, the six seconds a refusal is said, and the five seconds the
ear's final words are waited for. The tick before an event may be long past, and a span counted from it would
be shorter than what the person saw. So after a `heard` line 4000 ms of ticks always pass before a wish is
made, however seldom the sender ticks. A tick at the time of the last one is at or after the event, and
starts the span; one that goes back does not. What a tick itself begins (a refusal at the end of the four
seconds, the wait after a hold that ended at 60 000 ms) is counted from that tick.

**A hold's length is the one span between two events:** it is the time between the tick before the key went
down and the tick before it went up. A sender that ticks with each key event measures it rightly; one that
does not can only make a hold a slip or a slip a hold, and a heard wish still has its whole four seconds.

An example, `crates/jellymoon-core/tests/fixtures/hear-events.jsonl`:

```json
{"tick":0}
{"key":"down"}
{"said":"partial","words":"make the up"}
{"tick":1200}
{"key":"up"}
{"said":"final","words":"make the uploader retry twice"}
{"tick":1250}
{"tick":3250}
{"tick":5250}
{"key":"down"}
{"tick":5300}
{"said":"cannot","why":"microphone_refused"}
{"tick":5350}
{"tick":11350}
```

**A line that is no event** is passed over and counted, and nothing of it is printed: text that is not JSON, a
`key`, `said` or `why` this build does not know, a name it does not know beside one it does, a number where
words belong, a `tick` that is negative or not whole, and a line over 64 KiB with its line break, which is read
past: the input's last line with no line break has one byte fewer, 65 535, to be an event in. When the input
ends, one line on stderr says how many, and the exit is still 0:

```
jellymoon: lines of the input that were no event, and were passed over: 2
```

An empty line is not counted, and a line may end in `\r\n`.

**An event out of order changes nothing:** a key let go that was not pressed, a second press while it is held,
the ear's words with no hold, final words while the key is still held (they are an earlier hold's), and words
of any kind while a heard wish is shown.

## Lines out

Every line has five names, in this order, and up to three more:

| Name | What it holds |
|---|---|
| `state` | `quiet`, `listening`, `heard`, `working`, `asks`, `waits` or `cannot` |
| `says` | core's own sentence for the state; empty when the state says nothing |
| `words` | the words so far while `listening`, the heard wish while `heard`; else empty |
| `project` | the project a heard wish will be made for; else empty |
| `left` | the milliseconds a heard wish is still shown for; 0 in every other state |
| `going` | with `working`: how many of the day's runs are going |
| `question` | with `asks`: the run's question, as the door left it ([run.md](run.md)) |
| `made` | the id of the wish that was just made: on the first line after it, whatever its state, and on no other |

What the example's events print, `tests/fixtures/hear-lines.jsonl`, with `alpha` the one project offered. The
binary's test `the_events_of_core_s_example_print_the_lines_of_core_s_example` holds the two files together:

```json
{"state":"quiet","says":"","words":"","project":"","left":0}
{"state":"listening","says":"","words":"","project":"","left":0}
{"state":"listening","says":"","words":"make the up","project":"","left":0}
{"state":"heard","says":"press the key again to take it back","words":"make the uploader retry twice","project":"alpha","left":4000}
{"state":"heard","says":"press the key again to take it back","words":"make the uploader retry twice","project":"alpha","left":2000}
{"state":"quiet","says":"","words":"","project":"","left":0,"made":"m1"}
{"state":"listening","says":"","words":"","project":"","left":0}
{"state":"cannot","says":"nothing was heard: Jellymoon may not use the microphone; allow it in System Settings, Privacy & Security, Microphone","words":"","project":"","left":0}
{"state":"quiet","says":"","words":"","project":"","left":0}
```

And what the day's runs give, `tests/fixtures/hear-runs.jsonl`:

```json
{"state":"working","says":"","words":"","project":"","left":0,"going":2}
{"state":"asks","says":"a run asks, and waits for your answer on the page","words":"","project":"","left":0,"question":"Which of the two uploaders did you mean?"}
{"state":"waits","says":"a run's work waits for you on the page","words":"","project":"","left":0}
```

A line is printed only when it differs from the last. `left` is 4000 on the first `heard` line and still 4000
at the first tick at or after the final words, which starts the four seconds and so prints nothing. In the
example that is the tick at 1250: the wish is made at 5250. After it `left` changes with each tick, so each
tick prints a line: whoever sends the ticks sets how often the ring is drawn.

**When more than one state holds**, the first of these is shown: `listening`, `heard`, `cannot`, `asks`,
`working`, `waits`, `quiet`. So what the ear refuses while a heard wish is shown is said once the wish is made
or taken back, if its six seconds are not over by then.

## A hold, from the key to the wish

1. **The key goes down:** `listening`, with no words. A key pressed while a heard wish is shown takes the wish
   back, and one pressed while the ear's final words are waited for drops that hold: the ear listens anew.
2. **The ear's words so far** are shown as they come. They are cleaned as a statement is (`still.rs`): one line,
   nothing unseen, credential shapes replaced by `[redacted]`, cut to 200 characters. They are only ever shown.
3. **The key goes up.** A hold under 300 ms is a slip of the hand: nothing is heard, and the final words that
   may follow are passed over. Otherwise the state stays `listening` while the ear's final words are waited
   for, 5000 ms of ticks at most. A hold ends by itself at 60 000 ms, as if the key had been let go then.
4. **The final words pass the door a typed wish passes** ([wish.md](wish.md), "What is kept"): cleaned until
   cleaning changes nothing, at most 200 characters, never cut. Words that are empty once cleaned make no wish
   and are not said to be refused: the hold is over, so one line is printed, of what the jellyfish showed
   before the hold (`quiet`, when nothing else holds). Words the door refuses are refused in a made wish's own
   sentence, below.
5. **The project is picked** (below), and the heard wish is shown with it: `heard`, for 4000 ms of ticks from
   the first tick at or after the final words.
6. **At 4000 ms the wish is made**, by the call the page's entry uses (`wish_offered`): once, in the working
   state. The next line carries its id in `made`. With `--run` its run is then started by `run_apart`, with
   this same `jellymoon` program, the logs and the names switched off that `hear` was given, and the harness
   `JELLYMOON_CLAUDE` names: what `jellymoon wish --run` does. The hearing reads no event while it waits for
   that answer, as `wish --run` prints nothing while it waits.

**Until the 4000 ms have passed nothing is written.** A heard wish is in the hearing's memory and nowhere else,
and Jellymoon's home is not even made. A key pressed in that time takes it back: tapped, the words are dropped;
held, it listens anew, and what is said then is the heard wish. Input that ends in that time drops it too.

## Which project

Asked once for each heard wish, when its final words come:

1. the project of the day's last made wish, when a wish can still be made for it;
2. else the only project a wish can be made for;
3. else none: nothing is kept, and the jellyfish says so.

A wish can be made for a project when a folder of that name was typed in on the day, in the logs `hear` reads,
and is not switched off, which is what `wishable` offers the page's entry. A folder's name is a session log's
text: one that could not be a made wish's project (a path, something unseen, a credential's shape, over 100
characters) is never offered and never printed. The made wish is checked again when it is made
(`wish_offered`), so a project switched off in those four seconds makes no wish.

## What is written, and when

| When | What |
|---|---|
| The hearing begins | nothing. The day's made wishes are read |
| Any event before a heard wish's 4000 ms | nothing |
| A tick at or after its 4000 ms | `pages/<day>.wishes.json`, by `wish_offered`, behind the day's lock; with `--run`, whatever a run started apart writes ([run.md](run.md)) |
| Once for each 1000 ms of ticks, and after a wish is made | nothing: the day's made wishes are read again, and the record of a run that finished |

That look is how `working`, `asks` and `waits` are known. It reads what the made wishes say and writes nothing,
unlike a look at the page. So a run that was cut off is still `working` here until the page is next looked at,
which is when its wish is told ([run.md](run.md), "A run that was cut off").

- `working`: a made wish that is working and names a run. `going` counts them.
- `asks`: a made wish that needs the person and carries a question; the first such wish's question is shown.
- `waits`: a made wish that is done, whose run's record says it finished with a commit of its own and that its
  work was neither kept nor tossed. The repo is not asked.

## Every sentence

`says` is one of these and nothing else, apart from a refusal of core's own that is passed on (the last table).

| State | `says` |
|---|---|
| `heard` | `press the key again to take it back` |
| `asks` | `a run asks, and waits for your answer on the page` |
| `waits` | `a run's work waits for you on the page` |
| `quiet`, `listening`, `working` | empty |

`cannot` is said for 6000 ms of ticks, from the first tick at or after the event that was refused, or from the
tick that refused it; then the jellyfish shows what else holds. A later refusal replaces an earlier one.

| Refused | `says` |
|---|---|
| no project can be picked | `nothing was kept: make the day's first wish on the page, so that Jellymoon knows which project a wish said aloud is for` |
| the ear gave no final words in 5000 ms | `nothing was heard: the ear did not say what it heard` |
| ear: `no_ear` | `voice does not work on this system yet: type the wish on the page` |
| ear: `no_microphone` | `nothing was heard: no microphone was found` |
| ear: `microphone_refused` | `nothing was heard: Jellymoon may not use the microphone; allow it in System Settings, Privacy & Security, Microphone` |
| ear: `speech_refused` | `nothing was heard: Jellymoon may not recognize speech; allow it in System Settings, Privacy & Security, Speech Recognition` |
| ear: `not_on_device` | `nothing was heard: this Mac cannot recognize that language on the device, and Jellymoon sends no sound anywhere` |
| ear: `no_such_language` | `nothing was heard: the language in voice.toml is not one this Mac recognizes` |
| ear: `not_bundled` | `nothing was heard: voice works only in Jellymoon.app, started as an app` |
| ear: `failed` | `nothing was heard: the recognizer failed` |

The ear's refusal is said whenever it comes, with a hold or without: it ends a hold, and leaves a heard wish as
it is. The ear gives a reason's name and no text of its own.

What core refuses of the wish itself is said in the sentence a typed wish is refused with:

| Refused | `says` | When |
|---|---|---|
| words over 200 characters once cleaned, or over 4096 bytes as heard | `a wish is at most 200 characters, and this one is longer` | as the final words come |
| words of which cleaning leaves only `[redacted]` | `nothing is left of these words but the mark of what cleaning replaced, so nothing was made of them` | as the final words come |
| words that cleaning still changes after 8 times | `a wish is kept as cleaning leaves it, and cleaning was not done with these words after 8 times` | as the final words come |
| the day has its eight made wishes | `the page of <day> has its 8 made wishes already, so this one was not kept` | at 4000 ms |
| the day is locked, after the lock's own 3 seconds | `another change to the page of <day> has not finished, so this wish was not kept` | at 4000 ms |
| the day's made wishes cannot be used | `the made wishes of <day> are not used: <why>; this wish was not kept` | at 4000 ms |
| the project is no longer offered | `no folder of that project that is switched on was typed in on <day>, so this wish was not kept` | at 4000 ms |
| with `--run`, no run was started, or the run had ended by the time its start was answered | the wish is kept, `made` names it, and `says` is what `jellymoon wish --run` prints as `why`: why none was started, or how the run ended | at 4000 ms |

In each row but the last nothing is kept.

## The settings: `voice.toml`

`voice.toml` in Jellymoon's home names the key and the language. The person writes it by hand; nothing of
Jellymoon's writes it. `jellymoon hear --settings`, which takes nothing else, prints what it gives:

```json
{"key":"Control+Shift+Space","language":"default"}
```

That line is the defaults, which a file that is absent gives, and a name the file leaves out has its default.

```toml
# the key that is held, and the language that is heard
key = "Option+J"
language = "en-US"
```

- **Read:** lines that are empty or start with `#`, and `key = "..."` and `language = "..."`, each once at
  most, each on a line of its own. No TOML library reads it: a table, a comment after a value, single quotes,
  an escape, any other name and a name twice each refuse the whole file.
- **A key** is one or more of `Control`, `Shift`, `Option` and `Command`, each once, and then one of `A` to `Z`,
  `0` to `9`, `F1` to `F12` or `Space`, joined by `+`. A key held with `Shift` alone (`Shift+A`, `Shift+Space`)
  is refused: it is how capitals and signs are typed, and the system would take them from every program. Core
  checks the shape; whether the system lets the key be registered is the shell's to find out (F58).
- **A language** is `default`, the system's own, or a tag: two or three small letters, then a script and then a
  region if there is one (`en`, `en-US`, `zh-Hans`, `zh-Hans-CN`, `es-419`). Whether the Mac recognizes it is
  the ear's to say (`no_such_language`).

A file that is not used is refused in one line, exit 1, and nothing is printed or written. The line names the
value, cleaned and cut to 40 characters as any text core prints, or the number of the line. A value that holds
a character that cannot be seen is not named: cleaned, it would read as a value that is right. That is a
control or format character, a letter that draws as nothing or as a blank (a joiner, a filler, a variation
selector, the object replacement character, a code point that is private or not assigned), any space but
the plain one, and a plain space at an end of the value or beside another. It is the list a file's name is
held to before a page gives it a yes ([show.md](show.md), `plain`).

| The file | The line, after `jellymoon: voice.toml is not used: ` |
|---|---|
| a key that is not one | `the key "Hyper+Space" is not one or more of Control, Shift, Option and Command, each once, and then one of A to Z, 0 to 9, F1 to F12 or Space, joined by +` |
| a language that is not one | `the language "English" is not `default` or a tag such as en-US` |
| a key held with Shift alone | `the key is held with Shift alone, which typing uses: add Control, Option or Command to it` |
| a key or a language that holds a character that cannot be seen | `the key holds a character that cannot be seen`, `the language holds a character that cannot be seen` |
| a line that is neither name | ``line 3 is not `key = "..."` or `language = "..."` `` |
| a name twice | `line 3 names key a second time` |
| over 4096 bytes | `it is too large` |
| not UTF-8 | `it is not text` |
| a link or a folder | `it is not a plain file` |
| a file its group or others can write to | `others can write to it` |

`jellymoon hear` itself does not read `voice.toml`: the key and the ear are the shell's, and core's part is to
say what the file means.

## Every bound

| Bound | Value | Where |
|---|---|---|
| A slip of the hand | under 300 ms | `SLIP_MS`, `hear.rs` |
| A heard wish is shown | 4000 ms of ticks, from the first tick at or after the final words | `WINDOW_MS` |
| A refusal is said | 6000 ms of ticks, from the first tick at or after it | `CANNOT_MS` |
| A hold ends by itself | 60 000 ms | `HOLD_MS` |
| The ear's final words are waited for | 5000 ms of ticks, from the first tick at or after the key went up | `EAR_MS` |
| A look at the day's made wishes | once for each 1000 ms of ticks | `LOOK_MS` |
| One line of events | 64 KiB with its line break, 65 535 bytes without one | `HEAR_LINE_MAX` |
| Changes of the words so far in one hold | 4096 | `SAID_SO_FAR_MAX` |
| The words so far, as shown | 200 characters, cut | `MADE_SAID_MAX`, `wish.rs` |
| A heard wish's words | 200 characters, 4096 bytes as heard, never cut | `MADE_SAID_MAX`; `LOOKED`, `still.rs` |
| Makings of the words that change them | 8 | `CHANGES`, `still.rs` |
| Made wishes in a day | 8 | `MADE_MAX`, `wish.rs` |
| `voice.toml` | 4096 bytes | `VOICE_BYTES`, `hear/voice.rs` |
| A refused value named back | 40 characters | `SHOWN`, `hear/voice.rs` |

Nothing here grows with the input: a hearing holds one hold's words, one refusal and one count.

## Not closed

- **Final words are not tied to their hold.** The ear's events carry no hold's number. Final words that come
  while the key is held are passed over, but an earlier hold's that come after a later hold's key went up are
  taken for the later one. F60, which joins the ear to the key, can close this by numbering holds.
- **A run that was cut off reads `working`** until the page is next looked at: a hearing never writes a run's
  end.
- **`waits` is the record's word alone.** A branch moved or deleted by hand is not seen here; `run show` sees it.
- **The run is started by the program that runs `hear`**, as `wish --run` does, not by the installed command.
  The app starts runs with the installed command (F42), and F58 does the same through `Hearing`.
- **The four seconds are counted by whoever sends the ticks.** A sender that stops sending them stops the
  window: the wish is neither made nor dropped until a tick or the end of the input. They are never shorter
  than 4000 ms of ticks after the words were printed; they are longer by the time to the first tick.
- **A hold's length is counted from the ticks before its key events**, as above: a sender that does not tick
  with a key event can make a slip of a hold. Key events carry no time of their own.
- **A tick below the first one counts as the first.** Until a later tick has come, such a tick is at the time
  the clock stands at, and starts a refusal's six seconds. It cannot start a heard wish's four or the ear's
  five, which need a hold of 300 ms on the clock first.
- **An ear that keeps refusing keeps `cannot` shown.** Each refusal replaces the last and starts its six
  seconds anew. F58 does not send a refusal that stands a second time.
- **A run that finished before its start was answered** prints no line of its own: the hearing goes `quiet`
  with the wish it made, and the day's rest shows the work that waits. Read, not run: no test can make that
  order come.
- **A wish is made on the day the hearing began.** One that goes on past midnight keeps that day.
- **A project needs a folder typed in that day** (ADR 0019, "Risks left open", 1).
- **Answering a run that asks by voice** waits on F52, F54 and F61.

## Tests

`cargo test -p jellymoon --test f55` runs `jellymoon hear` as a black box, with the made-up `fake-agent` as the
harness (`crates/jellymoon/tests/f55.rs`, and `f55/events.rs`, `project.rs`, `started.rs`, `voice.rs`).
`cargo test -p jellymoon --test f30` holds F30's three steps for a wish entered this way. The contract is
`crates/jellymoon-core/tests/hear.rs` with the fixtures above, which also holds this doc to them. The smoke
chain's `hear` step runs the built command ([smoke.md](smoke.md)).
