//! Undoing a change that was kept by itself (F92, ADR 0028; `docs/development/keep.md`, "Undone"): the person's
//! branch names the commit it named before the keep again, and the files and the index are as that commit has
//! them. It is not a revert: no commit is made. The step is two commands, and neither goes by `HEAD`: the files
//! and the index are moved by `git read-tree -m -u <kept> <before>`, which refuses whole for an unsaved or staged
//! change in a file of the change and for an untracked file where one would come back, and the branch is moved
//! by `git update-ref` of its whole name, given the commit it must name now. So no other ref is ever written,
//! wherever the checkout stands by then. git writes over an ignored file where a file comes back: that is asked
//! here first, twice, the second time with nothing but the writing of the record before the step.

use std::path::Path;

use super::super::git::Repo;
use super::super::work::{REF_MAX, tip_of};
use super::{At, Keep, Places, SAID_MAX, Unkept, Unread, fits, opened, runs, stands};

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
    /// The branch names another commit than the keep landed.
    Moved,
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

fn undone(at: &At, back: &Back, note: &mut dyn FnMut() -> bool) -> Result<(), Stays> {
    let of = at.of;
    // An undo that was cut off after its step: the branch is back, and only the record is to be written.
    if back.again && tip_of(at.repo, back.onto).ok().as_deref() == Some(back.before) {
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
        return Err(Stays::Moved);
    }
    if fits::in_way(at.mine, at.of.folder, files, back.again)? { Err(Stays::Way) } else { Ok(()) }
}

/// Whether a name of the repo other than the branch itself has the kept commit in its history: a branch, a tag,
/// a remote's branch. A run's branch is Jellymoon's own name and holds nothing that was sent anywhere.
fn held(at: &At, back: &Back) -> Result<bool, Stays> {
    let names = ["for-each-ref", "--contains", back.kept, "--format=%(refname)"];
    let names = at.repo.bytes(&names, &[], REF_MAX).filter(|names| names.len() < REF_MAX);
    let names =
        String::from_utf8(names.ok_or(Unkept::Unread(Unread::Repo))?).map_err(|_| Unkept::Unread(Unread::Repo))?;
    Ok(names.lines().any(|name| name != back.onto && !runs(name)))
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
