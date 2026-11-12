//! Taking a copy back (`docs/development/run.md`, "A copy nothing names"): one no run was started in, by the
//! command that made it, and one a killed command left, by a look.

use std::fs;
use std::io::ErrorKind;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use super::git::{Git, Repo};
use super::record::{self, Listed, Record, read, write};
use super::{Places, branch, gone, hold, known, refused, standing, unlist};
use crate::Error;

/// How long a left copy that could not be taken back is kept for a later look to try: thirty days.
const PUT_OFF: Duration = Duration::from_secs(30 * 24 * 3600);

/// Undoes a copy no run was started in (F39: the harness was too old, or another run took the wish first): its
/// folder, git's registration of it, its branch, which is where the checkout was, and everything kept for the run.
pub(crate) fn take_back(home: &Path, run: &str, wait: Option<Duration>) -> Result<(), Error> {
    undo(home, run, wait)
}

/// [`take_back`] for a copy whose command was killed before any run was started in it (`run/left.rs`). Nothing
/// ran there, so its branch is where the checkout was, and goes, unless it is someone's by now ([`theirs`]).
///
/// What is taken back here was found, not made by the caller, so the repo its record names is asked first
/// (`record::asked`), and nothing is done when the run's start lock is held once the copies' lock is had: its
/// command lives.
/// - The repo lists the copy as its own: the copy's folder goes, a link or a file at its name as itself, then
///   the branch, then the repo's listing of the copy, then what is kept for the run.
/// - The repo answers and does not list it: the record was written by hand, or an earlier look got as far as
///   the listing. Nothing is asked of git there and no branch is removed; what the home holds of the run goes.
/// - The repo could not be asked, or a step failed: nothing more is done, and the record is kept for the next
///   look, with the time of the first that failed. After [`PUT_OFF`] only what the home holds of the run goes,
///   and the line returned says so: the one thing a look has to tell of a left copy.
pub(crate) fn take_back_left(home: &Path, run: &str, wait: Option<Duration>) -> Option<String> {
    let at = Places::of(home);
    let _held = hold::held(&at.lock(), false, wait).ok()?;
    let left = known(run).is_ok() && hold::let_go(home, run);
    if left && back(&at, run, true).is_err() { put_off(&at, run) } else { None }
}

fn undo(home: &Path, run: &str, wait: Option<Duration>) -> Result<(), Error> {
    known(run)?;
    let at = Places::of(home);
    let _held = hold::held(&at.lock(), false, wait).map_err(Error::Store)?;
    back(&at, run, false)
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
            let name = format!("refs/heads/{}", branch(run));
            let tip = repo.ask(None, &[], &["rev-parse", "--verify", "--quiet", &name]).map_err(refused)?;
            // The branch goes before the repo's listing of the copy does: the listing is how the next look knows
            // the copy as the repo's own, so a look cut off here is finished by the next, and no branch is left
            // that nothing but a record's word says is Jellymoon's.
            let goes = match &tip {
                Some(tip) if careful => !theirs(&repo, &record, &name, tip).map_err(refused)?,
                tip => tip.is_some(),
            };
            if goes {
                repo.run(None, &[], &["update-ref", "-d", &name]).map_err(refused)?;
            }
            if record.own.exists() {
                unlist(&repo, &folder).map_err(|said| refused(format!("git still lists the copy: {said}")))?;
            }
        }
    }
    gone::remove(&at.runs, run).map_err(Error::Store)
}

/// Whether a left copy's branch, which is at `tip`, is someone's by now, whoever made it so, and is kept:
/// - a checkout stands on it ([`stands`]): the repo's own, or a worktree of it other than the copy, whose own
///   `HEAD` is this branch because the copy was made so;
/// - another ref is a name for it (a symbolic ref), and would be left naming nothing;
/// - it has moved since the copy was made, and holds a commit of its own, one that no other ref of the repo
///   reaches. Every ref is named by `--glob`, since `--all` would count the copy's own `HEAD`. No `HEAD` is
///   asked: one that names a branch adds nothing to the refs, and a detached one is no ref, so a commit only
///   this branch and a detached `HEAD` hold is the branch's own.
///
/// A branch still at the commit its record says the copy was made at has nothing of its own, whatever the refs
/// say: a copy made of a checkout detached at a commit no ref has would else keep its branch for good. That
/// commit is a word from a file, and is asked last: it never undoes that a branch is stood on or named. It is
/// compared whole with what git printed for the branch, so nothing shorter than a full hash is equal to it.
fn theirs(repo: &Repo, record: &Record, name: &str, tip: &str) -> Result<bool, String> {
    let own = fs::canonicalize(&record.own).ok();
    let kept = match fs::read_dir(record.repo.join("worktrees")) {
        Ok(kept) => kept.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?,
        Err(e) if e.kind() == ErrorKind::NotFound => Vec::new(),
        Err(e) => return Err(e.to_string()),
    };
    let others = kept.iter().map(|kept| kept.path()).filter(|kept| kept.is_dir() && fs::canonicalize(kept).ok() != own);
    let mut stood_on = stands(repo, &record.repo, name)?;
    for other in others {
        stood_on |= stands(&repo.through(&other), &other, name)?;
    }
    let named = repo.run(None, &[], &["for-each-ref", "--format=%(symref)"])?;
    if stood_on || named.lines().any(|named| named == name) {
        return Ok(true);
    }
    if record.base.as_deref() == Some(tip) {
        return Ok(false);
    }
    let only = ["rev-list", "--max-count=1", name, "--not", &format!("--exclude={name}"), "--glob=refs/*"];
    Ok(!repo.run(None, &[], &only)?.is_empty())
}

