# Go on from an answer: a wish that changed nothing can be gone on with

## Built
- **Core** (`run/follow.rs`, the coordinator's ruling that widens ADR 0018). `followed` accepts a run that finished
  and changed nothing (`show::unchanged`: its record's line says so, and it was not tossed), beside one with work.
  It is followed with no work, neither waiting nor kept, so `Base { at: None, kept: None }` makes the follow-up's
  copy where the person's checkout stands now, with the same-repo check and no other. The session lookup is
  untouched: the follow-up forks the followed run's session. Refused still, in a new sentence: a run that changed
  nothing in a copy of work it follows that changed files and is not kept, or whose record is not there (`run <run>
  changed nothing on work that is not kept: follow the wish of that work instead`), since its session saw files the
  checkout does not hold. `followable(home, day, with_work)` gives the wishes with work (`true`, unchanged for
  `run waiting`'s `follows` and the app's offer) or those without (`false`).
- **App** (`answering.rs`, `working.rs`). The note of a made wish core lists without work gets the field of `More
  like this` under no offer: hint "Type what to do next, then Enter", key `more-<wish>`. No offer, no choice, no
  `Show me`. An answer's field is now told by no offer and a key that is not `more-`. Enter goes through the same
  `wish_after` path. A wish of the reading is never given the field.
- **Behaviour change found:** a note with a field is in the person's hand (`working::in_hand`), so a told-alone note
  now holds every look while it is on show, as a kept wish's note with `More like this` does. The old pin in
  `tests/told_alone.rs` (the next wish's work came over it after three looks) was changed to the new truth.

## Red first
- Binary `tests/f53/answer.rs` (both) and the changed `refused.rs` case: red on main's code (refused in the old
  sentence).
- App `tests/go_on_from_an_answer.rs`, the done note with the field and the follow-up: red on main's code (no field
  on show, so the made-up window found no field to type in). Its second test (work, at work, read wish as before)
  was green on main, as it should be.

## Mutations (one hold, each taken out after)
| # | mutation | run against | result |
|---|---|---|---|
| 1 | followed accepts a no-change run on work that is not kept (`on_unkept` dropped) | `f53` refused | killed |
| 2 | the copy made at the old commit (`waits` true for a no-change run) | `f53` answer | killed |
| 3 | the session not forked for a no-change run | `f53` answer | killed (2) |
| 4 | `More like this` offered over the field | app `go_on_from_an_answer` | killed |
| 5 | the field shown for a wish of the reading | app `go_on_from_an_answer` | killed |
| 6 | the field shown for any made wish, one at work too | app `go_on_from_an_answer` | killed |

A failed run is refused before any of this, by `found` (a look's refusals), which is untouched: no mutation of
mine reaches it.

## Open
- A follow-up of a no-change run that itself changes nothing, with the checkout not moved, says core's existing
  line for a follow-up that changed nothing: "the branch ... is where the work it follows ended". True, but worded
  for work.
- Not run: Windows and Linux; a real window; the real Claude Code.
