//! A run's own copy of the person's repo (ADR 0011, section 7; ADR 0014, decision 4; `docs/development/copy.md`): a
//! git worktree on a branch of its own, directly inside `copies` in Jellymoon's folder. [`make_copy`] makes one.
//! [`clean_copy`] ends one: a completed run's files are committed onto its branch and the folder is removed; a failed
//! run's copy is kept for a look, the newest [`KEPT`] only. [`take_back`] undoes one that no run was started in.
//!
//! An agent could have rewritten every file in its copy, the `.git` file too. So once a copy is made nothing in it is
//! read to decide anything: where the repo is comes from the record written when the copy was made, kept in `runs`,
//! outside the copy, and the folder and the branch come from the run id alone.

mod git;
mod record;

use std::collections::hash_map::RandomState;
use std::ffi::OsStr;
use std::fmt;
use std::fs::{self, OpenOptions};
use std::hash::{BuildHasher, Hasher};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use git::{Git, Repo};
use record::{Record, forget, read, write};
use serde::Serialize;

use crate::Error;
use crate::call::private_dir;

/// How many failed copies are kept for a look.
pub const KEPT: usize = 5;

/// Who a run's commit is by when the person's git config names nobody.
const JELLYMOON: (&str, &str) = ("Jellymoon", "jellymoon@localhost");

/// What `copy make` and `copy clean` print: the run, its folder, its branch, and whether the folder is still there.
#[derive(Debug, Serialize)]
pub struct RunCopy {
    pub run: String,
    pub folder: String,
    pub branch: String,
    pub kept: bool,
}

/// How a run ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Completed,
    Failed,
}

impl Outcome {
    pub fn named(name: &str) -> Option<Outcome> {
        match name {
            "completed" => Some(Outcome::Completed),
            "failed" => Some(Outcome::Failed),
            _ => None,
        }
    }
}

impl fmt::Display for RunCopy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&serde_json::to_string(self).map_err(|_| fmt::Error)?)
    }
}

/// The two folders in Jellymoon's own: `copies` holds each copy under its run id and nothing else, and `runs` holds
/// each run's record.
struct Places {
    root: PathBuf,
    runs: PathBuf,
}

impl Places {
    fn of(home: &Path) -> Places {
        Places { root: home.join("copies"), runs: home.join("runs") }
    }

    fn record(&self, run: &str) -> PathBuf {
        self.runs.join(run).join("copy.json")
    }

    /// The file a clean, and a copy being made, holds the system's lock on while it works.
    fn lock(&self) -> PathBuf {
        self.runs.join("clean.lock")
    }

    fn printed(&self, run: &str, kept: bool) -> RunCopy {
        let folder = self.root.join(run).to_string_lossy().into_owned();
        RunCopy { run: run.to_string(), folder, branch: branch(run), kept }
    }
}

fn refused(why: impl Into<String>) -> Error {
    Error::Copy(why.into())
}

pub(crate) fn branch(run: &str) -> String {
    format!("jellymoon/run-{run}")
}

/// A run id is `r` and sixteen of `0-9a-f`: a name with no dot or separator in it, so `copies/<id>` is always
/// directly inside `copies`. One a caller gives is taken only in that shape.
fn known(run: &str) -> Result<(), Error> {
    let why = "a run id is `r` and sixteen of 0-9 and a-f, as `copy make` prints it; nothing was done";
    if is_run_id(run) { Ok(()) } else { Err(refused(why)) }
}

pub(crate) fn is_run_id(run: &str) -> bool {
    let hex = |byte: u8| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte);
    run.strip_prefix('r').is_some_and(|rest| rest.len() == 16 && rest.bytes().all(hex))
}

/// Where a run's own files are kept, beside its copy's record. The id is one in a run id's shape.
pub(crate) fn kept_at(home: &Path, run: &str) -> PathBuf {
    Places::of(home).runs.join(run)
}

/// Takes a run id by making its folder in `runs`, which only one caller can: the one given, or a new one.
fn claim(runs: &Path, given: Option<&str>) -> Result<String, Error> {
    for _ in 0..8 {
        let fresh = || format!("r{:016x}", RandomState::new().build_hasher().finish());
        let run = given.map_or_else(fresh, str::to_string);
        match fs::create_dir(runs.join(&run)) {
            Ok(()) => return Ok(run),
            Err(e) if e.kind() != ErrorKind::AlreadyExists => return Err(Error::Store(e)),
            Err(_) if given.is_some() => return Err(refused(format!("run {run} has a copy already"))),
            Err(_) => {}
        }
    }
    Err(refused("no free run id was found"))
}

/// Makes a copy of the repo at `repo` for a run: a worktree of the commit the person's checkout is on, on the new
/// branch `jellymoon/run-<id>`. Nothing of the person's checkout, index or `HEAD` is written.
pub fn make_copy(home: &Path, repo: &Path, run: Option<&str>) -> Result<RunCopy, Error> {
    make(home, repo, run, false)
}

