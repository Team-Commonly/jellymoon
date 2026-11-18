//! A told note (`Note::Told`): someone else's words under the note's own labels. Each row is drawn to its last
//! character or left out, and a last line, `+N more`, counts the rows left out: a row is never cut. Each text of
//! the other's is a `Quote` piece and nothing else; the sheet's own runs are its title, its labels, the count and
//! its own sentences, which are drawn whole after the rows, or the note is left out.
//!
//! It is planned after the note that follows it in the scene ([`planned`]), with the rows that fit the room that
//! note leaves, or at its least: its title, the count and its own sentences. So that note is placed, and drawn,
//! as it is without it, the notes after them give way, and the told note is drawn in its own place, over it.
//! Where any other note is not as the layout without told notes has it, that layout is the one kept
//! ([`guarded`]).

use super::notes::{Plan, Sheet, TIGHTER, close, counted, heading, more_sheet, open, plan};
use super::{Ctx, PieceKind, Placed, TextRun, line_height, piece, run};
use crate::geom::Rect;
use crate::scene::{Note, Scene, ToldRow};
use crate::text::{Role, clean, is_blank, is_unseen, wrap};

/// How many rows of a told note are laid out at most. The ones after are only counted.
const MAX_TOLD_ROWS: usize = 12;

/// The role a row's label is written in: the note's own words.
const LABEL: Role = Role::RowMain;

/// Lines of a text drawn whole: none when the cleaning every text is drawn with would cut it.
type Lines = Vec<(String, f32)>;

/// The role of the note's own small word beside a quote.
const MARK: Role = Role::RowSub;

/// A row as it is drawn: its label's lines, each quote's lines with the word beside it, where a quote's ground
/// starts from the sheet's left, the padding down a ground and the gap under it, and the row's height.
struct Row {
    label: Lines,
    quotes: Vec<(Option<(String, f32)>, Lines)>,
    column: f32,
    close: (f32, f32),
    height: f32,
}

fn whole(cx: &Ctx, text: &str, width: f32, role: Role) -> Option<Lines> {
    let kept = cx.clean(text);
    let uncut = kept == clean(text, usize::MAX);
    uncut.then(|| wrap(&kept, width, usize::MAX, role, cx.size(role), cx.measure))
}

/// A row's label lines, its quotes and the words beside them: the name on the label's line when it holds no blank,
/// unseen or control character and the two lay out as one line; else the label alone and the name the first quote.
fn named(cx: &Ctx, inner: f32, row: &ToldRow) -> Option<(Lines, Vec<String>, Vec<String>)> {
    let plain = |c: char| !(is_blank(c) || is_unseen(c) || c.is_control());
    if !row.name.is_empty() && row.name.chars().all(plain) {
        let joined = whole(cx, &format!("{} {}", row.label, row.name), inner, LABEL);
        if let Some(label) = joined.filter(|lines| lines.len() == 1) {
            return Some((label, row.quotes.clone(), row.marks.clone()));
        }
    }
    let label = whole(cx, &row.label, inner, LABEL)?;
    if row.name.is_empty() {
        return Some((label, row.quotes.clone(), row.marks.clone()));
    }
    let marks = if row.marks.is_empty() { Vec::new() } else { [vec![String::new()], row.marks.clone()].concat() };
    Some((label, [vec![row.name.clone()], row.quotes.clone()].concat(), marks))
}

/// A row wrapped to the sheet, or None when a text of it would be cut. A quote that cleans to nothing is left out.
/// A row with a word beside a quote sets its quotes close together, each ground after the widest word.
fn shape(cx: &Ctx, inner: f32, row: &ToldRow) -> Option<Row> {
    let (label, texts, marks) = named(cx, inner, row)?;
    let marks: Vec<(String, f32)> = marks.iter().map(|mark| cx.line(mark, MARK, inner)).collect();
    let widest = marks.iter().map(|(_, width)| *width).fold(0.0, f32::max);
    let column = if widest > 0.0 { widest + cx.px(8.0) } else { 0.0 };
    let close = if widest > 0.0 { (cx.px(3.0), cx.px(2.0)) } else { (cx.px(7.0), cx.px(6.0)) };
    let quoted = |text: &String| whole(cx, text, inner - column - 2.0 * cx.px(10.0), Role::Quoted);
    let lines: Vec<Lines> = texts.iter().map(quoted).collect::<Option<_>>()?;
    let mark = |at: usize| marks.get(at).filter(|(word, _)| !word.is_empty()).cloned();
    let mut quotes: Vec<_> = lines.into_iter().enumerate().map(|(at, lines)| (mark(at), lines)).collect();
    quotes.retain(|(_, lines)| !lines.is_empty());
    let (said, quote) = (line_height(cx.size(LABEL)), line_height(cx.size(Role::Quoted)));
    let blocks: f32 = quotes.iter().map(|(_, lines)| lines.len() as f32 * quote + 2.0 * close.0 + close.1).sum();
    // A row with no label starts with its first quote.
    let labelled = if label.is_empty() { 0.0 } else { label.len() as f32 * said + cx.px(4.0) };
    let height = labelled + blocks + cx.px(6.0);
    Some(Row { label, quotes, column, close, height })
}

