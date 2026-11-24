# A change that does not fit says so at once (2026-11-24 20:41 UTC)

Branch `does-not-fit-said-at-once`, on `23c7919`. No row of `feature_list.json` changes. Core and the app's
tests; the kit and the command line are as they were.

## Why
Outside the tests a finished change is kept by itself (F92). Where two wishes changed the same place, the second
waited with `Keep it` under it, and only a tap on that said the change does not fit: core had refused that very
keep when the run ended, and had written down nothing of why (ADR 0030, decision 10, as it stood).

## Built
- **The reason is written** (`run/keep/itself.rs`, `unkept`; `run/record.rs`): when the keep by itself keeps
  nothing, core's typed reason goes into the run's record as `unkept`, one word (`clash`, `in_way`, ...) and no
  sentence, under the keep's lock. Only that keep writes it; no later keep writes or clears it; a change that
  landed has none. `Unkept` and `Unread` are serialised by name for it.
- **An old record reads as none**: a record without the field, and one with a word this build does not know
  (`known`), are read as before, with no reason.
- **`Waits` carries it** (`unkept`, never printed: `run waiting` is byte for byte what it was).
- **The note** (`run/waits/unfit.rs`): for `Clash` and no other reason, the lines begin with `CLASH`
  (`run/waits/plain.rs`) and `DOES` (`run/waits/again.rs`) in the place of the line that the work waits for a yes,
  and the choices are `Show me`, `Make it again`, `Toss it`. Every other reason, and none: as before.
- **The tap** on `Make it again` there needed nothing new: the app carries a choice of `work-<wish>` already, and
  core answers `WorkTap::Again` while the work waits and is read.
- `ShownFile`'s two functions moved from `run/waits.rs` to `run/waits/plain.rs`, unchanged, for the file's limit.

## Red first
The code was written before the tests were run. Red was then seen with two switches in the built code, the
reason not written and the note not reading it, and the tests as they are now:
- core `run/waits/unfit.rs`, the first test: the lines were the waiting line's.
- binary `tests/f92.rs`, the new test: the record held no `unkept` where `clash` was expected.
- app `tests/a_change_that_no_longer_fits.rs`, the new test: the note had the waiting line and `Keep it`.

Not red in that build: core's test of the record's reading (it is red under mutation 6), and core's test that
every other reason leaves the note as it was, which is a pin.

## Mutations (one hold)
| Mutation | Caught by |
|---|---|
| The reason is not written | binary `f92`, the new test |
| A reason is written for a change that landed | binary `f92`, the new test |
| `Keep it` is still offered for `Clash` | core `unfit`, the first test |
| `Make it again` is offered for `InWay` | core `unfit`, the second test |
| The sentences are missing from the lines | core `unfit`, the first test |
| An old record is read as `Clash` | core `keep/tests.rs`, the record's test; the app's decision-10 tests |

## Open
- A drawn note places none of its words while it is closed; the two sentences are read after a tap on the
  picture or on `?`. See "After the look" for what the closed note says now.
- The reason is the run's end's. Nothing asks git again, so the note can say it of a folder that has changed
  back since; `Make it again` works all the same.
- Work that waits from before this build has no reason in its record and keeps `Keep it`, with the refusal
  after a tap, as before.
- Not run: a real window, a real harness, Windows, Linux.

## After the look
The first commit's picture was looked at as a person would: the closed note had `Make it again` and still said
"not in your folder yet", so nothing on the page said why. A second commit gives the kit's sheets picture one
more state, `Kept::Unfit`: waiting work with its arrow barred in the taken ink and the caption "does not fit
your folder". The app draws it where core says the work did not fit (`Waits::unfit`, now public). Seen red
first: the kit's test of each state's arrow, folder and caption, and the app's test at the made-up window.
