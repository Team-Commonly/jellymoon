//! Notes stack down the right-hand side, each under a strip of tape, in the scene's order. A list gives up rows to
//! the notes after it. When the notes do not all fit, the first that does not and every one after it are left out,
//! and a last small sheet says how many: no note goes without a word.

use super::{Ctx, PieceKind, Placed, TextRun, line_height, piece, run};
use crate::geom::Rect;
use crate::scene::{Mark, Note, Scene};
use crate::text::{Role, wrap};

const MAX_OPTIONS: usize = 4;

/// The ways to make room, tried in order until every note fits: how many lines an option is shown in, the gap under
/// a note in design units, and how large the options of a `whole` choice are written. A list gives up rows before any
/// of them is tried. The scene keeps the whole text. A choice that is `whole` gives up no line: its sheet is as wide
/// as the column, its options are written smaller when that places more notes, and it is drawn in full or left out.
/// A `Whole` list is written at that same size, gives up rows down to the one line that counts them, and takes a
/// tighter way when that draws more of its rows.
const TIGHTER: [Way; 5] = [(3, 48.0, 1.0), (2, 48.0, 1.0), (2, 32.0, 1.0), (2, 32.0, 0.85), (2, 32.0, 0.72)];

/// One way to make room: an option's lines, the gap under a note, the size of a whole choice's options.
type Way = (usize, f32, f32);

/// How many notes are laid out at most. The ones after are only counted, so a scene of any length costs the same.
pub const MAX_NOTES: usize = 12;

/// The id of the sheet that says how many notes were left out. Its index is the number of notes in the scene.
pub const MORE_NOTES_ID: &str = "more";

/// How many rows of a whole list are laid out at most. The ones after are only counted.
const MAX_WHOLE_ROWS: usize = 12;

/// A note being filled from the top down.
struct Sheet {
    rect: Rect,
    /// Left edge and width of the text column.
    left: f32,
    inner: f32,
    cursor: f32,
    runs: Vec<TextRun>,
    options: Vec<Placed>,
}

pub(crate) fn notes(cx: &mut Ctx, scene: &Scene) {
    let bottom = cx.paper().bottom() - cx.px(26.0);
    let top = cx.y(cx.theme.metrics.note_top);
    let total = scene.notes.len();
    let seen = &scene.notes[..total.min(MAX_NOTES)];
    let whole = seen.iter().any(|note| matches!(note, Note::Whole { .. }));
    // Room is made in this order: lists give up rows, options their third line, the gaps a third; then notes are
    // counted. A tighter way is taken only when it places more notes, or as many and more rows of a whole list.
    let mut best = lay(cx, seen, total, plan(cx, seen, total, TIGHTER[0], bottom - top), (top, bottom));
    for way in &TIGHTER[1..] {
        let plan = plan(cx, seen, total, *way, bottom - top);
        if plan.placed > best.placed || (whole && plan.placed == best.placed) {
            let tighter = lay(cx, seen, total, plan, (top, bottom));
            if (tighter.placed, tighter.rows) > (best.placed, best.rows) {
                best = tighter;
            }
        }
    }
    cx.out.extend(best.pieces);
}

/// The notes of one plan, laid out: how many it placed, and how many rows of whole lists it drew.
struct Laid {
    pieces: Vec<Placed>,
    placed: usize,
    rows: usize,
}

fn lay(cx: &Ctx, seen: &[Note], total: usize, plan: Plan, (mut top, bottom): (f32, f32)) -> Laid {
    let Plan { way, gap, more, needs, placed, used } = plan;
    let kept = if placed < total { more } else { 0.0 };
    let (mut after, mut drawn, mut pieces) = (used, 0, Vec::new());
    for (index, note) in seen.iter().enumerate().take(placed) {
        after -= needs[index];
        // A list is as long as the room it is given: what the notes after it and the count do not need.
        let gives = matches!(note, Note::List { .. } | Note::Whole { .. });
        let room = if gives { bottom - top - after - kept } else { f32::INFINITY };
        let Some((sheet, id, tilt)) = fill(cx, note, index, top, room, way) else {
            break;
        };
        drawn += 1;
        let mut paper = piece(PieceKind::Note, &id, index, sheet.rect);
        paper.tilt_degrees = cx.theme.tilted(tilt);
        paper.text = sheet.runs;
        let tape_width = cx.px(104.0);
        let tape_left = sheet.rect.x + (sheet.rect.w - tape_width) / 2.0;
        let tape_rect = Rect::new(tape_left, sheet.rect.y - cx.px(14.0), tape_width, cx.px(28.0));
        let mut tape = piece(PieceKind::Tape, &id, index, tape_rect);
        tape.tilt_degrees = cx.theme.tilted(cx.theme.tilt.note_tape);
        top = sheet.rect.bottom() + gap;
        pieces.push(paper);
        pieces.extend(sheet.options);
        pieces.push(tape);
    }
    if drawn < total {
        let sheet = more_sheet(cx, total - drawn, total, top);
        let mut paper = piece(PieceKind::Note, MORE_NOTES_ID, total, sheet.rect);
        paper.text = sheet.runs;
        pieces.push(paper);
    }
    let rows = pieces.iter().filter(|piece| piece.piece == PieceKind::Mark).count();
    Laid { pieces, placed, rows }
}

