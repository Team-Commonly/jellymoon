//! The cards of the column: a note of the scene filled from its top down at the column's width, as high as what
//! it holds, and put on the page under a strip of tape. Nothing here knows of room: a card is never cut, made
//! smaller or left out because the page is long.

use super::text_note::{Parts, text};
use super::told;
use super::{Ctx, PieceKind, Placed, TextRun, line_height, piece, run};
use crate::geom::Rect;
use crate::scene::{Mark, Note};
use crate::text::{Role, wrap};

pub(super) const MAX_OPTIONS: usize = 4;

/// The lines an option of a choice that is not `whole`, or of a text note, is drawn in at most.
pub(super) const OPTION_LINES: usize = 3;

/// How many rows of a whole list are laid out at most. The ones after are only counted.
const MAX_WHOLE_ROWS: usize = 12;

/// How many rows of a list are laid out at most, so a list of any length costs the same. The ones after are
/// counted in a last line.
const MAX_LIST_ROWS: usize = 24;

/// A note being filled from the top down.
pub(super) struct Sheet {
    pub rect: Rect,
    /// Left edge and width of the text column.
    pub left: f32,
    pub inner: f32,
    pub cursor: f32,
    pub runs: Vec<TextRun>,
    pub options: Vec<Placed>,
}

/// Puts a filled sheet on the page under its strip of tape. Returns its bottom edge.
pub(super) fn stick(cx: &mut Ctx, sheet: Sheet, (id, index): (&str, usize), tilt: f32) -> f32 {
    let mut paper = piece(PieceKind::Note, id, index, sheet.rect);
    paper.tilt_degrees = cx.theme.tilted(tilt);
    paper.text = sheet.runs;
    let tape_width = cx.px(104.0);
    let tape_left = sheet.rect.x + (sheet.rect.w - tape_width) / 2.0;
    let tape_rect = Rect::new(tape_left, sheet.rect.y - cx.px(14.0), tape_width, cx.px(28.0));
    let mut tape = piece(PieceKind::Tape, id, index, tape_rect);
    tape.tilt_degrees = cx.theme.tilted(cx.theme.tilt.note_tape);
    cx.out.push(paper);
    cx.out.extend(sheet.options.into_iter().map(|mut mark| {
        if mark.piece == PieceKind::Mark {
            mark.id = id.to_string();
        }
        mark
    }));
    cx.out.push(tape);
    sheet.rect.bottom()
}

/// The card of `note`, the scene's `index`th, from `top`: its sheet, the id its pieces carry and its tilt. None for
/// a told note one of whose own sentences would be cut, and for what the person typed, which is no card.
pub(super) fn fill(cx: &Ctx, note: &Note, index: usize, top: f32) -> Option<(Sheet, String, f32)> {
    let tilt = cx.theme.tilt;
    let fallback = format!("note-{index}");
    let named = |id: &String| if id.is_empty() { fallback.clone() } else { id.clone() };
    Some(match note {
        Note::List { title, rows } => (list(cx, title, rows, top), fallback, tilt.list_note),
        Note::Choice { id, title, options, footer, struck_first, whole } => {
            let lines = if *whole { usize::MAX } else { OPTION_LINES };
            let mut sheet = open(cx, top, (30.0, 20.0));
            heading(cx, &mut sheet, title, Role::ChoiceTitle);
            for (i, option) in options.iter().take(MAX_OPTIONS).enumerate() {
                let role = if *struck_first && i == 0 { Role::OptionStruck } else { Role::Option };
                choice_option(cx, &mut sheet, (id, i), option, role, lines);
            }
            paragraph(cx, &mut sheet, footer, Role::ChoiceFooter, 1);
            (close(cx, sheet, 18.0), id.clone(), tilt.choice_note)
        }
        Note::Text { id, title, body, options, quoted, field } => {
            let parts = Parts { body, options, quoted, field: field.as_ref() };
            (text(cx, (id, title), parts, top), named(id), tilt.list_note)
        }
        Note::Whole { title, rows } => (whole(cx, title, rows, top), fallback, tilt.list_note),
        Note::Told { id, title, rows, end } => {
            let sheet = told::sheet(cx, (id.as_str(), title.as_str(), rows.as_slice(), end.as_slice()), top)?;
            (sheet, named(id), tilt.list_note)
        }
        Note::Plain { title, body } => {
            let mut sheet = open(cx, top, (26.0, 22.0));
            heading(cx, &mut sheet, title, Role::NoteTitle);
            paragraph(cx, &mut sheet, body, Role::Body, 6);
            (close(cx, sheet, 20.0), fallback, tilt.plain_note)
        }
        Note::Said { .. } => return None,
    })
}

