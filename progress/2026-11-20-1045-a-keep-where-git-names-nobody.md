# A keep where git names nobody (2026-11-20)

## Found
A walk with a real harness: three wishes made together in a made-up repo whose git config named nobody. The
first `Keep it` landed. The second was refused in git's words (no `user.name` and `user.email`), and the page
could do nothing about it. A person who never used git meets this at the second keep of two wishes made together.

## Built
`copy/keep/made.rs`, `author`: where git names nobody, the one new commit is by Jellymoon's own name and address,
as a run's commit is. Where git names the person, the commit is theirs, as before. ADR 0017 has the change of
its decision 7 and what is given up.

Deleted: the refusal (`Unkept::Nobody`), its sentence, its row in `keep.md`.

## Red first
`tests/f45.rs`, "where git names nobody": a name or an address unset, empty, of spaces, or of the marks git takes
off. It failed on the code before with the refusal's sentence. The test that pinned the refusal is gone.

## Open
- The page still has no way to ask a person their name. Nothing asks for one now.
- Windows and Linux not run here.