/// Which notes are placed when room is made in one way.
struct Plan {
    way: Way,
    gap: f32,
    /// What the sheet that counts the notes left out takes, with the gap above it.
    more: f32,
    /// What each note takes with the gap under it; for a list, the least it can be shown in.
    needs: Vec<f32>,
    placed: usize,
    used: f32,
}

/// The notes placed are the first ones that fit `room`, with room kept for the count when any is left out.
fn plan(cx: &Ctx, seen: &[Note], total: usize, way: Way, room: f32) -> Plan {
    let gap = cx.px(way.1);
    let more = gap + more_sheet(cx, 0, total, 0.0).rect.h;
    let needs: Vec<f32> =
        seen.iter().enumerate().map(|(index, note)| height_needed(cx, note, index, way) + gap).collect();
    let (mut placed, mut used) = (0, 0.0);
    for (index, need) in needs.iter().enumerate() {
        let kept = if index + 1 < total { more } else { 0.0 };
        if used + need - gap + kept > room {
            break;
        }
        (placed, used) = (index + 1, used + need);
    }
    Plan { way, gap, more, needs, placed, used }
}

/// The level sheet that stands for the notes left out: one line, `+N more`.
fn more_sheet(cx: &Ctx, left_out: usize, total: usize, top: f32) -> Sheet {
    let mut sheet = open(cx, top, cx.px(14.0), 330.0, (18.0, 22.0));
    paragraph(cx, &mut sheet, &format!("+{left_out} more of {total} notes"), Role::RowSub, 1);
    sheet.rect.h = sheet.cursor - sheet.rect.y + cx.px(18.0 - 9.0);
    sheet
}

/// The height a note takes. A list is measured with two rows: one of its own and the line that counts the rest. A
/// whole list is measured with the one line that counts its rows.
fn height_needed(cx: &Ctx, note: &Note, index: usize, way: Way) -> f32 {
    let least = match note {
        Note::List { title, rows } => list(cx, title, &rows[..rows.len().min(2)], 0.0, f32::INFINITY),
        Note::Whole { title, .. } => {
            whole(cx, (index, title), &[(Mark::Star, String::new())], (0.0, f32::INFINITY), way)
        }
        _ => fill(cx, note, index, 0.0, f32::INFINITY, way).map(|(sheet, ..)| sheet),
    };
    least.map_or(0.0, |sheet| sheet.rect.h)
}

fn fill(cx: &Ctx, note: &Note, index: usize, top: f32, room: f32, way: Way) -> Option<(Sheet, String, f32)> {
    let tilt = cx.theme.tilt;
    let fallback = format!("note-{index}");
    match note {
        Note::List { title, rows } => {
            let sheet = list(cx, title, rows, top, room)?;
            Some((sheet, fallback, tilt.list_note))
        }
        Note::Choice { id, title, options, footer, struck_first, whole } => {
            let (lines, scale, width) = if *whole { (usize::MAX, way.2, f32::INFINITY) } else { (way.0, 1.0, 330.0) };
            let mut sheet = open(cx, top, cx.px(14.0), width, (30.0, 20.0));
            heading(cx, &mut sheet, title, Role::ChoiceTitle);
            for (i, option) in options.iter().take(MAX_OPTIONS).enumerate() {
                let role = if *struck_first && i == 0 { Role::OptionStruck } else { Role::Option };
                choice_option(cx, &mut sheet, (id, i), option, role, (lines, scale));
            }
            paragraph(cx, &mut sheet, footer, Role::ChoiceFooter, 1);
            close(cx, sheet, 18.0, room).map(|sheet| (sheet, id.clone(), tilt.choice_note))
        }
        Note::Whole { title, rows } => {
            let sheet = whole(cx, (index, title), rows, (top, room), way)?;
            Some((sheet, fallback, tilt.list_note))
        }
        Note::Plain { title, body } => {
            let mut sheet = open(cx, top, cx.px(14.0), 330.0, (26.0, 22.0));
            heading(cx, &mut sheet, title, Role::NoteTitle);
            paragraph(cx, &mut sheet, body, Role::Body, 6);
            close(cx, sheet, 20.0, room).map(|sheet| (sheet, fallback, tilt.plain_note))
        }
    }
}

