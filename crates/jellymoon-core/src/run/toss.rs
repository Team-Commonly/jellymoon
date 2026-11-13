//! `jellymoon run toss` (F46, ADR 0017, decision 7; `docs/development/toss.md`): the person refuses what a made
//! wish's run left. The run's record takes `tossed` first, then what the run left goes, then the wish is let go:
//! the branch of a run that finished, as the run's own name at the commit its record names (`copy/keep/toss.rs`),
//! and the kept copy of a run that did not finish, with its branch when that is still where the run started.
//! Which run is read from the home as a stop reads it (`stop.rs`); a run that is going is refused. Here are the
//! record's part and every word that is printed: none is git's, the agent's or the repo's.

use std::fmt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::end::{self, bury, held, of_another, recorded, soon};
use super::keep::{BUSY, NO_LOCK, intent};
use super::record::Record;
use super::show::{Asked, not_cores, unread_words};
use super::{RunOptions, RunOutcome, WorkState, finished, no_wish, place, record, refused};
use crate::busy::cut;
use crate::copy::{self, Branch, Keep, Left, Named, Unkept, Untossed};
use crate::wish::{self, MadeWish};
use crate::{Error, WishState};

const TOSSED: Asked =
    Asked { to: "toss", done: "tossed", going: "is going: it is stopped first, and nothing was tossed" };

/// Where a test's build ends a toss, as a power cut would (`busy::cut`): before the record is written (1), after
/// it (2), after the kept copy of a run that did not finish went (3), and before the wish is told (4).
const CUT: &str = "JELLYMOON_TEST_TOSS_CUT";

/// What `jellymoon run toss` prints.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tossed {
    pub run: String,
    pub wish: String,
    /// Whether this command changed anything. False only of a run that was tossed before, with nothing left to do.
    pub tossed: bool,
    /// The commit the record names for the toss: the one the run's branch was at, or was to be deleted at. None
    /// when the record names no commit.
    pub commit: Option<String>,
    /// What was done, in core's fixed words.
    pub why: String,
}

impl fmt::Display for Tossed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&serde_json::to_string(self).map_err(|_| fmt::Error)?)
    }
}

/// A made wish's run that has ended, as the home has it, and what its record names.
struct Ended {
    day: String,
    made: MadeWish,
    run: String,
    record: Record,
    /// The run finished; `work`, that it made its one commit.
    done: bool,
    work: bool,
    /// The repo, the start, and the commit the branch is deleted at: the end, or the start again.
    named: Option<(PathBuf, String, String)>,
    /// What a keep that was cut off wrote down.
    was: Option<(String, String)>,
}

/// What a toss did, or found done.
struct Did {
    wrote: bool,
    /// Whether the kept copy's folder went now. None of a run that finished, which has no copy.
    copy: Option<bool>,
    branch: Branch,
}

/// The line a wish that was let go is given.
fn let_go(run: &str) -> String {
    format!("run {run} was tossed, and the wish was let go")
}

