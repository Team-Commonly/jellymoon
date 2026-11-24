//! `jellymoon run undo` (F92, ADR 0028; `docs/development/keep.md`, "Undone"): a change that was kept by itself
//! is taken out of the person's folder again, and its work waits as it did before. What is done in the repo is
//! `copy/keep/undo.rs`. Here are the record's part and every word that is said: one fixed sentence for each
//! reason a change stays, with no word of git's or of the repo's, and no run's id, so a page says them as they are.

use std::fmt;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::keep::{BUSY, NO_LOCK, intent};
use super::show::{Asked, found, not_cores};
use super::{RunOptions, WorkState, place, record, refused};
use crate::Error;
use crate::copy::{self, Back, Keep, Stands, Stays, Unkept};

const UNDONE: Asked = Asked { to: "undo", done: "undone", going: "is going: there is nothing of it to undo yet" };

const NOTHING: &str = "nothing of this wish is kept, so there is no change to undo";
const TAPPED: &str = "this change was kept by a tap, or was kept by itself before a later day's change was, and such \
                      a change is not undone here: nothing was changed";
const COMMITTED: &str = "the branch has a commit since this change was kept, so it was not undone, and nothing was \
                         changed";
const GONE: &str = "the branch no longer holds this change, so nothing was undone, and nothing was changed";
const FORGED: &str = "the record of this change is not one Jellymoon wrote, so it was not undone, and nothing was \
                      changed";
const LATER: &str = "a later change was kept on top of this one, and the newest kept change is undone first: nothing \
                     was changed";
const EDITS: &str = "a file of this change has unsaved or staged edits, so it was not undone, and nothing was changed";
const WAY: &str = "a file is in the way where one this change removed would come back, so it was not undone, and \
                   nothing was changed";
const AWAY: &str = "the folder stands on another branch than the change was kept on, or on no branch, so it was not \
                    undone, and nothing was changed";
const MIDWAY: &str = "a merge, a cherry-pick, a revert, a rebase or a bisect is going on in the folder, so the \
                      change was not undone, and nothing was changed";
const HELD: &str = "another name in the repo holds this change (a branch, a tag or a remote's branch), and a change \
                    that was already sent elsewhere is not undone here: nothing was changed";
const STEPPED: &str = "git stopped the undo part way: files of the change may already be as they were before, and \
                       the next `run undo` goes on from there";
const UNWRITTEN: &str = "the run's record could not be written: the next `run undo` goes on from where this one \
                         stopped";
const UNREAD: &str = "the repo or the folder could not be read as an undo needs it, so the change was not undone, and \
                      nothing was changed";
const UNCHANGED: &str = "the change was not undone, and nothing was changed";

/// What `jellymoon run undo` prints.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Undone {
    pub run: String,
    pub wish: String,
    /// What was done, or with `--ask` what would be or why not, in core's fixed words.
    pub why: String,
    /// With `--ask` only: whether an undo would be done.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fits: Option<bool>,
}

impl fmt::Display for Undone {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&serde_json::to_string(self).map_err(|_| fmt::Error)?)
    }
}

/// Undoes the keep of the wish's change, or with `ask` says whether an undo would be done. Only a change that
/// was kept by itself is undone, and only while the person's branch still names the commit that keep landed.
/// Refused as a look is, and for each reason above, with nothing changed; with `ask` such a reason is printed.
pub fn undo_run(home: &Path, options: &RunOptions, ask: bool) -> Result<Undone, Error> {
    let alone = copy::alone(home);
    let found = found(home, options, &UNDONE)?;
    let (run, wish, mut record) = (found.run.clone(), options.wish.clone(), found.record.clone());
    let said =
        |why: String, fits: bool| Undone { run: run.clone(), wish: wish.clone(), why, fits: ask.then_some(fits) };
    let no = |why: &str| if ask { Ok(said(why.to_string(), false)) } else { Err(refused(why.to_string())) };
    if record.work != Some(WorkState::Kept) {
        return no(NOTHING);
    }
    let was = intent(&found.record.kept, &found.record.onto).map_err(|()| not_cores(&run))?;
    let before = record.before.clone().filter(|before| copy::is_id(before));
    let (Some((kept, onto)), Some(before), Some(named)) = (was, before, found.named()?) else { return no(TAPPED) };
    let _alone = match alone {
        Ok(alone) => alone,
        Err(Unkept::Busy) => return no(&format!("{BUSY}: {UNCHANGED}")),
        Err(_) => return no(&format!("{NO_LOCK}: {UNCHANGED}")),
    };
    let Ok(folder) = place::folder(home, &found.day, &found.made, options) else { return no(UNREAD) };
    let (again, itself) = (record.back.is_some(), true);
    let of = Keep { home, folder: &folder, run: &run, named: &named, was: None, ask, held: None, itself };
    let mut noted = record.clone();
    let mut note = || {
        noted.back = Some(before.clone());
        record::write(home, &run, &noted).is_ok()
    };
    let undone = copy::undo(&of, &Back { kept, before: &before, onto, again }, &mut note);
    match undone {
        Ok(()) if ask => Ok(said("an undo would put the branch back at the commit it named before".to_string(), true)),
        Ok(()) => {
            (record.work, record.kept, record.onto, record.before, record.back) = (None, None, None, None, None);
            if record::write(home, &run, &record).is_err() {
                return no(UNWRITTEN);
            }
            Ok(said(
                "the change was undone: the branch names the commit it named before, and the work waits".into(),
                true,
            ))
        }
        Err(stays) => {
            // An undo that goes on after one that was cut off, and finds nothing to go on with, begins no third.
            if again && !ask && !matches!(stays, Stays::Stepped | Stays::Unwritten) {
                record.back = None;
                let _ = record::write(home, &run, &record);
            }
            no(why(home, &found.record, &stays))
        }
    }
}

/// Where the wish's change that was kept by itself is now, for a page: asked of the repo at every look, and
/// [`Stands::Elsewhere`] of whatever cannot be asked.
pub(super) fn stands(home: &Path, options: &RunOptions) -> Stands {
    let asked = || {
        let found = found(home, options, &UNDONE).ok()?;
        let (kept, onto) = intent(&found.record.kept, &found.record.onto).ok()??;
        let before = found.record.before.clone().filter(|before| copy::is_id(before))?;
        let (named, run) = (found.named().ok()??, found.run.as_str());
        let folder = place::folder(home, &found.day, &found.made, options).ok()?;
        let of = Keep { home, folder: &folder, run, named: &named, was: None, ask: true, held: None, itself: true };
        Some(copy::stands_now(&of, &Back { kept, before: &before, onto, again: false }))
    };
    asked().unwrap_or(Stands::Elsewhere)
}

/// The sentence of a change that stays. Of a branch that moved: whether what moved it is a later change that
/// was kept by itself on top of this one, by the records of the home.
fn why(home: &Path, record: &record::Record, stays: &Stays) -> &'static str {
    let on_top = |(_, other): &(String, record::Record)| {
        other.work == Some(WorkState::Kept)
            && other.before.is_some()
            && other.before == record.kept
            && other.repo == record.repo
    };
    match stays {
        Stays::Moved if record::all(home).iter().any(on_top) => LATER,
        Stays::Moved => COMMITTED,
        Stays::Gone => GONE,
        Stays::Forged => FORGED,
        Stays::Away => AWAY,
        Stays::Midway => MIDWAY,
        Stays::Held => HELD,
        Stays::Way => WAY,
        Stays::Edits => EDITS,
        Stays::Stepped => STEPPED,
        Stays::Unwritten => UNWRITTEN,
        Stays::Unread => UNREAD,
    }
}
