//! `jellymoon run toss` of a made wish that has had no run (`docs/development/toss.md`, "A wish that has had no
//! run"; ADR 0018, "Found after F53"): the wish is let go, and no record, copy, branch or repo is read or
//! written. Whether the wish names a run is read behind the day's lock, where a run's start writes its name and
//! refuses a wish that was let go ([`open`]): of a toss and a start, the one that had that lock first is done.

use std::fs::File;
use std::path::Path;

use super::keep::{BUSY, NO_LOCK};
use super::toss::Tossed;
use super::{RunOptions, end, no_wish, refused};
use crate::copy::{self, Unkept};
use crate::wish::{self, MadeWish};
use crate::{Error, WishState, busy};

/// The line a wish that had no run is let go with.
pub(super) const LET_GO: &str = "the wish had no run, and was let go";

/// Refused: a wish that was let go. No run is started for it.
pub(super) fn open(wish: &MadeWish) -> Result<(), Error> {
    if wish.state == WishState::LetGo {
        return Err(refused(format!("wish {} was let go, so no run is started for it", wish.id)));
    }
    Ok(())
}

/// Lets go the wish `options` names, which named no run when the toss read it. `alone` is the lock of keeps and
/// tosses as the toss took it: a follow-up's start holds it until its wish names its run. Refused, with
/// nothing changed: that lock not had, and a wish that names a run by the time the day's lock is had.
pub(super) fn let_go(home: &Path, options: &RunOptions, alone: Result<File, Unkept>) -> Result<Tossed, Error> {
    let (day, id) = (wish::day_of(options.day.as_deref())?, options.wish.as_str());
    // A home no run was started in has no `runs` folder for the lock: it is made, the person's alone, and the lock is asked for again.
    let alone = alone.or_else(|unkept| match unkept {
        Unkept::NoLock if crate::ruling::private_dir(&home.join("runs")).is_ok() => copy::alone(home),
        unkept => Err(unkept),
    });
    let _alone = alone.map_err(|unkept| {
        refused(format!("{}: wish {id} was not let go", if unkept == Unkept::Busy { BUSY } else { NO_LOCK }))
    })?;
    let made = wish::kept_of(home, &day)?.into_iter().find(|made| made.id == id).ok_or_else(|| no_wish(&day))?;
    let printed = |tossed, why| Tossed { run: None, wish: id.to_string(), tossed, commit: None, why };
    if open(&made).is_err() {
        return Ok(printed(false, format!("wish {id} was let go before, and nothing was changed")));
    }
    // A test's build can wait here, with the wish read as one that names no run.
    busy::pause("JELLYMOON_TEST_TOSS_WAIT_MS");
    wish::change(home, &day, id, end::wait(), |wish| {
        if let Some(run) = &wish.run {
            let again = "nothing was changed, and `run toss` is asked again";
            return Err(refused(format!("wish {id} has a run by now, {run}: {again}")));
        }
        (wish.state, wish.why) = (WishState::LetGo, Some(LET_GO.to_string()));
        Ok(())
    })?;
    Ok(printed(true, format!("wish {id} had no run and was let go: nothing of a repo or a copy was touched")))
}
