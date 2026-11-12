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

pub(super) const OLD: &str = "nothing was started: this Claude Code is too old for runs, as it does not know \
                              --restricted; update Claude Code and run the wish again";
/// What the system said of it follows this where `jellymoon run` prints it, and is not kept with the wish.
pub(super) const UNSTARTABLE: &str = "nothing was started: Claude Code could not be started";
/// What a look writes for a run whose command could not tell its wish which of the two it was.
pub(super) const UNTOLD: &str = "nothing was started: Claude Code is too old for runs or could not be started; \
                                 run the wish again when that is mended";

/// The wish as a run that was never started leaves it.
pub(super) fn mark(wish: &mut MadeWish, why: &str) {
    (wish.run, wish.why, wish.state) = (None, Some(why.to_string()), WishState::NeedsYou);
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