/// Starts a sheet as wide as the column, with `pad` design units over its first line and at its sides.
pub(super) fn open(cx: &Ctx, top: f32, pad: (f32, f32)) -> Sheet {
    let (x, width) = cx.column();
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

/// Ends the sheet under what it holds.
pub(super) fn close(cx: &Ctx, mut sheet: Sheet, pad_bottom: f32) -> Sheet {
    sheet.rect.h = sheet.cursor - sheet.rect.y + cx.px(pad_bottom - 9.0);
    sheet
}

pub(super) fn heading(cx: &Ctx, sheet: &mut Sheet, title: &str, role: Role) {
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

/// One option of a choice, in at most `max_lines` lines of text.
pub(super) fn choice_option(
    cx: &Ctx,
    sheet: &mut Sheet,
    (id, index): (&str, usize),
    text: &str,
    role: Role,
    max_lines: usize,
) {
    let size = cx.size(role);
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
fn list(cx: &Ctx, title: &str, rows: &[(String, String)], top: f32) -> Sheet {
    let mut sheet = open(cx, top, (26.0, 24.0));
    heading(cx, &mut sheet, title, Role::NoteTitle);
    let size = cx.size(Role::RowMain);
    let pitch = line_height(size) + cx.px(10.0);
    let star = cx.size(Role::Bullet);
    let star_width = cx.measure.width("★", Role::Bullet, star);
    let text_left = sheet.left + cx.px(30.0);
    let text_room = sheet.inner - cx.px(30.0);
    for (main, sub) in rows.iter().take(MAX_LIST_ROWS) {
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
    if rows.len() > MAX_LIST_ROWS {
        let (role, mid) = (Role::RowSub, sheet.cursor + line_height(size) / 2.0);
        let (more, width) = cx.line(&format!("+{} more", rows.len() - MAX_LIST_ROWS), role, text_room);
        sheet.runs.push(run(more, role, text_left, mid, width, cx.size(role)));
        sheet.cursor += pitch;
    }
    close(cx, sheet, 20.0)
}

/// A titled list drawn whole: each row after its mark, to its last word. The rows past the first
/// [`MAX_WHOLE_ROWS`] are counted in a last line.
fn whole(cx: &Ctx, title: &str, rows: &[(Mark, String)], top: f32) -> Sheet {
    let mut sheet = open(cx, top, (26.0, 24.0));
    heading(cx, &mut sheet, title, Role::NoteTitle);
    let (role, size) = (Role::RowMain, cx.size(Role::RowMain));
    let (height, gap, indent) = (line_height(size), cx.px(10.0), cx.px(30.0));
    for (row, (mark, text)) in rows.iter().take(MAX_WHOLE_ROWS).enumerate() {
        let lines = wrap(&cx.clean(text), sheet.inner - indent, usize::MAX, role, size, cx.measure);
        let at = Rect::new(sheet.left, sheet.cursor, cx.px(20.0), height);
        let mut sign = piece(PieceKind::Mark, "", row, at);
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
        sheet.cursor += lines.len().max(1) as f32 * height + gap;
    }
    if rows.len() > MAX_WHOLE_ROWS {
        let (role, mid) = (Role::RowSub, sheet.cursor + height / 2.0);
        let (more, width) = cx.line(&format!("+{} more", rows.len() - MAX_WHOLE_ROWS), role, sheet.inner - indent);
        sheet.runs.push(run(more, role, sheet.left + indent, mid, width, cx.size(role)));
        sheet.cursor += height + gap;
    }
    close(cx, sheet, 20.0)
}
