# A tossed change is no star (2026-11-20)

## Found
The same walk as the one answer: a change thrown away kept its star on a greyed character, and its line under
"came true today". Nothing of it came true.

## Built
`scene::ruled`: a made wish that was let go draws no star and has no row under "came true today". A reading's wish
that was let go keeps the stars of the jobs that finished. Core is unchanged: it keeps the star, since the run did
report it finished (F46 pins that).

Deleted: the star and the row of a tossed change.

## Red first
`src/scene/tests.rs`, a made and a reading's let-go wish side by side: failed on the code before with both rows.
`tests/f47/asked.rs` pinned one star on a tossed wish; it now pins none, and no list. The whole app suite: 295
tests, none failed.

## Open
- A made wish let go without a toss (before any run) has no star either way.
- Windows and Linux not run here.
