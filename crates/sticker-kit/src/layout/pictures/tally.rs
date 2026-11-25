//! A tally (`Picture::Tally`): a title with a chevron at the line's end, then a row for each of the first
//! [`MAX_TALLY_ROWS`] rows, a label in the hand and its marks in the order given, then one quiet line for the
//! rows past those and one quiet line at the foot. A row is one line: after [`MAX_TALLY_MARKS`] marks the rest
//! are a count, `+N`. The marks are the pencil's: a star, a ring with three dots, a ring with a `?`, and a cross
//! in the quiet ink, each what `Mark` means on a whole list's row. The picture knows what it is given and
//! nothing of what the marks count.

use std::f32::consts::PI;

use super::super::{line_height, run};
use super::Draft;
use crate::geom::Rect;
use crate::pencil::{Ink, Point};
use crate::scene::{MAX_TALLY_MARKS, MAX_TALLY_ROWS, Mark, TallyRow};
use crate::text::{Role, fit_middle};

/// In design units: the title's line, as high as the kit's smallest tap target; a row; the room of a label; the
/// room of one mark; and a mark's reach from its middle.
const HEAD: f32 = 24.0;
const ROW: f32 = 28.0;
const LABEL: f32 = 190.0;
const PITCH: f32 = 24.0;
const REACH: f32 = 7.0;
/// The least contrast ([`crate::Color::contrast`]) a mark's colour is drawn at: what WCAG 2 asks of a line.
const FLOOR: f32 = 3.0;

pub(super) fn draw(
    draft: &mut Draft,
    title: &str,
    rows: &[TallyRow],
    (more, under): (&str, &str),
    open: bool,
) -> String {
    let (left, top, head) = (draft.at.x, draft.at.y, draft.u(HEAD));
    let (wide, right, u) = (draft.at.w, draft.at.right(), draft.u(1.0));
    say(draft, title, Role::Caption, (left, top + head / 2.0), wide - 20.0 * u);
    chevron(draft, (right - 12.0 * u, top + head / 2.0), open);
    let mut y = top + head + draft.u(4.0);
    for row in rows.iter().take(MAX_TALLY_ROWS) {
        let mid = y + draft.u(ROW) / 2.0;
        let name = draft.cx.clean(&row.label);
        let size = draft.cx.size(Role::Caption);
        let (name, width) = fit_middle(&name, draft.u(LABEL), Role::Caption, size, draft.cx.measure);
        if !name.is_empty() {
            draft.runs.push(run(name, Role::Caption, left, mid, width, size));
        }
        let from = left + draft.u(LABEL + 14.0);
        for (place, mark) in row.marks.iter().take(MAX_TALLY_MARKS).enumerate() {
            sign(draft, *mark, (from + PITCH * u * (place as f32 + 0.5), mid));
        }
        if let Some(over) = row.marks.len().checked_sub(MAX_TALLY_MARKS).filter(|over| *over > 0) {
            let at = from + draft.u(PITCH) * MAX_TALLY_MARKS as f32 + draft.u(4.0);
            say(draft, &format!("+{over}"), Role::Caption, (at, mid), right - at);
        }
        draft.part("row", Rect::new(left, y, wide, ROW * u));
        y += ROW * u;
    }
    let past = rows.len().saturating_sub(MAX_TALLY_ROWS);
    let counted = if more.is_empty() { format!("+{past} more") } else { more.to_string() };
    let rows_end = y;
    for line in [(past > 0).then_some(counted.as_str()), Some(under)].into_iter().flatten() {
        let high = line_height(draft.cx.size(Role::Footer));
        if say(draft, line, Role::Footer, (left, y + 3.0 * u + high / 2.0), wide) {
            y += high + 3.0 * u;
        }
    }
    if y == rows_end && !rows.is_empty() {
        // With no line under the rows the picture ends where the last row's words end, as a picture that ends in
        // a caption does, and not at the foot of the row's room.
        draft.low = y - (ROW * u - line_height(draft.cx.size(Role::Caption))).max(0.0) / 2.0;
    }
    format!("tally rows={} {}", rows.len(), if open { "open" } else { "closed" })
}

/// One line of `text` from `x`, level with `mid` and cut to `room`. Whether anything was written.
fn say(draft: &mut Draft, text: &str, role: Role, (x, mid): Point, room: f32) -> bool {
    let (text, width) = draft.cx.line(text, role, room);
    if text.is_empty() {
        return false;
    }
    let size = draft.cx.size(role);
    draft.low = draft.low.max(mid + line_height(size) / 2.0);
    draft.runs.push(run(text, role, x, mid, width, size));
    true
}

