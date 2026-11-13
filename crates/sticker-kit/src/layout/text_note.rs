//! A text note: a title, the texts of its body each from a line of its own, and its options under them. It is as
//! long as the room it is given, like a list: when its texts do not all fit, the first ones are drawn and a last
//! line, `+N more`, counts the rest. No text is hidden without that count, and none is drawn but as text.

use super::notes::{MAX_OPTIONS, Sheet, choice_option, close, heading, open};
use super::{Ctx, line_height, run};
use crate::text::{Role, wrap};

/// The most lines of its body a text note is drawn with.
pub const MAX_TEXT_LINES: usize = 24;

/// The fewest lines of body a text note is planned with: one of its own and the line that counts the rest.
pub(super) const LEAST_TEXT_LINES: usize = 2;

/// The sheet of a text note at `top`, with at most `most` lines of `body` and never taller than `room`. None when
/// its body does not fit and there is no room for one text of it and the count.
pub(super) fn text(
    cx: &Ctx,
    (id, title): (&str, &str),
    (body, options): (&[String], &[String]),
    top: f32,
    (room, most): (f32, usize),
    option_lines: usize,
) -> Option<Sheet> {
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
    // Only the texts that could be drawn are wrapped, so a body of any length costs the same.
    let lines = |text: &String| wrap(&cx.clean(text), sheet.inner, fits + 1, role, size, cx.measure);
    let wrapped: Vec<Vec<(String, f32)>> = body.iter().take(fits + 1).map(lines).collect();
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
    close(cx, sheet, 20.0, room)
}
