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
    /// With `whole`, every option is drawn to its last word, on as many lines as that takes; without it, an option
    /// is drawn in three lines at most.
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
    /// A text and the choices under it. Each of `body` starts on a line of its own and is wrapped, in at most
    /// [`crate::layout::MAX_TEXT_LINES`] lines; a last line counts the ones of `body` that are not drawn to their
    /// end. A tap on an option is the note's `id` and the option's place, as a choice's.
    ///
    /// `quoted` is someone else's words: drawn under the body on a ground of their own, in the tone for quoted
    /// words, cut to at most [`crate::layout::MAX_QUOTED_LINES`] lines, and never anything but text. `field` is
    /// one line a person types in under the note; what Enter gives carries the note's `id`. It stands under the
    /// options, or before them when it is first ([`Field::first`]).
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
    /// lines as that takes; a last line, `+N more`, counts the rows past the twelfth. No row is cut short.
    Whole {
        title: String,
        rows: Vec<(Mark, String)>,
    },
    /// What someone else says of a thing, under the note's own title: each row is a label in the note's own words
    /// and the other's texts under it, each a `Quote` piece on a ground of its own and never anything but text. A
    /// row is drawn to its last character or left out, and a last line, `+N more`, counts the rows left out. `end`
    /// is the note's own sentences after the rows, each drawn whole, or the note is left out. It has no option
    /// and no field.
    Told {
        id: String,
        title: String,
        rows: Vec<ToldRow>,
        #[serde(default)]
        end: Vec<String>,
    },
    /// What the person typed: a bubble at the right of the column, a `Quote` piece of this `id` and never anything
    /// but text. The character whose id is what follows the first `-` of `id` ([`speaker`]) is drawn small at
    /// the bubble's end, with its state and stars, and a tap on it or on the bubble is a tap on that character.
    /// `answer` is what the person answered to go on, when they did: more of their own words, in the same
    /// bubble on lines of their own under `text`, drawn by the same rules. They are set in behind a small hook
    /// the pencil draws, and no word tells the two apart. With an empty one the bubble is `text` alone.
    /// `small` is a picture drawn small under the bubble, at its right end: a sign, the picture's own captions
    /// on one line and a chevron (`layout/pictures/small.rs`). A tap on it is the tap on the bubble. With none,
    /// or with a picture that has no small form, the bubble is placed and drawn as if the word were not there.
    Said {
        id: String,
        text: String,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        answer: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        small: Option<Picture>,
    },
    /// A picture drawn in pencil, which draws itself when the note first comes onto a page. `options` are the
    /// choices under it, tapped as a text note's are. `words` are the note's own sentences: while `open` is
    /// false none of them is drawn and a small `?` stands at the picture's top right when there are any; with
    /// `open` each is drawn under the picture, on a line of its own, to its last word. A tap on the picture or
    /// on the `?` is the note's `id` at [`PICTURE_PLACE`]. The scene carries `open`; the kit keeps no state for it.
    /// A picture that takes no tap ([`Picture::taps`]) has no `?` either.
    /// `field` is a text note's: one line a person types in, under the options, open or closed.
    Drawn {
        id: String,
        picture: Picture,
        #[serde(default)]
        words: Vec<String>,
        #[serde(default)]
        open: bool,
        #[serde(default)]
        options: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        field: Option<Field>,
    },
}

impl Scene {
    /// The words of option `index` (from 0) of the first note whose id is `note`, as the scene holds them: what a
    /// tap on that option carries ([`crate::draw::Tap`]). None of a note with no such option.
    pub fn option(&self, note: &str, index: usize) -> Option<&str> {
        let options = self.notes.iter().find_map(|one| match one {
            Note::Choice { id, options, .. } | Note::Text { id, options, .. } | Note::Drawn { id, options, .. }
                if id == note =>
            {
                Some(options)
            }
            _ => None,
        })?;
        options.get(index).map(String::as_str)
    }
}

/// The place a tap on a drawn note's picture is reported at: the place of no option.
pub const PICTURE_PLACE: usize = usize::MAX;

