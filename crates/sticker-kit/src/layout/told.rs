//! A told note (`Note::Told`): someone else's words under the note's own labels. Each row is drawn to its last
//! character or left out, and a last line, `+N more`, counts the rows left out: a row is never cut. Each text of
//! the other's is a `Quote` piece and nothing else; the sheet's own runs are its title, its labels, the count and
//! its own sentences, which are drawn whole after the rows, or the note is left out.
//!
//! It is planned after the note that follows it in the scene ([`planned`]), with the rows that fit the room that
//! note leaves, or at its least: its title, the count and its own sentences. So that note is placed, and drawn,
//! as it is without it, the notes after them give way, and the told note is drawn in its own place, over it.

use super::notes::{Sheet, close, heading, open};
use super::{Ctx, PieceKind, line_height, piece, run};
use crate::geom::Rect;
use crate::scene::{Note, ToldRow};
use crate::text::{Role, clean, wrap};

/// How many rows of a told note are laid out at most. The ones after are only counted.
const MAX_TOLD_ROWS: usize = 12;

/// The role a row's label is written in: the note's own words.
const LABEL: Role = Role::RowMain;

/// Lines of a text drawn whole: none when the cleaning every text is drawn with would cut it.
type Lines = Vec<(String, f32)>;

/// A row as it is drawn: its label's lines, each quote's lines, and the height it takes with the gap under it.
struct Row {
    label: Lines,
    quotes: Vec<Lines>,
    height: f32,
}

fn whole(cx: &Ctx, text: &str, width: f32, role: Role) -> Option<Lines> {
    let kept = cx.clean(text);
    let uncut = kept == clean(text, usize::MAX);
    uncut.then(|| wrap(&kept, width, usize::MAX, role, cx.size(role), cx.measure))
}

/// The padding of a quote's ground: across and down.
fn pad(cx: &Ctx) -> (f32, f32) {
    (cx.px(10.0), cx.px(7.0))
}

/// A row wrapped to the sheet, or None when a text of it would be cut. A quote that cleans to nothing is left out.
fn shape(cx: &Ctx, inner: f32, row: &ToldRow) -> Option<Row> {
    let label = whole(cx, &row.label, inner, LABEL)?;
    let (pad_x, pad_y) = pad(cx);
    let quoted = |text: &String| whole(cx, text, inner - 2.0 * pad_x, Role::Quoted);
    let mut quotes: Vec<Lines> = row.quotes.iter().map(quoted).collect::<Option<_>>()?;
    quotes.retain(|lines| !lines.is_empty());
    let (said, quote) = (line_height(cx.size(LABEL)), line_height(cx.size(Role::Quoted)));
    let blocks: f32 = quotes.iter().map(|lines| lines.len() as f32 * quote + 2.0 * pad_y + cx.px(6.0)).sum();
    // A row with no label starts with its first quote.
    let labelled = if label.is_empty() { 0.0 } else { label.len() as f32 * said + cx.px(4.0) };
    let height = labelled + blocks + cx.px(6.0);
    Some(Row { label, quotes, height })
}

/// Draws a row at the sheet's cursor: its label, then each quote on a ground of its own, numbered on from `next`.
fn draw(cx: &Ctx, sheet: &mut Sheet, id: &str, row: &Row, next: &mut usize) {
    let (size, quoted) = (cx.size(LABEL), cx.size(Role::Quoted));
    let (pad_x, pad_y) = pad(cx);
    for (line, width) in &row.label {
        let height = line_height(size);
        sheet.runs.push(run(line.clone(), LABEL, sheet.left, sheet.cursor + height / 2.0, *width, size));
        sheet.cursor += height;
    }
    if !row.label.is_empty() {
        sheet.cursor += cx.px(4.0);
    }
    let height = line_height(quoted);
    for lines in &row.quotes {
        let rect = Rect::new(sheet.left, sheet.cursor, sheet.inner, lines.len() as f32 * height + 2.0 * pad_y);
        let mut block = piece(PieceKind::Quote, id, *next, rect);
        for (i, (line, width)) in lines.iter().enumerate() {
            let mid = rect.y + pad_y + (i as f32 + 0.5) * height;
            block.text.push(run(line.clone(), Role::Quoted, rect.x + pad_x, mid, *width, quoted));
        }
        sheet.options.push(block);
        sheet.cursor = rect.bottom() + cx.px(6.0);
        *next += 1;
    }
    sheet.cursor += cx.px(6.0);
}

