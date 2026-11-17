# PLAN-HARNESS-AMEND: ADR 0021 after its review (F74 to F82)

**What changed.** On branch `plan-amend`, on main `02393d1`. A plan only: no Rust. The after-merge review failed
ADR 0021 with six faults. The ADR is amended in place and lists each change under "Amended after review".
`feature_list.json` gains F74 to F82; no earlier row changes. The glossary gains login shell.

**Rows that are never built: F67, F68, F70, F73.** They stay in the list as written, `passes: false` for good.
F74 and F75 stand in for F73; F76 and F82 for F70; F77, F78 and F81 for F67; F79 and F80 for F68. F69, F71 and
F72 are built as written. The order to build in is the ADR's table: F74, F75, F76, F77, F78, F79, F80, F81,
F82, then F69, F71, F72.

**Two sentences of `2026-11-15-0849-PLAN-HARNESS.md` were wrong.** "Core never works out or opens the session
store in a run" is false: a first start with no `--repo` reads the day through the reader (`run/place.rs`,
`found`); only an answer and a stop never open it. And the stop on a Mac and on Linux reaches the harness's
group only: a program it moved away goes on (`tests/f40/deaf.rs`).

**The evidence.** `cargo fmt --all --check` and `cargo xtask check`. Every corrected "today" was read in the
code. Two things were measured by the coordinator on one machine, labels and counts only: `entrypoint` is on
every typed line of its 300 newest logs (`cli`, `sdk-cli`), and an interactive login shell named more folders
than one with no terminal. Nothing was built or run here.

**What's next.** F74, the door. F78 must not be built before F74 and F75: without them a test asks the
builder's own login shell.

**Surprising.** A session takes the first row with `passes: false` and no `blockedBy`, and that is now F67, a
dead row. Nothing in the list itself says so; the ADR leaves the fix to workflow.md and the coordinator.
