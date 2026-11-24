# A change that no longer fits (2026-11-24 16:48 UTC)

Branch `a-change-that-no-longer-fits`, on `bf92295`. No row of `feature_list.json` changes. ADR 0030, decision 10.

## Built
- **Two refusals of a tapped keep in the window's own sentences** (`run/waits/plain.rs`, `unkept`), under
  "The change was not kept.", with no run's id and no word of git's:
  - `Unkept::Clash`: "Your folder changed in the same place after this change was made, so the two do not fit
    together." / "Nothing in your folder was changed."
  - `Unkept::InWay`: "Your folder has changes of your own that are not saved to the project yet, where this
    change would go." / "Nothing in your folder was changed. Save or put away your own changes, then tap Keep
    it again."
  Every other reason is told as before. `run keep` prints what it printed.
- **The reason reaches the page typed** (`run/keep/tapped.rs`): the page's keep, `keep_named`, gives back core's
  `Unkept` beside the sentence. `Yes::To` carries a cell that the last arm of `landed` sets; no sentence is
  matched and none was changed.
- **`Make it again`**, for `Clash` only: one more sentence ("Make it again throws this one away and makes the
  wish again on your folder as it is now.") and the one choice on `said-<wish>`. The tap is carried as every
  choice is, after the word check of decision 9 (`OnShow::carrying` now takes a choice of `said-<wish>`).
  Order (`app/src/again.rs`, `Today::asked`):
  1. core says the wish's words and project, only while its work waits and is read (`WorkDone::Again`);
  2. the wish is made by `jellymoon_core::wish_offered`, the call the entry makes for typed words; refused
     (the day's eight, a project offered no more): the typed wish's title and core's sentence, then "The change
     that does not fit was not thrown away.", and nothing is tossed;
  3. core is asked for the toss by the call `Toss it` is carried by; refused: what `Toss it` says then;
  4. the run is started by `Today::asking`, as a typed wish's is; the note ends "Your wish was made again, and
     it is being worked on.", or with what a typed wish that has no run is told.

## Found while building
- The run is started after the toss, not before. Started first, the toss was refused ("could not be read from
  the repo") in the test's first green attempt: a toss asks git where every copy of the repo stands
  (`copy/keep.rs`, `stood_on`) while the new run's copy was being made. So a run that cannot be started is found
  after the toss; the wish is then kept with no run, as a typed one is.
- The reason a change was not kept by itself is recorded nowhere: `run/keep/itself.rs` gives back only whether
  it landed. Nothing was built on it.
- `Ask` (the carried call of `working.rs`) is `Fn` now, since it is asked twice.

## Red first
With `plain::unkept` answering nothing: the core test of the two sentences, and all three tests of
`app/tests/a_change_that_no_longer_fits.rs` (two could not tap a choice that was not there).

## Mutations (one hold, against `app/tests/a_change_that_no_longer_fits.rs`)
| Mutation | Caught by |
|---|---|
| The plain words used for every reason | the reason told as before |
| The run's id left in (the command line's sentence after the plain ones) | both plain notes |
| The toss before the new wish is made | the answer after the tap; the full day that tosses nothing |
| The toss even when the new wish was refused | the full day that tosses nothing |
| `Make it again` offered for `InWay` | changes in the way have no choice |
| The new wish made with other words | the new wish has the old words |

## Tests changed, and why
- `app/tests/f47/asked.rs`: its refused keep is `InWay`, so it now reads the window's sentence and that the
  run's id is not in it. A reason told as the command line has it is in the new test file.

## Lines
Rust: 362 added, 33 removed. Core's cap 19412 to 19536 (147 added, 23 removed); the app's 18770 to 18975 (215
added, 10 removed). Deleted: `keep_named` left `run/keep.rs` for `run/keep/tapped.rs`; nothing else.

## Open
- Not run: a real window, a real harness, Windows and Linux. The whole workspace suite was not run here.
- Not proved: a run that could not be started after its wish was kept; a toss refused after the wish was made.
- The new wish is its words alone: they are not sharpened again, a sharper wish of the old one is not carried,
  and a follow-up is made again as a wish of its own.
- Core does not ask git again whether the change still does not fit when `Make it again` is tapped.
- A toss asked while any run's copy is being made is refused; that was so before, and `Toss it` has it too.

## After the gate
A gate by reading passed the first commit and found two endings a person can come to, fixed in a second commit:
- With no installed command the run could not be started, and that was found only after the toss: neither the
  old change nor a running wish was left. The command is now looked for before the wish is made
  (`app/src/again.rs`); with none, nothing is made and nothing is tossed. Test, seen red first:
  `with_no_installed_command_make_it_again_makes_no_wish_and_tosses_nothing`. The full-day test now names an
  installed command, so that it comes to core's refusal of the wish.
- A wish made again whose run core then refused was told only core's reason; it is now told a typed wish's
  sentence for that first (`WorkDone::remade`, with a unit test).
- Left as it is: `Make it again` beside a toss that was refused can be tapped again after a later `Keep it`,
  and makes a second wish with the same words; every sentence is true then and nothing is lost.
Core's cap 19536 to 19559, the app's 18975 to 18993.
