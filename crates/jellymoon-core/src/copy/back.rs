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
            let there = repo.ask(None, &[], &["rev-parse", "--verify", "--quiet", &name]).map_err(refused)?.is_some();
            // The branch goes before the repo's listing of the copy does: the listing is how the next look knows
            // the copy as the repo's own, so a look cut off here is finished by the next, and no branch is left
            // that nothing but a record's word says is Jellymoon's.
            if there && !(careful && theirs(&repo, &record, &name).map_err(refused)?) {
                repo.run(None, &[], &["update-ref", "-d", &name]).map_err(refused)?;
            }
            if record.own.exists() {
                unlist(&repo, &folder).map_err(|said| refused(format!("git still lists the copy: {said}")))?;
            }
        }
    }
    gone::remove(&at.runs, run).map_err(Error::Store)
}

/// Whether a left copy's branch is someone's by now, whoever made it so, and is kept:
/// - a checkout stands on it: `HEAD` is this branch in the repo's own checkout or in a worktree of it other than
///   the copy, whose own `HEAD` is this branch because the copy was made so;
/// - another ref is a name for it (a symbolic ref), and would be left naming nothing;
/// - it holds a commit of its own, one that no other ref of the repo reaches, nor its checkout. Every ref is
///   named by `--glob`, since `--all` would count the copy's own `HEAD`. The checkout's `HEAD` is left out when
///   it names no commit, as on a branch with none yet: git fails on such a name.
fn theirs(repo: &Repo, record: &Record, name: &str) -> Result<bool, String> {
    let on =
        |repo: &Repo| repo.ask(None, &[], &["symbolic-ref", "--quiet", "HEAD"]).map(|on| on.as_deref() == Some(name));
    let own = fs::canonicalize(&record.own).ok();
    let kept = match fs::read_dir(record.repo.join("worktrees")) {
        Ok(kept) => kept.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?,
        Err(e) if e.kind() == ErrorKind::NotFound => Vec::new(),
        Err(e) => return Err(e.to_string()),
    };
    let others = kept.iter().map(|kept| kept.path()).filter(|kept| kept.is_dir() && fs::canonicalize(kept).ok() != own);
    let mut stood_on = on(repo)?;
    for other in others {
        stood_on |= on(&repo.through(&other))?;
    }
    let named = repo.run(None, &[], &["for-each-ref", "--format=%(symref)"])?;
    if stood_on || named.lines().any(|named| named == name) {
        return Ok(true);
    }
    let head = repo.ask(None, &[], &["rev-parse", "--verify", "--quiet", "HEAD^{commit}"])?.map(|_| "HEAD");
    let only = ["rev-list", "--max-count=1", name, "--not", &format!("--exclude={name}"), "--glob=refs/*"];
    let only: Vec<&str> = only.into_iter().chain(head).collect();
    Ok(!repo.run(None, &[], &only)?.is_empty())
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
