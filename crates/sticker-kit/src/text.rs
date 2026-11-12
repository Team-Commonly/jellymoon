//! Scene text is only ever text: cleaned of control characters, cut to a
//! measured width, and carried as a string for the drawing side to show.

use serde::{Deserialize, Serialize};

use crate::theme::Theme;

/// What a run of text is for; it decides the font, size and colour.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Date,
    Title,
    Label,
    LabelStruck,
    NoteTitle,
    ChoiceTitle,
    RowMain,
    RowSub,
    Body,
    Option,
    OptionStruck,
    ChoiceFooter,
    /// A star glyph in front of a list row or a legend entry.
    Bullet,
    Legend,
    Footer,
}

/// Which of the theme's three font families a role uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Face {
    Display,
    Body,
    Hand,
}

impl Role {
    pub fn face(self) -> Face {
        match self {
            Role::Label | Role::LabelStruck => Face::Display,
            Role::Date | Role::Title | Role::NoteTitle | Role::ChoiceTitle | Role::ChoiceFooter => Face::Hand,
            _ => Face::Body,
        }
    }

    /// The font size in logical pixels at page scale `k`.
    pub fn size(self, theme: &Theme, k: f32) -> f32 {
        let m = &theme.metrics;
        let base = match self {
            Role::Date => m.date_size,
            Role::Title => m.title_size,
            Role::Label | Role::LabelStruck => theme.labels.font_size,
            Role::NoteTitle | Role::ChoiceTitle => m.note_title_size,
            Role::ChoiceFooter => m.note_title_size * 0.74,
            Role::RowMain => m.body_size,
            Role::Bullet => m.body_size * 1.2,
            Role::Body | Role::Option | Role::OptionStruck | Role::Legend => m.body_size - 1.0,
            Role::RowSub | Role::Footer => m.small_size,
        };
        (base * k).max(base.min(theme.labels.min_font_size))
    }
}

/// How wide a single line of text is. The drawing side answers with real
/// glyph metrics; tests answer with a fixed advance.
pub trait Measure {
    fn width(&self, text: &str, role: Role, size: f32) -> f32;
}

/// Every character advances by a fixed share of the font size; wide
/// characters (CJK, emoji) by a whole em.
#[derive(Clone, Copy, Debug)]
pub struct FixedAdvance(pub f32);

impl Measure for FixedAdvance {
    fn width(&self, text: &str, _role: Role, size: f32) -> f32 {
        text.chars().map(|c| if (c as u32) >= 0x1100 { size } else { size * self.0 }).sum()
    }
}

const ELLIPSIS: char = '…';

/// What nobody sees: the line and paragraph separators, and every format character (Unicode category Cf: direction
/// marks, zero-width space, soft hyphen, word joiner, byte-order mark, tag characters and the rest) other than the
/// joiners U+200C and U+200D, which emoji and several scripts are written with. The list is written out because the
/// kit carries no Unicode tables.
pub(crate) fn is_unseen(c: char) -> bool {
    matches!(
        c as u32,
        0x00AD | 0x0600..=0x0605 | 0x061C | 0x06DD | 0x070F | 0x0890 | 0x0891 | 0x08E2 | 0x180E
            | 0x200B | 0x200E | 0x200F | 0x2028..=0x202E | 0x2060..=0x206F | 0xFEFF | 0xFFF9..=0xFFFB
            | 0x110BD | 0x110CD | 0x13430..=0x1343F | 0x1BCA0..=0x1BCA3 | 0x1D173..=0x1D17A | 0xE0000..=0xE007F
    )
}

/// What is a character and is drawn as nothing: every kind of space, the two joiners, the variation selectors, the
/// combining grapheme joiner, the Hangul fillers and the blank braille pattern.
fn is_blank(c: char) -> bool {
    c.is_whitespace()
        || matches!(
            c as u32,
            0x034F | 0x115F | 0x1160 | 0x200C | 0x200D | 0x2800 | 0x3164 | 0xFE00..=0xFE0F | 0xFFA0 | 0xE0100..=0xE01EF
        )
}

