//! A strip (`Picture::Strip`): up to [`MAX_STRIP_PANELS`] panels in one row, left to right. Each panel has a
//! third of the picture's width: a small sketch in its middle and its caption under it, centred, in two lines at
//! most. An arrow runs from each sketch to the next, and is drawn before the sketch it points at, so the strip
//! draws itself in the order it is read. The sketches are other pictures' shapes: the sheet and pencil of a run
//! that is making (`track.rs`), with the pencil at rest, and the folder with its tick of work that is in
//! (`sheets.rs`). Nothing of a strip takes a tap, and it knows nothing of what its panels are about.
//!
//! Every length is the column's design unit, so a narrow window draws the same row smaller and never a second
//! row. A caption that does not fit its panel's width wraps, and what is left after its second line ends in the
//! kit's ellipsis.

use super::{Draft, sheets, track};
use crate::geom::Rect;
use crate::pencil::{Ink, Point};
use crate::scene::{MAX_STRIP_PANELS, Panel, Shows};
use crate::text::{Role, wrap};

/// In design units: how far a sketch reaches to each side of its panel's middle, with the gap an arrow leaves;
/// how far down the arrows run; where the captions begin; and what a caption keeps clear at each side of its panel.
const HALF: f32 = 50.0;
const ARROW: f32 = 32.0;
const CAPTIONS: f32 = 66.0;
const CLEAR: f32 = 8.0;
/// The most lines of one caption.
const LINES: usize = 2;

/// Draws the picture and returns what it is, for the print.
pub(super) fn draw(d: &mut Draft, panels: &[Panel]) -> String {
    let shown = &panels[..panels.len().min(MAX_STRIP_PANELS)];
    let room = d.wide() / MAX_STRIP_PANELS as f32;
    for (n, panel) in shown.iter().enumerate() {
        let mid = room * (n as f32 + 0.5);
        if n > 0 {
            let (from, to) = (d.point(mid - room + HALF, ARROW), d.point(mid - HALF, ARROW));
            d.pencil.arrow(from, to, false);
            d.part("arrow", Rect::new(from.0, from.1 - d.u(5.0), to.0 - from.0, d.u(10.0)));
        }
        match panel.shows {
            Shows::Speech => speech(d, mid),
            Shows::Sheet => track::pencil(d, mid, false),
            Shows::Folder => folder(d, mid),
        }
        caption(d, &panel.caption, mid, room - 2.0 * CLEAR);
    }
    format!("strip panels={}", shown.len())
}

/// A panel's caption under its sketch, centred on the panel's middle `mid` and wrapped to `room`.
fn caption(d: &mut Draft, text: &str, mid: f32, room: f32) {
    let (size, x) = (d.cx.size(Role::Caption), d.at.x + d.u(mid));
    let lines = wrap(&d.cx.clean(text), d.u(room), LINES, Role::Caption, size, d.cx.measure);
    for (n, line) in lines.into_iter().enumerate() {
        let top = d.at.y + d.u(CAPTIONS) + n as f32 * d.line();
        d.write(line, (x, top), 0.5);
    }
}

/// A speech bubble: one stroke from the tip of its tail, round the bubble and back to the tip, and in it two
/// lines of writing nobody reads.
fn speech(d: &mut Draft, mid: f32) {
    let round = |n: usize| {
        let turn = (130.0 + 340.0 / 24.0 * n as f32).to_radians();
        (mid + 32.0 * turn.cos(), 28.0 + 19.0 * turn.sin())
    };
    let tip = (mid - 25.0, 57.0);
    let outline = [tip].into_iter().chain((0..=24).map(round)).chain([tip]);
    let outline: Vec<Point> = outline.map(|(x, y)| d.point(x, y)).collect();
    d.pencil.path(&outline);
    for (down, end) in [(23.0, 18.0), (33.0, 6.0)] {
        let (from, to) = (d.point(mid - 18.0, down), d.point(mid + end, down));
        d.pencil.squiggle(from, to);
    }
    d.part("speech", d.rect(mid - 32.0, 9.0, 64.0, 49.0));
}

/// A folder with a tick in it, and beside it a small cross in the taken ink.
fn folder(d: &mut Draft, mid: f32) {
    sheets::folder_at(d, mid - 41.0, false);
    let cross = d.rect(mid + 25.0, 24.0, 16.0, 16.0);
    d.pencil.ink(Ink::Taken);
    d.pencil.cross(cross);
    d.pencil.ink(Ink::Pencil);
    d.part("cross", cross);
}
