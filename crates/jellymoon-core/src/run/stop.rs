//! A stop the person asks for (F40, `docs/development/run.md`, "A stop the person asks for"). The harness child is
//! owned by the `jellymoon run` that started it, which is another process, so [`stop_run`] leaves a request in the
//! run's own folder, `runs/<id>/stop`, and that `jellymoon run` finds it while it waits for its child ([`asked`]):
//! it interrupts the child, signals it, kills it (`child.rs`), and writes the run down as cancelled, as it writes
//! any end (`end.rs`). [`stop_run`] then waits for that end and prints it.
//!
//! The request is a file with nothing in it, and only its being there is read. So one made by hand can do what a
//! stop does and nothing more: it stops the run whose folder it is in. Nobody reads it once that run has ended.

use std::fs::{self, OpenOptions};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

use super::end::{bury, of_another, recorded};
use super::{Ran, RunOutcome, STAR, cut, no_wish, record, refused};
use crate::busy::or_test;
use crate::wish::{self, MadeWish};
use crate::{Error, WishState, copy};

/// How long a stop waits for the run to write its end down: the five seconds after the interrupt and after the
/// signal, the two waits of five for what the child printed, the forty for the day's lock, and five more. A test's
/// build reads a shorter wait.
const WAIT: Duration = Duration::from_secs(65);

fn file(home: &Path, run: &str) -> PathBuf {
    copy::kept_at(home, run).join("stop")
}

/// Whether a stop was asked for this run: anything at all is at the request's name.
pub(super) fn asked(home: &Path, run: &str) -> bool {
    fs::symlink_metadata(file(home, run)).is_ok()
}

/// Leaves the request, for the person alone to write. It is made only as a new file in the run's folder, which is
/// never made here: a run whose folder is gone was never started, and stays so. A request that is there already
/// is the same request.
fn ask(home: &Path, run: &str) -> Result<(), Error> {
    let mut open = OpenOptions::new();
    open.write(true).create_new(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut open, 0o600);
    match open.open(file(home, run)) {
        Err(e) if e.kind() != ErrorKind::NotFound && !asked(home, run) => Err(Error::Store(e)),
        _ => Ok(()),
    }
}

/// Stops the run of a made wish, and returns how that run ended, once it has. Refused, with nothing written: a
/// wish that is not there, one that names no run, and one that names a run whose record names another wish. A run that had ended is returned as it ended, and nothing is
/// changed. A run that was cut off is written down as one, with the child it left killed (`end::bury`). A run that
/// is going is asked to stop, and waited for: one that has not ended when the wait is over is refused in one line,
/// with the request left for it.
pub fn stop_run(home: &Path, wish: &str, day: Option<&str>) -> Result<Ran, Error> {
    let day = wish::day_of(day)?;
    let deadline = Instant::now() + or_test(WAIT, "JELLYMOON_TEST_STOP_WAIT_MS");
    loop {
        let made = wish::kept_of(home, &day)?.into_iter().find(|made| made.id == wish);
        // A run that nothing holds any more is given its end here, as a page gives it.
        let made = bury(home, &day, made.ok_or_else(|| no_wish(&day))?);
        let Some(run) = made.run.clone() else {
            return Err(refused(format!("wish {wish} has no run going, so nothing was stopped")));
        };
        if of_another(record::kept(home, &run).as_ref(), &day, wish) {
            let why = "which is another wish's run, so nothing was stopped";
            return Err(refused(format!("wish {wish} names run {run}, {why}")));
        }
        if made.state != WishState::Working {
            return Ok(ended(home, &day, &made, &run));
        }
        ask(home, &run)?;
        if Instant::now() >= deadline {
            let why = "was asked to stop and has not ended yet; the page shows its end when it has";
            return Err(refused(format!("run {run} {why}")));
        }
        thread::sleep(Duration::from_millis(20));
    }
}

/// A run that has ended, as `jellymoon run` printed it: the end in its record when that is core's and fits the
/// wish, else what the wish says. The star is the wish's. The count of what was denied is the record's only with
/// its end: of a record whose end is not believed, nothing is printed.
fn ended(home: &Path, day: &str, wish: &MadeWish, run: &str) -> Ran {
    let kept = record::kept(home, run);
    let done = wish.state == WishState::Done;
    let end = kept.as_ref().and_then(|kept| recorded(home, day, &wish.id, run, kept));
    let end = end.filter(|(outcome, _)| (*outcome == RunOutcome::Completed) == done);
    let denied = end.as_ref().and(kept).map_or(0, |kept| kept.denied);
    let (outcome, why) = end.unwrap_or_else(|| {
        let outcome = if done { RunOutcome::Completed } else { RunOutcome::Failed };
        (outcome, wish.why.clone().unwrap_or_else(|| cut(run)))
    });
    let star = done.then(|| STAR.to_string());
    Ran { run: Some(run.to_string()), wish: wish.id.clone(), outcome, star, denied, why }
}
