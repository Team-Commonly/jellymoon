//! How many runs go at once (`docs/development/run.md`, "Three runs at once"). A run goes while its
//! `jellymoon run` holds the system's lock on `runs/<id>/run.lock`. A run takes that lock behind the system's lock
//! on `runs/going.lock`, where the runs going are counted first: so the count and the claim are one step, and of
//! any number of `jellymoon run` started at once no more than [`MOST`] ever hold a run.
//!
//! Behind that same lock the wish is asked first whether it has a run, and is made to name the new one last. So
//! a command that can never become a run, as its wish has one already, is told that before anything is counted;
//! and outside that lock a run's own lock is held only by a run its wish names. Commands started at once for one
//! wish are never counted against each other: whoever starts a wish four times is told of its one run.

use std::fs::{self, File, OpenOptions, TryLockError};
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

use super::end::{held, lock_file};
use super::refused;
use crate::busy::{self, or_test};
use crate::{Error, copy};

/// The most runs that go at once.
pub(super) const MOST: usize = 3;

fn full() -> Error {
    refused(format!(
        "nothing was started: {MOST} runs are going, and no more than {MOST} go at once; run the wish again when one \
         has ended"
    ))
}

/// How many runs a `jellymoon run` that lives holds. A folder of `runs` that is not named as a run is none.
fn count(home: &Path) -> usize {
    let runs = fs::read_dir(home.join("runs")).into_iter().flatten().flatten();
    let runs = runs.filter_map(|entry| entry.file_name().into_string().ok()).filter(|run| copy::is_run_id(run));
    runs.filter(|run| held(home, run)).count()
}

/// Refuses when [`MOST`] runs go already: asked before a copy is made, so a run that is one too many makes none.
/// It is asked again, and for good, at the claim.
pub(super) fn room(home: &Path) -> Result<(), Error> {
    if count(home) >= MOST { Err(full()) } else { Ok(()) }
}

/// Takes the run's own lock, which no run has yet, when fewer than [`MOST`] runs go. The count and the taking are
/// behind the system's lock on `runs/going.lock`, held for no longer than that: it is waited for as the day's lock
/// is, and never opened when something other than a plain file is at its name. `free` is asked before the count,
/// and refuses for a wish that has a run. `name` is called with the lock taken, and has the wish name the run:
/// when it fails the lock is let go of before the next command counts.
pub(super) fn claim(
    home: &Path,
    run: &str,
    free: impl FnOnce() -> Result<(), Error>,
    name: impl FnOnce() -> Result<(), Error>,
) -> Result<File, Error> {
    let door = home.join("runs").join("going.lock");
    if fs::symlink_metadata(&door).is_ok_and(|meta| !meta.is_file()) {
        return Err(refused("nothing was started: the lock runs are counted behind is not a plain file"));
    }
    let door = OpenOptions::new().write(true).create(true).truncate(false).open(&door).map_err(Error::Store)?;
    let (from, wait) = (Instant::now(), or_test(busy::WAIT, "JELLYMOON_TEST_DOOR_WAIT_MS"));
    loop {
        match door.try_lock() {
            Ok(()) => break,
            Err(TryLockError::WouldBlock) if from.elapsed() < wait => thread::sleep(Duration::from_millis(10)),
            Err(TryLockError::WouldBlock) => {
                return Err(refused("nothing was started: the lock runs are counted behind stayed held"));
            }
            Err(TryLockError::Error(e)) => return Err(Error::Store(e)),
        }
    }
    free()?;
    room(home)?;
    // A test's build can wait here, counted and not yet holding, which is where a second count would be wrong.
    busy::pause("JELLYMOON_TEST_CLAIM_WAIT_MS");
    let lock = OpenOptions::new().write(true).create_new(true).open(lock_file(home, run)).map_err(Error::Store)?;
    lock.lock().map_err(Error::Store)?;
    // And here, holding and not yet named by its wish, which is where a command must not be counted as a run.
    busy::pause("JELLYMOON_TEST_NAME_WAIT_MS");
    name()?;
    Ok(lock)
}
