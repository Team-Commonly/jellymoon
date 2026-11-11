//! The wire shapes: what `jellymoon day`, `page`, `rule` and `hook install` print, and the journal app's shell reads. These types are
//! the contract. A field's place here is its place on the wire; a missing field, a wrong type and an unknown state or
//! tool are refused when one is read.

use std::fmt;

use serde::{Deserialize, Serialize};

/// One local calendar day of what a person typed to their agents: the reader's whole output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReaderDay {
    /// The local calendar day, as YYYY-MM-DD.
    pub day: String,
    /// The day's turns, oldest first.
    pub turns: Vec<ReaderTurn>,
}

/// One thing a person typed to an agent, with the agent's last text reply before the person typed again.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReaderTurn {
    /// The local time the person typed it, as HH:MM on a 24-hour clock.
    pub at: String,
    pub tool: Harness,
    /// The last segment of the folder the session ran in; inside a harness worktree of a repo, the repo folder's name.
    pub project: String,
    /// What the person typed, at most 700 characters.
    pub said: String,
    /// The agent's last text reply in the turn, at most 500 characters. Empty when it has not replied in text.
    pub reply: String,
}

/// Which agent program a turn was typed to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Harness {
    #[serde(rename = "claude-code")]
    ClaudeCode,
}

/// One local day's page as data: the whole output of `jellymoon page`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageData {
    /// The local calendar day, as YYYY-MM-DD.
    pub day: String,
    /// At most 8.
    pub wishes: Vec<Wish>,
    /// At most 2.
    pub clashes: Vec<Clash>,
    pub read: PageRead,
}

/// Something a person wants to exist or be true, worked out from what they typed to their agents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Wish {
    /// The wish's id on this page: w1 to w8, in page order. A made wish's is m1 to m8 (`wish.rs`).
    pub id: String,
    /// A short name for the wish, in the model's words, at most 40 characters.
    pub label: String,
    /// The person's own words for it, copied from one thing they typed that day.
    pub said: String,
    /// The project the wish came from: one of the page's `read.projects`, or the folder a made wish names.
    pub project: String,
    pub state: WishState,
    /// One short text for each finished job an agent reported for the wish, at most 4.
    pub stars: Vec<String>,
}

/// How a wish stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WishState {
    /// In progress, and the answer when unsure.
    Working,
    /// The person got it.
    Done,
    /// An agent waits on the person.
    NeedsYou,
    /// The person dropped or replaced it.
    LetGo,
}

/// Two things the person said that cannot both stand.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Clash {
    /// The id of the wish the first statement belongs to.
    pub a: String,
    /// The id of the wish the second statement belongs to.
    pub b: String,
    /// The person's own words for the first statement.
    pub quote_a: String,
    /// The person's own words for the second statement.
    pub quote_b: String,
    /// True when the later statement already replaced the earlier one, so nothing is left to rule on.
    pub settled: bool,
}

/// What a page was made from, so the page can say so.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageRead {
    /// How many turns the reader collected for the day.
    pub turns: usize,
    /// The projects those turns ran in, each once, in the order first typed in.
    pub projects: Vec<String>,
}

/// The person's choice on a clash, as it is kept and as `jellymoon rule` prints it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ruling {
    /// When it was made, in milliseconds since 1970, as digits: its name in the store and its place in the order.
    pub id: String,
    /// The statement that stands, in the person's own words, at most 200 characters.
    pub stands: String,
    /// The statement it replaces, in the person's own words, at most 200 characters.
    pub over: String,
}

/// What `jellymoon hook install` and `jellymoon hook uninstall` print.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct HookState {
    /// Whether Jellymoon's entry is in the harness settings now.
    pub installed: bool,
    /// Whether this command changed the settings file.
    pub changed: bool,
}

/// A day prints as its JSON on one line, with no line break: what `jellymoon day` prints and what the model reads.
impl fmt::Display for ReaderDay {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        json(self, f)
    }
}

/// A page prints as its JSON on one line, with no line break: what `jellymoon page` prints.
impl fmt::Display for PageData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        json(self, f)
    }
}

fn json<T: Serialize>(value: &T, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(&serde_json::to_string(value).map_err(|_| fmt::Error)?)
}

/// A ruling prints as its JSON on one line, with no line break: what `jellymoon rule` prints and what is kept.
impl fmt::Display for Ruling {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        json(self, f)
    }
}

/// What a hook command did prints as its JSON on one line, with no line break.
impl fmt::Display for HookState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        json(self, f)
    }
}