/// Whether `text` has a character a person sees. Text with none is no words, whatever it holds: spaces, what is
/// [`is_unseen`], joiners and selectors with nothing to join or select, fillers. The one rule for it in the kit.
pub fn has_visible(text: &str) -> bool {
    text.chars().any(|c| !(c.is_control() || is_unseen(c) || is_blank(c)))
}

/// Removes control characters and what is [`is_unseen`], turns line breaks and tabs into single spaces, and keeps at
/// most `max_chars` characters, then an ellipsis when any were cut.
pub fn clean(text: &str, max_chars: usize) -> String {
    let mut out = String::new();
    let mut count = 0;
    let mut pending_space = false;
    for c in text.chars() {
        if matches!(c, '\n' | '\r' | '\t' | ' ') {
            pending_space = !out.is_empty();
            continue;
        }
        if c.is_control() || is_unseen(c) {
            continue;
        }
        let needed = if pending_space { 2 } else { 1 };
        if count + needed > max_chars {
            // The character that does not fit may be the rest of the last one kept: then that one goes too.
            let mut with = out.clone();
            with.push(c);
            out.truncate(ends(&with).iter().rev().nth(1).copied().unwrap_or(0).min(out.len()));
            out.truncate(out.trim_end_matches([' ', '\u{200C}', '\u{200D}']).len());
            out.push(ELLIPSIS);
            return out;
        }
        if pending_space {
            out.push(' ');
        }
        out.push(c);
        count += needed;
        pending_space = false;
    }
    out
}

/// What is drawn onto the character before it: the combining marks of Latin, Greek and Cyrillic and of symbols, the
/// two kana voicing marks, the variation selectors, the two joiners and the emoji skin tones.
fn is_mark(c: char) -> bool {
    matches!(
        c as u32,
        0x0300..=0x036F | 0x0483..=0x0489 | 0x1AB0..=0x1AFF | 0x1DC0..=0x1DFF | 0x200C | 0x200D | 0x20D0..=0x20FF
            | 0x3099 | 0x309A | 0xFE00..=0xFE0F | 0xFE20..=0xFE2F | 0x1F3FB..=0x1F3FF | 0xE0100..=0xE01EF
    )
}

/// Where each character a reader sees ends, in bytes. No cut or line break falls anywhere else. A character is a
/// base with every [`is_mark`] after it, whatever U+200D joins to it, or two regional indicators (a flag). This is
/// not all of Unicode's rule, which needs tables the kit does not carry: the marks of other scripts (Hebrew, Arabic,
/// Indic, Thai) and Hangul's jamo each count as a character of their own.
pub(crate) fn ends(text: &str) -> Vec<usize> {
    let mut ends: Vec<usize> = Vec::new();
    let (mut before, mut flags) = ('\0', 0);
    for (at, c) in text.char_indices() {
        let flag = matches!(c as u32, 0x1F1E6..=0x1F1FF);
        flags = if flag { flags + 1 } else { 0 };
        if at > 0 && !(is_mark(c) || before == '\u{200D}' || (flag && flags % 2 == 0)) {
            ends.push(at);
        }
        before = c;
    }
    if !text.is_empty() {
        ends.push(text.len());
    }
    ends
}

/// The longest prefix of `text` that fits `max_width`, with an ellipsis when
/// it was cut. Returns the text and its measured width.
pub fn fit(text: &str, max_width: f32, role: Role, size: f32, measure: &dyn Measure) -> (String, f32) {
    let full = measure.width(text, role, size);
    if full <= max_width {
        return (text.to_string(), full);
    }
    let cut = |end: usize| format!("{}{ELLIPSIS}", text[..end].trim_end_matches(' '));
    let mut cuts = vec![0];
    cuts.extend(ends(text));
    let fitting = cuts.partition_point(|end| measure.width(&cut(*end), role, size) <= max_width);
    match fitting.checked_sub(1) {
        Some(last) => (cut(cuts[last]), measure.width(&cut(cuts[last]), role, size)),
        None => (String::new(), 0.0),
    }
}

