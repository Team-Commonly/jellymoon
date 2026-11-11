//! A copy's end (`docs/development/copy.md`): a completed run's files are committed onto its branch and its folder
//! is removed; a failed run's copy is kept for a look, the newest [`KEPT`] only.

use std::fs;
use std::io::ErrorKind;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use super::git::{Git, Repo};
use super::record::{Record, forget, read, write};
use super::{JELLYMOON, KEPT, Outcome, Places, RunCopy, branch, gone, hold, known, refused, standing};
use crate::Error;

/// Ends a run's copy. Completed: what is in the copy is committed onto the run's branch, the folder is removed and
/// the branch is kept. Failed: the copy stays, and of all failed copies only the newest [`KEPT`] do.
pub fn clean_copy(home: &Path, run: &str, outcome: Outcome) -> Result<RunCopy, Error> {
    end_copy(home, run, outcome, None).map(|(copy, _)| copy)
}

/// [`clean_copy`], which also says whether a completed run's commit was made: false when the copy held nothing new.
pub(crate) fn end_copy(
    home: &Path,
    run: &str,
    outcome: Outcome,
    wait: Option<Duration>,
) -> Result<(RunCopy, bool), Error> {
    known(run)?;
    let at = Places::of(home);
    let file = at.record(run);
    let none = || refused(format!("no copy is kept for run {run}"));
    // One clean at a time, of any run: a second call waits here, then finds what the first left (`hold.rs`).
    // The file is made with the first copy, so a clean that is refused writes nothing.
    let held = hold::held(&at.lock(), false, wait);
    let _held = held.map_err(|e| if e.kind() == ErrorKind::NotFound { none() } else { Error::Store(e) })?;
    sweep(&at);
    let mut record = read(&file)?.ok_or_else(none)?;
    if record.failed.is_some() {
        return Err(refused(format!("run {run} has ended: its copy is one of the failed copies kept for a look")));
    }
    if outcome == Outcome::Failed {
        // Later than every run that failed before it, whatever the clock says: the records hold the order.
        let now = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |since| since.as_nanos());
        let after = failed(&at).iter().map(|before| before.0.saturating_add(1)).max().unwrap_or(0);
        record.failed = Some(u64::try_from(now).unwrap_or(u64::MAX).max(after));
        write(&file, &record)?;
        prune(&at);
        return Ok((at.printed(run, at.root.join(run).exists()), false));
    }
    let folder = at.root.join(run);
    let there = standing(&at.root, &folder)?;
    let git = Git::find(&at.runs, record.found).map_err(refused)?;
    let made = there && commit(&git.open(&record.own).map_err(refused)?, &folder, run)?;
    gone(&git, &record, &folder, there)?;
    forget(&file);
    Ok((at.printed(run, false), made))
}

/// Removes what a git stopped at its limit, or a child of it, wrote after its copy was taken back: a real folder in
/// `copies` under a run id that has no folder in `runs`. A copy being made has one from before its folder is there.
fn sweep(at: &Places) {
    let names = fs::read_dir(&at.root).into_iter().flatten().flatten();
    let names = names.filter_map(|entry| entry.file_name().into_string().ok()).filter(|run| known(run).is_ok());
    for run in names.filter(|run| fs::symlink_metadata(at.runs.join(run)).is_err()) {
        if standing(&at.root, &at.root.join(&run)).is_ok_and(|there| there) {
            let _ = fs::remove_dir_all(at.root.join(&run));
        }
    }
}

/// Commits what is in the copy onto the run's branch, with git told the copy's git folder and its work tree. The
/// paths that hold another repo are left out: adding one makes git start a second git inside that folder, under a
/// config the agent could have written. No hook can start: the tree and the commit are written with git's plumbing,
/// which has none, and the one hook of a ref update is off with the rest (`git.rs`). False: there was nothing new.
fn commit(repo: &Repo, folder: &Path, run: &str) -> Result<bool, Error> {
    let failed = |said: String| refused(format!("git could not commit the copy, which is kept: {said}"));
    let git = |tree: Option<&Path>, vars: &[(&str, &str)], args: &[&str]| repo.run(tree, vars, args).map_err(failed);
    let staged = git(None, &[], &["ls-files", "-z", "--stage"])?;
    let inner = staged.split('\0').filter_map(|entry| entry.strip_prefix("160000 ")?.split_once('\t'));
    let inner: Vec<String> = inner.map(|(_, path)| format!(":(top,exclude,literal){path}")).collect();
    let add = ["add", "-A", "--", ":(top)"].into_iter().chain(inner.iter().map(String::as_str));
    git(Some(folder), &[], &add.collect::<Vec<_>>())?;
    let tree = git(None, &[], &["write-tree"])?;
    let name = format!("refs/heads/{}", branch(run));
    let tip = git(None, &[], &["rev-parse", "--verify", &format!("{name}^{{commit}}")])?;
    if git(None, &[], &["rev-parse", "--verify", &format!("{tip}^{{tree}}")])? == tree {
        return Ok(false);
    }
    // The person's own name and address, as git reads them for this repo; Jellymoon's when either is missing.
    let own = |key: &str| repo.ask(None, &[], &["config", "--get", key]).map_err(failed);
    let (name_is, address) = match (own("user.name")?, own("user.email")?) {
        (Some(name_is), Some(address)) if !name_is.is_empty() && !address.is_empty() => (name_is, address),
        _ => (JELLYMOON.0.to_string(), JELLYMOON.1.to_string()),
    };
    let who = [
        ("GIT_AUTHOR_NAME", name_is.as_str()),
        ("GIT_AUTHOR_EMAIL", address.as_str()),
        ("GIT_COMMITTER_NAME", name_is.as_str()),
        ("GIT_COMMITTER_EMAIL", address.as_str()),
    ];
    let made = git(None, &who, &["commit-tree", "-p", &tip, "-m", &format!("Jellymoon run {run}"), &tree])?;
    git(None, &[], &["update-ref", &name, &made, &tip]).map(|_| true)
}

/// Removes every failed copy but the newest [`KEPT`], with its registration. What cannot be removed now (a link
/// where the copy was, a repo that will not answer) stays on the books and is tried at the next failed run.
fn prune(at: &Places) {
    let mut failed = failed(at);
    failed.sort_by(|a, b| (b.0, &b.1).cmp(&(a.0, &a.1)));
    let Some(git) = Git::find(&at.runs, true).ok().filter(|_| failed.len() > KEPT) else { return };
    for (_, run, record) in failed.into_iter().skip(KEPT) {
        let folder = at.root.join(&run);
        let removed = standing(&at.root, &folder).and_then(|there| gone(&git, &record, &folder, there));
        if removed.is_ok() {
            forget(&at.record(&run));
        }
    }
}

/// Every failed run on the books: when it failed, its id and its record.
fn failed(at: &Places) -> Vec<(u64, String, Record)> {
    let runs = fs::read_dir(&at.runs).into_iter().flatten().flatten();
    let runs = runs.filter_map(|entry| entry.file_name().into_string().ok()).filter(|run| known(run).is_ok());
    let failed = |run: String| {
        let record = read(&at.record(&run)).ok()??;
        Some((record.failed?, run, record))
    };
    runs.filter_map(failed).collect()
}
