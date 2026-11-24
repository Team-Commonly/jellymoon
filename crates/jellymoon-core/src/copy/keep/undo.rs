//! Undoing a change that was kept by itself (F92, ADR 0028; `docs/development/keep.md`, "Undone"): the person's
//! branch names the commit it named before the keep again, and the files and the index are as that commit has
//! them. It is not a revert: no commit is made. The step is two commands, and neither goes by `HEAD`: the files
//! and the index are moved by `git read-tree -m -u <kept> <before>`, which refuses whole for an unsaved or staged
//! change in a file of the change and for an untracked file where one would come back, and the branch is moved
//! by `git update-ref` of its whole name, given the commit it must name now. So no other ref is ever written,
//! wherever the checkout stands by then. git writes over an ignored file where a file comes back: that is asked
//! here first, twice, the second time with nothing but the writing of the record before the step.

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::super::git::Repo;
use super::super::work::{REF_MAX, tip_of};
use super::{At, Keep, Places, SAID_MAX, Unkept, Unread, fits, line, opened, runs, stands};

/// What an undo goes by, all of it from the run's record: the commit the keep landed, the commit the branch named
/// before it, the branch by its whole name, and whether an undo wrote down before that it was about to step.
pub(crate) struct Back<'a> {
    pub kept: &'a str,
    pub before: &'a str,
    pub onto: &'a str,
    pub again: bool,
}

/// Why a change stays kept.
pub(crate) enum Stays {
    /// The branch names another commit than the keep landed, with the kept one in its history.
    Moved,
    /// The branch does not hold the kept commit any more.
    Gone,
    /// The record's two commits are not a commit and its only parent: no keep wrote them.
    Forged,
    /// The checkout stands on another branch, or on none.
    Away,
    Midway,
    /// Another name of the repo holds the commit: a branch, a tag, a remote's branch.
    Held,
    /// Something is on the disk where a file would come back.
    Way,
    /// git's trial of the step refused: an unsaved or staged change in a file of the change.
    Edits,
    /// git stopped the step, or the branch does not name the commit after it.
    Stepped,
    Unwritten,
    /// The repo or the folder could not be read as an undo needs it.
    Unread,
}

impl From<Unkept> for Stays {
    fn from(unkept: Unkept) -> Stays {
        match unkept {
            Unkept::NoBranch => Stays::Away,
            Unkept::Midway => Stays::Midway,
            _ => Stays::Unread,
        }
    }
}

/// The files and the index: the word after `-c` is one a test finds this command by, and switches off nothing
/// this command would do.
const FILES: [&str; 5] = ["-c", "gc.auto=0", "read-tree", "-m", "-u"];

/// Where a test's build ends the command when `JELLYMOON_TEST_UNDO_CUT` is this step's number (`busy::cut`).
fn cut(step: u8) {
    crate::busy::cut("JELLYMOON_TEST_UNDO_CUT", step);
}

/// Undoes the keep, or with `ask` says whether it would and changes nothing. The caller holds the keeps' lock.
/// `note` writes down that the step is about to be taken, and false from it ends the undo.
pub(crate) fn undo(of: &Keep, back: &Back, note: &mut dyn FnMut() -> bool) -> Result<(), Stays> {
    opened(of, |at| Ok(undone(at, back, note)))?
}

/// Where a change that was kept by itself is now, asked of the repo at every look of a page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stands {
    /// The checkout stands on the branch, and the branch names the kept commit.
    Here,
    /// The checkout stands on the branch, which has commits since the kept one.
    Past,
    /// Anything else: another branch or none, a branch that does not hold the commit, a repo or a record that
    /// is not as a keep left it.
    Elsewhere,
}

/// [`Stands`] of the change `back` names. Nothing is written and no lock is taken.
pub(crate) fn stands_now(of: &Keep, back: &Back) -> Stands {
    let asked = opened(of, |at| {
        let head = line(at.mine, &[], &["symbolic-ref", "--quiet", "HEAD"])?;
        Ok(match tip_of(at.repo, back.onto) {
            _ if !proven(at, back) || head.as_deref() != Some(back.onto) => Stands::Elsewhere,
            Ok(tip) if tip == back.kept => Stands::Here,
            Ok(tip) if within(at, back.kept, &tip) => Stands::Past,
            _ => Stands::Elsewhere,
        })
    });
    asked.unwrap_or(Stands::Elsewhere)
}

