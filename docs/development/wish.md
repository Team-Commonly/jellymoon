# A made wish: `jellymoon wish`

A made wish is one the person made themselves, not one the reading call worked out (F37,
[ADR 0014](../adr/0014-a-run-is-started-headless.md)). The rules are core's, in `crates/jellymoon-core/src/wish.rs`.
`jellymoon wish` alone starts nothing: a made wish gets its run from `jellymoon run` ([run.md](run.md)), which adds
`run` and `why` to the kept wish. With `--run` it keeps the wish and starts that run itself, as a process of its
own that goes on after this command has exited.

```
jellymoon wish --say <your words> --project <folder name> [--date <YYYY-MM-DD>]
jellymoon wish --say <your words> --project <folder name> [--date <YYYY-MM-DD>] --run [--logs <dir>] [--skip <names>]
jellymoon wish --say <your words> --after <id> [--date <YYYY-MM-DD>] [--run [--logs <dir>] [--skip <names>]]
jellymoon shape --say <your words> --project <folder name> [--date <YYYY-MM-DD>] [--logs <dir>] [--skip <names>]
jellymoon wish --say <your words> --project <folder name> [--date <YYYY-MM-DD>] --shaped [--run [--logs <dir>] [--skip <names>]]
```

`shape` keeps no wish: it asks the person's Claude Code, which only reads, to say the wish more sharply, and a
sharper wish that passed core's check waits in `pages/<day>.shape.json`. `wish --shaped` keeps the wish with the
shape that waits for these same words and project (the kept wish then has `shaped`, the prompt its run is given),
and as typed when there is none, which it says on stderr
([ADR 0025](../adr/0025-the-wish-is-shaped-before-a-run.md)).

It prints one line of JSON and exits 0. With no `--date` the day is today in the computer's time zone, which `TZ`
overrides.

```json
{"wish":{"id":"m1","said":"ship the changelog","project":"alpha","state":"working","stars":[]},"run":null,"why":"nothing was started: `jellymoon wish` keeps a wish and starts no run"}
```

With `--run` (F41, [run.md](run.md), "A run started apart") the line names the run once the wish names it:

```json
{"wish":{"id":"m1","said":"ship the changelog","project":"alpha","state":"working","stars":[]},"run":"r0123456789abcdef","why":"run r0123456789abcdef is going, in a copy of the repo"}
```

