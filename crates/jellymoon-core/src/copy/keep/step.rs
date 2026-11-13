//! The one step of a keep that touches the person's checkout, and what is found after it (`keep.rs`;
//! `docs/development/keep.md`, "The steps" and "A checkout moved just before the step"). git moves whatever
//! `HEAD` names at the moment it runs, which the checks before it cannot hold still: so after the step git is
//! asked where the commit is, and the keep says that and nothing else.

use super::super::work::tip_of;
use super::{At, How, Landing, Note, SAID_MAX, Unkept, fits, line, nameable, reaches, runs};

/// What a record's `onto` says of work that landed on no branch a record names.
pub(crate) const LOOSE: &str = "HEAD";

/// The step. Each word after `--ff-only` takes away something a repo's config could add to a merge
/// (`branch.<name>.mergeOptions`, `merge.autoStash`, `merge.verifySignatures`), keeps git from tidying the store
/// afterwards, or makes an ignored file in the way a reason to stop, as an untracked one is. Hooks, filters,
/// signing and submodules are off for every git (`git.rs`).
const FORWARD: [&str; 14] = [
    "-c",
    "gc.auto=0",
    "-c",
    "maintenance.auto=false",
    "merge",
    "--ff-only",
    "--no-squash",
    "--no-autostash",
    "--no-verify-signatures",
    "--no-overwrite-ignore",
    "--no-stat",
    "--no-edit",
    "--no-progress",
    "--quiet",
];

/// Where a test's build ends the command, as a power cut would, when `JELLYMOON_TEST_KEEP_CUT` is this step's
/// number. A release holds none of this: the variable is not read there.
pub(super) fn cut(step: u8) {
    #[cfg(debug_assertions)]
    if std::env::var("JELLYMOON_TEST_KEEP_CUT").is_ok_and(|asked| asked == step.to_string()) {
        std::process::exit(70);
    }
    #[cfg(not(debug_assertions))]
    let _ = step;
}

/// Writes down what is about to land, and lands it: the one step that touches the person's checkout and moves
/// their branch. git checks every file before it writes one; what it cannot check (a folder it may not write
/// to) stops it part way, and then the keep says so ([`Unkept::Stepped`]). A step git stopped in a checkout that
/// by then stands on a run's branch is said as that ([`Unkept::OnRun`]): git could not move that branch forward.
pub(super) fn forward(
    at: &At,
    commit: String,
    onto: String,
    how: How,
    note: &mut dyn FnMut(Note) -> bool,
) -> Result<Landing, Unkept> {
    cut(1);
    if !note(Note::Intent(&commit, &onto)) {
        return Err(Unkept::Unwritten);
    }
    cut(2);
    let forward: Vec<&str> = FORWARD.iter().copied().chain([commit.as_str()]).collect();
    let moved = at.mine.answer(Some(at.of.folder), &[], &forward, &[], SAID_MAX);
    cut(3);
    match moved {
        Some((Some(0), _)) => landed(at, commit, onto, how),
        _ if fits::locked(at.own) => Err(Unkept::Locked),
        _ if stood(at).is_some_and(|name| runs(&name)) => Err(Unkept::OnRun),
        _ => Err(Unkept::Stepped),
    }
}

/// The branch the checkout's `HEAD` names by now, asked through the checkout's own git folder: a linked
/// worktree's `HEAD` is its own.
fn stood(at: &At) -> Option<String> {
    line(at.mine, &[], &["symbolic-ref", "--quiet", "HEAD"]).ok().flatten()
}

/// Where the commit is after a step git said it took. On the branch the keep meant, it landed as meant. If not,
/// git acted on what `HEAD` named by then, and `HEAD` is asked. On a run's branch nothing was kept, whatever git
/// did: said as kept there, the next keep would delete the branch the work is on. At the commit, the work is in
/// the checkout and is kept where it is: on the branch `HEAD` names, when a record holds that name, and
/// otherwise on no branch that is named. Ahead of the commit on such a branch, git had nothing to do: that
/// branch held the work already. Anywhere else nothing is known to have landed.
fn landed(at: &At, commit: String, onto: String, how: How) -> Result<Landing, Unkept> {
    if tip_of(at.repo, &onto).ok().as_deref() == Some(commit.as_str()) {
        return Ok(Landing { commit: Some(commit), onto, how });
    }
    let stood = stood(at);
    if stood.as_deref().is_some_and(runs) {
        return Err(Unkept::OnRun);
    }
    let head = line(at.mine, &[], &["rev-parse", "--verify", "--quiet", "HEAD^{commit}"]);
    let head = head.ok().flatten().ok_or(Unkept::Stepped)?;
    let (onto, how) = match stood.filter(|name| nameable(name)) {
        Some(name) if head == commit => (name, How::Elsewhere),
        None if head == commit => (LOOSE.to_string(), How::Loose),
        Some(name) if reaches(at, &commit, &head) == Ok(true) => (name, How::Held),
        _ => return Err(Unkept::Stepped),
    };
    Ok(Landing { commit: Some(commit), onto, how })
}
