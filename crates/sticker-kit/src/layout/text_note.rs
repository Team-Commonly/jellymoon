//! A text note: a title, the texts of its body each from a line of its own, and its options under them. It is as
//! long as the room it is given, like a list: when its texts do not all fit, the first ones are drawn and a last
//! line, `+N more`, counts the rest. No text is hidden without that count, and none is drawn but as text.
//!
//! An option is an answer to what the note says, so it is drawn only under a title and a body that were both
//! drawn whole: the title and every text of the body, each to its last character. Under a title cut at its
//! second line, a body with a text cut short, or texts the count stands for, no option is placed, and a tap
//! finds none.

use super::notes::{MAX_OPTIONS, Sheet, choice_option, close, heading, open};
use super::{Ctx, line_height, run};
use crate::text::{Role, clean, wrap};

/// The most lines of its body a text note is drawn with.
pub const MAX_TEXT_LINES: usize = 24;

/// The fewest lines of body a text note is planned with: one of its own and the line that counts the rest.
pub(super) const LEAST_TEXT_LINES: usize = 2;

/// The sheet of a text note at `top`, with at most `most` lines of `body` and never taller than `room`: with its
/// options when the body is drawn whole in the room they leave, and else with none. None when its body does not
/// fit and there is no room for one text of it and the count.
pub(super) fn text(
    cx: &Ctx,
    (id, title): (&str, &str),
    (body, options): (&[String], &[String]),
    top: f32,
    (room, most): (f32, usize),
    option_lines: usize,
) -> Option<Sheet> {
    let with = |options: &[String]| written(cx, (id, title), (body, options), top, (room, most), option_lines);
    match with(options) {
        // With its options there may be no room for one text and the count: without them there may be.
        Some((_, false)) | None if !options.is_empty() => with(&[]).map(|(sheet, _)| sheet),
        written => written.map(|(sheet, _)| sheet),
    }
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
    (body, options): (&[String], &[String]),
    top: f32,
    (room, most): (f32, usize),
    option_lines: usize,
) -> Option<(Sheet, bool)> {
    let start = || {
        let mut sheet = open(cx, top, 0.0, cx.theme.metrics.note_width, (26.0, 24.0));
        heading(cx, &mut sheet, title, Role::NoteTitle);
        sheet
    };
    let end = |sheet: &mut Sheet| {
        for (index, option) in options.iter().take(MAX_OPTIONS).enumerate() {
            choice_option(cx, sheet, (id, index), option, Role::Option, (option_lines, 1.0));
        }
    };
    let (role, size) = (Role::Body, cx.size(Role::Body));
    let (height, gap) = (line_height(size), cx.px(9.0));
    // What the title and the options take with nothing between them.
    let mut bare = start();
    end(&mut bare);
    let fixed = bare.cursor - top + cx.px(20.0 - 9.0) + gap;
    let fits = (((room - fixed) / height).max(0.0) as usize).min(most);
    let mut sheet = start();
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
    if !all && fits < LEAST_TEXT_LINES {
        return None;
    }
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
    end(&mut sheet);
    let whole = titled(cx, &sheet, title) && drawn == body.len() && near.iter().all(|(_, cut)| !cut);
    close(cx, sheet, 20.0, room).map(|sheet| (sheet, whole))
}
