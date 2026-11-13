//! Keeping a run's work (F45, ADR 0017, decision 4; `docs/development/keep.md`): the one commit a run ended as is
//! landed on the branch the person's checkout stands on. It is the first thing core does that writes to a
//! person's branch and working tree, so every step is one git refuses whole when it cannot do it:
//!
//! - what is landed is the run's own commit when the branch is still at the run's start, or one new commit by
//!   the person on top of theirs, whose tree git merged in the store and which nothing names until it lands;
//! - the checkout is touched by one step, `git merge --ff-only`, which leaves a changed, an untracked or an
//!   ignored file of the person's alone and then does nothing at all;
//! - the run's branch is deleted last, as itself, and only while it is at the end the record names.
//!
//! What the record says is written through [`Note`], before the checkout is touched and after, so the next keep
//! can tell a keep that landed from one that did not ([`before`]). git answers with a code, and no word of git's
//! and no name from the repo is kept of any failure ([`Unkept`]); every answer is read to a size ([`Repo::answer`]).

mod attrs;
mod fits;
mod made;
mod step;
mod why;

use std::fs::{self, File};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub(crate) use fits::Touched;
pub(crate) use step::LOOSE;
pub(crate) use why::{How, Unkept};

use super::git::{Git, Repo, Unopened};
use super::work::{REF_MAX, parent, same, tip, tip_of};
use super::{Named, Places, Unread, back, branch, hold, is_id};
use crate::busy::or_test;

/// How long a keep waits for another keep of this home to end.
const WAIT: Duration = Duration::from_secs(10);
/// The most bytes read of what a step of git's prints, and of what it says beside: a step that worked prints an
/// id or nothing.
const SAID_MAX: usize = 1024 * 1024;
/// What git keeps in a checkout's git folder while a merge, a cherry-pick, a revert, a rebase or a bisect is
/// going on there.
const MIDWAY: [&str; 8] = [
    "MERGE_HEAD",
    "CHERRY_PICK_HEAD",
    "REVERT_HEAD",
    "BISECT_LOG",
    "BISECT_START",
    "rebase-merge",
    "rebase-apply",
    "sequencer",
];

/// What a keep landed, or with [`Keep::ask`] would land.
pub(crate) struct Landing {
    /// The commit on the branch. None only of a commit that would have to be made, when asked.
    pub commit: Option<String>,
    /// The branch, by its whole name; [`LOOSE`] of work that landed on none that is named.
    pub onto: String,
    pub how: How,
}

/// What a keep has the record say: the commit it is about to land on a branch, and that it has landed. And what
/// it has the answer name: a `.gitattributes` the work removes that is still in the checkout after the step.
pub(crate) enum Note<'a> {
    Intent(&'a str, &'a str),
    Kept(&'a str, &'a str),
    Stayed(&'a [u8]),
}

pub(crate) struct Keep<'a> {
    pub home: &'a Path,
    /// The person's checkout, found as a run finds a repo.
    pub folder: &'a Path,
    pub run: &'a str,
    pub named: &'a Named<'a>,
    /// The commit and the branch an earlier keep wrote down before it touched the checkout.
    pub was: Option<(&'a str, &'a str)>,
    /// Only say what a keep would do.
    pub ask: bool,
}

/// What the steps of one keep share.
struct At<'a> {
    of: &'a Keep<'a>,
    /// The repo by the git folder its checkouts share, and by the git folder of the person's checkout.
    repo: &'a Repo<'a>,
    mine: &'a Repo<'a>,
    shared: &'a Path,
    own: &'a Path,
    /// The run's branch, by its whole name.
    name: String,
}

/// Whether a run's record holds this name of a branch: a branch's whole name of at most 1024 bytes, with no
/// space of any kind, no line break, no control character and nothing git reads as a pattern. It is the one
/// rule, for the keep that writes the name ([`stands`]) and for the command that reads it back (`run/keep.rs`).
pub(crate) fn nameable(onto: &str) -> bool {
    let odd = |c: char| c.is_control() || c.is_whitespace() || "*?[\\~^:".contains(c);
    onto.starts_with("refs/heads/") && onto.len() <= 1024 && !onto.contains(odd)
}

