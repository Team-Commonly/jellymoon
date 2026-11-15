//! What a keep did, or with `--ask` would do, in core's fixed words (`keep.rs`).

use crate::copy::How;

/// What a keep did, or would do. `own` is whether what landed is the run's own commit.
pub(super) fn done(run: &str, how: How, ask: bool, own: bool) -> String {
    match (how, ask) {
        (How::Forward, false) => format!("run {run} was kept: the branch was moved forward to the run's commit"),
        (How::Forward, true) => format!("a keep of run {run} would move the branch forward to the run's commit"),
        (How::Joined, false) => {
            format!("run {run} was kept: one new commit on top of the branch's own holds its work and the run's")
        }
        (How::Joined, true) => format!(
            "a keep of run {run} would land one new commit on top of the branch's own, holding its work and the run's"
        ),
        (How::Held, false) => {
            format!("run {run} was kept: the branch held the run's work already, and nothing was landed")
        }
        (How::Held, true) => {
            format!("a keep of run {run} would land nothing: the branch holds the run's work already")
        }
        (How::Finished, false) => {
            format!("run {run} was kept: a keep that was cut off had landed its work, and this one finished it")
        }
        (How::Finished, true) => {
            format!("a keep of run {run} would finish one that was cut off after it had landed the work")
        }
        // Only a keep that took the step finds the checkout moved: nothing is asked that would.
        (How::Elsewhere, _) => format!(
            "run {run} was kept, on another branch than the keep began on: the checkout was moved to the branch \
             that `onto` names while the keep was at work, and git landed the work there"
        ),
        (How::Loose, _) => format!(
            "run {run} was kept, on no branch that is named here: while the keep was at work the checkout's HEAD \
             was detached, or moved to a branch whose name Jellymoon does not write down or the repo does not \
             list under that spelling, and git landed the work there; the run's branch is left and still holds \
             the run's own commit, {}: `kept` is its id",
            if own {
                "which is the commit git landed"
            } else {
                "and the new commit the keep made to join the work with the person's commits is named by nothing \
                 of Jellymoon's"
            }
        ),
    }
}
