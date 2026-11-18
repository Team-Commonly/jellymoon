# No work is shown right after a choice (work shows)

The gate failed `77dd910` on one blocker: after `Keep it` or `Toss it` on one wish's work, the next look put
another waiting wish's `Show me` / `Keep it` / `Toss it` where the tapped ones stood, so a slow second tap kept or
tossed work nobody had read.

## Built
- `app/src/showing.rs`: `Turns` holds back the looks after an act of the person. `Showing::chose` (when the tap
  did something), `typing`, `answered` and `ended` set `hold` to `HOLD` (3); `Turns::asks` counts one down per
  look and gives none while any is left, after its bookkeeping, so every turn is kept. Counted again from
  `ended`, so a slow answer is held as long as a quick one. A tap on a character (`poked`) is not held.
- `HOLD` is three looks: a look is made once a second, so the answer stands alone three to four seconds, which is
  past a slow double tap and a tap made again by someone not sure the first was taken.
- `Asks` carries the made wishes of the scene the look was made of, and `whom` asks only of those: a wish made and
  ended elsewhere since has its turn at the next look, when the page has its character.
- Work opened by a look still goes over `said-<wish>`: the hold makes it safe, as the answer has stood alone for
  three looks before any option comes over it.
- `docs/development/app.md`: the hold, only the wishes the page had, and plainly that a look now reads the repo
  through core's git reads once per run's end, where before only a tap did.

## Not taken
- A separate hold "while core's answer is awaited": the page holds `busy-<wish>` or "That run is being stopped."
  for that time, which `in_hand` already refuses, and the count starts again at the answer.
- A count of acts to drop a look in flight: an act changes the scene on show, and `OnShow::seen` drops a look of
  a scene that is gone. The lib test holds that.

## Red first
- `tests/work_shows.rs` against the source of `77dd910`: two of six failed, the two changed (toss, then looks:
  `work-m2` came at the first look; keep with a slow answer: the same).
- `src/showing/tests.rs`: the three new tests name `HOLD` and `acted`, so they do not build before the change.
  Two were seen failing under mutations 3 and 4. The third (a look in flight, then a tap on an option) holds
  what `77dd910` already did, as the gate asked, and was not seen red.

## Mutations (one hold, with red and green)
| # | Mutation | Run against | Killed by |
|---|---|---|---|
| 1 | the hold removed (`acted` sets 0) | `tests/work_shows.rs` | the toss test and the keep test |
| 2 | the hold counted from `chose` only (`ended` does not count again) | `tests/work_shows.rs` | the keep test, whose answer is slow |
| 3 | the turn lost while held (`turn` cleared) | lib `showing` | `no_look_shows_work_in_the_looks_after_an_act_of_the_person_and_the_turn_is_kept` |
| 4 | `whom` asks of any wish of the new scene | lib `showing` | `a_wish_that_was_not_on_the_scene_the_look_was_made_of_is_not_asked_of_by_that_look` |

## Lines
- Added 112 lines of Rust, none removed net: `src/showing/tests.rs` 62, `src/showing.rs` 28, `tests/work_shows.rs`
  22. `entropy.toml` `"app"` 14472 to 14584. Nothing was deleted but the unsafe assertions of the two changed
  tests.

## Holds
- Three, where two were asked for. The final round's `cargo xtask check` refused the new lib test: its made-up
  day had no harness named, which a test may not have. One line named one; a third hold ran the format check,
  `cargo xtask check`, clippy and the lib's tests again, and made the commit. The other test files were run on
  the tree before that line and not after it.

## Not run, not proved
- Windows and Linux. A window, the real Claude Code, a real session log.
- The test files of the app outside the look, the taps and the walk.
- That three seconds is enough for every person: it is a number chosen, not measured.
- The older note of the gate, a press in the kit that spans a scene change, is not touched.