/// Draws a row at the sheet's cursor: its label, then each quote on a ground of its own, numbered on from `next`,
/// with the word beside it level with its first line.
fn draw(cx: &Ctx, sheet: &mut Sheet, id: &str, row: &Row, next: &mut usize) {
    let (size, quoted, small) = (cx.size(LABEL), cx.size(Role::Quoted), cx.size(MARK));
    let (pad_x, (pad_y, gap)) = (cx.px(10.0), row.close);
    for (line, width) in &row.label {
        let height = line_height(size);
        sheet.runs.push(run(line.clone(), LABEL, sheet.left, sheet.cursor + height / 2.0, *width, size));
        sheet.cursor += height;
    }
    if !row.label.is_empty() {
        sheet.cursor += cx.px(4.0);
    }
    let height = line_height(quoted);
    for (mark, lines) in &row.quotes {
        let (left, wide) = (sheet.left + row.column, sheet.inner - row.column);
        let rect = Rect::new(left, sheet.cursor, wide, lines.len() as f32 * height + 2.0 * pad_y);
        if let Some((word, width)) = mark {
            sheet.runs.push(run(word.clone(), MARK, sheet.left, rect.y + pad_y + height / 2.0, *width, small));
        }
        let mut block = piece(PieceKind::Quote, id, *next, rect);
        for (i, (line, width)) in lines.iter().enumerate() {
            let mid = rect.y + pad_y + (i as f32 + 0.5) * height;
            block.text.push(run(line.clone(), Role::Quoted, rect.x + pad_x, mid, *width, quoted));
        }
        sheet.options.push(block);
        sheet.cursor = rect.bottom() + gap;
        *next += 1;
    }
    sheet.cursor += cx.px(6.0);
}

/// The sheet of a told note at `top`, never taller than `room`: its rows in their order, each whole, up to the
/// first that does not fit, and none after it; at its `least`, none of them. None when the title, the note's own
/// sentences and the count do not fit.
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
    for row in shaped.iter().filter(|_| !least) {
        let Some(row) = row.as_ref().filter(|row| sheet.cursor + row.height + count <= bottom) else {
            break;
        };
        draw(cx, &mut sheet, id, row, &mut next);
        shown += 1;
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

/// The count of the `left_out` notes of `total` under the last note, whose bottom is `top`, and what it takes from
/// there: a level sheet after the `gap`, or, with a told note among the notes, one plain line in the same words,
/// close under them, so the told note keeps the room a sheet would take.
pub(super) fn count(
    cx: &Ctx,
    seen: &[(usize, &Note)],
    (left_out, total): (usize, usize),
    (top, gap): (f32, f32),
) -> (Sheet, PieceKind, f32) {
    if !seen.iter().any(|(_, note)| matches!(note, Note::Told { .. })) {
        let sheet = more_sheet(cx, left_out, total, top + gap);
        let takes = gap + sheet.rect.h;
        return (sheet, PieceKind::Note, takes);
    }
    let mut sheet = open(cx, top + cx.px(12.0), cx.px(14.0), 330.0, (0.0, 18.0));
    let (role, size) = (Role::RowSub, cx.size(Role::RowSub));
    let (line, width) = cx.line(&counted(left_out, total), role, sheet.inner);
    sheet.runs.push(run(line, role, sheet.left, sheet.cursor + line_height(size) / 2.0, width, size));
    sheet.rect.h = line_height(size);
    let takes = sheet.rect.bottom() - top;
    (sheet, PieceKind::More, takes)
}

type At<'a> = (usize, &'a Note);

fn is_told(at: &At) -> bool {
    matches!(at.1, Note::Told { .. })
}

