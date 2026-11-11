//! How a run's end is written down (`docs/development/run.md`, "The end", "A run that was cut off"). A run that
//! `jellymoon run` saw end gets its end in its own record first ([`note`]), and only then is its wish told
//! ([`tell`]), behind the day's lock, which another change to the day may hold. So a run that ended is never
//! taken for one cut off: whoever next finds the wish still working, with nothing holding the run, gives the wish
//! the end its record has, and only a run whose record has none was cut off ([`bury`]).

use std::fs::File;
use std::path::{Path, PathBuf};
use std::time::Duration;

use super::child::{End, Ended};
use super::{Ran, RunOutcome, STAR, cut, dead, failed, finished, record, refused};
use crate::busy::or_test;
use crate::copy::{self, Outcome, RunCopy};
use crate::reading::STARS_MAX;
use crate::redact::line;
use crate::wish::{self, MadeWish};
use crate::{Error, WishState};

/// How long the run that just ended waits for the day's lock: longer than a lock can be held, since one older than
/// thirty seconds is taken as left behind (`tap.rs`). A test's build reads a shorter wait.
const WAIT: Duration = Duration::from_secs(40);

pub(super) fn lock_file(home: &Path, run: &str) -> PathBuf {
    copy::kept_at(home, run).join("run.lock")
}

/// Whether a `jellymoon run` that still lives holds the run. A lock that cannot be asked about is no lock.
fn held(home: &Path, run: &str) -> bool {
    let free = File::open(lock_file(home, run)).is_ok_and(|lock| lock.try_lock_shared().is_ok());
    !free && lock_file(home, run).is_file()
}

/// What a `jellymoon run` or a page does with a wish that still says working while nothing holds its run. It is
/// returned as it is then drawn, written down or not, never as working for ever. A run whose record has its end
/// ended while the day was locked: the wish is given that end, its star included. Any other was cut off: the
/// harness child it left is killed, if the pid in the record is still that process (`dead.rs`), and the run is
/// written down as failed, the wish first, so that nothing is written for a wish that no longer says working.
pub(crate) fn bury(home: &Path, day: &str, wish: MadeWish) -> MadeWish {
    let cut_off = |run: &String| wish.state == WishState::Working && !held(home, run);
    let Some(run) = wish.run.clone().filter(cut_off) else { return wish };
    let record = record::kept(home, &run);
    // The words go into the wish's file, which is used only when cleaning them changes nothing.
    let clean = |why: &String| !why.is_empty() && line(why, 200) == *why;
    let ended = record.as_ref().and_then(|record| Some((record.outcome?, record.why.clone().filter(clean)?)));
    let (outcome, why) = ended.clone().unwrap_or_else(|| (RunOutcome::Failed, cut(&run)));
    let mut told = tell(home, day, &wish.id, &run, outcome, &why, Duration::ZERO);
    if ended.is_none() {
        if let Some((pid, mark)) = record.and_then(|record| Some((record.pid?, record.mark?))) {
            dead::kill(pid, &mark);
        }
        told = told.and_then(|told| note(home, &run, outcome, (0, 0), &why).map(|()| told));
    }
    told.unwrap_or_else(|_| {
        let mut drawn = wish;
        apply(&mut drawn, outcome, &why);
        drawn
    })
}

