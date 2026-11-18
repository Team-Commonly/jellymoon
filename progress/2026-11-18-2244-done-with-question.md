# Done with a question

## Built
- **Core reads `done` with a `question` as done** (`run/asks.rs`, `last` and `Word::result`). Seen with a real
  harness: a plain question typed as a wish ended `{"state":"done","question":"<the person's question>","result":…}`,
  the run failed as unfit, and its answer was lost. Beside `done` a `question` of any shape, written any number of
  times, is not kept, printed, asked or counted, as a `result` is not counted; the `result` beside it is read as
  beside any `done`. A question written twice beside `done` is therefore done, not unfit: `last`'s rule for a
  twice-named part is "one part too many", and a part that is not counted is never one too many. `asks` is unchanged.
- **`UNFIT`'s words stay** ("`done` alone, or `asks` with one question"): a record's word, read back from records of
  older builds by `end::recorded`, which the app no longer shows.
- **The schema says when to write a question.** `last-word.v1.json` gains a `description` on `state` and on
  `question`. It lets through no word it did not, so the file keeps its name (`run.md` says so); the byte-for-byte
  pin in `tests/f39/line.rs` and the copy in `run.md` follow it. The run's fixed words are unchanged: they never
  invite a question beside `done`.
- ADR 0023 gains a dated note: decision 1 widened, against ADR 0018's "a harness that enforces the schema never
  sends them".
- The made-up agent's unfit word 3 was `done` with a question; it is now `asks` with no question.

## Red first
- `run::asks` tests: `done` with a question (and a result) read `Unfit`, not `Done`.
- `tests/f87.rs`, `a_run_done_with_a_question_beside_it_is_done_with_its_result_and_keeps_no_question`: the run
  `failed` with no star.

## Mutations (one hold)
| # | mutation | test file | killed |
|---|---|---|---|
| 1 | `done` with a question is unfit | `run::asks` tests | yes, 2 tests |
| 2 | the question beside `done` is kept as the wish's question | `tests/f87.rs` | yes |
| 3 | `asks` with a `result`-only word passes | `run::asks` tests | yes, 2 tests |
| 4 | `Word::result` gives nothing for the three-part word | `run::asks` tests | yes |
| 5 | a question written twice beside `done` is unfit | `run::asks` tests | yes |
| 6 | `Word::result` reads beside any state | `run::asks` tests | yes |

## Budgets
core 17906 -> 17916 (+10, of it 1 in `run/told/tests.rs`: its schema check takes a `description`), the binary 29867 -> 29893 (+26, all tests). Deleted: three unfit test lines of the old
rule, and the made-up agent's `done`-with-a-question word.

## Open
- No real harness was run: whether the model still writes a question beside `done` under the new descriptions is
  the coordinator's to see.
- Windows and Linux not run. The app is not touched and its suite was not run.
