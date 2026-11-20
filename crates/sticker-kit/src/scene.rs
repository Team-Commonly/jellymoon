//! What is on a page, as plain data. Nothing here knows how it is drawn.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Scene {
    pub date_label: String,
    pub title: String,
    pub hero: Option<Character>,
    pub characters: Vec<Character>,
    pub notes: Vec<Note>,
    pub legend: Vec<LegendItem>,
    pub footer: String,
}

/// A place to type one line and enter it, with a small set of options of which one is always chosen. It stands
/// beside a scene and is not a field of one: its spot is its own and it moves no other piece
/// ([`crate::layout_entry`]), and a scene built field by field outside the kit stays as it is. What is typed is the
/// view's ([`crate::entry::EntryState`]); this says only what the entry is.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Entry {
    pub id: String,
    /// Shown while nothing is typed.
    pub hint: String,
    /// The first [`crate::entry::MAX_ENTRY_OPTIONS`] are offered, each on a chip; the rest are counted.
    pub options: Vec<String>,
    /// The most characters the entry takes: at most [`crate::entry::MAX_ENTRY_CHARS`], which 0 also means.
    pub most: usize,
    /// One thing to do that is no option, after the chips, or nothing when empty: a tap on it is
    /// `Tap::Action` with the entry's id. It is never chosen and Enter never gives it.
    pub action: String,
    /// The choice beside the field that does what Enter does, or nothing when empty. It is drawn as not ready
    /// while the field holds no words, and a tap on it then enters nothing: it is `Tap::Go` with the entry's id,
    /// so whoever made the entry can say what to type first.
    pub go: String,
    /// Whether the action is the entry's loud choice: filled as a chosen chip is, while no chip is drawn chosen.
    pub loud: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Character {
    pub id: String,
    pub label: String,
    /// Index into the theme's hues; wraps when the theme has fewer.
    pub hue: u8,
    pub stars: u8,
    pub state: State,
    pub mood: Mood,
    /// A few words of what a busy character is doing, drawn on a small bubble over it when there is room and it
    /// has no star: text only, cut to fit. Nothing when empty, and never of a character that is not busy.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub says: String,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum State {
    #[default]
    Idle,
    Busy,
    NeedsYou,
    Dimmed,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mood {
    #[default]
    Happy,
    Cross,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Note {
    List {
        title: String,
        rows: Vec<(String, String)>,
    },
    /// With `whole`, every option is drawn to its last word, on as many lines as that takes: the note is left out
    /// and counted when it does not fit, and never drawn with an option cut short.
    Choice {
        id: String,
        title: String,
        options: Vec<String>,
        footer: String,
        struck_first: bool,
        #[serde(default)]
        whole: bool,
    },
    Plain {
        title: String,
        body: String,
    },
    /// A text and the choices under it. Each of `body` starts on a line of its own and is wrapped, as many lines
    /// as the page has room for and at most [`crate::layout::MAX_TEXT_LINES`]; a last line counts the ones of `body`
    /// that are not drawn to their end. A tap on an option is the note's `id` and the option's place, as a choice's.
    ///
    /// `quoted` is someone else's words: drawn under the body on a ground of their own, in the tone for quoted
    /// words, cut to at most [`crate::layout::MAX_QUOTED_LINES`] lines, and never anything but text. `field` is
    /// one line a person types in under the note; what Enter gives carries the note's `id`.
    Text {
        id: String,
        title: String,
        #[serde(default)]
        body: Vec<String>,
        #[serde(default)]
        options: Vec<String>,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        quoted: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        field: Option<Field>,
    },
    /// A list of texts no word of which may be hidden. Each row is drawn after its mark to its last word, on as many
    /// lines as that takes, or it is left out; a last line, `+N more`, counts the rows left out. No row is cut short.
    Whole {
        title: String,
        rows: Vec<(Mark, String)>,
    },
    /// What someone else says of a thing, under the note's own title: each row is a label in the note's own words
    /// and the other's texts under it, each a `Quote` piece on a ground of its own and never anything but text. A
    /// row is drawn to its last character or left out, and a last line, `+N more`, counts the rows left out. `end`
    /// is the note's own sentences after the rows, each drawn whole, or the note is left out. It has no option
    /// and no field. It is planned after the note that follows it, in the room that note leaves, so that note is
    /// drawn as it is without it, lower, and the notes after them give way; it is drawn in its place over that note.
    Told {
        id: String,
        title: String,
        rows: Vec<ToldRow>,
        #[serde(default)]
        end: Vec<String>,
    },
}

/// One row of a told note: the note's own label, which may be empty, and someone else's texts under it. `marks`
/// are the note's own small words, each beside the quote of its place (an empty one beside none); a row with one
/// sets its quotes close together. A row is drawn whole or not at all.
///
/// `name` is someone else's word the label names, such as a file's: it shares the label's line only when the
/// label and it lay out as that one line and it holds no blank, unseen or control character; else the label
/// stands alone and the name is the row's first quote. So no word of it is ever a line of the note's own.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToldRow {
    pub label: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
    pub quotes: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub marks: Vec<String>,
}

/// One line a person types in under a text note. `offer` is the few words over it that say what it is for, and
/// a tap on them gives the field the keys; a field with no `offer` has none. `hint` is shown while the field is
/// empty, and `most` is the most characters it takes.
///
/// `key` names the field apart from its note: the fields of two notes that carry the same key and the same
/// offer are one field, and what is typed in it is kept while no note of the scene carries it, and is there
/// when one does again. A field with no key is its note's: it is told by the note's id, and goes with the note.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Field {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub offer: String,
    pub hint: String,
    pub most: usize,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub key: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LegendItem {
    pub mark: Mark,
    pub text: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mark {
    Star,
    Busy,
    NeedsYou,
    Dimmed,
    /// No mark: a legend line that says something of every little one.
    Plain,
}
