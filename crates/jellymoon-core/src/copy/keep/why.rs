//! Why a keep did not keep a run's work, and how one did (`keep.rs`). `run/keep.rs` has the words of each.

use super::Unread;

/// Why a run's work was not kept.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Unkept {
    /// The repo or the run's branch is not as the record names it (`work.rs`).
    Unread(Unread),
    /// Another keep of this home is at work.
    Busy,
    /// The keeps' lock could not be made or taken.
    NoLock,
    /// The repo's config names what cannot be switched off on a command line (`git.rs`).
    Config,
    /// A checkout stands on the run's branch.
    Stood,
    /// The person's checkout stands on no branch of theirs.
    NoBranch,
    /// The checkout stands on a branch whose name a record does not hold (`nameable`).
    Unnamed,
    /// The checkout's `HEAD` names a branch the repo lists under no such name, and yet is at a commit: on a disk
    /// that takes two spellings for one name, it is another spelling of a branch's name.
    Spelled,
    /// A merge, a cherry-pick, a revert, a rebase or a bisect is going on in the checkout.
    Midway,
    /// The branch's history does not hold the run's start.
    History,
    /// The work and the person's commits must be merged, and this git cannot do that in the store.
    Old,
    /// Both changed the same lines, or the same file in ways git does not merge.
    Clash,
    /// git ended the merge in the store with an error: the store takes no new object, or git did not answer.
    Unmerged,
    /// A commit must be made, and the repo names no author.
    Nobody,
    /// The work adds, changes or removes another repo's entry or `.gitmodules`.
    Repos,
    /// The work touches a file the person's tools obey, and that was not asked for.
    Settings,
    /// A file of the work has a name the disk does not take.
    Name,
    /// A file of the work would be passed through a filter of the repo's.
    Filter,
    /// A changed, an untracked or an ignored file of the person's is where the work's would go.
    InWay,
    /// The checkout's index is locked.
    Locked,
    /// git stopped the step though every check had passed, or took it and the commit is not where `HEAD` is.
    Stepped,
    /// The checkout was moved onto a run's branch just before the step: no work is ever kept there (`runs`).
    OnRun,
    /// The run's record could not be written.
    Unwritten,
}

impl From<Unread> for Unkept {
    fn from(unread: Unread) -> Unkept {
        Unkept::Unread(unread)
    }
}

/// How the work came to be on the branch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum How {
    /// The branch was at the run's start and was moved forward to the run's commit.
    Forward,
    /// One new commit on top of the person's holds their work and the run's.
    Joined,
    /// The branch held the run's commit already.
    Held,
    /// A keep that was cut off had landed the commit its record names.
    Finished,
    /// The checkout was moved to another branch just before the step, and git landed the commit there.
    Elsewhere,
    /// The checkout was moved just before the step, and git landed the commit on no branch a record names: its
    /// `HEAD` was detached, or stood on a branch whose name is not written down. The run's branch is left.
    Loose,
}
