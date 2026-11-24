//! A run at work, left to right: its steps so far as tally marks; what it is doing now, a sheet with a magnifier
//! or a pencil over it, or three dots while it thinks, and the stage in two words under it with the file's name;
//! then a dashed line to an empty circle, the place its result will stand in.

use super::Draft;
use crate::geom::Rect;
use crate::scene::Stage;

/// The most tally marks drawn. Steps past them are one small plus after the last.
const MAX_TALLIES: usize = 25;

/// Draws the picture and returns what it is, for the print.
pub(super) fn draw(d: &mut Draft, steps: usize, stage: Stage, file: &str) -> String {
    let (wide, mid) = (d.wide(), d.wide() * 0.44);
    let (from, high) = (d.point(8.0, 14.0), d.u(28.0));
    let marks = d.pencil.tally(from, steps.min(MAX_TALLIES), high);
    for (n, mark) in marks.into_iter().enumerate() {
        d.part(if n % 5 < 4 { "tally" } else { "across" }, mark);
    }
    if steps > MAX_TALLIES {
        let more = d.rect(194.0, 23.0, 10.0, 10.0);
        d.pencil.plus(more);
        d.part("more", more);
    }
    let (name, caption) = match stage {
        Stage::Thinking => ("thinking", "thinking"),
        Stage::Looking => ("looking", "looking around"),
        Stage::Making => ("making", "making the change"),
    };
    match stage {
        Stage::Thinking => dots(d, mid),
        Stage::Looking => magnifier(d, mid),
        Stage::Making => pencil(d, mid),
    }
    let (centre, top, room) = (d.at.x + d.u(mid), d.at.y + d.u(66.0), d.u(wide * 0.4));
    d.caption(caption, (centre, top), 0.5);
    let named = d.name(file, room, (centre, top + d.line()), 0.5);

    let (from, to) = (d.point(mid + 64.0, 32.0), d.point(wide - 76.0, 32.0));
    d.pencil.dashes(from, to);
    d.part("dashed", Rect::new(from.0, from.1 - d.u(1.0), to.0 - from.0, d.u(2.0)));
    let place = d.rect(wide - 68.0, 14.0, 36.0, 36.0);
    d.pencil.circle(place.center(), place.w / 2.0);
    d.part("place", place);
    format!("track steps={steps} stage={name} file={}", if named { file } else { "" })
}

fn dots(d: &mut Draft, mid: f32) {
    for along in [-14.0, 0.0, 14.0] {
        let dot = d.rect(mid + along - 2.5, 27.5, 5.0, 5.0);
        d.pencil.circle(dot.center(), dot.w / 2.0);
        d.part("dot", dot);
    }
}

/// A small sheet of paper with three lines of writing nobody reads.
fn sheet(d: &mut Draft, mid: f32) {
    let sheet = d.rect(mid - 20.0, 6.0, 40.0, 52.0);
    d.pencil.frame(sheet, false);
    for line in [20.0, 31.0, 42.0] {
        let (from, to) = (d.point(mid - 13.0, line), d.point(mid + 13.0, line));
        d.pencil.squiggle(from, to);
    }
    d.part("sheet", sheet);
}

fn magnifier(d: &mut Draft, mid: f32) {
    sheet(d, mid);
    let (centre, radius) = (d.point(mid + 12.0, 28.0), d.u(12.0));
    let (from, to) = (d.point(mid + 21.0, 37.0), d.point(mid + 32.0, 50.0));
    d.pencil.rocking(true);
    d.pencil.circle(centre, radius);
    d.pencil.line(from, to);
    d.pencil.rocking(false);
    d.part("magnifier", d.rect(mid, 16.0, 32.0, 34.0));
}

/// A pencil with its point on the sheet: two sides, the end, and two strokes to the point.
fn pencil(d: &mut Draft, mid: f32) {
    sheet(d, mid);
    let (tip, (along, down), (aside, below)) = ((mid - 2.0, 44.0), (0.664, -0.747), (2.39, 2.12));
    let (neck, end) = ((tip.0 + along * 9.0, tip.1 + down * 9.0), (tip.0 + along * 46.0, tip.1 + down * 46.0));
    let side = |of: (f32, f32), sign: f32| (of.0 + sign * aside, of.1 + sign * below);
    let strokes = [
        (side(neck, 1.0), side(end, 1.0)),
        (side(neck, -1.0), side(end, -1.0)),
        (side(end, 1.0), side(end, -1.0)),
        (side(neck, 1.0), tip),
        (side(neck, -1.0), tip),
    ];
    d.pencil.rocking(true);
    for (from, to) in strokes {
        let (from, to) = (d.point(from.0, from.1), d.point(to.0, to.1));
        d.pencil.line(from, to);
    }
    d.pencil.rocking(false);
    d.part("pencil", d.rect(mid - 4.0, 6.0, 38.0, 40.0));
}
