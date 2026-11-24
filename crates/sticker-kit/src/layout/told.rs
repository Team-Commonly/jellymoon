//! A told note (`Note::Told`): someone else's words under the note's own labels. Each row is drawn to its last
//! character or left out, and a last line, `+N more`, counts the rows left out: a row is never cut. Each text of
//! the other's is a `Quote` piece and nothing else; the sheet's own runs are its title, its labels, the count and
//! its own sentences, which are drawn whole after the rows, or the note is left out.

use super::notes::{Sheet, close, heading, open};
use super::{Ctx, PieceKind, line_height, piece, run};
use crate::geom::Rect;
use crate::scene::ToldRow;
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
/// starts from the sheet's left, and the padding down a ground and the gap under it.
struct Row {
    label: Lines,
    quotes: Vec<(Option<(String, f32)>, Lines)>,
    column: f32,
    close: (f32, f32),
}

fn whole(cx: &Ctx, text: &str, width: f32, role: Role) -> Option<Lines> {
    let kept = cx.clean(text);
    let uncut = kept == clean(text, usize::MAX);
    uncut.then(|| wrap(&kept, width, usize::MAX, role, cx.size(role), cx.measure))
}

/// Whether a name is one a line of the note's own may carry: not empty, and no blank, unseen or control character.
pub(super) fn plain(name: &str) -> bool {
    !name.is_empty() && name.chars().all(|c| !(is_blank(c) || is_unseen(c) || c.is_control()))
}

/// `text` when it is drawn whole on one line of `width`.
pub(super) fn one_line(cx: &Ctx, text: &str, width: f32, role: Role) -> Option<(String, f32)> {
    whole(cx, text, width, role).filter(|lines| lines.len() == 1)?.pop()
}

/// A row's label lines, its quotes and the words beside them: the name on the label's line when it is [`plain`]
/// and the two lay out as one line; else the label alone and the name the first quote.
fn named(cx: &Ctx, inner: f32, row: &ToldRow) -> Option<(Lines, Vec<String>, Vec<String>)> {
    if plain(&row.name)
        && let Some(label) = one_line(cx, &format!("{} {}", row.label, row.name), inner, LABEL)
    {
        return Some((vec![label], row.quotes.clone(), row.marks.clone()));
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
    Some(Row { label, quotes, column, close })
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

/// The sheet of a told note at `top`: its rows in their order, each whole, up to the first that the cleaning every
/// text is drawn with would cut, and none after it. None when one of the note's own sentences would be cut.
pub(super) fn sheet(cx: &Ctx, (id, title, rows, end): (&str, &str, &[ToldRow], &[String]), top: f32) -> Option<Sheet> {
    let mut sheet = open(cx, top, (26.0, 24.0));
    heading(cx, &mut sheet, title, Role::NoteTitle);
    let (body, gap) = (cx.size(Role::Body), cx.px(9.0));
    let ends: Vec<Lines> = end.iter().map(|text| whole(cx, text, sheet.inner, Role::Body)).collect::<Option<_>>()?;
    let (mut shown, mut next) = (0, 0);
    for row in rows.iter().take(MAX_TOLD_ROWS) {
        let Some(row) = shape(cx, sheet.inner, row) else { break };
        draw(cx, &mut sheet, id, &row, &mut next);
        shown += 1;
    }
    if shown < rows.len() {
        let (more, width) = cx.line(&format!("+{} more", rows.len() - shown), Role::RowSub, sheet.inner);
        let mid = sheet.cursor + line_height(cx.size(Role::RowSub)) / 2.0;
        sheet.runs.push(run(more, Role::RowSub, sheet.left, mid, width, cx.size(Role::RowSub)));
        sheet.cursor += line_height(cx.size(Role::RowSub)) + gap;
    }
    for lines in ends {
        for (line, width) in lines {
            sheet.runs.push(run(line, Role::Body, sheet.left, sheet.cursor + line_height(body) / 2.0, width, body));
            sheet.cursor += line_height(body);
        }
        sheet.cursor += gap;
    }
    Some(close(cx, sheet, 20.0))
}