/// Whether a branch's whole name is one a run's branch has, under any spelling of its ASCII letters: a disk that
/// folds case takes `Jellymoon/run-<id>` for the run's branch itself. No work is ever kept on such a branch: a
/// keep refuses a checkout that stands on one ([`stands`]), says so of one moved there just before the step
/// (`step.rs`), and a record that names one is not core's (`run/keep.rs`).
pub(crate) fn runs(onto: &str) -> bool {
    const RUNS: &str = "refs/heads/jellymoon/run-";
    onto.get(..RUNS.len()).is_some_and(|start| start.eq_ignore_ascii_case(RUNS))
}

/// Whether git said yes with a 0 or no with a 1. Any other end is no answer.
fn yes(answer: Option<(Option<i32>, Vec<u8>)>) -> Result<bool, Unkept> {
    match answer {
        Some((Some(0), _)) => Ok(true),
        Some((Some(1), _)) => Ok(false),
        _ => Err(Unread::Repo.into()),
    }
}

/// The one line git printed, or None for a 1.
fn line(repo: &Repo, vars: &[(&str, &str)], args: &[&str]) -> Result<Option<String>, Unkept> {
    let (code, printed) = repo.answer(None, vars, args, &[], REF_MAX).ok_or(Unread::Repo)?;
    let text = String::from_utf8(printed).ok().filter(|text| text.len() < REF_MAX).ok_or(Unread::Repo)?;
    match code {
        Some(0) => Ok(Some(text.trim_end_matches(['\r', '\n']).to_string())),
        Some(1) => Ok(None),
        _ => Err(Unread::Repo.into()),
    }
}

/// The system's lock on `runs/keep.lock`, which a keep holds from before it reads the run's record until it
/// ends: one keep of a home at a time, so a second one reads what the first wrote. It is waited for a short
/// while, and the system drops it when its holder ends, however it ends. The file is made when it is not there,
/// by a question too, and is never removed: a question waits for a keep as a keep does, so that it answers of
/// what that keep left.
pub(crate) fn alone(home: &Path) -> Result<File, Unkept> {
    let wait = or_test(WAIT, "JELLYMOON_TEST_KEEP_WAIT_MS");
    let held = hold::held(&Places::of(home).runs.join("keep.lock"), true, Some(wait));
    held.map_err(|e| if e.kind() == ErrorKind::TimedOut { Unkept::Busy } else { Unkept::NoLock })
}

/// Keeps the work, or with `ask` says what a keep would do and changes nothing. The caller holds the keeps' lock
/// ([`alone`]). `judge` is asked of the files the work touches, and of the files the one new commit would touch,
/// before anything is made; `note` writes the record, and false from it ends the keep.
pub(crate) fn keep(
    of: &Keep,
    judge: &mut dyn FnMut(&[Touched]) -> Result<(), Unkept>,
    note: &mut dyn FnMut(Note) -> bool,
) -> Result<Landing, Unkept> {
    let git = Git::find(&Places::of(of.home).runs, true).map_err(|_| Unread::Home)?;
    let (shared, own) = checkout(&git, of)?;
    // What is switched off is what the config names as git reads it through the shared git folder and through
    // the checkout's own, which may have a config of its own: every call is given both lists.
    let unopened = |unopened| match unopened {
        Unopened::Unasked(_) => Unkept::Unread(Unread::Repo),
        Unopened::Unswitched(_) => Unkept::Config,
    };
    let mine = git.opened(&shared).and_then(|repo| repo.seen_from(&own)).map_err(unopened)?;
    let repo = mine.through(&shared);
    let name = format!("refs/heads/{}", branch(of.run));
    let at = At { of, repo: &repo, mine: &mine, shared: &shared, own: &own, name };
    let landing = match before(&at) {
        Some(landing) => landing,
        None => land(&at, judge, note)?,
    };
    let (Some(commit), false) = (landing.commit.as_deref(), of.ask) else { return Ok(landing) };
    if !note(Note::Kept(commit, &landing.onto)) {
        return Err(Unkept::Unwritten);
    }
    step::cut(4);
    // The run's branch goes only when a branch holds the commit: work on a detached `HEAD` has no other name.
    if landing.how != How::Loose {
        drop_branch(&at);
    }
    Ok(landing)
}