/// Whether room is kept for the count when the notes from `from` on are left out: when one of them, or one past
/// those planned, is not a told note. A told note left out is counted only where the count fits, so it never
/// takes room from the note it is over.
pub(super) fn reserved(seen: &[At], from: usize, total: usize) -> bool {
    seen.len() < total || seen[from.min(seen.len())..].iter().any(|at| !is_told(at))
}

/// The plan made in the first way, or a tighter one where it places more notes or quotes more of a told note.
pub(super) fn chosen(cx: &Ctx, seen: &[At], total: usize, room: f32) -> Plan {
    let mut best = plan(cx, seen, total, TIGHTER[0], room);
    for way in &TIGHTER[1..] {
        let tighter = plan(cx, seen, total, *way, room);
        // A plan that shows more of what a told note quotes beats one that places more notes.
        let more_told = tighter.told > best.told;
        if more_told || tighter.told == best.told && best.placed < total && tighter.placed > best.placed {
            best = tighter;
        }
    }
    best
}

/// Lays the notes out with `draw` and, with a told note among them, again without the told notes. Where a note
/// other than a told note or a whole list is drawn in both and differs but for its place down the page, or the
/// note a drawn told note is over, or any note with an option or a field, is drawn only without them, the layout
/// without the told notes is the one kept.
/// So a result costs no other note anything, whatever the layout does at an edge, for a second layout when one
/// is on the page.
pub(super) fn guarded<'a>(cx: &mut Ctx<'a>, scene: &Scene, draw: impl Fn(&mut Ctx<'a>, &Scene, bool)) {
    let start = cx.out.len();
    draw(cx, scene, true);
    if !scene.notes.iter().any(|note| matches!(note, Note::Told { .. })) {
        return;
    }
    let with = cx.out.split_off(start);
    draw(cx, scene, false);
    if kept(scene, &with, &cx.out[start..]) {
        cx.out.truncate(start);
        cx.out.extend(with);
    }
}

/// The id a note's pieces carry.
fn id_of(index: usize, note: &Note) -> String {
    match note {
        Note::Choice { id, .. } => id.clone(),
        Note::Text { id, .. } | Note::Told { id, .. } if !id.is_empty() => id.clone(),
        _ => format!("note-{index}"),
    }
}

/// Whether the layout `with` the told notes costs no note what it has `without` them ([`guarded`]).
fn kept(scene: &Scene, with: &[Placed], without: &[Placed]) -> bool {
    let of = |all: &[Placed], id: &str| all.iter().filter(|piece| piece.id == id).cloned().collect::<Vec<_>>();
    // Whether the next note is one a drawn told note is over.
    let mut over = false;
    for (index, note) in scene.notes.iter().enumerate() {
        let id = id_of(index, note);
        match note {
            Note::Told { .. } => over |= with.iter().any(|piece| piece.id == id),
            Note::Whole { .. } => {}
            _ => {
                let (was, now) = (of(without, &id), of(with, &id));
                let lost = now.is_empty() && !was.is_empty() && (over || chooses(note));
                if lost || !now.is_empty() && !moved(&was, &now) {
                    return false;
                }
                over = false;
            }
        }
    }
    true
}

/// Whether a note has something to tap or type in: an option or a field.
fn chooses(note: &Note) -> bool {
    match note {
        Note::Choice { options, .. } => !options.is_empty(),
        Note::Text { options, field, .. } => !options.is_empty() || field.is_some(),
        _ => false,
    }
}

/// Whether `now` is `was` moved down or up the page and otherwise the same, to a hundredth of a pixel.
fn moved(was: &[Placed], now: &[Placed]) -> bool {
    let dy = match (was.first(), now.first()) {
        (Some(was), Some(now)) => now.rect.y - was.rect.y,
        _ => return was.len() == now.len(),
    };
    let near = |a: f32, b: f32| (a - b).abs() < 0.01;
    let rect = |a: &Rect, b: &Rect| near(a.x, b.x) && near(a.y + dy, b.y) && near(a.w, b.w) && near(a.h, b.h);
    let runs = |a: &Placed, b: &Placed| {
        let same = |r: &TextRun, s: &TextRun| (&r.text, r.role, r.size) == (&s.text, s.role, s.size);
        a.text.len() == b.text.len() && a.text.iter().zip(&b.text).all(|(r, s)| same(r, s) && rect(&r.rect, &s.rect))
    };
    was.len() == now.len()
        && was.iter().zip(now).all(|(a, b)| {
            (a.piece, a.index, a.z, a.tilt_degrees) == (b.piece, b.index, b.z, b.tilt_degrees)
                && rect(&a.rect, &b.rect)
                && runs(a, b)
        })
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
