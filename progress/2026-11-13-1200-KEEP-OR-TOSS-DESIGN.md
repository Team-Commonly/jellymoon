# Design: a run's work is shown, kept or tossed (ADR 0017, F44 to F47)

Docs only, on branch `keep-or-toss`. No code changed; 0 lines of Rust added or removed.

## What was decided

The owner moved the product's centre on 2026-11-12: building with agents with no terminal and no editor, the page as
the record. The loop is wish, watch, answer, see, keep or toss. The first two are built. ADR 0017 decides the last
two, and four features carry them: F44 `run show`, F45 `run keep`, F46 `run toss`, F47 the tap in the window.

## Order

`feature_list.json` is read top down and F30 and F31 stand above the new ones. ADR 0017 states the order:
**F44 to F47 first.** F44 can start now. F45 and F46 wait on F44 and each waits for `GATE: PASS`. F47 waits on both.

## For whoever builds F45 and F46

- The git steps of a keep were measured on git 2.50.1 only (the table in ADR 0017). Not measured: git 2.38 and
  2.39, Windows, Linux, filters and LFS, a keep cut off between its steps. Measure before building on them.
- PR 148's fifth gate left five non-blocking findings in `copy/back.rs` (the last comment on that PR). They go
  into the first of these two changes that touches that file: the graft file named by its whole path, the tip read
  by the exact ref name, the symbolic check before the `rev-parse`, and tests that its four surviving mutations
  fail.
- A toss is a delete at the person's word, so it does not ask whether another ref holds the commit. Everything else
  about the delete is a look's: the run's own ref, never what it points at, at the commit that was shown.

## Not decided here

A run that answers back and "more like this", the held key (F30), Codex (F31) and ACP. Each gets its own note.