/// Starts a sheet `width` design units wide, capped at the note column.
fn open(cx: &Ctx, top: f32, indent: f32, width: f32, pad: (f32, f32)) -> Sheet {
    let m = cx.theme.metrics;
    let width = cx.px(width.min(m.note_width));
    let x = cx.x(m.note_left) + indent.min(cx.px(m.note_width) - width);
    let side = cx.px(pad.1);
    Sheet {
        rect: Rect::new(x, top, width, 0.0),
        left: x + side,
        inner: width - 2.0 * side,
        cursor: top + cx.px(pad.0),
        runs: Vec::new(),
        options: Vec::new(),
    }
}

/// Ends the sheet; `None` when it does not fit the room left on the paper.
fn close(cx: &Ctx, mut sheet: Sheet, pad_bottom: f32, room: f32) -> Option<Sheet> {
    sheet.rect.h = sheet.cursor - sheet.rect.y + cx.px(pad_bottom - 9.0);
    (sheet.rect.h <= room).then_some(sheet)
}

fn heading(cx: &Ctx, sheet: &mut Sheet, title: &str, role: Role) {
    paragraph(cx, sheet, title, role, 2);
}

/// Adds wrapped lines of `text` and moves the cursor below them.
fn paragraph(cx: &Ctx, sheet: &mut Sheet, text: &str, role: Role, max_lines: usize) {
    let size = cx.size(role);
    let height = line_height(size);
    let lines = wrap(&cx.clean(text), sheet.inner, max_lines, role, size, cx.measure);
    if lines.is_empty() {
        return;
    }
    for (line, width) in lines {
        let mid = sheet.cursor + height / 2.0;
        sheet.runs.push(run(line, role, sheet.left, mid, width, size));
        sheet.cursor += height;
    }
    sheet.cursor += cx.px(9.0);
}

/// One option of a choice, in at most `max_lines` lines of text written at `scale` of the role's size.
fn choice_option(
    cx: &Ctx,
    sheet: &mut Sheet,
    (id, index): (&str, usize),
    text: &str,
    role: Role,
    (max_lines, scale): (usize, f32),
) {
    let size = cx.size(role) * scale;
    let height = line_height(size);
    let (pad_x, pad_y) = (cx.px(14.0), cx.px(9.0));
    let lines = wrap(&cx.clean(text), sheet.inner - 2.0 * pad_x, max_lines, role, size, cx.measure);
    let rect = Rect::new(sheet.left, sheet.cursor, sheet.inner, lines.len().max(1) as f32 * height + 2.0 * pad_y);
    let mut button = piece(PieceKind::Option, id, index, rect);
    for (i, (line, width)) in lines.into_iter().enumerate() {
        let mid = rect.y + pad_y + (i as f32 + 0.5) * height;
        button.text.push(run(line, role, rect.x + pad_x, mid, width, size));
    }
    sheet.options.push(button);
    sheet.cursor = rect.bottom() + cx.px(10.0);
}