/// `text` whole when it fits `max_width`; else its start and its end with an ellipsis between them, as much of both
/// as fits, the end the longer by a character when they differ: two texts that begin alike are still told apart.
/// Returns the text and its measured width.
pub fn fit_middle(text: &str, max_width: f32, role: Role, size: f32, measure: &dyn Measure) -> (String, f32) {
    let full = measure.width(text, role, size);
    if full <= max_width {
        return (text.to_string(), full);
    }
    let mut cuts = vec![0];
    cuts.extend(ends(text));
    let count = cuts.len() - 1;
    let cut = |kept: usize| {
        let (start, end) = (&text[..cuts[kept / 2]], &text[cuts[count - (kept - kept / 2)]..]);
        format!("{}{ELLIPSIS}{}", start.trim_end_matches(' '), end.trim_start_matches(' '))
    };
    let kept: Vec<usize> = (0..count).collect();
    let fitting = kept.partition_point(|kept| measure.width(&cut(*kept), role, size) <= max_width);
    match fitting.checked_sub(1) {
        Some(most) => (cut(most), measure.width(&cut(most), role, size)),
        None => (String::new(), 0.0),
    }
}

/// Breaks `text` into at most `max_lines` lines that fit `max_width`: at a space, and inside a word only when the
/// word is too long for a line of its own (a long address, or text in a script written without spaces), never inside
/// a character ([`ends`]). When text is left over, the last line ends with an ellipsis.
pub fn wrap(
    text: &str,
    max_width: f32,
    max_lines: usize,
    role: Role,
    size: f32,
    measure: &dyn Measure,
) -> Vec<(String, f32)> {
    let fits = |line: &str| measure.width(line, role, size) <= max_width;
    let mut lines: Vec<String> = Vec::new();
    let mut rest = text.trim_matches(' ');
    while !rest.is_empty() && lines.len() < max_lines {
        let tail = if lines.len() + 1 == max_lines { "…" } else { "" };
        let end = line_end(rest, tail, &fits);
        let cut = rest[..end].trim_end_matches(' ');
        lines.push(if end < rest.len() { format!("{cut}{tail}") } else { cut.to_string() });
        rest = rest[end..].trim_start_matches(' ');
    }
    lines.iter().map(|line| fit(line, max_width, role, size, measure)).filter(|(line, _)| !line.is_empty()).collect()
}

/// How much of `rest` goes on one line that ends with `tail` when text is left over. Never nothing.
fn line_end(rest: &str, tail: &str, fits: &dyn Fn(&str) -> bool) -> usize {
    if fits(rest) {
        return rest.len();
    }
    let with_tail = |end: usize| fits(&format!("{}{tail}", &rest[..end]));
    // Whole words first: `end` is after the last one that fits, `word` is the one that does not.
    let (mut end, mut word) = (0, 0);
    let word_end = loop {
        let word_end = rest[word..].find(' ').map_or(rest.len(), |space| word + space);
        if !with_tail(word_end) {
            break word_end;
        }
        end = word_end;
        word = word_end + rest[word_end..].len() - rest[word_end..].trim_start_matches(' ').len();
        if word >= rest.len() {
            return end;
        }
    };
    if end == 0 || !fits(&rest[word..word_end]) {
        let inside: Vec<usize> = ends(&rest[word..word_end]).iter().map(|at| word + at).collect();
        let fitting = inside.partition_point(|at| with_tail(*at));
        end = match fitting.checked_sub(1) {
            Some(last) => inside[last],
            None if end == 0 => inside.first().copied().unwrap_or(rest.len()),
            None => end,
        };
    }
    end
}
