# A theme file's own colours for the entry, contrast floors, and the rest of the review of PR 150

Branch `entry-own-colours`, from `main` at `0c4aa8c`, with `main` at `f99fb81` merged in. Not pushed. It is a
change to the kit and the app only: nothing in `crates/jellymoon-core` or `crates/jellymoon`.

The review of PR 150 (its last comment) found 1 blocking thing and 6 others. What each became:

- **Finding 1, blocking: an older theme file drew the chosen project's name unreadably.** A key of the entry that
  a theme file does not set is now the file's own colour (`crates/sticker-kit/src/theme/own.rs`):
  `chip_chosen_ink` its `soft`, `entry_hint` its `quiet`, `entry_ring` its `caret`. The same holds for the keys
  the entry came with: `entry` its `option`, `entry_edge` its `bubble_ring`, `caret` its `accent_dark`, `chip` its
  `dimmed.light`, `chip_chosen` its `star` (before, an older file got the sticker theme's yellow for
  `chip_chosen`). A file that sets neither a key nor the colour it is taken from is the sticker theme there, so
  a file that names no colour is still the sticker theme whole. The built-in themes are made in code and are
  unchanged. The review's dark file (`chip_chosen = "#3B3B52"`, `soft = "#C9C9D4"`, `caret = "#FFB347"`) draws
  the name at 6.61 to 1, where it was 1.33.
- **The chosen chip's mark.** A line in the colour of its name is drawn round the chosen chip only when its fill
  is close to the other chips' (no channel 48 apart, `Palette::chosen_edge`). Not always: both built-in themes
  tell the chosen chip by its fill and are drawn as they were.
- **Finding 2: a contrast floor.** `Color::contrast` is WCAG 2's ratio of relative luminance. The floors, chosen
  from what ships: the chosen chip's name on its chip 4.5 (sticker 9.99, plain 4.57), the hint on the field 2.5
  (2.80, 3.06), the ring on the paper 3.0 (5.35, 15.80). `tests/colours.rs` holds both built-in themes to them
  and to the measured numbers, in place of the sum of channel differences. At the door a file is never refused
  for them: a colour under its floor is replaced by the first of the file's own that is over it, or by black or
  white, and one warning names the key. Loading with a warning was chosen over refusing because a file that
  loaded yesterday must load today, and a refusal would take the whole theme away for one pale colour.
- **Finding 3: a row's spare width goes to its long chips** (`src/layout/entry.rs`). A chip is as wide as its
  name while its row has the room; in a row too narrow, the short ones keep their width and the long ones share
  what is left. A name that is cut is cut in the middle ("kitc…ntend"), so two names that begin alike are told
  apart. The README and the doc comment say this now.
- **Finding 4, the kit's half: one rule for a character a person sees** (`text::has_visible`). An entry of only
  U+200D, U+200C, U+3164, U+2800, U+FE0F (or spaces of any kind, or the other fillers and selectors) holds no
  words, Enter gives nothing for it, and the hint stays while it is all the entry holds.
- **Findings 5 and 6: the window and the untested numbers.** `app/tests/f43/window.rs` opens `window::page` on
  GPUI's test platform (no window on a screen, Mac only, as the kit's draw tests), pastes words and presses
  Enter: the entry is told what core said. The handle of the window no longer reaches the handlers through a
  cell that could be empty: a handler is given its window and takes the handle from it, so the miss the review
  named cannot happen and nothing waits on it. Tests now state the waiting words' colour, a chip's least width,
  and the 24 pixels kept from the big character. The path in `app/src/showing.rs` is right.
- **Finding 7: the hint "read whole".** `docs/development/app.md` says with which font and size, and
  `app/tests/f42.rs` asks for the whole hint at 960x640, 1200x760 and 1440x900 in both themes.

`tests/entry.rs` was at 398 lines: its layout cases are `tests/entry_layout.rs`, its colours `tests/colours.rs`.

**Tests first.** Each new test was seen red before its change, but for the window's two, which hold what the
window already did and were seen red under the mutations. Mutations, one at a time, all caught (21): the
review's M09 (a chip not held to its row), M11 (no least width), M22 (waiting words in ink), M23 (pink name on
the yellow chip, 2.01 to 1), M30 (the entry always told "kept"); and 16 more: the two calls at the door taken
out, the line round a close chip taken out or always drawn, the list of blank characters emptied, the hint
hidden by any text, the cut at the end and not the middle, the gap 23, every chip an equal share, each floor
moved, the entry always told "not kept", the file's own colours skipped for black or white, a key always taken
from the file's colour, the ring's own colour and its floor.

**Pictures.** Headless (`--example snapshot`, which now also draws a theme file): the made-up fullest page (eight
wishes, six projects) at 1440x900 and 960x640, with the hint and with typed words, in both built-in themes and
in a made-up dark theme without the three keys. Looked at: the dark theme's chosen chip is read and has its
line, every name is whole at both sizes, the hint is whole. None is kept in the repo.

**Lines.** 26 files: about 890 added, 230 removed. Deleted: the cell that carried the window's handle to its
handlers and both branches that did nothing when it was empty; the test of colours by channel sums; a chip's
cap at an equal share of its row.

## Not closed

- **Core's same rule for a character a person sees.** `jellymoon wish` still keeps a wish of U+200D alone
  (`Ok("\u{200d}")`), which makes a wish with a blank label. The window cannot send one now; a terminal can. It
  is owed to the next gated change to core, with the kit's list (`text::is_blank`) as the list to match.
- The floors hold three pairs. What is typed (`ink` on `entry`) and a chip's name (`soft` on `chip`) are not
  held to one.
- `window.rs`: the order of the reading's steps, which call a tap is carried to, the one-second timer and the
  theme key are run by no test. A person's walk at a real window is still owed.
- Not run on Windows or Linux; the window's test runs on the Mac only.
- Two names alike at both ends ("…-frontend-app", "…-backend-app") can still read the same when cut short.
