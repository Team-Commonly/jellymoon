//! The fixed lines a run keeps with its wish and in its record, and the two refusals every command of a run
//! shares. No word here is the agent's, a session log's or the repo's: a line names the run and nothing else.

use std::path::Path;

use super::refused;
use crate::{Error, copy};

pub(super) fn had(wish: &str, run: &str) -> Error {
    refused(format!("wish {wish} has had its run, {run}: a made wish gets one run"))
}

pub(super) fn going(run: &str) -> String {
    format!("run {run} is going, in a copy of the repo")
}

/// How the line of a run that did not finish ends when it is written, and how it ends when it is printed after
/// the copy has gone: a failed copy past the newest five is removed, and so is the copy of a run that was tossed.
const COPY_KEPT: &str = ": its copy is kept and no star was added";
const COPY_GONE: &str = ": its copy is no longer kept, and no star was added";

/// A run's line as it holds now: of a copy whose folder is not standing it is not said that it is kept.
pub(super) fn by_now(home: &Path, run: &str, why: String) -> String {
    match why.strip_suffix(COPY_KEPT).filter(|_| copy::no_folder(home, run)) {
        Some(head) => format!("{head}{COPY_GONE}"),
        None => why,
    }
}

pub(super) fn failed(run: &str, kind: &str) -> String {
    format!("run {run} did not finish ({kind}){COPY_KEPT}")
}

pub(super) fn cancelled(run: &str) -> String {
    format!("run {run} was cancelled by a stop{COPY_KEPT}")
}

/// The line of a run that asks. It says what holds of its copy and its branch for as long as it asks.
pub(super) fn asking(run: &str) -> String {
    format!("run {run} asks a question: its copy is kept as it is, and nothing was committed")
}

pub(super) fn no_wish(day: &str) -> Error {
    refused(format!("the page of {day} has no made wish of that id; a made wish's id is m1 to m8"))
}

pub(super) fn cut(run: &str) -> String {
    format!("run {run} was cut off before it ended{COPY_KEPT}")
}

/// The line of a run that was cut off with its copy gone: after it was committed and before the end was in the
/// record, or removed by hand, with nothing committed. It says no more than holds for both, and of the branch
/// only where the work was put: a branch can be removed by hand, and a look asks git nothing.
pub(super) fn cut_late(run: &str) -> String {
    let branch = copy::branch(run);
    format!(
        "run {run} was cut off before it ended: no copy of it is kept, what was committed of it was on the branch \
         {branch}, and no star was added"
    )
}

/// The line of a run that finished. Both forms say what git was last asked as the run ended (`end::settle`,
/// `copy/clean.rs`): the branch took the run's one commit, or was looked at once more, only while it still named
/// the commit the copy was made from, which is where the person's checkout was when the run started. Neither says
/// where the branch is after that check: until the copy's folder is removed, and ever after, a hand can move it,
/// and a look goes by what the repo says then (`show.rs`).
pub(super) fn finished(run: &str, changed: bool) -> String {
    let branch = copy::branch(run);
    if changed {
        format!("run {run} finished: its work is one commit on the branch {branch}")
    } else {
        format!("run {run} finished and changed nothing: the branch {branch} is where the checkout was")
    }
}
