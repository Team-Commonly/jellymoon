//! A text note: a title, the texts of its body each from a line of its own, and its options under them. Its body
//! is drawn in [`MAX_TEXT_LINES`] lines at most: when its texts do not all fit those, the first ones are drawn and a
//! last line, `+N more`, counts the rest. No text is hidden without that count, and none is drawn but as text.
//!
//! An option is an answer to what the note says, so it is drawn only under a title and a body that were both
//! drawn whole: the title and every text of the body, each to its last character. Under a title cut at its
//! second line, a body with a text cut short, or texts the count stands for, no option is placed, and a tap
//! finds none. What it holds besides (`typed.rs`: someone else's words, quoted, and a field) is under the body
//! and stays, with or without the options.

use super::notes::{MAX_OPTIONS, OPTION_LINES, Sheet, choice_option, close, heading, open};
use super::{Ctx, line_height, run, typed};
use crate::scene::Field;
use crate::text::{Role, clean, wrap};

/// The most lines of its body a text note is drawn with.
pub const MAX_TEXT_LINES: usize = 24;

/// What a text note holds under its title.
#[derive(Clone, Copy)]
pub(super) struct Parts<'a> {
    pub body: &'a [String],
    pub options: &'a [String],
    pub quoted: &'a str,
    pub field: Option<&'a Field>,
}

/// The sheet of a text note at `top`: with its options when its title and its body are drawn whole, and else
/// with none.
pub(super) fn text(cx: &Ctx, (id, title): (&str, &str), parts: Parts, top: f32) -> Sheet {
    let (sheet, whole) = written(cx, (id, title), parts, top);
    if whole || parts.options.is_empty() {
        return sheet;
    }
    written(cx, (id, title), Parts { options: &[], ..parts }, top).0
}

/// Whether [`heading`] draws the title to its last character: it draws two lines of it at most.
fn titled(cx: &Ctx, sheet: &Sheet, title: &str) -> bool {
    let (kept, size) = (cx.clean(title), cx.size(Role::NoteTitle));
    kept == clean(title, usize::MAX) && wrap(&kept, sheet.inner, 3, Role::NoteTitle, size, cx.measure).len() <= 2
}

/// The sheet with these options under its body, and whether the title and the body were drawn whole.
fn written(
    cx: &Ctx,
    (id, title): (&str, &str),
    Parts { body, options, quoted, field }: Parts,
    top: f32,
) -> (Sheet, bool) {
    let mut sheet = open(cx, top, (26.0, 24.0));
    heading(cx, &mut sheet, title, Role::NoteTitle);
    let (role, size, fits) = (Role::Body, cx.size(Role::Body), MAX_TEXT_LINES);
    let (height, gap) = (line_height(size), cx.px(9.0));
    // Only the texts that could be drawn are cleaned and wrapped, so a body of any number of texts costs the same.
    let near: Vec<(String, bool)> = body
        .iter()
        .take(fits + 1)
        .map(|text| {
            let kept = cx.clean(text);
            let cut = kept != clean(text, usize::MAX);
            (kept, cut)
        })
        .collect();
    let lines = |(text, _): &(String, bool)| wrap(text, sheet.inner, fits + 1, role, size, cx.measure);
    let wrapped: Vec<Vec<(String, f32)>> = near.iter().map(lines).collect();
    let all = body.len() <= fits && wrapped.iter().map(Vec::len).sum::<usize>() <= fits;
    // The last line is kept for the count unless every text is drawn to its end.
    let (mut left, mut drawn) = (if all { fits } else { fits - 1 }, 0);
    for lines in &wrapped {
        if lines.len() > left {
            break;
        }
        for (line, width) in lines {
            sheet.runs.push(run(line.clone(), role, sheet.left, sheet.cursor + height / 2.0, *width, size));
            sheet.cursor += height;
        }
        (left, drawn) = (left - lines.len(), drawn + 1);
    }
    if drawn < body.len() {
        let (more, width) = cx.line(&format!("+{} more", body.len() - drawn), Role::RowSub, sheet.inner);
        sheet.runs.push(run(more, Role::RowSub, sheet.left, sheet.cursor + height / 2.0, width, cx.size(Role::RowSub)));
        sheet.cursor += height;
    }
    if !sheet.runs.is_empty() {
        sheet.cursor += gap;
    }
    // Under the body: the quoted words, the options, and the field. The quoted words and the field stay when
    // the options do not.
    typed::quoted(cx, &mut sheet, id, quoted);
    for (index, option) in options.iter().take(MAX_OPTIONS).enumerate() {
        choice_option(cx, &mut sheet, (id, index), option, Role::Option, OPTION_LINES);
    }
    if let Some(field) = field {
        typed::field(cx, &mut sheet, id, field);
    }
    let whole = titled(cx, &sheet, title) && drawn == body.len() && near.iter().all(|(_, cut)| !cut);
    (close(cx, sheet, 20.0), whole)
}
