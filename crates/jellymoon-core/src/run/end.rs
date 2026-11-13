//! How a run's end is written down (`docs/development/run.md`, "The end", "A run that was cut off"). A run that
//! `jellymoon run` saw end gets its end in its own record first ([`note`]), and only then is its wish told
//! ([`tell`]), behind the day's lock, which another change to the day may hold. So a run that ended is never
//! taken for one cut off: whoever next finds the wish still working, with nothing holding the run, gives the wish
//! the end its record has, and only a run whose record has none was cut off ([`bury`]). A run the person stopped
//! (`stop.rs`) ends as cancelled, by the same steps.

use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

use super::child::{End, Ended};
use super::record::Record;
use super::{Ran, RunOutcome, STAR, cancelled, cut, cut_late, dead, failed, finished, left, never, record, refused};
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
pub(super) fn soon() -> Option<Duration> {
    Some(or_test(busy::WAIT, "JELLYMOON_TEST_COPY_WAIT_MS"))
}

/// Why a run did not finish, in every fixed form but the time limit's, which names its seconds ([`at_limit`]).
const KINDS: [&str; 9] = [
    "it printed more than Jellymoon reads of a run",
    "Claude Code ended without a result",
    "it did not start with the tools, servers and permission mode of a run",
    "it gave more than one result",
    "its result was not a success",
    "a line that could not be read came after its result",
    "Claude Code exited with a failure after its result",
    "its work could not be committed",
    "its branch was moved while it worked",
];

/// How long a look that finds a run's copy gone and no end in its record reads the record again. A run that
/// finished writes its end right after its copy went, with nothing between but that one file's write (the commit
/// is over by then, behind the copies' lock the look waited for): a second is many times that, and a third of a look.
const LATE: Duration = Duration::from_secs(1);

fn at_limit(limit: Duration) -> String {
    format!("it was stopped at its time limit of {} seconds", limit.as_secs())
}

pub(super) fn lock_file(home: &Path, run: &str) -> PathBuf {
    copy::kept_at(home, run).join("run.lock")
}

/// Whether a `jellymoon run` that still lives holds the run. A lock that cannot be asked about is no lock, and
/// neither is anything but a plain file at its name, which is not opened: a named pipe would keep whoever opens it
/// waiting for ever. A run with such a lock is held by nothing, so it was cut off.
pub(super) fn held(home: &Path, run: &str) -> bool {
    let lock = lock_file(home, run);
    let plain = fs::symlink_metadata(&lock).is_ok_and(|meta| meta.is_file());
    plain && File::open(&lock).is_ok_and(|lock| lock.try_lock_shared().is_err())
}

/// The end in a run's record, when it is one core wrote for this wish, by a run or by the look that found the
/// run cut off: the words are made here again, and the record's are used only to choose among them. A record is
/// a file, and anyone who can write the home can write it: a finished run is believed only when its copy is gone
/// as a finished run's is, with its work on its branch.
pub(super) fn recorded(home: &Path, day: &str, wish: &str, run: &str, record: &Record) -> Option<(RunOutcome, String)> {
    let (outcome, said) = (record.outcome?, record.why.as_deref()?);
    if record.wish != wish || record.day != day {
        return None;
    }
    let ours: Vec<String> = match outcome {
        RunOutcome::Completed if copy::is_gone(home, run) => vec![finished(run, true), finished(run, false)],
        RunOutcome::Failed => {
            let seconds = said.split(" seconds)").next().and_then(|to| to.rsplit(' ').next()?.parse::<u64>().ok());
            let limit = seconds.map(|seconds| failed(run, &at_limit(Duration::from_secs(seconds))));
            // And the two lines of a run that was cut off, which a look writes ([`written`]) and no run does.
            KINDS.iter().map(|kind| failed(run, kind)).chain(limit).chain([cut(run), cut_late(run)]).collect()
        }
        RunOutcome::Cancelled => vec![cancelled(run)],
        _ => Vec::new(),
    };
    ours.into_iter().find(|ours| ours == said && line(ours, 200) == *ours).map(|why| (outcome, why))
}

/// Whether a run is another wish's: its record names another wish, or another day, and either that wish names the
/// run or a `jellymoon run` still holds it. The day's wishes are a file that can be changed by hand, and a wish
/// made to name the run of another is given no say over it: that run is stopped, and written down, on the word of
/// its own wish only. While its command lives the run is that command's, whatever the wishes say by then. Once
/// nothing holds it, a record that names a wish which does not name the run is the one that was changed, and the
/// run is this wish's, cut off ([`recorded`]).
pub(super) fn of_another(home: &Path, run: &str, record: Option<&Record>, day: &str, wish: &str) -> bool {
    let Some(record) = record.filter(|record| record.wish != wish || record.day != day) else { return false };
    if held(home, run) {
        return true;
    }
    left::names(home, record, run)
}