/// What a drawn note shows (`layout/pictures`). Every count is drawn as marks, and every word in the theme's
/// handwriting.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Picture {
    /// A run at work: its steps so far as tally marks, what it is doing now, and the empty place its result
    /// will stand in. `file` is someone else's word: drawn by the rule a told row's `name` is, or left out.
    Track { steps: usize, stage: Stage, file: String },
    /// Work a run finished: a sheet for each file it made, changed or removed, and whether they are in the
    /// person's folder.
    Sheets { made: usize, changed: usize, removed: usize, state: Kept },
    /// Changed lines a file, to one scale. `more` counts the files that have no row.
    Bars { rows: Vec<Bar>, more: usize },
    /// A tally under a `title`: rows of a label and small marks, one mark for one thing ([`TallyRow`]), and a
    /// chevron that says a tap on the picture opens what the tally counts, turned while `open`. The rows past
    /// [`MAX_TALLY_ROWS`] are not drawn: `more` stands for them in one quiet line, or `+N more` when it is
    /// empty. `under` is one quiet line at its foot. A note with this picture stands at the column's top, over
    /// the small characters' rows, wherever it is among the scene's notes (`layout/column.rs`).
    Tally {
        title: String,
        rows: Vec<TallyRow>,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        more: String,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        under: String,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        open: bool,
        /// The tally stands where it is among the scene's notes, as any card does, and not at the column's
        /// top: with no rows it is one line, a title and its chevron, over what a tap on it opens.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        in_place: bool,
    },
    /// A strip: the first [`MAX_STRIP_PANELS`] panels in one row, left to right, each a small sketch with its
    /// caption under it, and an arrow from each to the next. It is a picture and nothing else: no part of it
    /// takes a tap, and its note has no `?` ([`Picture::taps`]). A note's words under it are drawn while the
    /// scene says the note is open, as under any picture.
    Strip { panels: Vec<Panel> },
}

impl Picture {
    /// Whether a tap on the picture is given to the page's handler. A strip takes none.
    pub fn taps(&self) -> bool {
        !matches!(self, Picture::Strip { .. })
    }
}

/// The most panels a [`Picture::Strip`] draws. The ones past them are not drawn.
pub const MAX_STRIP_PANELS: usize = 3;

/// One panel of a [`Picture::Strip`]: what its sketch shows, and a caption of a few words in the hand, cleaned
/// as every caption is and wrapped under the sketch in two lines at most.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Panel {
    pub shows: Shows,
    pub caption: String,
}

/// What the sketch of a [`Panel`] shows: a speech bubble with writing in it; a sheet with a pencil on it, the
/// tool at rest; a folder with a tick in it and a small cross beside it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Shows {
    #[default]
    Speech,
    Sheet,
    Folder,
}

/// The most rows a [`Picture::Tally`] draws, and the most marks of one row: the marks past them are a count.
pub const MAX_TALLY_ROWS: usize = 5;
pub const MAX_TALLY_MARKS: usize = 12;

/// One row of a [`Picture::Tally`]: a label, cleaned and cut in its middle as an entry's chip is, and its marks
/// in the order given. What each [`Mark`] means on a whole list's row it means here, drawn by the pencil: a
/// star, a ring with three dots, a ring with a `?`, and a cross in the quiet ink.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TallyRow {
    pub label: String,
    pub marks: Vec<Mark>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    #[default]
    Thinking,
    Looking,
    Making,
}

/// Where finished work is: in a copy still, in the person's folder, or thrown away. `Unfit` is work in a copy
/// that does not fit the folder as it is, so it cannot go in as it stands.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kept {
    #[default]
    Waiting,
    In,
    Tossed,
    Unfit,
}

/// One file's row of a [`Picture::Bars`]: its name, someone else's word, and the lines added and taken.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bar {
    pub name: String,
    pub added: usize,
    pub taken: usize,
}

/// The id of the character a [`Note::Said`] of this id is of: what follows the id's first `-`.
pub fn speaker(id: &str) -> &str {
    id.split_once('-').map_or("", |(_, who)| who)
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
///
/// `wants_keys` asks for the keys: the field takes them once, at the first drawing it stands on the page, unless
/// words are typed in the entry or where the keys are (`draw/field.rs`). A field that does not ask never does.
///
/// `first` puts the field of a text note before the note's options: directly under the quoted words, with the
/// options after it small and quiet, pills as wide as their words on one row (`layout/typed.rs`). Typing is then
/// the note's answer, and its options are the ways round it. A field that is not first stands under the options,
/// which are drawn wide. A drawn note's field is never first.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Field {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub offer: String,
    pub hint: String,
    pub most: usize,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub key: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub wants_keys: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub first: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mark {
    Star,
    Busy,
    NeedsYou,
    Dimmed,
}
