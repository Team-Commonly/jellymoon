# A new wish is followed wherever it is made

Two more commits on the day that folds (ADR 0030, decision 12), after its gate. No row of `feature_list.json`
changes.

## What was built

- **The move is read from the scenes.** `draw/column.rs`, `turn(shown, scene, typing)`: a scene that holds one
  more thing a person said (`Note::Said`) than the one on show takes a view no wheel moved to the column's end
  (`Scroll::entered`), wherever the wish was made: the entry, a card's field, `More like this`, `Make it again`,
  the command. Before, only Enter in the entry did, so a wish made any other way on a folded day stayed under
  the window's edge.
- **A page with a made wish opens at its end**, as it did before the tally: that wish may wait there for Keep
  or Toss. A page with a tally and no made wish opens held at its top. A reading that arrives over a made wish
  moves nothing.
- **The page does not leave typed words.** While a field of a note holds words the person is typing, nothing
  is followed: a wish made elsewhere meanwhile does not take the field out of the window. A tally turned in
  that moment still holds the view where it is.

## Tests

Two in the kit's column view (`draw/column.rs`), each seen red under its mutation: the opening held though
something was said; one more thing said not followed; followed though words are being typed.

## Open

- No test holds that the view calls `opening` and `turn` (`Page::new`, `Page::set_scene`): the two functions
  and `Scroll` are tested, the wiring was walked in a real window.
- A day reopened with a made wish and an open clash opens at its end: the clash and the tally are above the
  window's edge, a scroll away, as a clash was before the tally.
- A scene that adds a wish in the same moment a tally is turned follows the wish, and the tally leaves the
  pointer.
