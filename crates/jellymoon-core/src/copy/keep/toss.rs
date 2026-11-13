//! Tossing a run's work, in the repo (F46, ADR 0017, decision 7; `docs/development/toss.md`): the run's branch is
//! deleted as itself, only while it names the commit the record names and no checkout stands on it. No other
//! name is written, and nothing of a checkout is: no working tree, no `HEAD`, no index. It is a keep's git, a
//! keep's way of finding the repo and a keep's question whether work landed, so the two never disagree.

use super::{At, Keep, SAID_MAX, Unkept, before, checkout, line, stood_on, yes};
use crate::copy::git::Git;
use crate::copy::work::{REF_MAX, parent, tip};
use crate::copy::{Places, Unread, back, branch};

/// Leaves out of git's `--branches` every branch named as a run's is, as `keep::runs` reads a name: the start of
/// the short name in either case of each letter, and anything after it.
const NOT_RUNS: &str = "--exclude=[jJ][eE][lL][lL][yY][mM][oO][oO][nN]/[rR][uU][nN]-*";

/// What became of the run's branch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Branch {
    Deleted,
    /// The repo has no branch of the run's name.
    Gone,
    Left(Left),
}

/// Why the run's branch was left as it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Left {
    /// A checkout stands on it.
    Stood,
    /// It is a name for another branch.
    Symbolic,
    /// Another name of the repo's is a name for it, and would be left naming nothing.
    Pointed,
    /// It is not at the commit the record names, or that commit is not one a run's copy ended as.
    Moved,
    /// It was at that commit when asked, and git would not delete it as such a moment later.
    Raced,
    /// The repo could not be found or asked.
    Unasked,
    /// The record names no commit.
    Unnamed,
}

/// Why work that waits was not tossed, with nothing changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Untossed {
    Unread(Unread),
    /// A checkout stands on the run's branch.
    Stood,
    /// A keep that was cut off had landed the work.
    Landed,
    /// The run's record could not be written.
    Unwritten,
}

impl From<Unread> for Untossed {
    fn from(unread: Unread) -> Untossed {
        Untossed::Unread(unread)
    }
}

/// Deletes the run's branch while it is the run's own name at `of.named.end`, and says what became of it. The
/// caller holds the keeps' lock (`alone`). For work that waits (`strict`) every check comes first and any that
/// fails is a refusal; then `note` writes the record, and false from it ends the toss; only then is the branch
/// deleted. A branch that a name of the person's points at is no refusal: the toss is written and the branch is
/// left. Without `strict` the record says tossed already: nothing is refused but a repo that cannot be asked,
/// and a branch that is not the run's own at that commit is left.
pub(crate) fn toss(of: &Keep, strict: bool, note: &mut dyn FnMut() -> bool) -> Result<Branch, Untossed> {
    let git = Git::find(&Places::of(of.home).runs, true).map_err(|_| Unread::Home)?;
    let (shared, own) = checkout(&git, of).map_err(|unkept| match unkept {
        Unkept::Unread(unread) => unread,
        _ => Unread::Repo,
    })?;
    let repo = git.open(&shared).map_err(|_| Unread::Repo)?;
    let name = format!("refs/heads/{}", branch(of.run));
    let mine = repo.through(&own);
    let at = At { of, repo: &repo, mine: &mine, shared: &shared, own: &own, name };
    if landed(&at)? {
        return Err(Untossed::Landed);
    }
    let found = found(&at);
    if strict {
        match found {
            Err(Branch::Left(Left::Stood)) => return Err(Untossed::Stood),
            Err(Branch::Left(Left::Symbolic)) => return Err(Unread::Symbolic.into()),
            Err(Branch::Left(Left::Moved)) => return Err(Unread::Moved.into()),
            Err(Branch::Left(Left::Pointed) | Branch::Gone) | Ok(()) => {}
            Err(_) => return Err(Unread::Repo.into()),
        }
        if !note() {
            return Err(Untossed::Unwritten);
        }
    }
    Ok(found.map_or_else(|left| left, |()| delete(&at)))
}

