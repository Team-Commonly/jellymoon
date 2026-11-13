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
    /// The first [`crate::entry::MAX_ENTRY_OPTIONS`] are offered, each on a chip.
    pub options: Vec<String>,
    /// The most characters the entry takes: at most [`crate::entry::MAX_ENTRY_CHARS`], which 0 also means.
    pub most: usize,
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
    Text {
        id: String,
        title: String,
        #[serde(default)]
        body: Vec<String>,
        #[serde(default)]
        options: Vec<String>,
    },
    /// A list of texts no word of which may be hidden. Each row is drawn after its mark to its last word, on as many
    /// lines as that takes, or it is left out; a last line, `+N more`, counts the rows left out. No row is cut short.
    Whole {
        title: String,
        rows: Vec<(Mark, String)>,
    },
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
}