/// Whether the kept commit is a commit whose only parent is the commit before: what every keep by itself
/// leaves, and what a record that was edited to name any other two ids of the store does not show.
fn proven(at: &At, back: &Back) -> bool {
    let parents = line(at.repo, &[], &["rev-list", "--parents", "-n", "1", back.kept]);
    matches!(parents, Ok(Some(said)) if said == format!("{} {}", back.kept, back.before))
}

/// Whether `commit` is `tip` or in its history.
fn within(at: &At, commit: &str, tip: &str) -> bool {
    let asked = ["merge-base", "--is-ancestor", commit, tip];
    matches!(at.repo.answer(None, &[], &asked, &[], SAID_MAX), Some((Some(0), _)))
}

fn undone(at: &At, back: &Back, note: &mut dyn FnMut() -> bool) -> Result<(), Stays> {
    let of = at.of;
    if !proven(at, back) {
        return Err(Stays::Forged);
    }
    // An undo that was cut off after its step, or whose change the person took out since: the branch holds the
    // commit before and not the kept one, and only the record is to be written.
    let back_already = |tip: String| within(at, back.before, &tip) && !within(at, back.kept, &tip);
    if back.again && tip_of(at.repo, back.onto).is_ok_and(back_already) {
        return Ok(());
    }
    let files = fits::touched(at.repo, back.kept, back.before)?;
    close(at, back, &files)?;
    if held(at, back)? {
        return Err(Stays::Held);
    }
    let scratch = Places::of(of.home).runs.join(of.run);
    match fits::fits(at.mine, (of.folder, at.own, &scratch), (back.kept, back.before), &files, back.again) {
        Err(Unkept::InWay) => return Err(Stays::Edits),
        fit => fit?,
    }
    if of.ask {
        return Ok(());
    }
    // A file that was only touched is no unsaved change, to the trial and now to the step's own index.
    let _ = at.mine.answer(Some(of.folder), &[], &["update-index", "-q", "--refresh"], &[], SAID_MAX);
    close(at, back, &files)?;
    if !note() {
        return Err(Stays::Unwritten);
    }
    cut(1);
    step(at.mine, at.repo, of.folder, back)
}

/// What is asked first, and again just before the step: the checkout stands on the branch, the branch names the
/// commit that was kept, and nothing is on the disk where a file would come back. An undo that goes on after one
/// that was cut off may find there the files that undo's git wrote (`fits::in_way`).
fn close(at: &At, back: &Back, files: &[fits::Touched]) -> Result<(), Stays> {
    let (onto, tip) = stands(at)?;
    if onto != back.onto {
        return Err(Stays::Away);
    }
    if tip != back.kept {
        return Err(if within(at, back.kept, &tip) { Stays::Moved } else { Stays::Gone });
    }
    if fits::in_way(at.mine, at.of.folder, files, back.again, true)? { Err(Stays::Way) } else { Ok(()) }
}

/// Whether a name of the repo other than the branch itself has the kept commit in its history: a branch, a tag,
/// a remote's branch. A run's branch is Jellymoon's own name and holds nothing that was sent anywhere, and
/// neither does the stash, whose commits stand on the one the checkout was at.
fn held(at: &At, back: &Back) -> Result<bool, Stays> {
    let names = ["for-each-ref", "--contains", back.kept, "--format=%(refname)"];
    let names = at.repo.bytes(&names, &[], REF_MAX).filter(|names| names.len() < REF_MAX);
    let names =
        String::from_utf8(names.ok_or(Unkept::Unread(Unread::Repo))?).map_err(|_| Unkept::Unread(Unread::Repo))?;
    Ok(names.lines().any(|name| name != back.onto && name != "refs/stash" && !runs(name)))
}

/// The step, and nothing else: the files and the index go from the kept commit to the one before, and then the
/// branch, by its whole name, from the kept commit to the one before. Neither command reads `HEAD`.
fn step(mine: &Repo, repo: &Repo, folder: &Path, back: &Back) -> Result<(), Stays> {
    let did = |answer: Option<(Option<i32>, Vec<u8>)>| matches!(answer, Some((Some(0), _)));
    let files: Vec<&str> = FILES.iter().copied().chain([back.kept, back.before]).collect();
    if !did(mine.answer(Some(folder), &[], &files, &[], SAID_MAX)) {
        return Err(Stays::Stepped);
    }
    cut(2);
    let branch = ["update-ref", "--no-deref", back.onto, back.before, back.kept];
    let moved = did(repo.answer(None, &[], &branch, &[], SAID_MAX));
    cut(3);
    if moved && tip_of(repo, back.onto).ok().as_deref() == Some(back.before) { Ok(()) } else { Err(Stays::Stepped) }
}