/// What a `jellymoon run` or a page does with a wish that still says working while nothing holds its run. It is
/// returned as it is then drawn, written down or not, never as working for ever. A run whose record has its end
/// ended while the day was locked: the wish is given that end, its star included. Any other was cut off, and is
/// written down as failed: its copy is marked, then the wish is told, and only for a wish that took it the record.
/// Either way the harness child the record names is killed, if the pid is still that process (`dead.rs`), and
/// the copy of a run that did not finish is marked. A run whose record says it was never started, and names no
/// child, has its copy taken back, and only then is its wish told so (`never.rs`); one that names a child did start,
/// and was cut off. A wish that names another wish's run is returned as needing the person, in one fixed line,
/// with nothing written or killed ([`of_another`]).
pub(crate) fn bury(home: &Path, day: &str, wish: MadeWish) -> MadeWish {
    let Some(run) = wish.run.clone().filter(|_| wish.state == WishState::Working) else { return wish };
    // Asked first, and whether the run is held or not: the run is not this wish's to wait for.
    if of_another(home, &run, record::kept(home, &run).as_ref(), day, &wish.id) {
        // It has no run of its own that could ever end, so it is not drawn as working. Nothing is written.
        let why = format!("run {run} is another wish's run: this wish has had no run of its own");
        return MadeWish { state: WishState::NeedsYou, why: Some(why), ..wish };
    }
    if held(home, &run) {
        return wish;
    }
    // Nothing is kept of it, not even its folder in `runs`: it was never started, and its copy was taken back
    // while the day was locked.
    if fs::symlink_metadata(copy::kept_at(home, &run)).is_err() {
        return never::unstarted(home, day, wish, &run);
    }
    // The record is read once nothing holds the run: one read before that could miss the end the run wrote.
    let record = record::kept(home, &run);
    // A test's build can wait here, with the record read, for a run to finish meanwhile ([`written`]).
    busy::pause("JELLYMOON_TEST_LOOK_WAIT_MS");
    let kill = || {
        if let Some((pid, mark)) = record.as_ref().and_then(|record| Some((record.pid?, record.mark.as_deref()?))) {
            dead::kill(pid, mark);
        }
    };
    // A record is a file anyone who can write the home can write, and one that names a child is of a run that did
    // start: its copy can hold work, and is not removed on the record's word. That run was cut off (below).
    let never = |record: &Record| record.outcome == Some(RunOutcome::NotStarted) && record.pid.is_none();
    if record.as_ref().is_some_and(never) {
        // Its `jellymoon run` said so and then could not take the copy back, or tell the wish.
        // With the lock not had in time nothing is written: the wish goes on naming the run where it is kept, so
        // the next look tries again, and it is drawn as it will be kept, with no run, never as one going. A copy
        // that cannot be taken back for any other reason (its repo is gone, git fails) never will be: what the
        // home holds of the run goes, as after a `jellymoon run` that could not take it back, and the wish can be
        // run again.
        let taken = copy::take_back(home, &run, soon());
        if taken.is_err_and(|e| copy::timed_out(&e) || !copy::remove_ours(home, &run, soon())) {
            return never::drawn(wish);
        }
        return never::unstarted(home, day, wish, &run);
    }
    // Nothing holds the run, so nobody reads its child any more, whatever the record says of an end: a record
    // can be written by hand. A child that ended is not that process any more, and is not signalled.
    kill();
    written(home, day, wish, &run, record.as_ref())
}

/// Writes down a run that nothing holds, whose record was read as `record`: the end that record has, when it is
/// believed, and else that the run was cut off.
fn written(home: &Path, day: &str, wish: MadeWish, run: &str, record: Option<&Record>) -> MadeWish {
    let end = |record: Option<&Record>| record.and_then(|record| recorded(home, day, &wish.id, run, record));
    let again = || end(record::kept(home, run).as_ref());
    // A copy that is gone is not said to be kept: its work was committed, and the run cut off after that.
    let cut_off = || (RunOutcome::Failed, if copy::is_gone(home, run) { cut_late(run) } else { cut(run) });
    let drawn = |mut wish: MadeWish, (outcome, why): &(RunOutcome, String)| {
        apply(&mut wish, *outcome, why);
        wish
    };
    let first = end(record);
    // The copy is marked before anything is written. With its lock not had in time nothing is written at all:
    // the wish is drawn as it will be kept, and the next look, which finds it as this one did, does the rest.
    // An end that is believed and is not a finished run's can have been written by hand too, over a run whose
    // own command never marked its copy, so its copy is marked as a cut off run's is.
    let finished = first.as_ref().is_some_and(|(outcome, _)| *outcome == RunOutcome::Completed);
    if !finished && !mark(home, run, soon()) {
        return drawn(wish, &first.unwrap_or_else(cut_off));
    }
    // A run whose lock could not be asked about may have been going still, and have ended since its record was
    // read: the record is read again, behind the mark, and the wish is given the end it has by now.
    let (mut ended, until) = (first.or_else(again), Instant::now() + LATE);
    // With its copy gone as a finished run's is, the run may be about to write its end: it is waited for, by
    // the rule every end is believed by. A run that never writes one was cut off there, and is written so.
    while ended.is_none() && copy::is_gone(home, run) && Instant::now() < until {
        thread::sleep(Duration::from_millis(10));
        ended = again();
    }
    let (outcome, why) = ended.clone().unwrap_or_else(cut_off);
    let mut told = tell(home, day, &wish.id, run, outcome, &why, Duration::ZERO);
    if ended.is_none() {
        told = told.and_then(|told| note(home, run, outcome, (0, 0), &why, None).map(|()| told));
    }
    told.unwrap_or_else(|_| drawn(wish, &(outcome, why)))
}

