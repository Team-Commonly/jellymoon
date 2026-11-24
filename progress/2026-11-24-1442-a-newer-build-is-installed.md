# A newer build is installed (2026-11-24)

One slice in the app: a window that was open while `install.sh` replaced its bundle says so on its page. No row of
`feature_list.json` changes. The kit, core and the binary are as they were. No ADR: no dependency and no module
boundary changed.

## Built
- **The reader and the comparison** (`app/src/newer.rs`). `read` takes the `<string>` after the key
  `CFBundleShortVersionString` from a property list of at most 64 KiB, as `app/macos/bundle.sh` writes it.
  `version` reads dot-separated numbers with an optional `-suffix`; one is higher by its numbers as numbers, and
  with the same numbers the one without a suffix. `0.0.0` and anything else are no version. `bundle_of` finds the
  bundle of a program at `<bundle>/Contents/MacOS/<name>`.
- **The watch** (`newer::Watch`, held by `Showing`). The version is read once when the window opens
  (`Showing::of`, from the bundle the program runs from) and kept. With none there is nothing to read, ever.
  `Watch::check(now)` hands a look one read at most once a minute (`EVERY`); a higher version on disk sets one
  flag, and nothing is read after that.
- **The look carries it** (`Showing::look_at(now)`; `look()` is that with the clock's now). The read runs after
  the look's own work, in the same closure, off the window's thread: no thread and no timer were added, and
  `window.rs` is unchanged. The look that finds a newer build gives the scene on show back to be drawn again when
  it has nothing else to show; when the page changed under it, the next look does.
- **The note** is put on the scene only in `Showing::column`, where every scene goes to the kit, and
  `column::column` stands it last, under a wish that is not made yet too. It is on no scene the app keeps, so
  taps, choices, the hold and the offer of a follow-up do not know of it.
- `Showing::bundled(bundle)` is the seam a test gives its own folder by.
- Docs: `docs/development/app.md` ("A newer build is installed", the column's table, the table of acts),
  `docs/development/install.md` ("An update").

## The sentences
- `A newer Jellymoon is installed.`
- `Quit Jellymoon and open it again to use it.`

## Red first
With a `version` that knew no version and a column with no place for the note:
- `tests/newer_build.rs`: the comparison ("0.2.9 is a version"), the reader, the minute (the first check gave
  none), and the look that finds a newer build (no scene to draw again). The two tests of what must stay silent
  (a program in no bundle, no version at the open) passed red, as they should.
- `src/column.rs`: the note of a newer build stood first, where it was put, not last.

## Mutations (four, each killed by `tests/newer_build.rs`)
| Mutation | Killed by |
|---|---|
| any difference counts as newer | the minute's test (0.2.8 over 0.2.9 said newer), the look's test |
| the versions compared as strings | the minute's test (0.2.10 over 0.2.9 said nothing), the look's test |
| the note placed first in the column | the look's test: the last note of the column |
| the check on every look | the minute's test (the second look read), the look's test |

## Lines
The app: 412 added, 7 removed, cap 18365 to 18770. Nothing was deleted but the seven lines rewritten in
`column.rs` and `showing.rs`. The estimate was 120; the tests are 239 of the 405.

## Open
- A real install under a real open window was not walked: only a person at a Mac can.
- The read at the open is on the window's thread, before any window is on the screen; the reads after it are not.
- A window with a page-data file has no look, so it reads once and says nothing.
- Windows and Linux were not run here: there is no bundle there, and the code is the same on every system.