/// What is left of a keep whose record says kept: the run's branch, when the keep was cut off before it went.
/// Nothing is said of it, whatever happens. The caller holds the keeps' lock.
pub(crate) fn swept(of: &Keep) {
    let Ok(git) = Git::find(&Places::of(of.home).runs, true) else { return };
    let Ok((shared, own)) = checkout(&git, of) else { return };
    let Ok(repo) = git.open(&shared) else { return };
    let name = format!("refs/heads/{}", branch(of.run));
    drop_branch(&At { of, repo: &repo, mine: &repo, shared: &shared, own: &own, name });
}

/// The git folder the repo's checkouts share and the git folder of the person's checkout, when the folder is the
/// top of a checkout of the repo the record names.
fn checkout(git: &Git, of: &Keep) -> Result<(PathBuf, PathBuf), Unkept> {
    let found = git.discover(of.folder).map_err(|_| Unread::Repo)?;
    let [_, inside, below, own, shared] = &found[..] else { return Err(Unread::Repo.into()) };
    if inside != "true" || !below.is_empty() {
        return Err(Unread::Repo.into());
    }
    if !same(of.named.repo, Path::new(shared)) {
        return Err(Unread::Another.into());
    }
    Ok((PathBuf::from(shared), PathBuf::from(own)))
}

/// What an earlier keep landed before it was cut off: the commit its record names is in the history of the
/// branch its record names. It is asked of the record and the refs alone, before the run's branch is looked at,
/// so a keep that landed is finished whatever became of that branch. An id no commit has is in no history.
fn before(at: &At) -> Option<Landing> {
    let (commit, onto) = at.of.was?;
    let tip = tip_of(at.repo, onto).ok()?;
    let held = at.repo.answer(None, &[], &["merge-base", "--is-ancestor", commit, &tip], &[], REF_MAX);
    matches!(held, Some((Some(0), _))).then(|| Landing {
        commit: Some(commit.to_string()),
        onto: onto.to_string(),
        how: How::Finished,
    })
}

/// Whether a commit is in the history of another, or is it.
fn reaches(at: &At, old: &str, new: &str) -> Result<bool, Unkept> {
    yes(at.repo.answer(None, &[], &["merge-base", "--is-ancestor", old, new], &[], REF_MAX))
}