/// What a child that is gone comes to: the copy is ended as completed or failed, and the record and the wish say
/// so. Finished is all of: the stream started as a run's does, one result line came, it says success and no
/// error, nothing that could not be read came after it, and the harness exited with 0. A child that was stopped
/// for the person is a cancelled run whatever its stream holds: its copy is kept as a failed run's is.
///
/// `start` is the commit the copy was made from, as this command wrote it in the record. A run that would have
/// finished has failed when its branch is no longer at it, whoever moved the branch: nothing is committed onto
/// a moved branch and no end is written, so an end is only ever `start` or the one commit made on top of it.
pub(super) fn settle(
    home: &Path,
    (day, wish): (&str, &str),
    (copy, start): (&RunCopy, &str),
    ended: &Ended,
    limit: Duration,
) -> Result<Ran, Error> {
    let read = &ended.read;
    let at_limit = at_limit(limit);
    let kind = match (ended.end, read.results, read.fine, read.unread, ended.exit_0) {
        (End::Stopped, ..) => Some(""),
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
        None => copy::end_copy(home, run, Outcome::Completed, None, Some(start))
            .map_err(|e| if copy::moved(&e) { KINDS[8] } else { KINDS[7] }),
        Some(kind) => Err(kind),
    };
    let (outcome, why, end) = match committed {
        Ok((_, changed, end)) => (RunOutcome::Completed, finished(run, changed), end),
        Err(_) if ended.end == End::Stopped => (RunOutcome::Cancelled, cancelled(run), None),
        Err(kind) => (RunOutcome::Failed, failed(run, kind), None),
    };
    if outcome != RunOutcome::Completed {
        mark(home, run, None);
    }
    // A test's build can wait here, with a finished run's copy gone and its end not yet in its record.
    busy::pause("JELLYMOON_TEST_NOTE_WAIT_MS");
    // A record that could not be written does not keep the wish from being told.
    let noted = note(home, run, outcome, (read.denied, read.skipped), &why, end);
    let told = tell(home, day, wish, run, outcome, &why, wait()).map_err(|e| match e {
        Error::Store(e) => Error::Store(e),
        e if wish::was_locked(&e) => refused(format!(
            "run {run} has ended and its record says how; the page of {day} was busy, and shows it when it is next drawn"
        )),
        _ if told_first(home, day, wish, run) => refused(format!(
            "run {run} has ended; a look had taken it for cut off and told its wish first, so the wish shows what that look wrote"
        )),
        _ => refused(format!(
            "run {run} has ended and its record says how; the made wishes of {day} could not be used, so the wish does not show it"
        )),
    });
    told.and(noted)?;
    let star = (outcome == RunOutcome::Completed).then(|| STAR.to_string());
    Ok(Ran { run: Some(run.clone()), wish: wish.to_string(), outcome, star, denied: read.denied, why })
}

/// Whether the wish names this run and no longer says working: a look that took the run for one nothing holds
/// (its lock could not be asked about) told the wish before the run did.
fn told_first(home: &Path, day: &str, wish: &str, run: &str) -> bool {
    let told =
        |kept: &MadeWish| kept.id == wish && kept.run.as_deref() == Some(run) && kept.state != WishState::Working;
    wish::kept_of(home, day).is_ok_and(|kept| kept.iter().any(told))
}

/// The copy of a run that did not finish is marked failed, so that it is kept. One marked already, or gone, is
/// left as it is. False: the lock a copy is cleaned behind was not had within `wait`, and nothing was done.
pub(super) fn mark(home: &Path, run: &str, wait: Option<Duration>) -> bool {
    !copy::end_copy(home, run, Outcome::Failed, wait, None).is_err_and(|e| copy::timed_out(&e))
}

/// The run's own record takes its end, once, and with it, in the same writing, the commit a finished run's copy
/// `ended` as. No other end names one, and an end that stood in the record is not kept. That is not to say that
/// nothing of a run with no end was committed: a run whose commit was made and whose copy could then not be
/// removed is written failed, with no end, and its branch holds that commit. It is left so here, for `run toss`.
pub(super) fn note(
    home: &Path,
    run: &str,
    outcome: RunOutcome,
    (denied, skipped): (usize, usize),
    why: &str,
    ended: Option<String>,
) -> Result<(), Error> {
    let Some(mut record) = record::kept(home, run).filter(|record| record.outcome.is_none()) else { return Ok(()) };
    (record.outcome, record.why, record.denied, record.skipped) =
        (Some(outcome), Some(why.to_string()), denied, skipped);
    record.end = ended;
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
