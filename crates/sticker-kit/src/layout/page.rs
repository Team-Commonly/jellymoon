//! The page itself: paper, punched holes, the dated tape, the title and the
//! line under it that says what the page was made from.

use super::entry::entry_top;
use super::{Ctx, PieceKind, line_height, piece, run};
use crate::geom::Rect;
use crate::scene::Scene;
use crate::text::{Role, wrap};

/// Body outline of a character in its own units: 200 wide, and from 14
/// above its anchor to the feet 154 below.
pub(crate) const BODY_TOP: f32 = 14.0;
pub(crate) const BODY_BOTTOM: f32 = 154.0;

/// The lines the footer is drawn in at most.
const FOOTER_LINES: usize = 4;

/// The most holes punched down the paper's edge, however long the column is.
const MAX_HOLES: usize = 150;

/// The head of the column: the dated tape, the title, and the scene's footer in quiet lines under it. Returns
/// where the column goes on.
pub(crate) fn header(cx: &mut Ctx, scene: &Scene) -> f32 {
    date_tape(cx, scene);
    let under = title(cx, scene);
    footer(cx, scene, under + cx.px(6.0)) + cx.px(30.0)
}

/// The paper under a column that ends at `bottom`, and its holes, put under every piece placed so far. It is never
/// shorter than what a window shows over the entry's band.
pub(crate) fn paper(cx: &mut Ctx, bottom: f32) {
    let m = cx.theme.metrics;
    let top = cx.px(m.margin_y);
    let shown = entry_top(cx.theme, cx.viewport) - top;
    let high = (bottom + cx.px(40.0)).max(shown) - top;
    let rect = Rect::new(cx.x(m.margin_x), top, cx.px(m.width - 2.0 * m.margin_x), high);
    let mut under = vec![piece(PieceKind::Paper, "paper", 0, rect)];
    let radius = cx.px(7.5);
    let hole_x = cx.x(m.margin_x + 36.0);
    let mut y = top + cx.px(70.0);
    while y < rect.bottom() - cx.px(26.0) && under.len() <= MAX_HOLES {
        let hole = Rect::new(hole_x - radius, y - radius, 2.0 * radius, 2.0 * radius);
        under.push(piece(PieceKind::Hole, "hole", under.len() - 1, hole));
        y += cx.px(66.0).max(1.0);
    }
    under.append(&mut cx.out);
    cx.out = under;
}

fn date_tape(cx: &mut Ctx, scene: &Scene) {
    let m = cx.theme.metrics;
    let tape = Rect::new(cx.x(m.content_left - 2.0), cx.px(m.margin_y + 32.0), cx.px(232.0), cx.px(36.0));
    let size = cx.size(Role::Date);
    let (text, width) = cx.line(&scene.date_label, Role::Date, tape.w - cx.px(24.0));
    let (mid_x, mid_y) = tape.center();
    let mut date = piece(PieceKind::Tape, "date", 0, tape);
    date.tilt_degrees = cx.theme.tilted(cx.theme.tilt.date_tape);
    date.z = 2;
    if !text.is_empty() {
        date.text = vec![run(text, Role::Date, mid_x - width / 2.0, mid_y, width, size)];
    }
    cx.out.push(date);
}

/// The title, one line as wide as the column at most. Returns its bottom.
fn title(cx: &mut Ctx, scene: &Scene) -> f32 {
    let size = cx.size(Role::Title);
    let (left, wide) = cx.column();
    let (text, width) = cx.line(&scene.title, Role::Title, wide);
    let height = line_height(size);
    let top = cx.px(cx.theme.metrics.margin_y + 78.0);
    let mut title = piece(PieceKind::Title, "title", 0, Rect::new(left, top, width, height));
    title.text = vec![run(text, Role::Title, left, top + height / 2.0, width, size)];
    cx.out.push(title);
    top + height
}

/// The scene's footer at `top`, in at most [`FOOTER_LINES`] lines of the column's width. Returns its bottom, or
/// `top` when it says nothing.
fn footer(cx: &mut Ctx, scene: &Scene, top: f32) -> f32 {
    let size = cx.size(Role::Footer);
    let (left, wide) = cx.column();
    let lines = wrap(&cx.clean(&scene.footer), wide, FOOTER_LINES, Role::Footer, size, cx.measure);
    if lines.is_empty() {
        return top;
    }
    let height = line_height(size);
    let widest = lines.iter().map(|(_, width)| *width).fold(0.0, f32::max);
    let mut footer = piece(PieceKind::Footer, "footer", 0, Rect::new(left, top, widest, 0.0));
    for (line, width) in lines {
        footer.text.push(run(line, Role::Footer, left, top + footer.rect.h + height / 2.0, width, size));
        footer.rect.h += height;
    }
    let bottom = footer.rect.bottom();
    cx.out.push(footer);
    bottom
}
