//! Changed lines a file, to one scale: a row for each file with its name at the left, a box hatched in the added
//! ink as long as its added lines and, after a small gap, one in the taken ink for its taken lines, then the two
//! counts. The row with the most lines fills the room there is, and a ruler under the rows is labelled in lines.

use super::Draft;
use crate::geom::Rect;
use crate::pencil::Ink;
use crate::scene::Bar;

/// The most rows drawn. The files past them are counted in a last line.
const MAX_BARS: usize = 6;

/// Where the boxes begin, the room kept right of them for the counts, a row's height and the gap between a
/// row's two boxes, in design units.
const LEFT: f32 = 236.0;
const COUNTS: f32 = 116.0;
const ROW: f32 = 26.0;
const GAP: f32 = 6.0;

/// Draws the picture and returns what it is, for the print.
pub(super) fn draw(d: &mut Draft, rows: &[Bar], more: usize) -> String {
    let shown = &rows[..rows.len().min(MAX_BARS)];
    let lines = |row: &Bar| row.added.saturating_add(row.taken);
    let most = shown.iter().map(lines).max().unwrap_or(0);
    let full = d.wide() - LEFT - COUNTS - GAP;
    for (n, row) in shown.iter().enumerate() {
        let top = 6.0 + n as f32 * ROW;
        let at = d.point(8.0, top);
        if !d.name(&row.name, d.u(LEFT - 24.0), at, 0.0) {
            d.caption("a file", at, 0.0);
        }
        let share = |count: usize| if most == 0 { 0.0 } else { full * count as f32 / most as f32 };
        let added = d.rect(LEFT, top + 6.0, share(row.added), 14.0);
        let after = if row.added > 0 { share(row.added) + GAP } else { 0.0 };
        let taken = d.rect(LEFT + after, top + 6.0, share(row.taken), 14.0);
        for (count, ink, name, bar) in
            [(row.added, Ink::Added, "bar added", added), (row.taken, Ink::Taken, "bar taken", taken)]
        {
            if count > 0 {
                d.pencil.ink(ink);
                d.pencil.frame(bar, false);
                d.pencil.hatch(bar, d.u(5.0));
                d.part(name, bar);
            }
        }
        d.pencil.ink(Ink::Pencil);
        let counts = d.point(LEFT + full + GAP + 12.0, top);
        d.caption(&format!("+{} −{}", row.added, row.taken), counts, 0.0);
    }
    let under = 6.0 + shown.len() as f32 * ROW;
    let left_out = more.saturating_add(rows.len() - shown.len());
    if left_out > 0 {
        let at = d.point(8.0, under + 4.0);
        d.caption(&format!("+{left_out} more"), at, 0.0);
    }
    if most > 0 {
        ruler(d, under + 8.0, full, most);
    }
    format!("bars rows={} more={more} most={most}", rows.len())
}

/// A line as long as the longest row, `top` design units down, with a tick and a count of lines at its start,
/// its middle and its end.
fn ruler(d: &mut Draft, top: f32, full: f32, most: usize) {
    let (from, to) = (d.point(LEFT, top), d.point(LEFT + full, top));
    d.pencil.line(from, to);
    d.part("ruler", Rect::new(from.0, from.1, to.0 - from.0, d.u(6.0)));
    let half = if most.is_multiple_of(2) { format!("{}", most / 2) } else { format!("{}.5", most / 2) };
    let end = if most == 1 { "1 line".to_string() } else { format!("{most} lines") };
    for (along, label) in [(0.0, "0".to_string()), (full / 2.0, half), (full, end)] {
        let (high, low) = (d.point(LEFT + along, top), d.point(LEFT + along, top + 6.0));
        d.pencil.line(high, low);
        d.part("tick", Rect::new(high.0 - d.u(1.0), high.1, d.u(2.0), d.u(6.0)));
        d.caption(&label, (high.0, low.1 + d.u(2.0)), 0.5);
    }
}
