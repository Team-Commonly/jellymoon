//! Taking a copy back (`docs/development/run.md`, "A copy nothing names"): one no run was started in, by the
//! command that made it, and one a killed command left, by a look.

use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use super::git::{Git, Repo};
use super::record::{self, Listed, read, write};
use super::{Places, branch, gone, hold, known, refused, standing, unlist};
use crate::Error;

/// How long a left copy that could not be taken back is kept for a later look to try: thirty days.
const PUT_OFF: Duration = Duration::from_secs(30 * 24 * 3600);

/// Undoes a copy no run was started in (F39: the harness was too old, or another run took the wish first): its
/// folder, git's registration of it, its branch, which is where the checkout was, and everything kept for the run.
pub(crate) fn take_back(home: &Path, run: &str, wait: Option<Duration>) -> Result<(), Error> {
    undo(home, run, wait, false)
}

/// [`take_back`] for a copy whose command was killed before any run was started in it (`run/left.rs`). Nothing
/// ran there, so its branch is where the checkout was, and goes. A branch that holds a commit no other ref of the
/// repo and not its checkout has is someone's work, whoever made it: it is kept.
///
/// What is taken back here was found, not made by the caller, so the repo its record names is asked first
/// (`record::asked`), and nothing is done when the run's start lock is held once the copies' lock is had: its
/// command lives.
/// - The repo lists the copy as its own: the copy's folder goes, a link or a file at its name as itself, then
///   the branch, then the repo's listing of the copy, then what is kept for the run.
/// - The repo answers and does not list it: the record was written by hand, or an earlier look got as far as
///   the listing. Nothing is asked of git there and no branch is removed; what the home holds of the run goes.
/// - The repo could not be asked, or a step failed: nothing more is done, and the record is kept for the next
///   look, with the time of the first that failed. After [`PUT_OFF`] only what the home holds of the run goes.
pub(crate) fn take_back_left(home: &Path, run: &str, wait: Option<Duration>) -> Result<(), Error> {
    undo(home, run, wait, true)
}

fn undo(home: &Path, run: &str, wait: Option<Duration>, careful: bool) -> Result<(), Error> {
    known(run)?;
    let at = Places::of(home);
    let _held = hold::held(&at.lock(), false, wait).map_err(Error::Store)?;
    if careful && !hold::let_go(home, run) {
        return Ok(());
    }
    let done = back(&at, run, careful);
    if careful && done.is_err() {
        put_off(&at, run);
    }
    done
}

fn back(at: &Places, run: &str, careful: bool) -> Result<(), Error> {
    if let Some(record) = read(&at.record(run))? {
        let folder = at.root.join(run);
        let listed = if careful { record::asked(&record, &at.root, run) } else { Listed::Ours };
        if listed == Listed::Unasked {
            return Err(refused("the repo of that run's copy could not be asked; the copy is kept for the next look"));
        }
        let there = match standing(&at.root, &folder) {
            Err(_) if careful && gone::unlink(&at.root, &folder) => false,
            there => there?,
        };
        if there {
            fs::remove_dir_all(&folder).map_err(Error::Store)?;
        }
        if listed == Listed::Ours {
            let git = Git::find(&at.runs, record.found).map_err(refused)?;
            let repo = git.open(&record.repo).map_err(refused)?;
            unbranch(&repo, run, careful)?;
            if record.own.exists() {
                unlist(&repo, &folder).map_err(|said| refused(format!("git still lists the copy: {said}")))?;
            }
        }
    }
    gone::remove(&at.runs, run).map_err(Error::Store)
}

/// Removes a copy's branch, if it is there. It goes before the repo's listing of the copy does: the listing is
/// how the next look knows the copy as the repo's own, so a look cut off here is finished by the next, and no
/// branch is left that nothing but a record's word says is Jellymoon's. When `careful`, a branch that holds a
/// commit of its own is kept: one that no other ref of the repo reaches, nor its checkout. Every ref is named by
/// `--glob`, since `--all` would count the copy's own `HEAD`, which is this branch.
fn unbranch(repo: &Repo, run: &str, careful: bool) -> Result<(), Error> {
    let name = format!("refs/heads/{}", branch(run));
    if repo.ask(None, &[], &["rev-parse", "--verify", "--quiet", &name]).map_err(refused)?.is_none() {
        return Ok(());
    }
    let only = ["rev-list", "--max-count=1", &name, "--not", &format!("--exclude={name}"), "--glob=refs/*", "HEAD"];
    if !careful || repo.run(None, &[], &only).map_err(refused)?.is_empty() {
        repo.run(None, &[], &["update-ref", "-d", &name]).map_err(refused)?;
    }
    Ok(())
}

/// A left copy that could not be taken back at this look. The first time, the time is put in its record. Once
/// that is [`PUT_OFF`] ago the repo is taken to be gone for good: the copy's folder and what is kept for the run
/// go, and nothing is done in a repo.
fn put_off(at: &Places, run: &str) {
    let Ok(Some(mut record)) = read(&at.record(run)) else { return };
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |since| since.as_nanos());
    let now = u64::try_from(now).unwrap_or(u64::MAX);
    match record.unasked {
        None => {
            record.unasked = Some(now);
            let _ = write(&at.record(run), &record);
        }
        Some(first) if u128::from(now.saturating_sub(first)) > PUT_OFF.as_nanos() => {
            let folder = at.root.join(run);
            if standing(&at.root, &folder).is_ok_and(|there| there) {
                let _ = fs::remove_dir_all(&folder);
            }
            let _ = gone::remove(&at.runs, run);
        }
        Some(_) => {}
    }
}

/// What is left to do for a copy [`take_back`] could not undo, because git could not be asked (the repo was moved
/// away): the copy's folder and everything kept for the run go, so nothing of it stays in Jellymoon's home. Its
/// branch and git's registration of it are in the repo, wherever that is now, and are not reached. False: the lock
/// was not had, within `wait` when there is one, and nothing was removed.
pub(crate) fn remove_ours(home: &Path, run: &str, wait: Option<Duration>) -> bool {
    let at = Places::of(home);
    let held = hold::held(&at.lock(), false, wait);
    if known(run).is_err() || held.is_err() {
        return false;
    }
    let folder = at.root.join(run);
    if standing(&at.root, &folder).is_ok_and(|there| there) {
        let _ = fs::remove_dir_all(&folder);
    }
    let _ = gone::remove(&at.runs, run);
    true
}
