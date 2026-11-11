# A made wish: `jellymoon wish`

A made wish is one the person made themselves, not one the reading call worked out (F37,
[ADR 0014](../adr/0014-a-run-is-started-headless.md)). The rules are core's, in `crates/jellymoon-core/src/wish.rs`.
`jellymoon wish` starts nothing: a made wish gets its run from `jellymoon run` ([run.md](run.md)), which adds
`run` and `why` to the kept wish.

```
jellymoon wish --say <your words> --project <folder name> [--date <YYYY-MM-DD>]
```

It prints one line of JSON and exits 0. With no `--date` the day is today in the computer's time zone, which `TZ`
overrides.

```json
{"wish":{"id":"m1","said":"ship the changelog","project":"alpha","state":"working","stars":[]},"run":null,"why":"nothing was started: `jellymoon wish` keeps a wish and starts no run"}
```

## What is kept

- **Where.** `pages/<day>.wishes.json` in Jellymoon's own folder: `{day, wishes}`, each wish as printed. It is not in
  the kept page `pages/<day>.json`, because every reading writes that file again.
- **The words.** What is kept is what a page draws, by the rule of a clash's statement: one line, each run of
  spaces made one, nothing that cannot be seen, credential shapes replaced by `[redacted]`, each `"` made `'`.
- **Cleaned until nothing changes.** Cleaning once is not always cleaning twice: a private key's first line with a
  tab in it is one only once it is on one line, and a key straight after another is found only once the first is
  `[redacted]`. So the words are cleaned again, at most 8 times, until a cleaning changes nothing, and that is
  what is kept (`still.rs` in core, the same door a clash's statement goes through). What is kept is therefore
  unchanged by one more cleaning, which is what reading the file back does. The cleaning itself is the reader's,
  unchanged ([reader.md](reader.md)); a session log's turn is still cleaned once.
- **The id.** `m1` to `m8`, by the wish's place in the file. A reading's ids are `w1` to `w8`, which core writes
  itself whatever the model called them, so the two never meet.
- **The write.** Behind the day's lock, `pages/<day>.ruled.lock`, which is the tap's
  ([delivery.md](delivery.md), "A tap on a clash"): one change to a day at a time. The file is written beside its
  place and moved there, 0600 in a folder set to 0700. What Windows does differently when two changes meet is in
  delivery.md, "On Windows".

## What is refused

Each is one line on stderr, exit 1, and nothing is written.

| Refused | Why |
|---|---|
| Words that are empty once cleaned | There is nothing to draw |
| Words over 200 characters once cleaned to the end, or over 4096 bytes as typed | A wish is never cut: cut words would not be the person's |
| Words a cleaning still changes after 8 times (nine keys or more with nothing between them) | They would be kept half cleaned |
| A ninth made wish for one day | A day holds eight |
| A project that is not one folder's name: empty, over 100 characters, a path, `.` or `..`, or anything cleaning would change (a line break, a character that cannot be seen, a credential shape, which a name of 40 letters and digits with no break is) | F39 finds the folder by this name |
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
page writes nothing. The same holds for a statement in a day's settlement and in a ruling that waits.

## On the page

`jellymoon_core::with_made` puts a day's made wishes after the reading's when a page is drawn. `jellymoon page`
prints them so, and the app draws them so ([app.md](app.md)): on the kept page, on a page just read, and, before a
day has been read or when it could not be, under the one note that says so. A made wish's label is the start of its
own words. The kept page itself never holds a made wish.

## Not covered yet

- A made wish cannot be taken back or changed. Its state moves only with its one run ([run.md](run.md)).
- `jellymoon wish` does not check that the project is a folder on this computer. `jellymoon run` does, and
  refuses a name two folders share.
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
