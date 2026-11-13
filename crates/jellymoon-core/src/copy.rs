//! A run's own copy of the person's repo (ADR 0011, section 7; ADR 0014, decision 4; `docs/development/copy.md`): a
//! git worktree on a branch of its own, directly inside `copies` in Jellymoon's folder. [`make_copy`] makes one.
//! [`clean_copy`] ends one: a completed run's files are committed onto its branch and the folder is removed; a failed
//! run's copy is kept for a look, the newest [`KEPT`] only. [`take_back`] undoes one that no run was started in
//! (`copy/back.rs`).
//!
//! An agent could have rewritten every file in its copy, the `.git` file too. So once a copy is made nothing in it is
//! read to decide anything: where the repo is comes from the record written when the copy was made, kept in `runs`,
//! outside the copy, and the folder and the branch come from the run id alone.

mod back;
mod clean;
mod git;
mod gone;
mod hold;
mod record;
mod work;

use std::collections::hash_map::RandomState;
use std::ffi::OsStr;
use std::fmt;
use std::fs::{self, File};
use std::hash::{BuildHasher, Hasher};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

pub(crate) use back::{remove_ours, take_back, take_back_left};
pub use clean::clean_copy;
pub(crate) use clean::end_copy;
use git::{Git, Repo};
pub(crate) use hold::{left, start_lock, timed_out};
use record::{Record, write};
use serde::Serialize;
pub(crate) use work::{Done, Entry, LISTED, TEXT_MAX, Unread, WorkFile, work_of};

use crate::call::private_dir;
use crate::{Error, busy};

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
    make(home, repo, run, false, false).map(|(copy, _)| copy)
}

/// [`make_copy`] for a run. A folder that was not `typed` by the person was read from a session log: no refusal,
/// now or when the copy is ended, repeats what git says of it, since git's words name the folder. With the copy
/// comes the system's lock on `runs/<id>/start.lock`, taken before anything of the copy was made: the command
/// holds it for as long as it lives, so what a command that was killed left is known from what one still at
/// work has (`hold.rs`, `run/left.rs`).
pub(crate) fn copy_of(home: &Path, repo: &Path, typed: bool) -> Result<(RunCopy, File), Error> {
    let (copy, held) = make(home, repo, None, !typed, true)?;
    held.map(|held| (copy, held)).ok_or_else(|| refused("the copy's own lock was not taken"))
}

fn make(
    home: &Path,
    repo: &Path,
    run: Option<&str>,
    found: bool,
    of_run: bool,
) -> Result<(RunCopy, Option<File>), Error> {
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
    // git reads every worktree of a repo while it adds one, and fails on one half made or half removed (`hold.rs`).
    let _held = hold::held(&at.lock(), true, None).map_err(Error::Store)?;
    let run = claim(&at.runs, run)?;
    let held = if of_run { hold::start(&at.runs, &run).map(Some) } else { Ok(None) };
    let made = held
        .map_err(Error::Store)
        .and_then(|held| add(&git, &person, &at, &run, Path::new(shared), &base).map(|copy| (copy, held)));
    // The lock is let go of by then: where a file that is open keeps its folder, the folder can go.
    if made.is_err() {
        let _ = gone::remove(&at.runs, &run);
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
    // A command killed while git makes the copy leaves this record, so a look can take back what git made by
    // then: git keeps a worktree's own git folder under the folder's name, which `note` then reads from git.
    let own = shared.join("worktrees").join(run);
    let mut record = Record { repo: shared.to_path_buf(), own, failed: None, found: git.quiet, put_off: None };
    write(&at.record(run), &record)?;
    let args = ["worktree", "add", "--quiet", "-b", &branch].map(OsStr::new);
    let args = [&args[..], &[folder.as_os_str(), OsStr::new(base)]].concat();
    let made = person.run(None, &[], &args).map_err(|said| refused(format!("git could not make the copy: {said}")));
    busy::pause("JELLYMOON_TEST_MADE_WAIT_MS");
    if let Err(e) = made.and_then(|_| note(git, at, run, &mut record, &folder)) {
        // The order of every take-back (`back.rs`): the branch, then the repo's listing of the copy.
        let _ = fs::remove_dir_all(&folder);
        let _ = person.run(None, &[], &["update-ref", "--no-deref", "-d", &name, base]);
        busy::pause("JELLYMOON_TEST_UNMADE_WAIT_MS");
        let _ = unlist(person, &folder);
        let _ = fs::remove_dir_all(&folder); // again: a child of the stopped git had those calls' time to write
        return Err(e);
    }
    Ok(at.printed(run, true))
}

/// Writes the copy's record again. This once, when nothing has run in the copy yet, git is asked from inside it
/// where the copy's own git folder is; the answer is kept in the record and never asked for again.
fn note(git: &Git, at: &Places, run: &str, record: &mut Record, folder: &Path) -> Result<(), Error> {
    record.own = match &git.discover(folder).unwrap_or_default()[..] {
        [_, inside, below, own, _] if inside == "true" && below.is_empty() => PathBuf::from(own),
        _ => PathBuf::new(), // git found no repo there, or one above the copy
    };
    if record.own.is_absolute() { write(&at.record(run), record) } else { Err(refused("git lost the copy")) }
}

/// Whether nothing of a run's copy is kept: a finished run's copy is gone, with its record, and its branch stays.
pub(crate) fn is_gone(home: &Path, run: &str) -> bool {
    let at = Places::of(home);
    known(run).is_ok() && !at.record(run).exists() && fs::symlink_metadata(at.root.join(run)).is_err()
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
