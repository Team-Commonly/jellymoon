//! What a text note holds besides its own words: someone else's words, quoted, and a field to type one line in
//! with the offer over it. Both are pieces of their own inside the note's sheet, so a reader of the layout tells
//! them from the note's own text by their kind, and the view draws each on a ground of its own.
//!
//! Quoted words are text and nothing else: cleaned as every text is, in the role for quoted words, and cut to
//! their room with an ellipsis. Nothing in them is a link or a control, whatever they say. The field and its
//! offer are the two pieces a person acts on.

use super::notes::Sheet;
use super::{Ctx, PieceKind, line_height, piece, run};
use crate::geom::Rect;
use crate::scene::Field;
use crate::text::{Role, wrap};

/// The most lines someone else's words are drawn in under a note. What does not fit is cut, and the last line
/// ends with an ellipsis.
pub const MAX_QUOTED_LINES: usize = 8;

/// Someone else's words, under what the sheet holds so far: one `Quote` piece with a run for each line. Nothing
/// for words that clean to nothing.
pub(super) fn quoted(cx: &Ctx, sheet: &mut Sheet, id: &str, words: &str) {
    let (role, size) = (Role::Quoted, cx.size(Role::Quoted));
    let (height, pad_x, pad_y) = (line_height(size), cx.px(10.0), cx.px(7.0));
    let lines = wrap(&cx.clean(words), sheet.inner - 2.0 * pad_x, MAX_QUOTED_LINES, role, size, cx.measure);
    if lines.is_empty() {
        return;
    }
    let rect = Rect::new(sheet.left, sheet.cursor, sheet.inner, lines.len() as f32 * height + 2.0 * pad_y);
    let mut block = piece(PieceKind::Quote, id, 0, rect);
    for (i, (line, width)) in lines.into_iter().enumerate() {
        let mid = rect.y + pad_y + (i as f32 + 0.5) * height;
        block.text.push(run(line, role, rect.x + pad_x, mid, width, size));
    }
    sheet.options.push(block);
    sheet.cursor = rect.bottom() + cx.px(10.0);
}

/// The field of a note, under what the sheet holds so far, as wide as the note's text: an `Offer` with the
/// field's offer over it when it has one, then the `Field` itself, with the hint as its text.
pub(super) fn field(cx: &Ctx, sheet: &mut Sheet, id: &str, field: &Field) {
    let (words, width) = cx.line(&field.offer, Role::Legend, sheet.inner - 2.0 * cx.px(10.0));
    if !words.is_empty() {
        let (size, pad) = (cx.size(Role::Legend), cx.px(10.0));
        let rect = Rect::new(sheet.left, sheet.cursor, width + 2.0 * pad, cx.px(24.0).max(line_height(size) + 2.0));
        let mut offer = piece(PieceKind::Offer, id, 0, rect);
        offer.text.push(run(words, Role::Legend, rect.x + pad, rect.center().1, width, size));
        sheet.options.push(offer);
        sheet.cursor = rect.bottom() + cx.px(6.0);
    }
    let (size, pad) = (cx.size(Role::RowSub), cx.px(14.0));
    let rect = Rect::new(sheet.left, sheet.cursor, sheet.inner, cx.px(32.0).max(line_height(size) + 2.0));
    let mut placed = piece(PieceKind::Field, id, 0, rect);
    let (hint, width) = cx.line(&field.hint, Role::RowSub, rect.w - 2.0 * pad);
    if !hint.is_empty() {
        placed.text.push(run(hint, Role::RowSub, rect.x + pad, rect.center().1, width, size));
    }
    sheet.options.push(placed);
    sheet.cursor = rect.bottom() + cx.px(10.0);
}
