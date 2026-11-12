//! The page itself: paper, punched holes, the dated tape, the title, the
//! big character, the legend and the footer line.

use super::entry::band_bottom;
use super::{Ctx, PieceKind, line_height, piece, run};
use crate::geom::Rect;
use crate::scene::{Mark, Scene};
use crate::text::Role;

/// Body outline of a character in its own units: 200 wide, and from 14
/// above its anchor to the feet 154 below.
pub(crate) const BODY_TOP: f32 = 14.0;
pub(crate) const BODY_BOTTOM: f32 = 154.0;

/// The most holes punched down the paper's edge, whatever height a theme gives the page.
const MAX_HOLES: usize = 150;

pub(crate) fn header(cx: &mut Ctx, scene: &Scene) {
    cx.out.push(piece(PieceKind::Paper, "paper", 0, cx.paper()));
    holes(cx);
    date_tape(cx, scene);
    title(cx, scene);
}

fn holes(cx: &mut Ctx) {
    let m = cx.theme.metrics;

    let radius = cx.px(7.5);
    let hole_x = cx.x(m.margin_x + 36.0);
    let mut y = m.margin_y + 70.0;
    let mut index = 0;
    while y < m.height - m.margin_y - 26.0 && index < MAX_HOLES {
        let rect = Rect::new(hole_x - radius, cx.y(y) - radius, 2.0 * radius, 2.0 * radius);
        cx.out.push(piece(PieceKind::Hole, "hole", index, rect));
        y += 66.0;
        index += 1;
    }
}

fn date_tape(cx: &mut Ctx, scene: &Scene) {
    let m = cx.theme.metrics;
    let tape = Rect::new(cx.x(m.content_left - 2.0), cx.y(m.margin_y + 32.0), cx.px(232.0), cx.px(36.0));
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

fn title(cx: &mut Ctx, scene: &Scene) {
    let m = cx.theme.metrics;
    let size = cx.size(Role::Title);
    let left = cx.x(m.content_left);
    let (text, width) = cx.line(&scene.title, Role::Title, cx.content_right() - left);
    let height = line_height(size);
    let top = title_bottom(cx) - height;
    let mut title = piece(PieceKind::Title, "title", 0, Rect::new(left, top, width, height));
    title.text = vec![run(text, Role::Title, left, top + height / 2.0, width, size)];
    cx.out.push(title);
}

pub(crate) fn title_bottom(cx: &Ctx) -> f32 {
    cx.y(cx.theme.metrics.margin_y + 78.0) + line_height(cx.size(Role::Title))
}

/// Places the big character and returns the top of the space left for the
/// small ones, which is never above the band an entry stands in.
pub(crate) fn hero(cx: &mut Ctx, scene: &Scene) -> f32 {
    let m = cx.theme.metrics;
    let Some(hero) = &scene.hero else {
        return band_bottom(cx);
    };
    let unit = cx.px(m.hero_size) / 200.0;
    let rect = Rect::new(
        cx.x(m.hero_x) - 100.0 * unit,
        cx.y(m.hero_top) - BODY_TOP * unit,
        200.0 * unit,
        (BODY_TOP + BODY_BOTTOM) * unit,
    );
    cx.out.push(piece(PieceKind::Hero, &hero.id, 0, rect));
    (rect.bottom() + cx.px(50.0)).max(band_bottom(cx))
}

/// The design line the legend sits on; the grid of characters stops above it.
pub(crate) fn legend_top(cx: &Ctx) -> f32 {
    cx.y(cx.theme.metrics.height - 118.0)
}

pub(crate) fn legend_and_footer(cx: &mut Ctx, scene: &Scene) {
    let right = cx.paper().right() - cx.px(40.0);
    legend(cx, scene, right);
    footer(cx, scene, right);
}

fn legend(cx: &mut Ctx, scene: &Scene, right: f32) {
    let m = cx.theme.metrics;
    let size = cx.size(Role::Legend);
    let height = line_height(size);
    let mid_y = legend_top(cx) + height / 2.0;
    let (mark, gap, between) = (cx.px(20.0), cx.px(6.0), cx.px(18.0));
    let mut left = cx.x(m.content_left);
    for (index, item) in scene.legend.iter().enumerate() {
        let room = right - left - mark - gap;
        let (text, width) = cx.line(&item.text, Role::Legend, room);
        if text.is_empty() {
            break;
        }
        let rect = Rect::new(left, mid_y - height / 2.0, mark + gap + width, height);
        let mut entry = piece(PieceKind::Legend, "legend", index, rect);
        if item.mark == Mark::Star {
            let star = cx.size(Role::Bullet);
            let star_width = cx.measure.width("★", Role::Bullet, star);
            let star_x = left + (mark - star_width) / 2.0;
            entry.text.push(run("★".into(), Role::Bullet, star_x, mid_y, star_width, star));
        }
        entry.text.push(run(text, Role::Legend, left + mark + gap, mid_y, width, size));
        cx.out.push(entry);
        left = rect.right() + between;
    }
}

fn footer(cx: &mut Ctx, scene: &Scene, right: f32) {
    let m = cx.theme.metrics;
    let size = cx.size(Role::Footer);
    let left = cx.x(m.content_left);
    let (text, width) = cx.line(&scene.footer, Role::Footer, right - left);
    if text.is_empty() {
        return;
    }
    let height = line_height(size);
    let top = cx.y(m.height - 84.0);
    let mut footer = piece(PieceKind::Footer, "footer", 0, Rect::new(left, top, width, height));
    footer.text = vec![run(text, Role::Footer, left, top + height / 2.0, width, size)];
    cx.out.push(footer);
}