/// The sheet of a told note at `top`, never taller than `room`: its rows in their order, each whole, up to the
/// first that does not fit, and none after it; at its `least`, none of them. A row that goes `with_last` is drawn
/// with the row before it or not at all. None when the title, the note's own sentences and the count do not fit.
pub(super) fn sheet(
    cx: &Ctx,
    (id, title, rows, end): (&str, &str, &[ToldRow], &[String]),
    (top, room): (f32, f32),
    least: bool,
) -> Option<Sheet> {
    let mut sheet = open(cx, top, 0.0, cx.theme.metrics.note_width, (26.0, 24.0));
    heading(cx, &mut sheet, title, Role::NoteTitle);
    let (body, gap) = (cx.size(Role::Body), cx.px(9.0));
    let ends: Vec<Lines> = end.iter().map(|text| whole(cx, text, sheet.inner, Role::Body)).collect::<Option<_>>()?;
    let own: f32 = ends.iter().map(|lines| lines.len() as f32 * line_height(body) + gap).sum();
    let bottom = top + room - cx.px(20.0 - 9.0) - own;
    let shaped: Vec<Option<Row>> = rows.iter().take(MAX_TOLD_ROWS).map(|row| shape(cx, sheet.inner, row)).collect();
    let all: f32 = shaped.iter().map(|row| row.as_ref().map_or(f32::INFINITY, |row| row.height)).sum();
    let line = line_height(cx.size(Role::RowSub)) + gap;
    let every = !least && shaped.len() == rows.len() && sheet.cursor + all <= bottom;
    let count = if every || rows.is_empty() { 0.0 } else { line };
    let (mut shown, mut next) = (0, 0);
    for unit in units(rows, shaped.len()).into_iter().filter(|_| !least) {
        let unit: Option<Vec<&Row>> = shaped[unit].iter().map(Option::as_ref).collect();
        let Some(unit) =
            unit.filter(|unit| sheet.cursor + unit.iter().map(|row| row.height).sum::<f32>() + count <= bottom)
        else {
            break;
        };
        for row in &unit {
            draw(cx, &mut sheet, id, row, &mut next);
        }
        shown += unit.len();
    }
    if shown < rows.len() {
        let (more, width) = cx.line(&format!("+{} more", rows.len() - shown), Role::RowSub, sheet.inner);
        let mid = sheet.cursor + line_height(cx.size(Role::RowSub)) / 2.0;
        sheet.runs.push(run(more, Role::RowSub, sheet.left, mid, width, cx.size(Role::RowSub)));
        sheet.cursor += line;
    }
    for lines in ends {
        for (line, width) in lines {
            sheet.runs.push(run(line, Role::Body, sheet.left, sheet.cursor + line_height(body) / 2.0, width, body));
            sheet.cursor += line_height(body);
        }
        sheet.cursor += gap;
    }
    close(cx, sheet, 20.0, room)
}

/// The first `seen` rows as the units they are drawn in: a row and the rows after it that go `with_last`.
fn units(rows: &[ToldRow], seen: usize) -> Vec<std::ops::Range<usize>> {
    let mut units: Vec<std::ops::Range<usize>> = Vec::new();
    for (at, row) in rows.iter().enumerate().take(seen) {
        match units.last_mut() {
            Some(unit) if row.with_last => unit.end = at + 1,
            _ => units.push(at..at + 1),
        }
    }
    units
}

/// The notes in the order they are planned in: a told note just after the note that follows it in the scene, so
/// that note is placed whatever room the told note would take. Each keeps its place in the scene.
pub(super) fn planned(seen: Vec<(usize, &Note)>) -> Vec<(usize, &Note)> {
    let (mut order, mut held) = (Vec::with_capacity(seen.len()), Vec::new());
    for at in seen {
        if matches!(at.1, Note::Told { .. }) {
            held.push(at);
            continue;
        }
        order.push(at);
        order.append(&mut held);
    }
    order.append(&mut held);
    order
}

/// How many texts a told note quotes in `room`: none for any other note.
pub(super) fn quoted(cx: &Ctx, note: &Note, room: f32) -> usize {
    let Note::Told { id, title, rows, end } = note else { return 0 };
    let parts = (id.as_str(), title.as_str(), rows.as_slice(), end.as_slice());
    sheet(cx, parts, (0.0, room), false).map_or(0, |sheet| sheet.options.len())
}