/// What a child that is gone comes to: the copy is ended as completed or failed, and the record and the wish say
/// so. Finished is all of: the stream started as a run's does, one result line came, it says success and no
/// error, nothing that could not be read came after it, and the harness exited with 0.
pub(super) fn settle(
    home: &Path,
    (day, wish): (&str, &str),
    copy: &RunCopy,
    ended: &Ended,
    limit: Duration,
) -> Result<Ran, Error> {
    let read = &ended.read;
    let at_limit = format!("it was stopped at its time limit of {} seconds", limit.as_secs());
    let kind = match (ended.end, read.results, read.fine, read.unread, ended.exit_0) {
        (End::Limit, ..) => Some(at_limit.as_str()),
        (End::Over, ..) => Some("it printed more than Jellymoon reads of a run"),
        (End::Exited, ..) if read.lines == 0 => Some("Claude Code ended without a result"),
        _ if read.start != Some(true) => Some("it did not start with the tools, servers and permission mode of a run"),
        (_, 0, ..) => Some("Claude Code ended without a result"),
        (_, 2.., ..) => Some("it gave more than one result"),
        (_, _, false, ..) => Some("its result was not a success"),
        (_, _, _, true, _) => Some("a line that could not be read came after its result"),
        (.., false) => Some("Claude Code exited with a failure after its result"),
        (.., true) => None,
    };
    let run = &copy.run;
    let committed = match kind {
        None => copy::end_copy(home, run, Outcome::Completed).map_err(|_| "its work could not be committed"),
        Some(kind) => Err(kind),
    };
    let (outcome, why) = match committed {
        Ok((ended, changed)) => (RunOutcome::Completed, finished(&ended, changed)),
        Err(kind) => (RunOutcome::Failed, failed(run, kind)),
    };
    note(home, run, outcome, (read.denied, read.skipped), &why)?;
    let wait = or_test(WAIT, "JELLYMOON_TEST_END_WAIT_MS");
    tell(home, day, wish, run, outcome, &why, wait).map_err(|e| match e {
        Error::Store(e) => Error::Store(e),
        _ => refused(format!(
            "run {run} has ended and its record says how; the page of {day} was busy, and shows it when it is next drawn"
        )),
    })?;
    let star = (outcome == RunOutcome::Completed).then(|| STAR.to_string());
    Ok(Ran { run: Some(run.clone()), wish: wish.to_string(), outcome, star, denied: read.denied, why })
}

/// The run's own record takes its end, once, and a failed run's copy is marked failed, so that it is kept.
fn note(
    home: &Path,
    run: &str,
    outcome: RunOutcome,
    (denied, skipped): (usize, usize),
    why: &str,
) -> Result<(), Error> {
    if outcome != RunOutcome::Completed {
        // A copy that is marked already, or gone, is left as it is.
        let _ = copy::clean_copy(home, run, Outcome::Failed);
    }
    let Some(mut record) = record::kept(home, run).filter(|record| record.outcome.is_none()) else { return Ok(()) };
    (record.outcome, record.why, record.denied, record.skipped) =
        (Some(outcome), Some(why.to_string()), denied, skipped);
    record::write(home, run, &record)
}

/// The wish takes its run's end, behind the day's lock, waited for `wait` at least. A wish that names another run,
/// or no longer says working, has been told already, and nothing is changed.
fn tell(
    home: &Path,
    day: &str,
    wish: &str,
    run: &str,
    outcome: RunOutcome,
    why: &str,
    wait: Duration,
) -> Result<MadeWish, Error> {
    wish::change(home, day, wish, wait, |wish| {
        if wish.run.as_deref() != Some(run) || wish.state != WishState::Working {
            return Err(refused("the wish no longer names this run as going; it was not changed"));
        }
        apply(wish, outcome, why);
        Ok(())
    })
}

/// A wish as its run's end leaves it: done with one more star, or needing the person.
fn apply(wish: &mut MadeWish, outcome: RunOutcome, why: &str) {
    let done = outcome == RunOutcome::Completed;
    wish.state = if done { WishState::Done } else { WishState::NeedsYou };
    if done && wish.stars.len() < STARS_MAX {
        wish.stars.push(STAR.to_string());
    }
    wish.why = Some(why.to_string());
}

#[cfg(test)]
mod tests {
    use super::*;

    // A page can find a run's lock free in the moment after that run wrote its end. The end stands.
    #[test]
    fn a_run_that_wrote_its_end_is_not_written_down_again_as_cut_off() -> Result<(), Box<dyn std::error::Error>> {
        let (home, day, run) = (tempfile::tempdir()?, "2026-03-10", "r0123456789abcdef");
        wish::wish(home.path(), Some(day), "ship the changelog", "alpha")?;
        let done = wish::change(home.path(), day, "m1", Duration::ZERO, |wish| {
            (wish.run, wish.state) = (Some(run.to_string()), WishState::Done);
            Ok(())
        })?;
        assert!(tell(home.path(), day, "m1", run, RunOutcome::Failed, &cut(run), Duration::ZERO).is_err());
        assert_eq!(wish::kept_of(home.path(), day)?, [done]);
        Ok(())
    }
}
