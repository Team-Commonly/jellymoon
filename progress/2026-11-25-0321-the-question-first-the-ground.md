# The question first: a quote's ground hugs its words (2026-11-25 03:21 UTC)

Branch `the-question-first`, a second commit on the first of that name. No row of `feature_list.json` changes.
The kit only.

## Why
The first commit drew every quote of a note on a wash as wide as the note. Looked at in the pictures, a bar of
that width and a field's height still read as a switched-off field, in both looks.

## Built
`layout/typed.rs` `lined`, which every quote of a text note and of a told note goes through: the ground's width
is now the right end of the quote's widest drawn line plus the padding its words have before them, from the same
left, and never wider than the width it was given (the note's text, or what a told row leaves after its marks).
Height, lines, the words' places, the wash, its floor and fallback, and the pencil's line are as they were. What
a person typed does not go through `lined` and keeps its bubble. A note's quote takes no tap (`draw/column.rs`
`taps` gives one only for typed words), so no target changed.

## Red first
`tests/typed.rs`, three new tests, all red on the full-width ground:
- `a_short_quote_s_ground_ends_its_padding_after_its_words_and_is_narrower_than_the_note`
- `a_quote_that_wraps_has_a_ground_as_wide_as_its_widest_line_and_never_wider_than_the_note`
- `two_quotes_of_different_lengths_in_one_note_have_grounds_of_different_widths_from_one_left_edge`

## Mutation (one)
| Mutation | Against | Killed by |
|---|---|---|
| The ground wide again, whatever its words | `tests/typed.rs` | the three above, and two older tests of that file |

The switch set a width past the note's, not the note's own, which is why the two older tests (a quote inside its
note) failed with the three. The switch is removed.

## Pictures
Drawn again, both looks: the question page, the sharper wish with no question, a result's note opened, a run's
question. Their notes were typed by hand for the picture.

## Open
- The marked sentences now read as quoted lines; a long one that wraps is still a bar nearly as wide as the note.
- No person was asked. Not run: a real window, the real harness, Windows, Linux.
