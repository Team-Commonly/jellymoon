//! The note of a change that was kept by itself (F92, ADR 0028; `docs/development/keep.md`, "Kept by itself"),
//! and the one choice under it. The record says that the change was kept; where it is now is asked of the repo
//! at every look (`undo::stands`), and the note claims no more than that answer: the folder has the change and
//! it can be undone, or has it under later commits, or stands somewhere else by now. Every word of it is here
//! or core's own sentence of why a change stays (`run/undo.rs`). `run waiting` prints the answer, not the words.

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::super::undo::{stands, undo_run};
use super::{RunOptions, Waits, WorkDone, read};
use crate::copy::Stands;

pub(super) const UNDO: &str = "Undo it";
pub(super) const NOT_UNDONE: &str = "The change was not undone.";
const TITLE: &str = "Your folder has this change now.";
const EARLIER: &str = "This change was put in your folder earlier.";
const PAST: &str = "It cannot be undone here any more: the branch has a commit since.";
const ELSEWHERE: &str = "Your folder stands somewhere else now, so the change is not undone from here.";
const UNREAD: &str = "Its details cannot be read now.";
const UNDONE: &str = "The change was undone.";
const AS_BEFORE: &str = "Your folder is as it was before the change, and the change waits for your yes.";

/// A change that was kept by itself: its work, read as work that waits is, and where it is now.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KeptItself {
    pub waits: Waits,
    pub stands: Stands,
}

impl KeptItself {
    /// The title of the note: that the folder has the change only while the repo says so.
    pub fn title(&self) -> &'static str {
        if self.stands == Stands::Elsewhere { EARLIER } else { TITLE }
    }

    /// How many files the change is, or that it cannot be read now; then why it is not undone from here.
    pub fn lines(&self) -> Vec<String> {
        let first = if self.waits.read { self.waits.counted() } else { UNREAD.to_string() };
        let then = match self.stands {
            Stands::Here => None,
            Stands::Past => Some(PAST),
            Stands::Elsewhere => Some(ELSEWHERE),
        };
        [first].into_iter().chain(then.map(str::to_string)).collect()
    }

    /// The one choice, only of a change the folder has at its branch's tip and whose files were read.
    pub fn choices(&self) -> Vec<&'static str> {
        if self.stands == Stands::Here && self.waits.read { vec![UNDO] } else { Vec::new() }
    }
}

/// The change of the wish's run, when it was kept by itself and was not undone. None of any other wish.
pub fn kept_itself(home: &Path, options: &RunOptions) -> Option<KeptItself> {
    let (waits, _) = read(home, options, true)?;
    Some(KeptItself { waits, stands: stands(home, options) })
}

/// A tap on `Undo it`: the undo of `run undo`, with its own checks. `then` is the sentence before core's reason.
pub(super) fn undo(home: &Path, options: &RunOptions, then: String) -> WorkDone {
    match undo_run(home, options, false) {
        Ok(_) => WorkDone::Said(UNDONE, vec![AS_BEFORE.to_string()]),
        Err(stays) => WorkDone::Said(NOT_UNDONE, vec![then, format!("{stays}.")]),
    }
}