/// [`make_copy`] for a run. A folder that was not `typed` by the person was read from a session log: no refusal,
/// now or when the copy is ended, repeats what git says of it, since git's words name the folder.
pub(crate) fn copy_of(home: &Path, repo: &Path, typed: bool) -> Result<RunCopy, Error> {
    make(home, repo, None, !typed)
}

fn make(home: &Path, repo: &Path, run: Option<&str>, found: bool) -> Result<RunCopy, Error> {
    if !repo.is_absolute() {
        return Err(refused("--repo must be a full path, from the root"));
    }
    run.map_or(Ok(()), known)?;
    let at = Places::of(home);
    for folder in [&at.root, &at.runs] {
        private_dir(folder).map_err(Error::Store)?;
    }
    let git = Git::find(&at.runs, found).map_err(refused)?;
    let found =
        git.discover(repo).map_err(|said| refused(format!("--repo is not a git repo git could open: {said}")))?;
    let [bare, inside, below, own, shared] = &found[..] else {
        return Err(refused("git did not say where the repo is; git 2.31 or newer is needed"));
    };
    if bare == "true" || inside != "true" {
        return Err(refused("--repo is a bare repo or a repo's own git folder: a run needs a checkout to copy"));
    }
    if !below.is_empty() {
        return Err(refused("--repo is not the top of a git repo: by its real path it is a folder inside one"));
    }
    let person = git.open(Path::new(own)).map_err(refused)?;
    let base = person.ask(None, &[], &["rev-parse", "--verify", "--quiet", "HEAD^{commit}"]).map_err(refused)?;
    let base = base.ok_or_else(|| refused("--repo has no commit yet, so there is nothing to copy"))?;
    // git reads every worktree of the repo while it adds one, and fails on one half made or half removed. So a
    // copy is made behind the lock a clean holds: one at a time, and never while another is removed.
    let held = OpenOptions::new().write(true).create(true).truncate(false).open(at.lock()).map_err(Error::Store)?;
    held.lock().map_err(Error::Store)?;
    let run = claim(&at.runs, run)?;
    let made = add(&git, &person, &at, &run, Path::new(shared), &base);
    if made.is_err() {
        let _ = fs::remove_dir_all(at.runs.join(&run));
    }
    made
}

/// The worktree and its record. A branch of that name already there is an earlier run's work, and is refused. A copy
/// that could not be finished (git failed, or ran past its limit and was killed) is taken back whole: nothing has
/// run in it, so its folder, git's registration of it and its new branch all go.
fn add(git: &Git, person: &Repo, at: &Places, run: &str, shared: &Path, base: &str) -> Result<RunCopy, Error> {
    let (folder, branch) = (at.root.join(run), branch(run));
    let name = format!("refs/heads/{branch}");
    if person.ask(None, &[], &["rev-parse", "--verify", "--quiet", &name]).map_err(refused)?.is_some() {
        return Err(refused(format!(
            "run {run} has its branch already, {branch}: an earlier run's work is kept there"
        )));
    }
    let args = ["worktree", "add", "--quiet", "-b", &branch].map(OsStr::new);
    let args = [&args[..], &[folder.as_os_str(), OsStr::new(base)]].concat();
    let made = person.run(None, &[], &args).map_err(|said| refused(format!("git could not make the copy: {said}")));
    if let Err(e) = made.and_then(|_| note(git, at, run, shared, &folder)) {
        let _ = fs::remove_dir_all(&folder);
        let _ = unlist(person, &folder);
        let _ = person.run(None, &[], &["update-ref", "-d", &name]);
        let _ = fs::remove_dir_all(&folder); // again: a child of the stopped git had those calls' time to write
        return Err(e);
    }
    Ok(at.printed(run, true))
}

/// Writes the copy's record. This once, when nothing has run in the copy yet, git is asked from inside it where the
/// copy's own git folder is; the answer is kept in the record and never asked for again.
fn note(git: &Git, at: &Places, run: &str, shared: &Path, folder: &Path) -> Result<(), Error> {
    let own = match &git.discover(folder).unwrap_or_default()[..] {
        [_, inside, below, own, _] if inside == "true" && below.is_empty() => PathBuf::from(own),
        _ => PathBuf::new(), // git found no repo there, or one above the copy
    };
    let record = Record { repo: shared.to_path_buf(), own, failed: None, found: git.quiet };
    if record.own.is_absolute() { write(&at.record(run), &record) } else { Err(refused("git lost the copy")) }
}

