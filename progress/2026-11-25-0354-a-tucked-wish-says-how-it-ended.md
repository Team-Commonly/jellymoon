# A tucked wish says how it ended

Parent `c6c06be`. No row of `feature_list.json` changes. ADR 0030, decision 11, is amended ("The mark").

## What was built
A made wish that is tucked away kept only its bubble and its little one. Now one small pencil mark stands
right under the bubble, at the bubble's end: a sign, a few words on one line, and a chevron. A tap on it is the
tap on the bubble, so it brings the wish's notes back, and the mark goes in that same drawing.

- **The seam is the kit's**: `Note::Said` has one optional word, `small: Option<Picture>`. The kit draws the
  picture small (`layout/pictures/small.rs`) as one `small` piece, 24 design units high, with the captions the
  picture's card already has (`sheets.rs` now shares `count`, `place` and `TOSSED`). The app passes a picture and
  never a word. A bubble with none is placed as before.
- **The app's rule** (`app/src/drawing/ended.rs`), read from the scene alone:

  | The scene says | The mark |
  |---|---|
  | `kept-<wish>` drawn with its sheets in the folder, little one idle | that picture: folder with tick, `N files` / `1 file`, `in your folder` |
  | little one greyed (core: let go), a note of the wish on the scene | crossed sheet, `thrown away`, quiet ink, no count |
  | `told-<wish>` whose first closing sentence is `told::UNCHANGED`, little one idle | empty sheet, `nothing changed` |
  | anything else (kept by a tap, a kept change the folder no longer shows, no note) | no mark |

- It is set in `Open::column`, after `column::column`, only for a wish that is tucked away and settled.
- The picture example has `--tucked` (`drawing::first`): the page as a window that opens on it draws it.

## What the ends are today
- Tossed: core's answer `said-<wish>` "The change was thrown away." and, after a look, `wish-<wish>` "This wish
  was let go."; the little one is greyed. Mark: `thrown away`.
- Undone (`Undo it`): core's answer "The change was undone." and the work's note is back and waits. The wish is
  not settled, so it is not tucked away and has no mark. Tossed after that, it is let go as above.
- Stopped, or a run that failed: the wish needs the person and is never tucked away. No mark.
- Kept by a tap: core's answer "The change was kept." and no drawing of the folder. No mark.

## What was red first
With the kit ignoring `small` and the app marking nothing: the kit's click test (no `small` piece placed), the
unit test of the marks (`[None, None, None, None]`), and two window tests (the folder mark after the second wish,
the let-go mark after a toss). Each failed on its own assertion. `tests/small.rs` was not run red: cargo stopped
at the first failing test file. It first ran in the green round, where one of its three tests failed on a wrong
expectation of the test (it counted the paper's pieces); the test was corrected.

## Mutations
| Mutation | Killed by |
|---|---|
| a mark on every wish, tucked or not | `ended.rs` unit test (the newest wish had a mark) |
| `in your folder` whatever the state | both `ended.rs` unit tests |
| the count of the newest wish's note | `ended.rs` unit test (five files, not three) |
| the mark kept when the wish comes back | `ended.rs` unit test, at the same assertion as the first |
| a tap on the mark does nothing | `tests/column_draw.rs` click test |
| a missing result note still draws a mark | window test of a change kept by a tap |

## Pictures
`.tools/pictures/ended-three-{sticker,plain}.png`, `ended-three-before-{sticker,plain}.png` and
`ended-answered-{sticker,plain}.png` (not tracked). The "before" pair was drawn with this build and no mark,
not with the parent's build: a bubble without the word is placed as before, which `tests/small.rs` holds.

## Open
- The let-go mark says `thrown away` of every let-go wish, also one let go with no run, where no change was made.
- A wish kept by a tap has no mark at all: the scene has no drawing of its folder, only core's sentence.
- The chevron is small; whether a first-time person takes it for "tap me" was judged from pictures only.
- Not run: a real window, a real harness, Windows, Linux.
