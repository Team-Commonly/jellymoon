//! The note of a change that was kept by itself, and the one choice under it (F92, ADR 0028;
//! `docs/development/keep.md`, "Kept by itself"): the folder has the change, how many files it is, and `Undo it`.
//! Every word of it is here or core's own sentence of why a change stays (`run/undo.rs`).

use std::path::Path;

use super::super::undo::undo_run;
use super::{RunOptions, Waits, WorkDone, read};

pub(super) const TITLE: &str = "Your folder has this change now.";
pub(super) const UNDO: &str = "Undo it";
pub(super) const NOT_UNDONE: &str = "The change was not undone.";
const UNDONE: &str = "The change was undone.";
const AS_BEFORE: &str = "Your folder is as it was before the change, and the change waits for your yes.";

/// The change of the wish's run, when it was kept by itself and can still be undone: read as work that waits
/// is ([`super::waits`]), and its note has the one choice. None of any other wish.
pub fn kept_itself(home: &Path, options: &RunOptions) -> Option<Waits> {
    read(home, options, true).map(|(kept, _)| kept)
}

/// A tap on `Undo it`: the undo of `run undo`, with its own checks. `then` is the sentence before core's reason.
pub(super) fn undo(home: &Path, options: &RunOptions, then: String) -> WorkDone {
    match undo_run(home, options, false) {
        Ok(_) => WorkDone::Said(UNDONE, vec![AS_BEFORE.to_string()]),
        Err(stays) => WorkDone::Said(NOT_UNDONE, vec![then, format!("{stays}.")]),
    }
}
