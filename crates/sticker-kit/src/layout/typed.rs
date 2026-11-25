//! What a text note holds besides its own words: someone else's words, quoted, and a field to type one line in
//! with the offer over it. Both are pieces of their own inside the note's sheet, so a reader of the layout tells
//! them from the note's own text by their kind, and the view draws each on a ground of its own.
//!
//! Quoted words are text and nothing else: cleaned as every text is, in the role for quoted words, and cut to
//! their lines with an ellipsis. What a person typed (`Note::Said`) is drawn by the same rules ([`block`]).
//! Nothing in them is a link or a control, whatever they say. The field and its offer are the two pieces a person
//! acts on. A note's quote has a line of the pencil down its left ([`lined`]) and none round it. A field that is
//! first (`Field::first`) stands before the options, which are then small and quiet ([`quiet`]).

use super::notes::Sheet;
use super::{Ctx, PieceKind, Placed, line_height, piece, run};
use crate::geom::Rect;
use crate::pencil::{Part, Pencil, Sketch};
use crate::scene::Field;
use crate::text::{Role, wrap};

/// The most lines someone else's words are drawn in under a note. What does not fit is cut, and the last line
/// ends with an ellipsis.
pub const MAX_QUOTED_LINES: usize = 8;

/// Someone else's words, under what the sheet holds so far: one `Quote` piece with a run for each line. Nothing
/// for words that clean to nothing.
pub(super) fn quoted(cx: &Ctx, sheet: &mut Sheet, id: &str, words: &str) {
    let at = Rect::new(sheet.left, sheet.cursor, sheet.inner, 0.0);
    if let Some(block) = block(cx, (id, 0), words, at, MAX_QUOTED_LINES) {
        sheet.cursor = block.rect.bottom() + cx.px(10.0);
        sheet.options.push(lined(cx, block));
    }
}

/// A note's quote with one line of the pencil down its left, before its words: its sketch, `quoted`, with the
/// one part `line`. The line is what marks the words as someone else's; no line goes round them, as round a field.
/// Its ground hugs its words: from the same left, as wide as its widest line and the padding the words have before
/// them, and never wider than it was given. A ground as wide as the note reads as a field one cannot type in.
pub(super) fn lined(cx: &Ctx, mut block: Placed) -> Placed {
    let pad = block.text.first().map_or(0.0, |run| run.rect.x - block.rect.x);
    let widest = block.text.iter().map(|run| run.rect.right()).fold(block.rect.x, f32::max);
    block.rect.w = block.rect.w.min(widest + pad - block.rect.x);
    let (x, inset) = (block.rect.x + cx.px(3.0), cx.px(4.0).min(block.rect.h / 4.0));
    let (top, low) = (block.rect.y + inset, block.rect.bottom() - inset);
    let mut pencil = Pencil::new(&format!("{}|{}", block.id, block.index), cx.k);
    pencil.line((x, top), (x, low));
    let part = Part { name: "line".to_string(), rect: Rect::new(x, top, 0.0, low - top) };
    block.sketch = Some(Sketch { what: "quoted".to_string(), parts: vec![part], strokes: pencil.finish() });
    block
}

/// The options of a note whose field is first, after that field: the ways round it, each a small pill on one
/// line, side by side and wrapped to a next row. Each is an `Option` piece of its index, as a wide one is.
pub(super) fn quiet(cx: &Ctx, sheet: &mut Sheet, id: &str, options: &[String]) {
    let (size, pad, gap) = (cx.size(Role::Legend), cx.px(10.0), cx.px(8.0));
    let (high, mut x) = (cx.px(24.0).max(line_height(size) + 2.0), sheet.left);
    for (index, option) in options.iter().enumerate() {
        let (words, width) = cx.line(option, Role::Legend, sheet.inner - 2.0 * pad);
        if x > sheet.left && x + width + 2.0 * pad > sheet.left + sheet.inner {
            (x, sheet.cursor) = (sheet.left, sheet.cursor + high + gap);
        }
        let rect = Rect::new(x, sheet.cursor, width + 2.0 * pad, high);
        let mut pill = piece(PieceKind::Option, id, index, rect);
        pill.text.push(run(words, Role::Legend, rect.x + pad, rect.center().1, width, size));
        sheet.options.push(pill);
        x = rect.right() + gap;
    }
    if !options.is_empty() {
        sheet.cursor += high + cx.px(10.0);
    }
}

/// Words that are not the scene's own, on a ground of their own from `at`'s top left and as wide as `at`: a
/// `Quote` piece of `id`, cleaned as every text is, in the role for quoted words and no other, a run for each of
/// at most `most` lines. None for words that clean to nothing.
pub(super) fn block(cx: &Ctx, (id, index): (&str, usize), words: &str, at: Rect, most: usize) -> Option<Placed> {
    let (role, size) = (Role::Quoted, cx.size(Role::Quoted));
    let (height, pad_x, pad_y) = (line_height(size), cx.px(10.0), cx.px(7.0));
    let lines = wrap(&cx.clean(words), at.w - 2.0 * pad_x, most, role, size, cx.measure);
    if lines.is_empty() {
        return None;
    }
    let rect = Rect::new(at.x, at.y, at.w, lines.len() as f32 * height + 2.0 * pad_y);
    let mut block = piece(PieceKind::Quote, id, index, rect);
    for (i, (line, width)) in lines.into_iter().enumerate() {
        let mid = rect.y + pad_y + (i as f32 + 0.5) * height;
        block.text.push(run(line, role, rect.x + pad_x, mid, width, size));
    }
    Some(block)
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
