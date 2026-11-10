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
fn is_unseen(c: char) -> bool {
    matches!(
        c as u32,
        0x00AD | 0x0600..=0x0605 | 0x061C | 0x06DD | 0x070F | 0x0890 | 0x0891 | 0x08E2 | 0x180E
            | 0x200B | 0x200E | 0x200F | 0x2028..=0x202E | 0x2060..=0x206F | 0xFEFF | 0xFFF9..=0xFFFB
            | 0x110BD | 0x110CD | 0x13430..=0x1343F | 0x1BCA0..=0x1BCA3 | 0x1D173..=0x1D17A | 0xE0000..=0xE007F
    )
}

/// Removes control characters and what is [`is_unseen`], turns line breaks and tabs into single spaces, and keeps at
/// most `max_chars` characters.
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
            trim_joiners(&mut out);
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

/// A cut must not leave half of a joined emoji or a dangling space.
fn trim_joiners(text: &mut String) {
    while text.chars().last().is_some_and(|c| matches!(c as u32, 0x200D | 0xFE0F | 0x20) || is_modifier(c)) {
        text.pop();
    }
}

fn is_modifier(c: char) -> bool {
    matches!(c as u32, 0x0300..=0x036F | 0x1F3FB..=0x1F3FF)
}

/// The longest prefix of `text` that fits `max_width`, with an ellipsis when
/// it was cut. Returns the text and its measured width.
pub fn fit(text: &str, max_width: f32, role: Role, size: f32, measure: &dyn Measure) -> (String, f32) {
    let full = measure.width(text, role, size);
    if full <= max_width {
        return (text.to_string(), full);
    }
    let starts: Vec<usize> = text.char_indices().map(|(i, _)| i).collect();
    let mut best = (String::new(), 0.0);
    let (mut low, mut high) = (0, starts.len().saturating_sub(1));
    while low <= high {
        let mid = (low + high) / 2;
        let mut candidate = text[..starts[mid]].to_string();
        trim_joiners(&mut candidate);
        candidate.push(ELLIPSIS);
        let width = measure.width(&candidate, role, size);
        if width <= max_width {
            best = (candidate, width);
            low = mid + 1;
        } else if mid == 0 {
            break;
        } else {
            high = mid - 1;
        }
    }
    best
}

/// Breaks `text` at spaces into at most `max_lines` lines that fit `max_width`. When text is left over, the last line
/// ends with an ellipsis after the last word that fits; a single word too long for a line is cut inside.
pub fn wrap(
    text: &str,
    max_width: f32,
    max_lines: usize,
    role: Role,
    size: f32,
    measure: &dyn Measure,
) -> Vec<(String, f32)> {
    let fits = |line: &str| measure.width(line, role, size) <= max_width;
    let words: Vec<&str> = text.split(' ').filter(|w| !w.is_empty()).collect();
    let mut lines: Vec<String> = Vec::new();
    let mut at = 0;
    while at < words.len() {
        let last_line = lines.len() + 1 >= max_lines;
        let mut line = words[at].to_string();
        at += 1;
        while at < words.len() {
            let joined = format!("{line} {}", words[at]);
            // The last line keeps room for the ellipsis unless the word is the last one.
            let whole = if last_line && at + 1 < words.len() { format!("{joined}{ELLIPSIS}") } else { joined.clone() };
            if !fits(&whole) {
                break;
            }
            (line, at) = (joined, at + 1);
        }
        if last_line && at < words.len() {
            line.push(ELLIPSIS);
            at = words.len();
        }
        lines.push(line);
    }
    lines.iter().map(|line| fit(line, max_width, role, size, measure)).filter(|(line, _)| !line.is_empty()).collect()
}