/// Whether any checkout of the repo stands on the run's branch: the repo's own, or a worktree of it.
fn stood_on(at: &At) -> Result<bool, Unkept> {
    let mut folders = vec![at.shared.to_path_buf()];
    match fs::read_dir(at.shared.join("worktrees")) {
        Ok(kept) => {
            for kept in kept {
                folders.extend(kept.ok().map(|kept| kept.path()).filter(|kept| kept.is_dir()));
            }
        }
        Err(e) if e.kind() == ErrorKind::NotFound => {}
        Err(_) => return Err(Unread::Repo.into()),
    }
    for folder in folders {
        if back::stands(&at.repo.through(&folder), &folder, &at.name).map_err(|_| Unread::Repo)? {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Deletes the run's branch as itself, while it names the record's end and nothing stands on it. git's own check
/// of the old value is the comparison: a branch that was moved, made a name for another or removed stays as it is.
fn drop_branch(at: &At) {
    if stood_on(at) == Ok(false) {
        let delete = ["update-ref", "--no-deref", "-d", &at.name, at.of.named.end];
        let _ = at.repo.answer(None, &[], &delete, &[], SAID_MAX);
    }
}

/// The branch the person's checkout stands on, by its whole name, and the commit it names. git follows a `HEAD`
/// that names a name for a branch to the branch itself, as it does when it says where a checkout stands, and
/// that branch is the one the work lands on.
fn stands(at: &At) -> Result<(String, String), Unkept> {
    let onto = line(at.mine, &[], &["symbolic-ref", "--quiet", "HEAD"])?.ok_or(Unkept::NoBranch)?;
    if !onto.starts_with("refs/heads/") || runs(&onto) {
        return Err(Unkept::NoBranch);
    }
    if !nameable(&onto) {
        return Err(Unkept::Unnamed);
    }
    let going =
        |name: &&str| !matches!(fs::symlink_metadata(at.own.join(name)), Err(e) if e.kind() == ErrorKind::NotFound);
    if MIDWAY.iter().any(going) {
        return Err(Unkept::Midway);
    }
    match tip_of(at.repo, &onto) {
        Ok(tip) => Ok((onto, tip)),
        // A branch with no commit yet has no history. A `HEAD` that is at a commit, on a branch the repo does not
        // list under that name, was read by a disk that takes another spelling for a branch's name.
        Err(Unread::Missing) => match line(at.mine, &[], &["rev-parse", "--verify", "--quiet", "HEAD^{commit}"])? {
            Some(_) => Err(Unkept::Spelled),
            None => Err(Unkept::History),
        },
        Err(Unread::Symbolic) => Err(Unkept::NoBranch),
        Err(unread) => Err(unread.into()),
    }
}

/// Checks the world against the record, decides what would land, and lands it. Nothing is written before every
/// check has passed, and with `ask` nothing is written at all but what git's trial merge leaves in the store,
/// which nothing names.
fn land(
    at: &At,
    judge: &mut dyn FnMut(&[Touched]) -> Result<(), Unkept>,
    note: &mut dyn FnMut(Note) -> bool,
) -> Result<Landing, Unkept> {
    let (of, start, end) = (at.of, at.of.named.start, at.of.named.end);
    // The record's end is compared with what git prints for the branch, and git is given what it printed.
    if tip(at.repo, of.run)? != end {
        return Err(Unread::Moved.into());
    }
    if parent(at.repo, of.run, end)? != start {
        return Err(Unread::Start.into());
    }
    if stood_on(at)? {
        return Err(Unkept::Stood);
    }
    let (onto, tip) = stands(at)?;
    if reaches(at, end, &tip)? {
        return Ok(Landing { commit: Some(end.to_string()), onto, how: How::Held });
    }
    let work = fits::touched(at.repo, start, end)?;
    judge(&work)?;
    let joined = tip != start;
    let (target, who, files) = if joined {
        if !reaches(at, start, &tip)? {
            return Err(Unkept::History);
        }
        let tree = made::merged(at, &tip)?;
        // The person's last commit holds the work already, whoever put it there: nothing more is landed.
        if line(at.repo, &[], &["rev-parse", "--verify", &format!("{tip}^{{tree}}")])?.as_deref() == Some(&tree) {
            return Ok(Landing { commit: Some(tip), onto, how: How::Held });
        }
        let files = fits::touched(at.repo, &tip, &tree)?;
        judge(&files)?;
        (tree, Some(made::author(at)?), files)
    } else {
        (end.to_string(), None, work)
    };
    let scratch = Places::of(of.home).runs.join(of.run);
    fits::fits(at.mine, (of.folder, at.own, &scratch), (&tip, &target), &files, of.was.is_some())?;
    let how = if joined { How::Joined } else { How::Forward };
    if of.ask {
        return Ok(Landing { commit: (!joined).then_some(target), onto, how });
    }
    let commit = match who {
        Some(who) => made::commit(at, &who, &tip, &target)?,
        None => target,
    };
    step::forward(at, commit, onto, how, &files, note)
}