/// Tosses what the wish's run left. Refused, with nothing changed: a wish that is not there, one with no run or
/// another wish's, a run that is going or has no end yet, a record that is not core's, work that was kept or that
/// a keep landed, and for work that waits every state of its branch but the run's own name at the record's
/// commit with no checkout on it, or no such branch at all. A run tossed before is finished or left as it is.
pub fn toss_run(home: &Path, options: &RunOptions) -> Result<Tossed, Error> {
    // Before the record is read: what a keep or a toss that ended a moment ago wrote is what this one goes by.
    let alone = copy::alone(home);
    let on = ended(home, options)?;
    let (run, id) = (on.run.clone(), options.wish.clone());
    let _alone = alone.map_err(|unkept| unheld(&run, unkept))?;
    let state = on.record.work.unwrap_or(WorkState::Waits);
    let commit = on.named.as_ref().map(|named| named.2.clone());
    let fits = match state {
        WorkState::Tossed => on.record.tossed == commit,
        _ => on.record.tossed.is_none() && (on.done || (state == WorkState::Waits && on.was.is_none())),
    };
    if !fits {
        return Err(not_cores(&run));
    }
    if state == WorkState::Kept {
        return Err(refused(format!("the work of run {run} was kept, so it is not tossed: nothing was changed")));
    }
    let (first, told) = (state == WorkState::Waits, on.made.state == WishState::LetGo);
    let (day, done, work) = (on.day.clone(), on.done, on.work);
    let did = act(home, options, on, first)?;
    cut(CUT, 4);
    if !told {
        let told = wish::change(home, &day, &id, end::wait(), |wish| {
            if wish.run.as_deref() != Some(run.as_str()) {
                return Err(refused("the wish no longer names this run"));
            }
            (wish.state, wish.why) = (WishState::LetGo, Some(let_go(&run)));
            Ok(())
        });
        if told.is_err() {
            return Err(refused(format!(
                "run {run} was tossed and its record says so, but its wish could not be told: the next `run toss` \
                 lets the wish go"
            )));
        }
    }
    let changed = did.wrote || !told || did.copy == Some(true) || did.branch == Branch::Deleted;
    let head = match (first, done, work) {
        _ if !changed => format!("run {run} was tossed before, and nothing was changed"),
        (false, ..) => format!("a toss of run {run} that was cut off was finished"),
        (_, true, true) => format!("the work of run {run} was tossed"),
        (_, true, false) => format!("run {run}, which changed nothing, was tossed"),
        (_, false, _) => format!("run {run}, which did not finish, was tossed"),
    };
    let why = words(&head, changed, &did);
    Ok(Tossed { run, wish: id, tossed: changed, commit, why })
}

/// Why the lock was not had, in a keep's words for it: another command holds it, or it could not be made or taken.
fn unheld(run: &str, unkept: Unkept) -> Error {
    let why = if unkept == Unkept::Busy { BUSY } else { NO_LOCK };
    refused(format!("{why}: nothing of run {run} was tossed"))
}

/// The sentence of a toss: what became of the copy and of the branch, after `head`.
fn words(head: &str, changed: bool, did: &Did) -> String {
    let left = match did.branch {
        Branch::Deleted => return format!("{head}: {}its branch was deleted, and the wish is let go", copy_words(did)),
        Branch::Gone if changed => "its branch was not in the repo",
        Branch::Gone => return head.to_string(),
        Branch::Left(Left::Stood) => "its branch is left, as a checkout stands on it",
        Branch::Left(Left::Symbolic) => "its branch is left, as it is a name for another branch by now",
        Branch::Left(Left::Moved) => "its branch is left, as it is not at the commit the record names for it",
        Branch::Left(Left::Raced) => "its branch is left, as it was changed while the toss was at work",
        Branch::Left(Left::Unasked) => "its branch is left, as the repo could not be found or asked",
        Branch::Left(Left::Unnamed) => "its branch is left, as the record names no commit for it",
    };
    if !changed {
        return format!("{head}: {left}");
    }
    format!("{head}: {}{left}, and the wish is let go", copy_words(did))
}

/// What became of the copy of a run that did not finish. Nothing is said of one for a run that finished.
fn copy_words(did: &Did) -> &'static str {
    match did.copy {
        Some(true) => "its kept copy was removed, ",
        Some(false) => "no copy of it was kept by now, ",
        None => "",
    }
}

/// Finds the wish's run, which must have ended, and reads what its record names. A run that nothing holds is
/// given its end first, as a stop and a page give it (`end::bury`).
fn ended(home: &Path, options: &RunOptions) -> Result<Ended, Error> {
    let day = wish::day_of(options.day.as_deref())?;
    let id = options.wish.as_str();
    let made = wish::kept_of(home, &day)?.into_iter().find(|made| made.id == id).ok_or_else(|| no_wish(&day))?;
    let made = bury(home, &day, made);
    let Some(run) = made.run.clone() else {
        return Err(refused(format!("wish {id} has had no run, so there is nothing to toss")));
    };
    let no = |why: &str| refused(format!("run {run} {why}"));
    let Some(record) = record::kept(home, &run) else {
        return Err(refused(format!("the record of run {run} cannot be read, so nothing was tossed")));
    };
    if of_another(home, &run, Some(&record), &day, id) {
        return Err(refused(format!("wish {id} names run {run}, which is another wish's run: nothing was tossed")));
    }
    if held(home, &run) {
        return Err(no(TOSSED.going));
    }
    if made.state == WishState::Working || record.outcome.is_none() {
        return Err(no("has no end written down yet: nothing was tossed"));
    }
    let (outcome, why) = recorded(home, &day, id, &run, &record).ok_or_else(|| not_cores(&run))?;
    let (done, work) = (outcome == RunOutcome::Completed, why == finished(&run, true));
    let named = if done { record.named(&run) } else { record.began_at() }.map_err(|()| not_cores(&run))?;
    let named = named.map(|named| (named.repo.to_path_buf(), named.start.to_string(), named.end.to_string()));
    // Of work the record says was kept nothing more is read: where a keep landed it is the keep's to say, and a
    // toss changes nothing for it whatever branch, or none, the record names.
    let kept = record.work == Some(WorkState::Kept);
    let was = if kept { None } else { intent(&record.kept, &record.onto).map_err(|()| not_cores(&run))? };
    let was = was.map(|(kept, onto)| (kept.to_string(), onto.to_string()));
    Ok(Ended { day, made, run, record, done, work, named, was })
}

