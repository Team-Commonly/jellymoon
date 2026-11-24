//! A drawn note (`Note::Drawn`): a card of the column's width with a picture, the note's own words under it when
//! the scene says it is open, and its options under those. While it is closed none of its words is placed, and a
//! small `?` in a pencil ring stands at the picture's top right when it has any. The kit keeps no state for
//! this: `open` is the scene's.

use super::notes::{MAX_OPTIONS, OPTION_LINES, Sheet, choice_option, close, open};
use super::{Ctx, PieceKind, Placed, line_height, pictures, piece, run};
use crate::geom::Rect;
use crate::pencil::{Part, Pencil, Sketch};
use crate::scene::Picture;
use crate::text::{Role, clean, has_visible, wrap};

/// What a card keeps clear at each side of its picture, in design units.
pub(super) const SIDE: f32 = 24.0;

/// The sheet of the drawn note `id` at `top`.
pub(super) fn sheet(
    cx: &Ctx,
    id: &str,
    (picture, words, opened, options): (&Picture, &[String], bool, &[String]),
    top: f32,
) -> Sheet {
    let mut sheet = open(cx, top, (22.0, SIDE));
    let drawn = pictures::draw(cx, id, picture, Rect::new(sheet.left, sheet.cursor, sheet.inner, 0.0));
    let (at, gap) = (drawn.rect, cx.px(9.0));
    sheet.cursor = at.bottom() + gap;
    sheet.options.push(drawn);
    let words: Vec<&String> = words.iter().filter(|text| has_visible(text)).collect();
    if !words.is_empty() {
        sheet.options.push(ask(cx, id, at));
    }
    let (size, shown) = (cx.size(Role::Body), if opened { words.as_slice() } else { &[] });
    for text in shown {
        // Each of the words is the note's own and is drawn to its end, on as many lines as that takes.
        for (line, width) in wrap(&clean(text, usize::MAX), sheet.inner, usize::MAX, Role::Body, size, cx.measure) {
            let mid = sheet.cursor + line_height(size) / 2.0;
            sheet.runs.push(run(line, Role::Body, sheet.left, mid, width, size));
            sheet.cursor += line_height(size);
        }
        sheet.cursor += gap;
    }
    for (index, option) in options.iter().take(MAX_OPTIONS).enumerate() {
        choice_option(cx, &mut sheet, (id, index), option, Role::Option, OPTION_LINES);
    }
    close(cx, sheet, 20.0)
}

/// The mark that says a picture has words: a `?` in a ring the pencil draws, at the picture's top right corner.
fn ask(cx: &Ctx, id: &str, picture: Rect) -> Placed {
    let side = cx.px(24.0);
    let rect = Rect::new(picture.right() - side, picture.y, side, side);
    let mut mark = piece(PieceKind::Ask, id, 0, rect);
    let mut pencil = Pencil::new(&format!("{id}?"), cx.k);
    pencil.circle(rect.center(), side / 2.0 - cx.px(2.0));
    let ring = Part { name: "ring".to_string(), rect };
    mark.sketch = Some(Sketch { what: "ask".to_string(), parts: vec![ring], strokes: pencil.finish() });
    let (size, width) = (cx.size(Role::Caption), cx.measure.width("?", Role::Caption, cx.size(Role::Caption)));
    mark.text.push(run("?".to_string(), Role::Caption, rect.center().0 - width / 2.0, rect.center().1, width, size));
    mark
}
