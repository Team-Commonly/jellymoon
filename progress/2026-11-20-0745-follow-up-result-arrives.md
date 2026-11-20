# Go on from an answer, walked: the follow-up's result did not arrive

## Found
The walk with a real harness: a question, its answer, a follow-up typed under the done note. The follow-up ran and
finished with work, and the page never showed it. The done note has a field, a field on show is in the person's
hand (`working::in_hand`), and so every look after it was the plain one. The same held for a follow-up typed under
a kept wish's note with `More like this`, since that field came in.

## Fixed
- `app/src/answering.rs`, `working::settled`: once core kept a follow-up, the note it was typed under goes when it
  is the wish's own with nothing to choose. A note with choices (work that waits) stays, and so does the note in
  the place of a tap core is answering. A follow-up core refuses leaves the note and the words.
- `app/tests/go_on_from_an_answer.rs` holds the made-up window open over the follow-up's run and takes its work
  from looks alone; red on the code before, at both new lines. `app/tests/f54/more.rs` pins the note going.
- `crates/jellymoon/tests/f54.rs` still listed a wish that changed nothing among those `wish --after` refuses.
  That is the feature now; CI caught it, the local round had run only `f53` and `f39` of the binary.

## Lesson
A change to what core refuses is run against the binary's whole suite before a push, not the feature's own tests.
A page feature is walked to its end with a real harness before its gate, not after: the gate read the code and
passed a page that left the person waiting.