The repo is the one folder of the project's name that something was typed in that day, found in the session
logs (`--logs`, or Claude Code's own) with the folders named by `--skip` left out, or else the folder listed
under that name in the list of projects Jellymoon knows ([project.md](project.md)); `wish` takes no `--repo`, and
takes `--logs` and `--skip` only with `--run`. A run that was not started leaves `run` null and says why in
`why`, in core's own line: no harness, no folder or two of the name, a name typed in and listed with two
folders, a folder that is gone, no repo's top, switched off or not the person's own, three runs going. The wish is kept in each case and the exit is 0; `jellymoon run --wish <id>`
can start it later. Words core refuses are refused as without `--run`, and nothing is started.

## A follow-up

A follow-up is a made wish that names the wish it follows (F53,
[ADR 0018](../adr/0018-a-run-asks-and-is-told-more.md), decisions 8 and 9, "Found while building F53"). It is
made with `--after <id>` in place of `--project`:

```json
{"wish":{"id":"m2","said":"and log each retry","project":"alpha","state":"working","stars":[],"after":"m1"},"run":null,"why":"nothing was started: `jellymoon wish` keeps a wish and starts no run"}
```

- **What it names.** `after` is the id of a made wish of the same day, made before it. The follow-up is a made
  wish in every other way: one of the day's eight, with its own words through the same door, its own one run,
  copy, branch and record. Its project is the followed wish's, so `--project` beside `--after` is refused as a
  wrong command line (exit 2), and so is neither.
- **A wish of another day** is followed on that day: `--date` names the day the followed wish was made on, and
  the follow-up is kept there and counts in that day's eight. Today's file never names a wish of another day.
- **What can be followed.** A wish whose run finished with work that waits or was kept, and a wish whose run
  finished and changed nothing (a question answered): that follow-up has no work to build on, so its copy is
  made where the person's checkout stands, as one of kept work's is ([run.md](run.md), "A follow-up's run"). A
  follow-up is followed in its turn. `jellymoon_core::followable` lists the day's wishes that can be, those with
  work apart from those without; `jellymoon run waiting` prints the first as `follows`, and the app's window
  offers `More like this` under each of them, and the field alone under the done note of each of the others, and
  makes the follow-up by this same call (F54, [app.md](app.md#a-question-answered-and-more-like-this-appsrcansweringrs)).
  Each of these is refused in one sentence, with nothing kept:

  | The followed wish | The sentence |
  |---|---|
  | is not a made wish of the day | `the page of <day> has no made wish of that id; a made wish's id is m1 to m8` |
  | has had no run | `wish <id> has had no run, so there is no work to follow` |
  | names a run that is going | `run <run> is going: its work can be followed once it has finished` |
  | names a run that asks | `run <run> asks a question: there is no work of it to follow` |
  | names a run that was stopped | `run <run> was cancelled by a stop: there is no work of it to follow` |
  | names a run that was cut off | `run <run> was cut off before it ended: there is no work of it to follow` |
  | names a run that failed | `run <run> did not finish: there is no work of it to follow` |
  | names a run that changed nothing in a copy of work that is not kept: the nearest run it follows that changed files, through runs that changed nothing, or a record on that way that is not there or is not such a run's | `run <run> changed nothing on work that is not kept: follow the wish of that work instead` |
  | had its work tossed | `the work of run <run> was tossed, so it cannot be followed` |
  | has a record from before F48 | `the record of run <run> is from before a run's record named its commits: its work is not followed by Jellymoon` |

  A record that cannot be read, is another wish's or is not one core wrote is refused in a look's words
  ([show.md](show.md)), with `followed` where a look says `shown`.
- **With `--run`** the follow-up is kept and its run started apart, as for any made wish. What a follow-up's
  run does differently is in [run.md](run.md), "A follow-up's run".
- **A follow-up that was made and not run, whose followed work is then tossed,** is refused on every run. `run
  toss` of it lets it go, as it lets go any made wish that has had no run ([toss.md](toss.md), "A wish that
  has had no run"). It is one of the day's eight still.
- **On the page** a follow-up is a made wish like any other. `after` is not part of a page.

## What is kept

- **Where.** `pages/<day>.wishes.json` in Jellymoon's own folder: `{day, wishes}`, each wish as printed. It is not in
  the kept page `pages/<day>.json`, because every reading writes that file again.
- **The words.** What is kept is what a page draws, by the rule of a clash's statement: one line, each run of
  spaces made one, nothing that cannot be seen, credential shapes replaced by `[redacted]`, each `"` made `'`.
- **A credential split by a line break.** A line break is read as a space, so a key typed on two lines was two
  words, neither a credential's shape, and both were kept. The door now reads each stretch of the words that
  holds a line break and no other space more than once, as it stands with each break a space and with breaks
  taken out, and when two readings clean differently the whole stretch is kept as `[redacted]`. Any reading
  may be the one that holds the shape: a key with a break in it, or a key id that the next line's capitals
  would hide. The breaks taken out are all of the stretch's, and those between every two to eight lines in a
  row with the lines around them left apart. The second is for a word that touches a split key across one
  more break (`the id is`, a break, half a key id, a break, its other half): joined to the key it takes the
  word's edge a key id is found by, and the reading that joins the whole stretch does not find it. Words on
  two lines that are no credential either way are kept as before, as two words. This is
  the door's rule, so it holds for a made wish, a clash's statement, a run's question and an answer alike.
  What these now keep differently: a wish or statement with such a stretch was kept in its own words and is
  kept as `[redacted]` there, and a file written before that holds one is drawn so. The stretch is whole: a
  key alone on its line takes the word that ends the line before and the word that starts the line after with
  it, and two long names on two lines with no space (two file paths, say) that together have a credential's
  shape are one `[redacted]`. The words on the lines around them go with them: `fix these`, two such paths and
  `please`, each on its own line, are kept as `fix [redacted]`, and `these` and `please` are not kept.
- **The reading across a break is added, never put in place.** The words are cleaned both ways, as they stand
  with each break a space and with the stretches above replaced, and a word is kept only where both readings
  keep it, in their order; whatever lies between two kept words is one `[redacted]`. So nothing the reading
  without the rule replaced is kept by the reading with it. It was not so at first: a word of 30 letters and
  digits on the line before a private key's first line took that line with it as one stretch, the key was no
  longer found, and its body, set in on the lines after, was kept.
- **Only marks is nothing.** Words of which cleaning leaves `[redacted]` and no letter or digit beside it
  (the `…` of a cut, and the quotes, brackets and stops a key was typed in, or any other sign, are neither) say nothing a person
  typed, so nothing is made of them: `wish`, a heard wish, `run answer`, `rule` and a tap
  that settles a clash refuse them in one sentence, `nothing is left of these words but the mark of what
  cleaning replaced, so nothing was made of them`. Nothing is kept and no harness is given them. `rename`, two
  long names and `now`, each on its own line, are such words. A file written before this that holds a wish of
  only the mark is still read, and an agent's question of only marks is still kept, as the mark.
- **Not closed.** A break with a space or a tab beside it (an indented second line) keeps the two sides apart.
  So does a break beside a terminal escape sequence that holds a space: the sequence goes, the space in it has
  already kept the two sides apart, and a key split there is kept in two pieces. A key split over more than
  eight lines may not be found when a word touches it across a further break: a shape that needs no word edge
  at its end is still found. A shape that spans spaces, a
  private key's first line, is not found when a break splits one of its words (`BEG`, a break, `IN`): a
  stretch is read by itself, and that line is several. Its body is still replaced where its lines are long
  enough to be a credential's shape themselves. A session log's turn is not read this way.
- **Cleaned until nothing changes.** Cleaning once is not always cleaning twice: a private key's first line with a
  tab in it is one only once it is on one line, and a key straight after another is found only once the first is
  `[redacted]`. So the words are cleaned again until a cleaning changes nothing, and that is what is kept
  (`still.rs` in core, the same door a clash's statement goes through). Up to 8 cleanings may change the words;
  the one after them must change nothing. That is the whole rule; how many keys it allows depends on their shapes.
  A row of keys made of letters alone, with nothing between them, takes one cleaning each, so eight of those are
  kept, each as `[redacted]`, and nine are refused; a row of nine or twelve keys that each end in a dash, or hold
  digits, was found whole by one cleaning and kept. What is kept is therefore unchanged by one
  more cleaning, which is what reading the file back does. The cleaning itself is the reader's,
  unchanged ([reader.md](reader.md)); a session log's turn is still cleaned once.
- **The id.** `m1` to `m8`, by the wish's place in the file. A reading's ids are `w1` to `w8`, which core writes
  itself whatever the model called them, so the two never meet.
- **A question.** The wish of a run that asks carries `question`, the run's question as the door left it
  ([run.md](run.md), "A run that asks"; example `wished-asks.json`). No other wish has the name. Made wishes
  are refused as not that day's when a wish has a question the door would change, an empty one, or one on a wish
  that is not `needs_you` or names no run. A toss of the run takes the question away with the wish's state.
- **An answer.** A wish whose run was answered carries `answer`, the last answer the person gave that run, as
  the door left it: one line, at most 600 characters ([run.md](run.md), "An answer"; example
  `wished-answered.json`). It is the one place Jellymoon keeps an answer. It stays when the run ends, a later
  answer takes its place, and no page shows it. Made wishes are refused as not that day's when a wish has an
  answer the door would change, an empty one, one over 600 characters, or one on a wish that names no run.
- **The wish it follows.** A follow-up carries `after`, the id of the wish it follows (above; example
  `wished-after.json`). No other wish has the name. Made wishes are refused as not that day's when a wish's
  `after` is not the id of a wish before it in the file.
- **The write.** Behind the day's lock, `pages/<day>.ruled.lock`, which is the tap's
  ([delivery.md](delivery.md), "A tap on a clash" and "The lock"): one change to a day at a time. The file is written beside its
  place and moved there, 0600 in a folder set to 0700. What Windows does differently when two changes meet is in
  delivery.md, "On Windows".

## What is refused

Each is one line on stderr, exit 1, and nothing is written.

| Refused | Why |
|---|---|
| Words that are empty once cleaned | There is nothing to draw |
| Words of which cleaning leaves only `[redacted]` | They say nothing the person typed ("Only marks is nothing") |
| Words over 200 characters once cleaned to the end, or over 4096 bytes as typed | A wish is never cut: cut words would not be the person's |
| Words a ninth cleaning still changes, for example nine keys of letters alone with nothing between them | They would be kept half cleaned |
| A ninth made wish for one day, a follow-up too | A day holds eight |
| `--after` a wish that cannot be followed ("A follow-up") | There is no work to follow |
| A project that is not one folder's name: empty, over 100 characters, a path, `.` or `..`, or anything cleaning would change (a line break, a character that cannot be seen, a credential shape, which a name of 40 letters and digits with no break is), or a character nobody sees that cleaning leaves, unless the list of projects holds that very name ([project.md](project.md), "A name that comes in, and a name that was kept") | F39 finds the folder by this name |
| A project whose name holds a character nobody sees that cleaning leaves, when the list of projects cannot be used: the list's own sentence is said (`the list of projects (projects.json in Jellymoon's folder) is not used: …`). For such a name the list is mended, not moved away as that sentence advises: `project add` of the folder is refused from then on | The list is why a name that worked before is refused |
| Made wishes that cannot be used (below) | They are never written over |
| A day whose lock is held for 3 seconds | Another change has not finished |

The file of made wishes is used only when it is a plain file, never a link, that its group and others cannot write
to, of at most 64 KiB, of this exact shape with no other field, of the day asked for, with at most eight wishes whose
ids are `m1`, `m2` and so on in order, whose every project and star is what cleaning would leave, and whose words
are a wish's: not empty, and not refused above. Otherwise no made wish of that day is drawn, and a new wish is
refused until the person moves the file away.

**Words read back are cleaned again, through the same door.** A build before the rule above kept words cleaned
once, so a file may hold words one more cleaning changes. Such a wish is not refused and the day is not lost: it
is drawn as its words stand once cleaned to the end, and the next wish made that day writes the file so. Drawing a
page writes nothing. The same holds for a statement in a day's settlement and in a ruling that waits. It holds
only for words the door brings to rest: a file holding a wish the door refuses (no words left, over 200
characters, or still changing after the eighth cleaning) is not used at all, as the paragraph above says, so
`page` draws no made wish of that day and `wish` and `run` refuse the day until the file is moved away.

## On the page

`jellymoon_core::with_made` puts a day's made wishes after the reading's when a page is drawn. `jellymoon page`
prints them so, and the app draws them so ([app.md](app.md)): on the kept page, on a page just read, and, before a
day has been read or when it could not be, under the one note that says so. A made wish's label is the start of its
own words. The kept page itself never holds a made wish. A made wish's `question` is on the page's wish too
(example `page-asks.json`); one the door would change is left out there.

## Not covered yet

- A made wish cannot be taken back or changed. Its state moves only with its one run ([run.md](run.md)),
  and with a toss, which lets the wish go: of what that run left, or of the wish alone when it has had no run
  ([toss.md](toss.md)). A wish that was let go stays one of the day's eight, and no run is started for it.
- `jellymoon wish` does not check that the project is a folder on this computer, or one Jellymoon knows.
  `jellymoon run` does, and refuses a name two folders share.
- The same words made twice are two wishes.
- With eight wishes read and eight made, a page holds sixteen. How many of them the kit shows is the kit's layout.
- A made wish's little one carries a short label, cut. Its words are in the note "wished here today", whole or
  counted (`+N more`). `layout` prints the note's lines and one `mark` piece a wish drawn.
- **A made wish never moves or shrinks what a person rules on.** The clashes and the delivery note are placed as
  on the same page with no made wish, at the same size, and the note takes the room they leave. One page is not
  so: a page whose reading holds a clash and no wish says "Nothing wished for yet today." above the clash, and a
  made wish is a wish, so that note goes and the clash stands where it was, higher, at the same size. On a full page
  the room left is little. Beside the list "came true today", the delivery note and one clash, with one made wish of 200
  characters and the rest short:

  | Made wishes | sticker, 960x640 and 1440x900 | plain, 960x640 and 1440x900 |
  |---|---|---|
  | 1 | none drawn, 1 counted | 1 drawn |
  | 2 | none drawn, 2 counted | 2 drawn |
  | 8 | none drawn, 8 counted | 2 drawn, 6 counted (the long one is passed over) |

  So a full page in the sticker theme says only how many made wishes there are, in the note and in the footer.
  Their words are drawn where there is room, as on a page with no clash and no delivery note. When the other notes
  leave no room for even a count (neither the note's `+N more` line nor the kit's `+N more of M notes` sheet fits
  under them), nothing of the made wishes is drawn among the notes: counting them there would move or push out a
  clash. Their little ones and the footer's count, "N made wishes are in your own words.", are what says them then.
