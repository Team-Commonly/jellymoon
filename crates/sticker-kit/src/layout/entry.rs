//! The entry: a field to type one line in, and under it a row of chips, one for each option. It stands in one spot,
//! right of the dated tape and above the title, up to where the notes begin, whatever else the page holds, so it is
//! placed by itself and moves no other piece.

use super::{Ctx, PieceKind, Placed, line_height, piece, run};
use crate::entry::MAX_ENTRY_OPTIONS;
use crate::geom::{Rect, Viewport};
use crate::scene::Entry;
use crate::text::{Measure, Role};
use crate::theme::Theme;

/// Where the pieces of an entry go: the field (`Entry`, with the hint as its text), then a `Chip` for each of the
/// first [`MAX_ENTRY_OPTIONS`] options, its `index` the option's. Nothing for a theme [`Theme::check`] refuses, or
/// one whose own sizes leave no room beside its dated tape.
pub fn layout_entry(entry: &Entry, theme: &Theme, viewport: Viewport, measure: &dyn Measure) -> Vec<Placed> {
    if theme.check().is_err() {
        return Vec::new();
    }
    let cx = Ctx::new(theme, viewport, measure, true);
    let m = &theme.metrics;
    // The dated tape is 232 wide from two units left of the content.
    let (left, right) = (cx.x(m.content_left + 254.0), cx.content_right());
    if right - left < cx.px(160.0) {
        return Vec::new();
    }
    let size = cx.size(Role::RowSub);
    let field = Rect::new(left, cx.y(m.margin_y + 6.0), right - left, cx.px(32.0).max(line_height(size) + 2.0));
    let pad = cx.px(14.0);
    let mut out = vec![piece(PieceKind::Entry, &entry.id, 0, field)];
    let (hint, width) = cx.line(&entry.hint, Role::RowSub, field.w - 2.0 * pad);
    if !hint.is_empty() {
        out[0].text.push(run(hint, Role::RowSub, field.x + pad, field.center().1, width, size));
    }
    chips(&cx, entry, field, &mut out);
    out
}

/// The options in a row under the field, each as wide as its name and no wider than its share of the row, so that
/// every one of them is there to be chosen. A name too long for its share is cut with an ellipsis.
fn chips(cx: &Ctx, entry: &Entry, field: Rect, out: &mut Vec<Placed>) {
    let count = entry.options.len().min(MAX_ENTRY_OPTIONS);
    if count == 0 {
        return;
    }
    let size = cx.size(Role::Legend);
    let (gap, pad) = (cx.px(6.0), cx.px(10.0));
    let height = cx.px(24.0).max(line_height(size) + 2.0);
    let share = (field.w - gap * (count - 1) as f32) / count as f32;
    let (mut left, top) = (field.x, field.bottom() + gap);
    for (index, option) in entry.options.iter().take(count).enumerate() {
        let (name, width) = cx.line(option, Role::Legend, share - 2.0 * pad);
        let rect = Rect::new(left, top, (width + 2.0 * pad).max(1.5 * height).min(share), height);
        let mut chip = piece(PieceKind::Chip, &entry.id, index, rect);
        if !name.is_empty() {
            chip.text.push(run(name, Role::Legend, rect.x + (rect.w - width) / 2.0, rect.center().1, width, size));
        }
        out.push(chip);
        left = rect.right() + gap;
    }
}
