//! A small character and what stands with it: its stars over it, a bubble or a badge beside it, and the pills of
//! its label and of what it says. Every one is the theme's one size.

use super::page::{BODY_BOTTOM, BODY_TOP};
use super::{Ctx, PieceKind, Placed, line_height, piece, run};
use crate::geom::Rect;
use crate::scene::{Character, State};
use crate::text::{Role, fit_middle};

/// Reach of a character's art from its anchor, in body units (the body is 200 wide): stars above, the body to the
/// left, the badge to the right.
const REACH_UP: f32 = 87.0;
const REACH_LEFT: f32 = 109.0;
const REACH_RIGHT: f32 = 137.0;
pub(crate) const MAX_STARS: u8 = 4;

/// Logical pixels per body unit.
fn unit(cx: &Ctx) -> f32 {
    cx.px(cx.theme.metrics.character_size) / 200.0
}

/// The room a small character takes with its stars and its bubble or badge: its width and its height.
pub(super) fn room(cx: &Ctx) -> (f32, f32) {
    ((REACH_LEFT + REACH_RIGHT) * unit(cx), (REACH_UP + BODY_BOTTOM) * unit(cx))
}

/// The character `who`, the scene's `index`th, in its room from (`left`, `top`).
pub(super) fn figure(cx: &mut Ctx, (left, top): (f32, f32), index: usize, who: &Character) {
    let unit = unit(cx);
    let (mid_x, top) = (left + REACH_LEFT * unit, top + REACH_UP * unit);
    let body = Rect::new(mid_x - 100.0 * unit, top - BODY_TOP * unit, 200.0 * unit, (BODY_TOP + BODY_BOTTOM) * unit);
    cx.out.push(piece(PieceKind::Character, &who.id, index, body));
    let stars = who.stars.min(MAX_STARS);
    for i in 0..stars {
        let offset = f32::from(i) - f32::from(stars - 1) / 2.0;
        let lift = if i == 0 || i == stars - 1 { 0.0 } else { 13.0 };
        let centre = (mid_x + offset * 54.0 * unit, top - (43.5 + lift) * unit);
        cx.out.push(piece(PieceKind::Star, &who.id, usize::from(i), round_rect(centre, 21.7 * unit)));
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
}

fn round_rect(centre: (f32, f32), radius: f32) -> Rect {
    Rect::new(centre.0 - radius, centre.1 - radius, 2.0 * radius, 2.0 * radius)
}

/// The pill of a character's label, one line cut to `wide`, with its top left at the origin. None when nothing of
/// the label is left.
pub(super) fn label(cx: &Ctx, index: usize, who: &Character, wide: f32) -> Option<Placed> {
    let role = if who.state == State::Dimmed { Role::LabelStruck } else { Role::Label };
    let height = (cx.size(Role::Label) * 2.14).round();
    pill(cx, (PieceKind::Label, index, who), (&who.label, role), (wide, height))
}

/// The pill of what a busy character says, one line cut to `wide`, with its top left at the origin. None for a
/// character that is not busy or says nothing.
pub(super) fn says(cx: &Ctx, index: usize, who: &Character, wide: f32) -> Option<Placed> {
    let height = line_height(cx.size(Role::Label)) + cx.px(6.0);
    let said = (who.state == State::Busy).then_some(who.says.as_str())?;
    pill(cx, (PieceKind::Says, index, who), (said, Role::Label), (wide, height))
}

/// One line of `text` on a pill `height` high and at most `wide`, a label's width at most. What a character says
/// is cut in its middle: it ends in a file's name, and two files of one folder are told apart there.
fn pill(
    cx: &Ctx,
    (kind, index, who): (PieceKind, usize, &Character),
    (text, role): (&str, Role),
    (wide, height): (f32, f32),
) -> Option<Placed> {
    let pad = cx.px(13.0);
    let room = (wide.min(cx.px(cx.theme.labels.max_width)) - 2.0 * pad).max(0.0);
    let (text, width) = match kind {
        PieceKind::Says => fit_middle(&cx.clean(text), room, role, cx.size(role), cx.measure),
        _ => cx.line(text, role, room),
    };
    if text.is_empty() {
        return None;
    }
    let mut tag = piece(kind, &who.id, index, Rect::new(0.0, 0.0, width + 2.0 * pad, height));
    tag.text = vec![run(text, role, pad, height / 2.0, width, cx.size(role))];
    Some(tag)
}

/// Puts `piece`, made at the origin, at (`x`, `y`). Returns where it is.
pub(super) fn put(cx: &mut Ctx, mut piece: Placed, (x, y): (f32, f32)) -> Rect {
    piece.rect = piece.rect.moved(x, y);
    for run in &mut piece.text {
        run.rect = run.rect.moved(x, y);
    }
    let rect = piece.rect;
    cx.out.push(piece);
    rect
}
