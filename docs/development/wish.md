# A made wish: `jellymoon wish`

A made wish is one the person made themselves, not one the reading call worked out (F37,
[ADR 0014](../adr/0014-a-run-is-started-headless.md)). The rules are core's, in `crates/jellymoon-core/src/wish.rs`.
Nothing is started for a made wish yet: a run is F39.

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
- **The id.** `m1` to `m8`, by the wish's place in the file. A reading's ids are `w1` to `w8`, which core writes
  itself whatever the model called them, so the two never meet.
- **The write.** Behind the day's lock, `pages/<day>.ruled.lock`, which is the tap's
  ([delivery.md](delivery.md), "A tap on a clash"): one change to a day at a time. The file is written beside its
  place and moved there, 0600 in a folder set to 0700.

## What is refused

Each is one line on stderr, exit 1, and nothing is written.

| Refused | Why |
|---|---|
| Words that are empty once cleaned | There is nothing to draw |
| Words over 200 characters once cleaned, or over 4096 bytes as typed | A wish is never cut: cut words would not be the person's |
| A ninth made wish for one day | A day holds eight |
| A project that is not one folder's name: empty, over 100 characters, a path, `.` or `..`, or anything cleaning would change (a line break, a character that cannot be seen, a credential shape, which a name of 40 letters and digits with no break is) | F39 finds the folder by this name |
| Made wishes that cannot be used (below) | They are never written over |
| A day whose lock is held for 3 seconds | Another change has not finished |

The file of made wishes is used only when it is a plain file, never a link, that its group and others cannot write
to, of at most 64 KiB, of this exact shape with no other field, of the day asked for, with at most eight wishes whose
ids are `m1`, `m2` and so on in order and whose every text is what cleaning would leave. Otherwise no made wish of
that day is drawn, and a new wish is refused until the person moves the file away.

## On the page

`jellymoon_core::with_made` puts a day's made wishes after the reading's when a page is drawn. `jellymoon page`
prints them so, and the app draws them so ([app.md](app.md)): on the kept page, on a page just read, and, before a
day has been read or when it could not be, under the one note that says so. A made wish's label is the start of its
own words. The kept page itself never holds a made wish.

## Not covered yet

- A made wish cannot be taken back or changed, and its state never moves: F39 gives it a run and a star.
- Nothing checks that the project is a folder on this computer. F39 does, and refuses a name two folders share.
- The same words made twice are two wishes.
- With eight wishes read and eight made, a page holds sixteen. How many of them the kit shows is the kit's layout.
- A made wish's little one carries a short label, cut. Its words are in the note "wished here today", whole or
  counted (`+N more`), and that note comes after the clashes and the delivery note, so it never pushes one out. On
  a full page the note has little room: beside the list, the delivery note and one clash, 960x640 in the sticker
  theme draws no wish of 200 characters, and counts it. `layout` prints the note's lines and one `mark` piece a
  wish drawn.
