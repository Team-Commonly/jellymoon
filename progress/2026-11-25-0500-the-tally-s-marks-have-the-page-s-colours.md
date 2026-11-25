# The tally's marks have the page's colours, and the reading is said once

A second commit on the day that folds (ADR 0030, decision 12). No row of `feature_list.json` changes.

## What was built

- **A star is filled.** A row of hollow pencil stars read as an empty rating. A tally's star is filled with the
  theme's star colour (`Ink::Star`, `palette.star`), the colour a star of the open day has, under its pencil
  outline. The outline is what tells the star from the note's ground, so no theme's star colour is refused.
- **What asks has the accent.** The ring of a `?` is filled with the theme's accent (`Ink::Asks`,
  `palette.accent`), the colour of the badge on a little one that asks, where the pencil's `?` on it has a
  contrast of 3 to 1: the sticker theme. Where it has not, the ring alone is in the accent if the accent has 3
  to 1 on the note's ground: the plain theme. Else the mark is pencil, as before. The mark at work and the
  cross are as they were. No colour is new.
- **The sentence stands once.** The kit leaves a tally's last line out where the footer it placed over the tally
  says that line whole (`layout/column.rs`, `once`). The app still gives the tally the sentence, so a day whose
  footer is cut at its four lines before the sentence has it in the tally, and only there.
- **The note ends under its last row.** With no line at its foot the picture ends where the last row's words
  end, so the card has the room under it every other drawn note has.

## What was red first

With the three switched off, by assertion: kit `tests/tally.rs`, the two new tests (a star's fill had no stroke
in the star's ink; the sentence was placed twice); app `tests/the_day_folds.rs`, the first test (the sentence
in the footer and in the picture). The unit test of which colour an ink is painted in (`draw/pencil.rs`) was
written with the code and was never red.

## Mutations

| Mutation | Killed by |
|---|---|
| a star is filled with the pencil's ink | kit `tests/tally.rs`, the test of the inks |
| the tally's last line is drawn whatever the footer says | kit `tests/tally.rs`, the test of the line; app `tests/the_day_folds.rs`, the first test |

## Open

- The plain theme's `?` has an accent ring and no fill: its accent is too near its pencil for the `?` to be read
  on it.
- A star's fill is seven wide strokes, not a filled shape: the kit paints a picture with strokes only. It was
  judged from pictures at one size.
- Nobody has looked at any of it in a real window, on Windows or on Linux.
- The earlier note of this day says the sentence is on the page twice; it is not any more.
