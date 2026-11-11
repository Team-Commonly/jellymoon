//! The small characters: staggered columns, each with its stars, a bubble
//! or badge, and a fitted label. More characters than the theme's slots
//! shrink and flow into more rows and columns; they never overlap.

use super::page::{BODY_BOTTOM, BODY_TOP, legend_top};
use super::{Ctx, PieceKind, piece, run};
use crate::geom::Rect;
use crate::scene::{Character, Scene, State};
use crate::text::Role;

/// Reach of a character's art from its anchor, in body units (the body is
/// 200 wide): stars above, the label below, the body to the left, the
/// badge to the right.
const REACH_UP: f32 = 87.0;
const LABEL_TOP: f32 = 187.0;
const REACH_LEFT: f32 = 109.0;
const REACH_RIGHT: f32 = 137.0;
pub(crate) const MAX_STARS: u8 = 4;

struct Grid {
    cols: usize,
    /// Logical pixels per body unit.
    unit: f32,
    gap: f32,
    first_x: f32,
    first_y: f32,
    pitch: f32,
    stagger: f32,
    label_height: f32,
    rim: f32,
}

pub(crate) fn characters(cx: &mut Ctx, scene: &Scene, top: f32) {
    let count = scene.characters.len();
    if count == 0 {
        return;
    }
    let mut order: Vec<usize> = (0..count).collect();
    order.sort_by_key(|&i| scene.characters[i].state == State::Dimmed);
    let grid = fit_grid(cx, count, top);
    for (slot, &index) in order.iter().enumerate() {
        place(cx, &grid, slot, index, &scene.characters[index]);
    }
}

/// Picks the column count that leaves the characters largest, starting
/// from the theme's own.
fn fit_grid(cx: &Ctx, count: usize, top: f32) -> Grid {
    let m = cx.theme.metrics;
    let left = cx.x(m.content_left);
    let right = if cx.has_notes { cx.x(m.note_left - m.note_gutter) } else { cx.paper().right() - cx.px(60.0) };
    let width = (right - left).max(1.0);
    let rim = cx.px(m.rim);
    let label_height = (cx.size(Role::Label) * 2.14).round();
    let row_gap = cx.px(m.row_gap);
    let stagger_step = label_height + cx.px(6.0);
    let room = legend_top(cx) - cx.px(10.0) - top - rim;
    let largest = cx.px(m.character_size) / 200.0;
    let base = m.columns.max(1);

    let mut best = (base, 0.0_f32, 0.0_f32);
    for cols in base..=base.max(count) {
        let rows = count.div_ceil(cols) as f32;
        let between = (cols - 1) as f32;
        let stagger = if cols > 1 && count > 1 { stagger_step } else { 0.0 };
        let fixed = (rows - 1.0) * (label_height + row_gap) + stagger + label_height;
        let by_height = (room - fixed) / (REACH_UP + LABEL_TOP + (rows - 1.0) * (REACH_UP + LABEL_TOP));
        let by_width = (width - 2.0 * rim - between * (2.0 * rim + 2.0))
            / (REACH_LEFT + REACH_RIGHT + between * (100.0 + REACH_RIGHT));
        let unit = largest.min(by_height).min(by_width);
        if unit > best.1 + 1e-4 {
            best = (cols, unit, stagger);
        }
        if unit >= largest - 1e-4 {
            break;
        }
    }
    let (cols, unit, stagger) = (best.0, best.1.max(0.01), best.2);
    let spread = width - (REACH_LEFT + REACH_RIGHT) * unit - 2.0 * rim;
    Grid {
        cols,
        unit,
        gap: if cols > 1 { spread / (cols - 1) as f32 } else { 0.0 },
        first_x: left + REACH_LEFT * unit + rim,
        first_y: top + REACH_UP * unit + rim,
        pitch: (REACH_UP + LABEL_TOP) * unit + label_height + row_gap,
        stagger,
        label_height,
        rim,
    }
}

fn place(cx: &mut Ctx, grid: &Grid, slot: usize, index: usize, who: &Character) {
    let (col, row) = (slot % grid.cols, slot / grid.cols);
    let unit = grid.unit;
    let mid_x = grid.first_x + col as f32 * grid.gap;
    let odd = if col % 2 == 1 { grid.stagger } else { 0.0 };
    let top = grid.first_y + row as f32 * grid.pitch + odd;

    let body = Rect::new(mid_x - 100.0 * unit, top - BODY_TOP * unit, 200.0 * unit, (BODY_TOP + BODY_BOTTOM) * unit);
    cx.out.push(piece(PieceKind::Character, &who.id, index, body));

    let stars = who.stars.min(MAX_STARS);
    for i in 0..stars {
        let offset = f32::from(i) - f32::from(stars - 1) / 2.0;
        let lift = if i == 0 || i == stars - 1 { 0.0 } else { 13.0 };
        let centre = (mid_x + offset * 54.0 * unit, top - (43.5 + lift) * unit);
        let star = round_rect(centre, 21.7 * unit);
        cx.out.push(piece(PieceKind::Star, &who.id, usize::from(i), star));
    }
    match who.state {
        State::Busy => {
            let rect = round_rect((mid_x + 108.7 * unit, top + 8.7 * unit), 24.0 * unit);
            cx.out.push(piece(PieceKind::Bubble, &who.id, index, rect));
        }
        State::NeedsYou => {
            let rect = round_rect((mid_x + 104.0 * unit, top + 4.3 * unit), 32.6 * unit);
            cx.out.push(piece(PieceKind::Badge, &who.id, index, rect));
        }
        State::Idle | State::Dimmed => {}
    }
    label(cx, grid, mid_x, top + LABEL_TOP * unit, index, who);
}

fn round_rect(centre: (f32, f32), radius: f32) -> Rect {
    Rect::new(centre.0 - radius, centre.1 - radius, 2.0 * radius, 2.0 * radius)
}

/// A pill under the character. Its half width is capped so that it cannot
/// reach a neighbour's art, the paper's edge or the notes.
fn label(cx: &mut Ctx, grid: &Grid, mid_x: f32, top: f32, index: usize, who: &Character) {
    let role = if who.state == State::Dimmed { Role::LabelStruck } else { Role::Label };
    let pad = cx.px(13.0);
    let mut half = cx.px(cx.theme.labels.max_width) / 2.0;
    if grid.cols > 1 {
        half = half.min(grid.gap - REACH_RIGHT * grid.unit - grid.rim - cx.px(4.0));
    }
    half = half.min(mid_x - cx.paper().x - cx.px(8.0)).min(cx.content_right() - mid_x);
    let (text, width) = cx.line(&who.label, role, 2.0 * (half - pad));
    if text.is_empty() {
        return;
    }
    let size = cx.size(role);
    let rect = Rect::new(mid_x - width / 2.0 - pad, top, width + 2.0 * pad, grid.label_height);
    let mut tag = piece(PieceKind::Label, &who.id, index, rect);
    tag.text = vec![run(text, role, mid_x - width / 2.0, top + grid.label_height / 2.0, width, size)];
    cx.out.push(tag);
}