/// Ends a run's copy. Completed: what is in the copy is committed onto the run's branch, the folder is removed and
/// the branch is kept. Failed: the copy stays, and of all failed copies only the newest [`KEPT`] do.
pub fn clean_copy(home: &Path, run: &str, outcome: Outcome) -> Result<RunCopy, Error> {
    end_copy(home, run, outcome).map(|(copy, _)| copy)
}

/// [`clean_copy`], which also says whether a completed run's commit was made: false when the copy held nothing new.
pub(crate) fn end_copy(home: &Path, run: &str, outcome: Outcome) -> Result<(RunCopy, bool), Error> {
    known(run)?;
    let at = Places::of(home);
    let file = at.record(run);
    let none = || refused(format!("no copy is kept for run {run}"));
    // One clean at a time, of any run: a second call waits here, then finds what the first left. The system drops
    // the lock when this process ends, however it ends, and no git child outlasts its limit, so the wait ends.
    // The file is made with the first copy, so a clean that is refused writes nothing.
    let held = OpenOptions::new().write(true).open(at.lock());
    let held = held.map_err(|e| if e.kind() == ErrorKind::NotFound { none() } else { Error::Store(e) })?;
    held.lock().map_err(Error::Store)?;
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

/// Whether nothing of a run's copy is kept: a finished run's copy is gone, with its record, and its branch stays.
pub(crate) fn is_gone(home: &Path, run: &str) -> bool {
    let at = Places::of(home);
    known(run).is_ok() && !at.record(run).exists() && fs::symlink_metadata(at.root.join(run)).is_err()
}

/// Undoes a copy no run was started in (F39: the harness was too old, or another run took the wish first): its
/// folder, git's registration of it, its branch, which is where the checkout was, and everything kept for the run.
pub(crate) fn take_back(home: &Path, run: &str) -> Result<(), Error> {
    known(run)?;
    let at = Places::of(home);
    let held = OpenOptions::new().write(true).open(at.lock()).map_err(Error::Store)?;
    held.lock().map_err(Error::Store)?;
    if let Some(record) = read(&at.record(run))? {
        let folder = at.root.join(run);
        let git = Git::find(&at.runs, record.found).map_err(refused)?;
        gone(&git, &record, &folder, standing(&at.root, &folder)?)?;
        let name = format!("refs/heads/{}", branch(run));
        git.open(&record.repo).and_then(|repo| repo.run(None, &[], &["update-ref", "-d", &name])).map_err(refused)?;
    }
    fs::remove_dir_all(at.runs.join(run)).map_err(Error::Store)
}

/// What is left to do for a copy [`take_back`] could not undo, because git could not be asked (the repo was moved
/// away): the copy's folder and everything kept for the run go, so nothing of it stays in Jellymoon's home. Its
/// branch and git's registration of it are in the repo, wherever that is now, and are not reached.
pub(crate) fn remove_ours(home: &Path, run: &str) {
    let at = Places::of(home);
    let held = OpenOptions::new().write(true).open(at.lock()).and_then(|held| held.lock().map(|()| held));
    if known(run).is_err() || held.is_err() {
        return;
    }
    let folder = at.root.join(run);
    if standing(&at.root, &folder).is_ok_and(|there| there) {
        let _ = fs::remove_dir_all(&folder);
    }
    let _ = fs::remove_dir_all(at.runs.join(run));
}

/// Whether the run's folder is there to be removed: a real folder directly inside a real `copies`. A link under that
/// name, or anything else, is refused and left as it is: removing through it, or committing what it leads to, would
/// reach outside the root.
fn standing(root: &Path, folder: &Path) -> Result<bool, Error> {
    let real = |path: &Path| fs::symlink_metadata(path).map(|meta| meta.file_type().is_dir());
    match (real(root), real(folder)) {
        (Ok(true), Ok(true)) => Ok(true),
        (Ok(true), Err(e)) if e.kind() == ErrorKind::NotFound => Ok(false),
        _ => Err(refused("the copy's place is not a folder Jellymoon made (a link, or a file); nothing was removed")),
    }
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

/// Removes the copy's folder, then the repo's registration of it. The folder goes first, by core and not by git, so
/// git is never pointed at it; the registration is in the person's repo, which the record names.
fn gone(git: &Git, record: &Record, folder: &Path, there: bool) -> Result<(), Error> {
    if there {
        fs::remove_dir_all(folder).map_err(Error::Store)?;
    }
    if !record.own.exists() {
        return Ok(());
    }
    let removed = unlist(&git.open(&record.repo).map_err(refused)?, folder);
    removed.map(|_| ()).map_err(|said| refused(format!("the copy is removed, but git still lists it: {said}")))
}

/// Has git drop its registration of the worktree at `folder`, which it does without the folder being there.
fn unlist(repo: &Repo, folder: &Path) -> Result<String, String> {
    repo.run(None, &[], &[OsStr::new("worktree"), OsStr::new("remove"), OsStr::new("--force"), folder.as_os_str()])
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
