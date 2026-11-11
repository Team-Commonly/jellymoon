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
