//! How a run's end is written down (`docs/development/run.md`, "The end", "A run that was cut off"). A run that
//! `jellymoon run` saw end gets its end in its own record first ([`note`]), and only then is its wish told
//! ([`tell`]), behind the day's lock, which another change to the day may hold. So a run that ended is never
//! taken for one cut off: whoever next finds the wish still working, with nothing holding the run, gives the wish
//! the end its record has, and only a run whose record has none was cut off ([`bury`]).

use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::time::Duration;

use super::child::{End, Ended};
use super::record::Record;
use super::{Ran, RunOutcome, STAR, cut, dead, failed, finished, record, refused};
use crate::busy::{self, or_test};
use crate::copy::{self, Outcome, RunCopy};
use crate::reading::STARS_MAX;
use crate::redact::line;
use crate::wish::{self, MadeWish};
use crate::{Error, WishState};

/// How long the run that just ended waits for the day's lock: longer than a lock can be held, since one older than
/// thirty seconds is taken as left behind (`tap.rs`). A test's build reads a shorter wait.
const WAIT: Duration = Duration::from_secs(40);

/// The wait for the day's lock of a `jellymoon run` that has something to write down.
pub(super) fn wait() -> Duration {
    or_test(WAIT, "JELLYMOON_TEST_END_WAIT_MS")
}

/// How long a look waits for the lock a copy is made and cleaned behind, which another command may hold for as
/// long as its git children take: the day's lock's own wait. A page never waits behind another command for more.
fn soon() -> Option<Duration> {
    Some(or_test(busy::WAIT, "JELLYMOON_TEST_COPY_WAIT_MS"))
}

/// Why a run did not finish, in every fixed form but the time limit's, which names its seconds ([`at_limit`]).
const KINDS: [&str; 8] = [
    "it printed more than Jellymoon reads of a run",
    "Claude Code ended without a result",
    "it did not start with the tools, servers and permission mode of a run",
    "it gave more than one result",
    "its result was not a success",
    "a line that could not be read came after its result",
    "Claude Code exited with a failure after its result",
    "its work could not be committed",
];

fn at_limit(limit: Duration) -> String {
    format!("it was stopped at its time limit of {} seconds", limit.as_secs())
}

pub(super) fn lock_file(home: &Path, run: &str) -> PathBuf {
    copy::kept_at(home, run).join("run.lock")
}

/// Whether a `jellymoon run` that still lives holds the run. A lock that cannot be asked about is no lock.
fn held(home: &Path, run: &str) -> bool {
    let free = File::open(lock_file(home, run)).is_ok_and(|lock| lock.try_lock_shared().is_ok());
    !free && lock_file(home, run).is_file()
}

/// The end in a run's record, when it is one core wrote for this wish: the words are made here again, and the
/// record's are used only to choose among them. A record is a file, and anyone who can write the home can write
/// it: a finished run is believed only when its copy is gone as a finished run's is, with its work on its branch.
fn recorded(home: &Path, day: &str, wish: &str, run: &str, record: &Record) -> Option<(RunOutcome, String)> {
    let (outcome, said) = (record.outcome?, record.why.as_deref()?);
    if record.wish != wish || record.day != day {
        return None;
    }
    let ours: Vec<String> = match outcome {
        RunOutcome::Completed if copy::is_gone(home, run) => {
            let copy = RunCopy { run: run.to_string(), folder: String::new(), branch: copy::branch(run), kept: false };
            vec![finished(&copy, true), finished(&copy, false)]
        }
        RunOutcome::Failed => {
            let seconds = said.split(" seconds)").next().and_then(|to| to.rsplit(' ').next()?.parse::<u64>().ok());
            let limit = seconds.map(|seconds| failed(run, &at_limit(Duration::from_secs(seconds))));
            KINDS.iter().map(|kind| failed(run, kind)).chain(limit).collect()
        }
        _ => Vec::new(),
    };
    ours.into_iter().find(|ours| ours == said && line(ours, 200) == *ours).map(|why| (outcome, why))
}

/// A wish that names a run of which nothing is kept, not even its folder in `runs`: the run was never started
/// and its copy was taken back while the day was locked (`run`). The wish is made as it was, to be run again.
fn unstarted(home: &Path, day: &str, wish: MadeWish, run: &str) -> MadeWish {
    let undo = |wish: &mut MadeWish| {
        if wish.run.as_deref() == Some(run) && wish.state == WishState::Working {
            (wish.run, wish.why) = (None, None);
        }
        Ok(())
    };
    wish::change(home, day, &wish.id, Duration::ZERO, undo).unwrap_or(MadeWish { run: None, why: None, ..wish })
}