/// Whether the checkout a git folder is of stands on a branch: its `HEAD` is that branch, or `HEAD` is detached
/// by a rebase or a bisect that goes back to it, as git says in the files it keeps for those, by the branch's
/// whole name or its short one. git itself refuses to delete such a branch.
fn stands(repo: &Repo, folder: &Path, name: &str) -> Result<bool, String> {
    let mut on = repo.ask(None, &[], &["symbolic-ref", "--quiet", "HEAD"])?.as_deref() == Some(name);
    for file in [&["rebase-merge", "head-name"][..], &["rebase-apply", "head-name"], &["BISECT_START"]] {
        let held = state(folder, file)?;
        on |= held.is_some_and(|held| held == name || name.strip_prefix("refs/heads/") == Some(&held));
    }
    Ok(on)
}

/// The line of a file git keeps in a git folder while something is going on there. None when it is not there.
/// Each name on the way is a real folder and the last a plain file no longer than a path, or the folder cannot
/// be asked: nothing is read through a link, which could lead out of the git folder.
fn state(folder: &Path, file: &[&str]) -> Result<Option<String>, String> {
    let mut at = folder.to_path_buf();
    for (nth, part) in file.iter().enumerate() {
        at.push(part);
        let meta = match fs::symlink_metadata(&at) {
            Ok(meta) => meta,
            Err(e) if e.kind() == ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.to_string()),
        };
        let last = nth + 1 == file.len();
        if !(if last { meta.is_file() && meta.len() <= 8192 } else { meta.is_dir() }) {
            return Err(format!("{} is not a file git keeps", at.display()));
        }
    }
    let held = fs::read_to_string(&at).map_err(|e| e.to_string())?;
    Ok(Some(held.trim_end().to_string()))
}

/// A left copy that could not be taken back at this look. The first time, the time is put in its record; a time
/// there that is later than now, written by hand or before a clock was set back, is taken as now and written so.
/// Once the time is [`PUT_OFF`] ago the repo is taken to be gone for good: what the home holds of the run goes,
/// nothing is done in a repo, and the line returned says that a branch may be left there.
fn put_off(at: &Places, run: &str) -> Option<String> {
    let mut record = read(&at.record(run)).ok()??;
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |since| since.as_nanos());
    let now = u64::try_from(now).unwrap_or(u64::MAX);
    let Some(first) = record.put_off.filter(|first| *first <= now) else {
        record.put_off = Some(now);
        let _ = write(&at.record(run), &record);
        return None;
    };
    (u128::from(now - first) > PUT_OFF.as_nanos()).then(|| {
        forget(at, run);
        format!(
            "the copy of run {run} could not be taken back for thirty days and is given up: what Jellymoon kept \
             of it is removed, and its branch {} may still be in its repo",
            branch(run)
        )
    })
}

/// What is left to do for a copy [`take_back`] could not undo, because git could not be asked (the repo was moved
/// away): the copy's folder and everything kept for the run go, so nothing of it stays in Jellymoon's home. Its
/// branch and git's registration of it are in the repo, wherever that is now, and are not reached. False: the lock
/// was not had, within `wait` when there is one, and nothing was removed.
pub(crate) fn remove_ours(home: &Path, run: &str, wait: Option<Duration>) -> bool {
    let at = Places::of(home);
    let held = hold::held(&at.lock(), false, wait);
    let ours = known(run).is_ok() && held.is_ok();
    if ours {
        forget(&at, run);
    }
    ours
}

/// Removes what the home holds of a run: its copy, when that is a real folder in a real `copies`, and `runs/<id>`.
fn forget(at: &Places, run: &str) {
    let folder = at.root.join(run);
    if standing(&at.root, &folder).is_ok_and(|there| there) {
        let _ = fs::remove_dir_all(&folder);
    }
    let _ = gone::remove(&at.runs, run);
}