/// The record's part and the repo's. Work that waits with a commit named for it is checked whole before the
/// record is written, inside `copy::toss`. Of anything else the record is written first: a run that did not
/// finish, once its copy's place is known to be one that can be removed, and a record that names no commit.
/// What follows the record is never a refusal of the toss: a branch that cannot be deleted as the run's own is left.
fn act(home: &Path, options: &RunOptions, on: Ended, first: bool) -> Result<Did, Error> {
    let Ended { day, made, run, mut record, done, named, was, .. } = on;
    let strict = first && done && named.is_some();
    let commit = named.as_ref().map(|named| named.2.clone());
    let mut wrote = false;
    let mut write = || {
        cut(CUT, 1);
        (record.work, record.tossed, record.kept, record.onto) = (Some(WorkState::Tossed), commit.clone(), None, None);
        wrote = record::write(home, &run, &record).is_ok();
        cut(CUT, 2);
        wrote
    };
    let kept = |after: &str| refused(format!("the kept copy of run {run} could not be removed: {after}"));
    if first && !strict {
        if !done {
            let nothing =
                "its place is not a folder Jellymoon made, or it could not be asked about, and nothing was tossed";
            copy::drop_copy(home, &run, soon(), true).map_err(|_| kept(nothing))?;
        }
        if !write() {
            return Err(refused(untossed_words(&run, Untossed::Unwritten)));
        }
    }
    let copy = if done {
        None
    } else {
        let next = "the record says tossed, and the next `run toss` goes on from there";
        let removed = copy::drop_copy(home, &run, soon(), false).map_err(|_| kept(next))?;
        cut(CUT, 3);
        Some(removed)
    };
    let branch = match named.as_ref().map(|named| (named, place::folder(home, &day, &made, options))) {
        None => Branch::Left(Left::Unnamed),
        Some((_, Err(e))) if strict => return Err(e),
        Some((_, Err(_))) => Branch::Left(Left::Unasked),
        Some(((repo, start, at), Ok(folder))) => {
            let named = Named { repo, start, end: at };
            let was = was.as_ref().map(|was| (was.0.as_str(), was.1.as_str()));
            let of = Keep { home, folder: &folder, run: &run, named: &named, was, ask: false };
            match copy::toss(&of, strict, &mut write) {
                Ok(branch) => branch,
                Err(untossed) if strict => return Err(refused(untossed_words(&run, untossed))),
                Err(_) => Branch::Left(Left::Unasked),
            }
        }
    };
    Ok(Did { wrote, copy, branch })
}

/// Why work that waits was not tossed: fixed words, with the run's id and nothing else.
fn untossed_words(run: &str, untossed: Untossed) -> String {
    match untossed {
        Untossed::Unread(unread) => unread_words(run, unread, &TOSSED),
        Untossed::Stood => {
            format!("a checkout stands on the branch of run {run}: its work was not tossed, and nothing was changed")
        }
        Untossed::Landed => format!(
            "a keep of run {run} that was cut off had landed its work, so it is kept and not tossed: `run keep` \
             finishes that keep, and nothing was changed"
        ),
        Untossed::Unwritten => format!("the record of run {run} could not be written: nothing was tossed"),
    }
}
