//! A run that was never started, after its wish had named it (`docs/development/run.md`, "A run that was never
//! started"): the harness is too old for runs, or could not be started. The wish names no run then, and can be run
//! again. It is not left as it was made: it says why in one of three fixed lines and needs the person, so a page
//! drawn from the wish alone shows that nothing is at work for it, and whoever asked for the run is told the same
//! line the wish keeps (`apart.rs`).

use std::fs::File;
use std::path::Path;
use std::time::Duration;

use super::{end, record};
use crate::wish::{self, MadeWish};
use crate::{Error, WishState, copy};

/// Of a harness that refused a run's command line for `option`, one it does not know (`child.rs`).
pub(super) fn old(option: &str) -> String {
    format!(
        "nothing was started: this Claude Code is too old for runs, as it does not know {option}; update Claude Code \
         and run the wish again"
    )
}
/// What the system said of it follows this where `jellymoon run` prints it, and is not kept with the wish.
pub(super) const UNSTARTABLE: &str = "nothing was started: Claude Code could not be started";
/// What a look writes for a run whose command could not tell its wish which of the two it was.
pub(super) const UNTOLD: &str = "nothing was started: Claude Code is too old for runs or could not be started; \
                                 run the wish again when that is mended";

/// The wish as a run that was never started leaves it.
pub(super) fn mark(wish: &mut MadeWish, why: &str) {
    (wish.run, wish.why, wish.state, wish.question) = (None, Some(why.to_string()), WishState::NeedsYou, None);
}

/// What `jellymoon run` does when the harness it started for a run never began one: the wish names no run, says
/// `why` and needs the person, and can be run again. The day is waited for as a run's end waits. Whatever came
/// of that, the record says the run was never started and the copy is taken back, both while the run's locks
/// are `held`: so no look ever finds a run that nothing holds and that has no end. A wish that still names the
/// run is told by the next look (`end::bury`).
pub(super) fn undo(
    home: &Path,
    (day, wish): (&str, &str),
    run: &str,
    why: &str,
    held: (File, File),
) -> Result<(), Error> {
    let undone = wish::change(home, day, wish, end::wait(), |wish| {
        mark(wish, why);
        Ok(())
    });
    record::unstarted(home, run);
    let taken = copy::take_back(home, run, None);
    drop(held);
    // Where a file that is open cannot be removed, it can be now. Where git cannot be asked (the repo was moved
    // away), what Jellymoon's own home holds of the run goes all the same.
    let taken = taken.or_else(|_| copy::take_back(home, run, None));
    taken.inspect_err(|_| _ = copy::remove_ours(home, run, None))?;
    undone.map(|_| ())
}

/// The wish as [`unstarted`] will keep it, for a look that could not write it yet.
pub(super) fn drawn(mut wish: MadeWish) -> MadeWish {
    mark(&mut wish, UNTOLD);
    wish
}

/// A wish that still names a run that was never started, of which nothing is kept or only the word that it never
/// started: its command took the copy back and could not tell the wish, as the day was locked. The look tells it.
pub(super) fn unstarted(home: &Path, day: &str, wish: MadeWish, run: &str) -> MadeWish {
    let undo = |wish: &mut MadeWish| {
        if wish.run.as_deref() == Some(run) && wish.state == WishState::Working {
            mark(wish, UNTOLD);
        }
        Ok(())
    };
    wish::change(home, day, &wish.id, Duration::ZERO, undo).unwrap_or_else(|_| drawn(wish))
}

#[cfg(test)]
mod tests {
    use super::*;

    // A look reads a wish, and the wish can change before the look writes: the wish as it is kept decides. One
    // that names the run and is at work no more has had its end, and one at work for another run is that run's.
    #[test]
    fn a_wish_is_told_no_run_was_started_only_while_it_names_that_run_and_is_at_work() -> Result<(), Error> {
        let (home, day, run) = (tempfile::tempdir().map_err(Error::Store)?, "2026-03-10", "r0123456789abcdef");
        wish::wish(home.path(), Some(day), "ship the changelog", "alpha")?;
        let kept_as = |named: &str, state: WishState| {
            wish::change(home.path(), day, "m1", Duration::ZERO, |wish| {
                (wish.run, wish.why, wish.state) = (Some(named.to_string()), None, state);
                Ok(())
            })
        };
        let read = kept_as(run, WishState::Working)?;
        for (named, state) in [(run, WishState::Done), ("r00000000000000aa", WishState::Working)] {
            let kept = kept_as(named, state)?;
            assert_eq!(unstarted(home.path(), day, read.clone(), run), kept, "{named} {state:?}");
            assert_eq!(wish::kept_of(home.path(), day)?, [kept]);
        }
        kept_as(run, WishState::Working)?;
        let told = unstarted(home.path(), day, read, run);
        assert_eq!((told.run.as_deref(), told.why.as_deref(), told.state), (None, Some(UNTOLD), WishState::NeedsYou));
        assert_eq!(wish::kept_of(home.path(), day)?, [told]);
        Ok(())
    }
}
