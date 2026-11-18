# 0022 · The page moves on to a new day by a person's tap, never by itself

Status: Written by a designer on the coordinator's rulings, 2026-11-17. Nothing here was built or run. Every
"today" below was read in the code at `d31fbf8` with the open change "jellyfish notes 2" (`0da22d6`) on it, and
names its file. What was not read says "not confirmed", and the feature that builds it checks it first.

## Why

The app's page is the day the app was opened on. Past local midnight, with the app still open, it stays that day:
what is typed on it is filed under the earlier day, and today has no page until the app is opened again. The
jellyfish was made honest about this ([app.md](../development/app.md), "What the jellyfish does not show past
midnight"): of the new day it takes only a run at work. The page itself is what is left.

The **page's day** is the day the page in the window shows. A day is **quiet** when core's look for the
jellyfish gives `quiet` for it: no question waits, no run is at work, no work waits to be kept or tossed. The
rulings said "settled"; a settlement is a clash's ([GLOSSARY.md](../GLOSSARY.md)), so this note says quiet.

## What the code is today

- **How the page's day is fixed.** `main.rs`, `day_open`, builds one `Today` with `Today::from_env`
  (`today.rs`), whose `date` is set once. `window()` in `main.rs` makes the first scene from it (`Today::first`,
  `Today::later`) and hands it to `shell.rs`, `wire`. Nothing changes `date` afterwards.
- **Where "today" comes from.** `today::date(None)`: the computer's local date, which `TZ` overrides. The shell
  asks it at each look of the jellyfish through `Asked.today`, a `Clock`, and a test gives its own. The page's
  look does not ask it.
- **What the window keeps of a day.** `Showing` (`showing.rs`) holds one `OnShow` (`taps.rs`): the `Today`, the
  page data and the scene on show, notes included. `State.view` (`shell.rs`) is the kit's view, which outlives
  its window. Three tasks started in `wire` go on by themselves: `window::reads` (the day's reading),
  `window::looks` (the page's look once a second) and `Opened::looks` (the jellyfish, whose look, `jelly_look`,
  holds the page's day as text). Each piece of work in flight (a wish, the asking for its run, an answer, a keep)
  holds a copy of the `Today` it began with.
- **How the kit keeps typed words.** `crates/sticker-kit/src/draw/field.rs`, `fit`, called by `Page::set_scene`:
  a field is the same field while its key and its offer are the same, and one with a key that holds words is
  kept apart, at most eight, while no note has it. The app's keys are `more-<wish>` and
  `answer-<wish>-<question>` (`answering.rs`), and a made wish's number is its place in its day's file (`m1` to
  `m8`, `wish.rs`). So `more-m1` of one day is `more-m1` of the next. The entry keeps its words too: an entry
  that is removed while it holds words stays (`entry.rs`, `set_entry`). The kit has no call that empties either.
- **What core's `jellyfish` reads.** `hear/day.rs`, `look`: the day's made wishes as they are kept, with nothing
  written. A wish that says working and names a run is counted as going on its word. Whether a living
  `jellymoon run` holds a run is `run/end.rs`, `held`, which is `pub(super)`; the public calls that ask it,
  `wish::made` and `wish::going`, go through `run::bury`, which writes a cut-off run down and stops its agent.
  The page's own look calls `made` for the page's day once a second, so a cut-off run of the page's day is
  drawn as cut off within a second. No look does that for another day.

## Decisions

### 1. The page never moves by itself

Reading a day is a call on the person's own Claude Code. No reading starts without an act of theirs. Once a
second the page's look asks which day it is, off the window's thread, as the jellyfish's look does. When that
is a later day than the page's and no day was named, the first note of the page says a new day has begun. The
days are compared as text, so a clock set back, or a move west, gives no note. A page with no day (`--page`,
`--waiting`) has none.

The note is the page's own, as "your sessions are not told yet" is: it comes and goes with what a look finds,
is never carried over as a note the app put on, and stands before every other note. `Today::again`
(`wishing.rs`) carries over the plain notes a scene begins with and stops at the first note it does not know,
so the note is put on after that pass and left out of it.

### 2. The move is offered only on a quiet day

Each look asks core what the jellyfish would show of the page's day, after the page's own look has written
down any run that was cut off. When that is `quiet` the note is a choice with one option. When it is not, the
note is plain, with one clause for the first thing the day holds in core's order (a question, a run at work,
work that waits), and no option. The offer appears at the first look that finds the day quiet.

Two things hold the offer back that core's look does not see. An Enter of this window whose wish core has kept
and whose run is still being asked for holds it back until the asking has answered: its run would start for
the earlier day just after the move. A tap on the offer asks core once more, off the window's thread, and a
day that is no longer quiet (a run started from a terminal) moves nothing: the next look draws the plain note.
A reading of the earlier day that is still going holds nothing back: it ends, its page is kept under its own
day, and nothing of it is shown. Neither does a wish that says working and has had no run.

### 3. The move is the path of opening the app, on a new view

The move makes a `Today` of today with what the app was opened with (the logs, the names switched off, the
model, the harness), takes its first scene and its reading as `window()` does, and wires them as `wire` does.
The few lines of `main.rs` that do this for the open move into the library, so both call one function.

- **A new view, and no day in the keys.** The window gets a new kit view, made by `window::view` with an empty
  scene of fields, and the earlier one is dropped. Typed words go with it: the entry, every field on show and
  every field kept apart. A day in the keys would leave the entry's words and the fields that wait, would keep
  the earlier day's fields apart for as long as the app lives, and would need the kit to learn to empty what it
  holds. The kit knows nothing of days and gains no call. The theme `t` chose is the window's and is given to
  the new view.
- **The earlier day is let go.** Its `Showing` is emptied, so its two tasks end at their next step and work in
  flight finds no day to show its answer on. The jellyfish's look reads its page's day from a place the move
  writes, so the same look goes on with today as the page's day.
- **The window stays.** The view is changed in the window that is open (GPUI's `replace_root`: not confirmed at
  the pinned commit). If that cannot be had, a window is opened on the new view where the old one stands, and
  the old one is removed after it, so the app never has no page window.
- **Nothing of the earlier day is written by the move.**

### 4. A day named with `--date` is never offered a move

`Asked.named` says so already. The page's look is told, and asks no clock.

### 5. Core says how many runs of a day are still held

One public function in core: for a home and a day, the number of the day's made wishes that say working, name
a run, and whose run a living command holds (`run::held`). It reads the made wishes and asks about each lock.
It writes nothing, stops nothing and takes no lock. None is the answer for a day that cannot be read.

The ruling asked for yes or no. A number answers that and one thing more: with one run of the later day held
and one cut off, core's count for the jellyfish says two. So the shell's look takes a later day's run at work
only when the number is above none, and draws that number as the count. A cut-off run of the new day no longer
swims, and no longer hides work that waits on the page's day. A run that has just ended may be found not held
for one look before its wish is told: the jellyfish then shows the page's day for that look. Core's own look
(`hear/day.rs`) is left as it is, for `jellymoon hear` and `layout --jelly`.

## Each sentence a person will read

| When | Note | Words |
|---|---|---|
| a later day, the page's day quiet | a choice, id `new-day` | title `a new day has begun`; option `Open today's page`; footer `this page stays until you tap` |
| the page's day has a question | plain | title `A new day has begun.`; body `This page stays while it has a question for you.` |
| the page's day has a run at work | plain | title `A new day has begun.`; body `This page stays while a run is at work.` |
| the page's day has work that waits | plain | title `A new day has begun.`; body `This page stays while work waits to be kept or tossed.` |

A choice's title is lower case with no full stop, as the delivery note's is. No sentence is new for a tap that
moved nothing. After the move the page says what a fresh open says (`Reading your day…`, or the kept page).

## Turned down

- **The page moving by itself at midnight.** A reading call nobody asked for, under a person who is typing.
- **The move offered while a run asks.** The run would wait on a question that no page and no jellyfish shows.
- **A second page beside the first.** Two windows that take keys, and a jellyfish that must say which page.

## Named limits, not built

- **Two midnights.** An app open across two midnights reads the page's day and today. A run filed under the day
  between is not seen by the jellyfish.
- **A day that never goes quiet is never offered a move.** Quitting and opening the app again opens today.
- **An earlier day left with a question or waiting work.** This is so today: an app opened fresh on a later day
  draws that day's made wishes only (`Today::made_alone`, `Today::draw`), and the jellyfish reads the page's day
  and a later one, never an earlier one. A run that asks holds no lock and is counted nowhere. `--date` shows
  it. This is the next feature after these two: today's page would open with one note that names each earlier
  day that still holds a question or waiting work, with a tap that opens that day's page.

## Features

F83: a run of a later day is read as going only while it is held (decision 5), with the five notes of the
review of "jellyfish notes 2". F84, blocked by F83: the page moves on by a tap (decisions 1 to 4). The notes:
`app.md` says work that waits on the page's day is not shown while a run of the new day goes; one bullet for a
run of a day between; "until that day's page is looked at" for "until the app is opened again", which F83 then
makes untrue of a cut-off run and removes; the 142-character line wrapped; one more row in the test of a later
day that asks and works at once.

Not confirmed: that GPUI at the pinned commit changes a window's root view and that the platform's text input
follows it; and anything on a screen. Both rows are proved on GPUI's test platform, on the Mac.