/// A titled list; each row is a star, a main text and a quieter second text.
fn list(cx: &Ctx, title: &str, rows: &[(String, String)], top: f32, room: f32) -> Option<Sheet> {
    let mut sheet = open(cx, top, 0.0, cx.theme.metrics.note_width, (26.0, 24.0));
    heading(cx, &mut sheet, title, Role::NoteTitle);
    let size = cx.size(Role::RowMain);
    let pitch = line_height(size) + cx.px(10.0);
    let pad_bottom = cx.px(20.0 - 9.0);
    let left_over = room - (sheet.cursor - top) - pad_bottom;
    let fits = (left_over / pitch).max(0.0) as usize;
    // A list that is cut gives its last line to saying how many rows are left out, and needs a row above that.
    let shown = if fits >= rows.len() { rows.len() } else { fits.saturating_sub(1) };
    if shown == 0 && !rows.is_empty() {
        return None;
    }
    let star = cx.size(Role::Bullet);
    let star_width = cx.measure.width("★", Role::Bullet, star);
    let text_left = sheet.left + cx.px(30.0);
    let text_room = sheet.inner - cx.px(30.0);
    for (main, sub) in rows.iter().take(shown) {
        let mid = sheet.cursor + line_height(size) / 2.0;
        sheet.runs.push(run("★".into(), Role::Bullet, sheet.left, mid, star_width, star));
        let (main, main_width) = cx.line(main, Role::RowMain, text_room * 0.64);
        let sub_left = text_left + main_width + cx.px(10.0);
        let sub_room = text_room - main_width - cx.px(10.0);
        let (sub, sub_width) = cx.line(sub, Role::RowSub, sub_room);
        sheet.runs.push(run(main, Role::RowMain, text_left, mid, main_width, size));
        if !sub.is_empty() {
            let sub_size = cx.size(Role::RowSub);
            sheet.runs.push(run(sub, Role::RowSub, sub_left, mid + cx.px(1.0), sub_width, sub_size));
        }
        sheet.cursor += pitch;
    }
    if shown < rows.len() {
        let (role, mid) = (Role::RowSub, sheet.cursor + line_height(size) / 2.0);
        let (more, width) = cx.line(&format!("+{} more", rows.len() - shown), role, text_room);
        sheet.runs.push(run(more, role, text_left, mid, width, cx.size(role)));
        sheet.cursor += pitch;
    }
    close(cx, sheet, 20.0, room)
}

/// A titled list drawn whole: each row after its mark, to its last word, or left out and counted in the last line.
/// A row that does not fit the room is passed over for the shorter ones after it.
fn whole(
    cx: &Ctx,
    (index, title): (usize, &str),
    rows: &[(Mark, String)],
    (top, room): (f32, f32),
    way: Way,
) -> Option<Sheet> {
    let mut sheet = open(cx, top, 0.0, cx.theme.metrics.note_width, (26.0, 24.0));
    heading(cx, &mut sheet, title, Role::NoteTitle);
    let (role, size) = (Role::RowMain, cx.size(Role::RowMain) * way.2);
    let (height, gap, indent) = (line_height(size), cx.px(10.0), cx.px(30.0));
    let bottom = top + room - cx.px(20.0 - 9.0);
    let seen = &rows[..rows.len().min(MAX_WHOLE_ROWS)];
    let lines = |text: &String| wrap(&cx.clean(text), sheet.inner - indent, usize::MAX, role, size, cx.measure);
    let wrapped: Vec<Vec<(String, f32)>> = seen.iter().map(|(_, text)| lines(text)).collect();
    let need = |lines: &Vec<(String, f32)>| lines.len().max(1) as f32 * height + gap;
    let all = sheet.cursor + wrapped.iter().map(need).sum::<f32>();
    // The last line is kept for the count unless every row is drawn.
    let count = if seen.len() == rows.len() && all <= bottom { 0.0 } else { height + gap };
    let mut shown = 0;
    for (row, ((mark, _), lines)) in seen.iter().zip(&wrapped).enumerate() {
        if sheet.cursor + need(lines) + count > bottom {
            continue;
        }
        let at = Rect::new(sheet.left, sheet.cursor, cx.px(20.0), height);
        let mut sign = piece(PieceKind::Mark, &format!("note-{index}"), row, at);
        if *mark == Mark::Star {
            let star = cx.size(Role::Bullet);
            let width = cx.measure.width("★", Role::Bullet, star);
            sign.text.push(run("★".into(), Role::Bullet, sheet.left, at.y + height / 2.0, width, star));
        }
        sheet.options.push(sign);
        for (i, (line, width)) in lines.iter().enumerate() {
            let mid = sheet.cursor + (i as f32 + 0.5) * height;
            sheet.runs.push(run(line.clone(), role, sheet.left + indent, mid, *width, size));
        }
        sheet.cursor += need(lines);
        shown += 1;
    }
    if shown < rows.len() {
        let (role, mid) = (Role::RowSub, sheet.cursor + height / 2.0);
        let (more, width) = cx.line(&format!("+{} more", rows.len() - shown), role, sheet.inner - indent);
        sheet.runs.push(run(more, role, sheet.left + indent, mid, width, cx.size(role)));
        sheet.cursor += height + gap;
    }
    close(cx, sheet, 20.0, room)
}
