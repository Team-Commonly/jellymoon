//! The entry: a field to type one line in, and under it a chip for each option, in one row or two. It stands in one spot,
//! in the left column under the title, from the content's left edge to short of the big character's place, whatever
//! else the page holds. Every page keeps that band free ([`band_bottom`]), so the entry is placed by itself.

use super::page::title_bottom;
use super::{Ctx, PieceKind, Placed, line_height, piece, run};
use crate::entry::MAX_ENTRY_OPTIONS;
use crate::geom::{Rect, Viewport};
use crate::scene::Entry;
use crate::text::{Measure, Role, fit_middle};
use crate::theme::Theme;

/// Where the pieces of an entry go: the field (`Entry`, with the hint as its text), then a `Chip` for each of the
/// first [`MAX_ENTRY_OPTIONS`] options, its `index` the option's. Nothing for a theme [`Theme::check`] refuses, or
/// one whose own sizes leave no room left of its big character.
pub fn layout_entry(entry: &Entry, theme: &Theme, viewport: Viewport, measure: &dyn Measure) -> Vec<Placed> {
    if theme.check().is_err() {
        return Vec::new();
    }
    let cx = Ctx::new(theme, viewport, measure, true);
    let m = &theme.metrics;
    // The big character's place is kept clear whether or not the page has one: the entry knows no scene.
    let hero = cx.x(m.hero_x) - cx.px(m.hero_size) / 2.0 - cx.px(GAP);
    let (left, right) = (cx.x(m.content_left), hero.min(cx.content_right()));
    if right - left < cx.px(160.0) {
        return Vec::new();
    }
    let size = cx.size(Role::RowSub);
    let field = Rect::new(left, field_top(&cx), right - left, field_height(&cx));
    let pad = cx.px(14.0);
    let mut out = vec![piece(PieceKind::Entry, &entry.id, 0, field)];
    let (hint, width) = cx.line(&entry.hint, Role::RowSub, field.w - 2.0 * pad);
    if !hint.is_empty() {
        out[0].text.push(run(hint, Role::RowSub, field.x + pad, field.center().1, width, size));
    }
    chips(&cx, entry, field, &mut out);
    out
}

/// What the field keeps from the big character's place, and what the page's small characters keep from the chips.
const GAP: f32 = 24.0;
const UNDER_TITLE: f32 = 6.0;
const CHIP_GAP: f32 = 6.0;

fn field_top(cx: &Ctx) -> f32 {
    title_bottom(cx) + cx.px(UNDER_TITLE)
}

fn field_height(cx: &Ctx) -> f32 {
    cx.px(32.0).max(line_height(cx.size(Role::RowSub)) + 2.0)
}

fn chip_height(cx: &Ctx) -> f32 {
    cx.px(24.0).max(line_height(cx.size(Role::Legend)) + 2.0)
}

/// Where the band an entry stands in ends, two rows of chips under the field and the room under those: no page
/// puts a small character above this line, entry or none, so an entry that comes or goes moves no piece.
pub(crate) fn band_bottom(cx: &Ctx) -> f32 {
    field_top(cx) + field_height(cx) + 2.0 * (cx.px(CHIP_GAP) + chip_height(cx)) + cx.px(GAP)
}

/// The least a chip is wide, in its heights: a name of one letter is still a chip to click.
const LEAST: f32 = 1.5;

/// The options under the field, each as wide as its name while its row has the room: in one row when that holds
/// them all, else in two. A row too narrow for its names gives the short ones their width and shares what is left
/// between the long ones ([`widest`]), which are cut in the middle with an ellipsis. So every option is there to be
/// chosen, no name is cut while its row has room, and two names that begin alike are told apart by their ends.
fn chips(cx: &Ctx, entry: &Entry, field: Rect, out: &mut Vec<Placed>) {
    let count = entry.options.len().min(MAX_ENTRY_OPTIONS);
    let size = cx.size(Role::Legend);
    let (gap, pad, height) = (cx.px(CHIP_GAP), cx.px(10.0), chip_height(cx));
    let names: Vec<String> = entry.options.iter().take(count).map(|name| cx.clean(name)).collect();
    let asks = |name: &String| (cx.measure.width(name, Role::Legend, size) + 2.0 * pad).max(LEAST * height);
    let asked: Vec<f32> = names.iter().map(asks).collect();
    let all = asked.iter().sum::<f32>() + gap * count.saturating_sub(1) as f32;
    let (one, two) = asked.split_at(if all <= field.w { count } else { count.div_ceil(2) });
    let (mut left, mut top) = (field.x, field.bottom() + gap);
    for (index, name) in names.iter().enumerate() {
        let row = if index < one.len() { one } else { two };
        if index == one.len() {
            (left, top) = (field.x, top + height + gap);
        }
        let width = asked[index].min(widest(row, field.w - gap * (row.len() - 1) as f32));
        let rect = Rect::new(left, top, width, height);
        // A chip as wide as its name asks holds it whole, whatever taking the padding off again rounds to.
        let (name, text) = fit_middle(name, rect.w - 2.0 * pad + 0.01, Role::Legend, size, cx.measure);
        let mut chip = piece(PieceKind::Chip, &entry.id, index, rect);
        if !name.is_empty() {
            chip.text.push(run(name, Role::Legend, rect.x + (rect.w - text) / 2.0, rect.center().1, text, size));
        }
        out.push(chip);
        left = rect.right() + gap;
    }
}

/// The widest a chip of a row may be, for chips that ask `asked` and have `room` between them: the narrow ones are
/// as wide as they ask, and the room they leave is shared by the ones that ask for more than their share of it.
fn widest(asked: &[f32], mut room: f32) -> f32 {
    let mut asked = asked.to_vec();
    asked.sort_by(f32::total_cmp);
    for (n, ask) in asked.iter().enumerate() {
        let share = room / (asked.len() - n) as f32;
        if *ask > share {
            return share;
        }
        room -= ask;
    }
    f32::INFINITY
}