/// The chevron at the end of the title's line: it points on while the tally is closed, and down while it is open.
fn chevron(draft: &mut Draft, (x, mid): Point, open: bool) {
    let u = draft.u(1.0);
    if open {
        draft.pencil.path(&[(x - 2.0 * u, mid - 2.0 * u), (x + 2.0 * u, mid + 2.0 * u), (x + 6.0 * u, mid - 2.0 * u)]);
    } else {
        draft.pencil.path(&[(x, mid - 4.0 * u), (x + 4.0 * u, mid), (x, mid + 4.0 * u)]);
    }
    draft.part(if open { "closes" } else { "opens" }, Rect::new(x - 2.0 * u, mid - 4.0 * u, 8.0 * u, 8.0 * u));
}

/// One mark, its middle at `centre`.
fn sign(draft: &mut Draft, mark: Mark, centre: Point) {
    let (u, reach) = (draft.u(1.0), draft.u(REACH));
    let name = match mark {
        Mark::Star => star(draft, centre, reach),
        Mark::Busy => {
            draft.pencil.circle(centre, reach);
            for dot in [-3.0, 0.0, 3.0] {
                draft.pencil.line((centre.0 + (dot - 0.6) * u, centre.1), (centre.0 + (dot + 0.6) * u, centre.1));
            }
            "busy"
        }
        Mark::NeedsYou => asks(draft, centre, reach),
        Mark::Dimmed => {
            draft.pencil.ink(Ink::Quiet);
            draft.pencil.cross(Rect::new(centre.0 - reach * 0.7, centre.1 - reach * 0.7, reach * 1.4, reach * 1.4));
            draft.pencil.ink(Ink::Pencil);
            "gone"
        }
    };
    draft.part(name, Rect::new(centre.0 - reach, centre.1 - reach, 2.0 * reach, 2.0 * reach));
}

/// The star of the kit's art, ten points with every other one half as far out: filled with the theme's star
/// colour, as a star of the page is, and outlined by the pencil, which is what tells it from the note's ground
/// whatever the star colour is.
fn star(draft: &mut Draft, centre: Point, reach: f32) -> &'static str {
    let at = |n: usize, far: f32| {
        let turn = -PI / 2.0 + n as f32 * PI / 5.0;
        (centre.0 + far * turn.cos(), centre.1 + far * turn.sin())
    };
    draft.pencil.ink(Ink::Star);
    // The middle, out to the star's inner corners, then an arm to each tip.
    draft.pencil.fill(&(0..=5).map(|n| at(2 * n, reach * 0.25)).collect::<Vec<Point>>(), reach * 0.5);
    (0..5).for_each(|n| draft.pencil.fill(&[centre, at(2 * n, reach * 0.72)], reach * 0.3));
    draft.pencil.ink(Ink::Pencil);
    let point = |n: usize| at(n, if n.is_multiple_of(2) { reach } else { reach / 2.0 });
    draft.pencil.path(&(0..=10).map(point).collect::<Vec<Point>>());
    "star"
}

/// The mark of a thing that asks for the person: a `?` in a ring. The ring is filled with the theme's accent, the
/// colour of the badge a little one that asks has, where the pencil's `?` on it meets [`FLOOR`]. Where it does
/// not, the ring alone is in the accent if that meets the floor on the note's ground, and else all is pencil.
fn asks(draft: &mut Draft, centre: Point, reach: f32) -> &'static str {
    let palette = &draft.cx.theme.palette;
    let filled = palette.pencil.contrast(palette.accent) >= FLOOR;
    let ringed = palette.accent.contrast(palette.note_list) >= FLOOR;
    if filled {
        // Six strokes across the middle: a wide stroke round a ring this small folds over itself.
        let end = |n: usize, far: f32| {
            let turn = (n as f32 * 30.0).to_radians();
            (centre.0 + far * turn.cos(), centre.1 + far * turn.sin())
        };
        draft.pencil.ink(Ink::Asks);
        (0..6).for_each(|n| draft.pencil.fill(&[end(n, -reach * 0.7), end(n, reach * 0.7)], reach * 0.45));
        draft.pencil.ink(Ink::Pencil);
    } else if ringed {
        draft.pencil.ink(Ink::Asks);
    }
    draft.pencil.circle(centre, reach);
    draft.pencil.ink(Ink::Pencil);
    let size = draft.cx.size(Role::Caption) * 0.8;
    let width = draft.cx.measure.width("?", Role::Caption, size);
    draft.runs.push(run("?".to_string(), Role::Caption, centre.0 - width / 2.0, centre.1, width, size));
    "asks"
}