/// What a `jellymoon run` or a page does with a wish that still says working while nothing holds its run. It is
/// returned as it is then drawn, written down or not, never as working for ever. A run whose record has its end
/// ended while the day was locked: the wish is given that end, its star included. Any other was cut off: the
/// harness child it left is killed, if the pid in the record is still that process (`dead.rs`), and the run is
/// written down as failed: its copy is marked, then the wish is told, and only for a wish that took it the record.
/// A run whose record says it was never started has the child it names killed the same way, then its copy taken back.
pub(crate) fn bury(home: &Path, day: &str, wish: MadeWish) -> MadeWish {
    let cut_off = |run: &String| wish.state == WishState::Working && !held(home, run);
    let Some(run) = wish.run.clone().filter(cut_off) else { return wish };
    if fs::symlink_metadata(copy::kept_at(home, &run)).is_err() {
        return unstarted(home, day, wish, &run);
    }
    let record = record::kept(home, &run);
    let kill = || {
        if let Some((pid, mark)) = record.as_ref().and_then(|record| Some((record.pid?, record.mark.as_deref()?))) {
            dead::kill(pid, mark);
        }
    };
    if record.as_ref().is_some_and(|record| record.outcome == Some(RunOutcome::NotStarted)) {
        // Its `jellymoon run` said so and then could not take the copy back, or tell the wish. A record is a file
        // anyone who can write the home can write: the child it names is stopped before its copy goes.
        kill();
        let _ = copy::take_back(home, &run, soon());
        return unstarted(home, day, wish, &run);
    }
    let ended = record.as_ref().and_then(|record| recorded(home, day, &wish.id, &run, record));
    let (outcome, why) = ended.clone().unwrap_or_else(|| (RunOutcome::Failed, cut(&run)));
    let drawn = |mut wish: MadeWish| {
        apply(&mut wish, outcome, &why);
        wish
    };
    if ended.is_none() {
        kill();
        // The copy is marked before anything is written. With its lock not had in time nothing is written at all:
        // the wish is drawn as it will be kept, and the next look, which finds it as this one did, does the rest.
        if !mark(home, &run, soon()) {
            return drawn(wish);
        }
    }
    let mut told = tell(home, day, &wish.id, &run, outcome, &why, Duration::ZERO);
    if ended.is_none() {
        told = told.and_then(|told| note(home, &run, outcome, (0, 0), &why).map(|()| told));
    }
    told.unwrap_or_else(|_| drawn(wish))
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
    let at_limit = at_limit(limit);
    let kind = match (ended.end, read.results, read.fine, read.unread, ended.exit_0) {
        (End::Limit, ..) => Some(at_limit.as_str()),
        (End::Over, ..) => Some(KINDS[0]),
        (End::Exited, ..) if read.lines == 0 => Some(KINDS[1]),
        _ if read.start != Some(true) => Some(KINDS[2]),
        (_, 0, ..) => Some(KINDS[1]),
        (_, 2.., ..) => Some(KINDS[3]),
        (_, _, false, ..) => Some(KINDS[4]),
        (_, _, _, true, _) => Some(KINDS[5]),
        (.., false) => Some(KINDS[6]),
        (.., true) => None,
    };
    let run = &copy.run;
    let committed = match kind {
        None => copy::end_copy(home, run, Outcome::Completed, None).map_err(|_| KINDS[7]),
        Some(kind) => Err(kind),
    };
    let (outcome, why) = match committed {
        Ok((ended, changed)) => (RunOutcome::Completed, finished(&ended, changed)),
        Err(kind) => (RunOutcome::Failed, failed(run, kind)),
    };
    if outcome != RunOutcome::Completed {
        mark(home, run, None);
    }
    // A record that could not be written does not keep the wish from being told.
    let noted = note(home, run, outcome, (read.denied, read.skipped), &why);
    let told = tell(home, day, wish, run, outcome, &why, wait()).map_err(|e| match e {
        Error::Store(e) => Error::Store(e),
        e if wish::was_locked(&e) => refused(format!(
            "run {run} has ended and its record says how; the page of {day} was busy, and shows it when it is next drawn"
        )),
        _ => refused(format!(
            "run {run} has ended and its record says how; the made wishes of {day} could not be used, so the wish does not show it"
        )),
    });
    told.and(noted)?;
    let star = (outcome == RunOutcome::Completed).then(|| STAR.to_string());
    Ok(Ran { run: Some(run.clone()), wish: wish.to_string(), outcome, star, denied: read.denied, why })
}

/// A failed run's copy is marked failed, so that it is kept. One marked already, or gone, is left as it is. False:
/// the lock a copy is cleaned behind was not had within `wait`, and nothing was done.
fn mark(home: &Path, run: &str, wait: Option<Duration>) -> bool {
    !copy::end_copy(home, run, Outcome::Failed, wait).is_err_and(|e| copy::timed_out(&e))
}

/// The run's own record takes its end, once.
fn note(
    home: &Path,
    run: &str,
    outcome: RunOutcome,
    (denied, skipped): (usize, usize),
    why: &str,
) -> Result<(), Error> {
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