/// Whether a keep that was cut off landed the work. Its record holds the commit it meant and the branch it meant,
/// and git moves whatever `HEAD` names at the moment of the step (`step.rs`): so the repo is asked where that
/// commit is. It is landed when the branch the record names holds it ([`before`]), when any branch of the
/// person's does, or when the `HEAD` of the checkout the toss is asked in names no branch and holds it. A branch
/// named as a run's is, in any case of its letters (`keep::runs`), is none of the person's: no work is ever kept
/// on one, and the run's own is one. git is asked for the commit with every other branch's history taken away,
/// and prints it or nothing: no name comes back. A record with no such commit, and an id no commit has, is no
/// landing.
fn landed(at: &At) -> Result<bool, Unread> {
    let Some((commit, _)) = at.of.was else { return Ok(false) };
    if before(at).is_some() {
        return Ok(true);
    }
    let known = format!("{commit}^{{commit}}");
    if !matches!(at.repo.answer(None, &[], &["cat-file", "-e", &known], &[], REF_MAX), Some((Some(0), _))) {
        return Ok(false);
    }
    let outside = ["rev-list", "--max-count=1", commit, "--not", NOT_RUNS, "--branches"];
    match at.repo.answer(None, &[], &outside, &[], REF_MAX) {
        Some((Some(0), left)) if left.is_empty() => return Ok(true),
        Some((Some(0), _)) => {}
        _ => return Err(Unread::Repo),
    }
    let asked = |unkept| if let Unkept::Unread(unread) = unkept { unread } else { Unread::Repo };
    if line(at.mine, &[], &["symbolic-ref", "--quiet", "HEAD"]).map_err(asked)?.is_some() {
        return Ok(false);
    }
    match line(at.mine, &[], &["rev-parse", "--verify", "--quiet", "HEAD^{commit}"]).map_err(asked)? {
        Some(head) => {
            yes(at.mine.answer(None, &[], &["merge-base", "--is-ancestor", commit, &head], &[], REF_MAX)).map_err(asked)
        }
        None => Ok(false),
    }
}

/// Whether the branch is there to be deleted: the run's own name, at the record's commit, which is the run's
/// start or the one commit a run's copy ends as on top of it, with no checkout standing on it and no other name
/// of the repo's that is a name for it (`back::pointed_at`, the question a take-back asks). Err says what it is
/// instead. What the record names is compared with what git prints, and git is given what it printed.
fn found(at: &At) -> Result<(), Branch> {
    let (start, end) = (at.of.named.start, at.of.named.end);
    match tip(at.repo, at.of.run) {
        Ok(tip) if tip == end => {}
        Err(Unread::Missing) => return Err(Branch::Gone),
        Err(Unread::Symbolic) => return Err(Branch::Left(Left::Symbolic)),
        Err(Unread::Repo) => return Err(Branch::Left(Left::Unasked)),
        _ => return Err(Branch::Left(Left::Moved)),
    }
    if start != end {
        match parent(at.repo, at.of.run, end) {
            Ok(parent) if parent == start => {}
            Err(Unread::Repo) => return Err(Branch::Left(Left::Unasked)),
            _ => return Err(Branch::Left(Left::Moved)),
        }
    }
    match (stood_on(at), back::pointed_at(at.repo, &at.name)) {
        (Ok(true), _) => Err(Branch::Left(Left::Stood)),
        (Ok(false), Ok(true)) => Err(Branch::Left(Left::Pointed)),
        (Ok(false), Ok(false)) => Ok(()),
        _ => Err(Branch::Left(Left::Unasked)),
    }
}

/// Deletes the name itself, never what a name points at, with git's own check of the old value: a branch that
/// was moved or made a name for a branch elsewhere since it was asked about stays as it is, and one that was
/// removed since is said to be gone. A checkout that came to stand on the branch in that moment is asked for
/// once more, and the name is put back where it was, as a new name only: the checkout stands on a branch again,
/// and nothing is written over.
fn delete(at: &At) -> Branch {
    let end = at.of.named.end;
    let step = |words: &[&str]| matches!(at.repo.answer(None, &[], words, &[], SAID_MAX), Some((Some(0), _)));
    if !step(&["update-ref", "--no-deref", "-d", &at.name, end]) {
        let gone = tip(at.repo, at.of.run) == Err(Unread::Missing);
        return if gone { Branch::Gone } else { Branch::Left(Left::Raced) };
    }
    let none = "0".repeat(end.len());
    if stood_on(at) != Ok(false) && step(&["update-ref", "--no-deref", &at.name, end, &none]) {
        return Branch::Left(Left::Stood);
    }
    Branch::Deleted
}
