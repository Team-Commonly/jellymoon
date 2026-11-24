//! Work a run finished, left to right: a small sheet for each file (one it made carries a plus and hatching in the
//! added ink, one it changed a line of writing and a hatched band, one it removed is crossed out in the taken
//! ink), their count in words under them, then an arrow to a folder. While the work waits the arrow and the
//! folder are dashed; kept, they are solid and a tick is in the folder; thrown away, there is no arrow and no
//! folder, and every sheet is crossed out.

use super::Draft;
use crate::geom::Rect;
use crate::pencil::Ink;
use crate::scene::Kept;

/// The most sheets drawn. The files past them are a stack with their count beside it.
const MAX_SHEETS: usize = 6;

const KINDS: [&str; 3] = ["sheet made", "sheet changed", "sheet removed"];

/// Draws the picture and returns what it is, for the print.
pub(super) fn draw(d: &mut Draft, [made, changed, removed]: [usize; 3], state: Kept) -> String {
    let files = made.saturating_add(changed).saturating_add(removed);
    let shown = files.min(MAX_SHEETS);
    for n in 0..shown.max(1) {
        let kind = if files == 0 { 3 } else { usize::from(n >= made) + usize::from(n >= made.saturating_add(changed)) };
        sheet(d, n, kind, state == Kept::Tossed);
    }
    let mut right = 8.0 + shown.max(1) as f32 * 44.0;
    if files > MAX_SHEETS {
        for step in [0.0, 5.0] {
            let rest = d.rect(right + step, 12.0 + step, 24.0, 32.0);
            d.pencil.frame(rest, false);
        }
        d.part("stack", d.rect(right, 12.0, 29.0, 37.0));
        let beside = d.point(right + 36.0, 18.0);
        d.caption(&format!("+{}", files - MAX_SHEETS), beside, 0.0);
        right += 76.0;
    }
    let count = match files {
        0 => "nothing changed".to_string(),
        1 => "1 file".to_string(),
        files => format!("{files} files"),
    };
    let under = d.at.y + d.u(58.0);
    d.caption(&count, (d.at.x + d.u(8.0), under), 0.0);
    let name = match state {
        Kept::Waiting => "waiting",
        Kept::In => "in",
        Kept::Tossed => "tossed",
    };
    if state == Kept::Tossed {
        let beside = d.point(right + 14.0, 18.0);
        d.caption("thrown away", beside, 0.0);
    } else if files > 0 {
        folder(d, right, state == Kept::Waiting, under);
    }
    format!("sheets made={made} changed={changed} removed={removed} state={name}")
}

/// The `n`th sheet: `kind` is a place in [`KINDS`], or past them for the one empty sheet of no file at all.
fn sheet(d: &mut Draft, n: usize, kind: usize, tossed: bool) {
    let left = 8.0 + n as f32 * 44.0;
    let sheet = d.rect(left, 8.0, 32.0, 42.0);
    d.pencil.frame(sheet, false);
    let writing = |d: &mut Draft, down: f32| {
        let (from, to) = (d.point(left + 6.0, down), d.point(left + 26.0, down));
        d.pencil.squiggle(from, to);
    };
    match kind {
        0 => {
            d.pencil.ink(Ink::Added);
            d.pencil.plus(d.rect(left + 10.0, 13.0, 12.0, 12.0));
            let (band, gap) = (d.rect(left + 4.0, 31.0, 24.0, 14.0), d.u(5.0));
            d.pencil.hatch(band, gap);
        }
        1 => {
            writing(d, 16.0);
            let (band, gap) = (d.rect(left + 4.0, 23.0, 24.0, 10.0), d.u(5.0));
            d.pencil.hatch(band, gap);
            writing(d, 41.0);
        }
        2 => [18.0, 29.0, 40.0].into_iter().for_each(|down| writing(d, down)),
        _ => {}
    }
    d.pencil.ink(Ink::Pencil);
    d.part(KINDS.get(kind).copied().unwrap_or("sheet empty"), sheet);
    if kind == 2 || tossed {
        let over = d.rect(left + 3.0, 12.0, 26.0, 34.0);
        d.pencil.ink(Ink::Taken);
        d.pencil.cross(over);
        d.pencil.ink(Ink::Pencil);
        d.part("cross", over);
    }
}

/// The arrow from the sheets, which end `right` design units in, to the folder, and the folder's caption level
/// with the count's at `under`: dashed while the work `waits`, solid with a tick in the folder once it is in.
fn folder(d: &mut Draft, right: f32, waits: bool, under: f32) {
    let left = d.wide() - 150.0;
    let (from, to) = (d.point(right + 12.0, 30.0), d.point(left - 12.0, 30.0));
    d.pencil.arrow(from, to, waits);
    let shaft = Rect::new(from.0, from.1 - d.u(5.0), to.0 - from.0, d.u(10.0));
    d.part(if waits { "arrow dashed" } else { "arrow solid" }, shaft);
    let body = d.rect(left, 14.0, 56.0, 36.0);
    d.pencil.frame(body, waits);
    // The folder's tab, over its top left corner.
    for (from, to) in [((0.0, 14.0), (7.0, 7.0)), ((7.0, 7.0), (24.0, 7.0)), ((24.0, 7.0), (30.0, 14.0))] {
        let (from, to) = (d.point(left + from.0, from.1), d.point(left + to.0, to.1));
        d.pencil.line(from, to);
    }
    d.part(if waits { "folder dashed" } else { "folder solid" }, d.rect(left, 7.0, 56.0, 43.0));
    if !waits {
        let tick = d.rect(left + 17.0, 23.0, 22.0, 18.0);
        d.pencil.ink(Ink::Added);
        d.pencil.tick(tick);
        d.pencil.ink(Ink::Pencil);
        d.part("tick", tick);
    }
    let caption = if waits { "not in your folder yet" } else { "in your folder" };
    d.caption(caption, (d.at.x + d.u(left + 28.0), under), 0.5);
}
