//! What a `jellymoon run` that was killed left behind (`docs/development/run.md`, "A copy nothing names"). A run
//! whose wish names it is written down by the look at that wish (`end::bury`). A command killed earlier, after
//! its copy was made and before its wish named the run, or later, between making a never-started run's wish as
//! it was and taking the copy back, leaves a copy, a branch and a folder in `runs` that no wish names: no look at
//! a wish ever finds them. [`tidy`] does, by the start lock every `jellymoon run` holds from before its copy is
//! made until it ends (`copy/hold.rs`): one that is there, with nothing holding it, is of a command that is gone.

use std::fs;
use std::path::Path;

use super::end::{held, mark, note, soon};
use super::record::{self, Record};
use super::{RunOutcome, cut, dead};
use crate::copy;
use crate::wish::{self, MadeWish};

/// Goes over the runs whose command was killed. `wishes` are the made wishes of the day being looked at.
///
/// The harness child such a run's record names is killed first, if the pid is still that process (`dead.rs`),
/// whichever wish names the run: so a `jellymoon run` that is about to count the runs going leaves no agent of a
/// dead command at work beside its own. Then, for a run no wish names:
/// - with no record, or one that names no child: nothing ever ran in the copy, and it is taken back, with its
///   branch unless that holds a commit of its own (`copy::take_back_left`);
/// - with a record that names a child and has no end: an agent ran in the copy, so it is kept as a cut-off run's
///   is, marked failed, and the record takes that end.
///
/// A run a wish names is left to the look at that wish, and so is one whose record is there and cannot be used.
/// A run that has its end is done with, and its start lock
/// is removed. Nothing is waited for longer than a look waits, and what could not be done is left for the next.
pub(crate) fn tidy(home: &Path, wishes: &[MadeWish]) {
    for run in copy::left(home) {
        // The command lets go of its start lock just before its run's own: a run still held is still ending.
        if held(home, &run) {
            continue;
        }
        let record = record::kept(home, &run);
        // A record that is there and cannot be used says nothing of who names the run: nothing is taken on it.
        if record.is_none() && fs::symlink_metadata(record::file(home, &run)).is_ok() {
            continue;
        }
        let ended =
            record.as_ref().is_some_and(|record| record.outcome.is_some_and(|end| end != RunOutcome::NotStarted));
        if ended {
            let _ = fs::remove_file(copy::start_lock(home, &run));
            continue;
        }
        let child = record.as_ref().and_then(|record| Some((record.pid?, record.mark.as_deref()?)));
        if let Some((pid, mark)) = child {
            dead::kill(pid, mark);
        }
        let named = |wish: &MadeWish| wish.run.as_deref() == Some(run.as_str());
        if wishes.iter().any(named) || record.as_ref().is_some_and(|record| names(home, record, &run)) {
            continue;
        }
        if record.as_ref().is_some_and(|record| record.pid.is_some()) {
            if mark(home, &run, soon()) {
                let why = cut(&run);
                let _ = note(home, &run, RunOutcome::Failed, (0, 0), &why);
            }
        } else {
            let _ = copy::take_back_left(home, &run, soon());
        }
    }
}

/// Whether the wish a record names, on the day it names, names the run. The day in a record is a word from a
/// file: it is a day, or no file is looked for.
pub(super) fn names(home: &Path, record: &Record, run: &str) -> bool {
    let theirs = wish::day_of(Some(&record.day)).and_then(|day| wish::kept_of(home, &day)).unwrap_or_default();
    theirs.iter().any(|theirs| theirs.id == record.wish && theirs.run.as_deref() == Some(run))
}
