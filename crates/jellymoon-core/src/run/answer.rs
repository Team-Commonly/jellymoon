//! `jellymoon run answer` (F52, ADR 0018, decision 4; `docs/development/run.md`, "An answer"): the person's
//! words to a run that asks. They pass the door a made wish passes, are kept with the wish, and are the whole
//! input of the harness, which is started in the run's own copy with a run's command line and told to go on
//! with the session the run asked in. It is the same run: its id, its branch and its record, which says the run
//! goes again, and it ends as any run ends (`end.rs`), or asks again.
//!
//! An answer holds the keeps' lock, then the lock runs are counted behind, then the run's own, and there reads
//! the record again: so of two answers one goes on, and no toss or look takes the run up meanwhile. The wish is
//! told before the record is written: an answer killed between the two leaves a record that still asks, which
//! the next look gives back to the wish (`end::bury`).

use std::path::Path;
use std::time::Duration;

use super::end::{self, bury, of_another, recorded, settle};
use super::keep::{BUSY, NO_LOCK};
use super::record::{self, Record};
use super::show::not_cores;
use super::steps::Live;
use super::{
    LIMIT, Ran, RunOptions, RunOutcome, Session, asks, cap, child, going, lined, never, no_wish, refused, stop,
};
use crate::busy::{self, or_test};
use crate::call::harness_of;
use crate::copy::{self, RunCopy};
use crate::still::{LOOKED, MARKS, Unkept, marks_only, still};
use crate::wish::{self, MadeWish};
use crate::{Error, WishState, find};

/// The most characters of an answer, once cleaned. A longer one is refused, never cut.
const ANSWER_MAX: usize = 600;

/// The most bytes of an answer that a command reads from its input: one more than the door looks at, so an
/// answer that is too long is refused as that, and never taken for a shorter one.
pub const ANSWER_READ: u64 = LOOKED as u64 + 1;

/// The words of an answer as they are kept and handed over: text, through the door, on one line, never cut.
fn cleaned(typed: &[u8]) -> Result<String, Error> {
    let long = || refused(format!("an answer is at most {ANSWER_MAX} characters, and this one is longer"));
    let text = str::from_utf8(typed).map_err(|_| refused("an answer is text, and these bytes are not"))?;
    match still(text, ANSWER_MAX, false) {
        Ok(kept) if kept.is_empty() => Err(refused("an answer needs words, and nothing is left of these once cleaned")),
        Ok(kept) if marks_only(&kept) => Err(refused(MARKS)),
        Ok(kept) => Ok(kept),
        Err(Unkept::Long) => Err(long()),
        Err(Unkept::Changing) => Err(refused(
            "an answer is kept as cleaning leaves it, and cleaning was not done with these words after 8 times",
        )),
    }
}

/// Whether an answer read from a wish's file is one the door left: a file is anyone's to write.
pub(crate) fn kept(answer: &str) -> bool {
    !answer.is_empty() && still(answer, ANSWER_MAX, false).as_deref() == Ok(answer)
}

/// What an answer holds once the run is its own to go on with.
struct Taken {
    /// The run's own lock, held until the answered run has ended.
    held: asks::Held,
    copy: RunCopy,
    /// The record and the wish as they were while the run asked, to put back when no harness could be started.
    record: Record,
    wish: MadeWish,
}

/// Answers the run of a made wish that asks, and waits for the run to end again. `typed` is the answer as it
/// was read from the command's input. Refused, with nothing started and nothing written: an answer that is no
/// text, is empty once cleaned, is over 600 characters or cannot be cleaned; a wish that is not there, has had no
/// run or names another wish's; a run that is going, that another command holds, or that does not ask; a record
/// that is not core's; a copy that is not kept as a run that asks leaves it; three runs going; no harness. A
/// harness that could not be started, or is too old to go on with a session, leaves the run asking as it did.
pub fn answer_run(home: &Path, options: &RunOptions, typed: &[u8]) -> Result<Ran, Error> {
    let answer = cleaned(typed)?;
    let day = wish::day_of(options.day.as_deref())?;
    let id = options.wish.as_str();
    let made = wish::kept_of(home, &day)?.into_iter().find(|made| made.id == id).ok_or_else(|| no_wish(&day))?;
    // A run that nothing holds is given its end first, as a stop and a page give it.
    let made = bury(home, &day, made);
    let Some(run) = made.run.clone() else {
        return Err(refused(format!("wish {id} has had no run, so there is nothing to answer")));
    };
    let on = (day.as_str(), id, run.as_str());
    asked(home, on, made.state == WishState::Working)?;
    let harness = find::named_or_found(options.harness.as_deref())?;
    let mut command = harness_of(harness).map_err(|e| refused(format!("nothing was started: {e}")))?;
    let Taken { held, copy, record, wish } = take(home, on, &answer)?;
    // Both are there, and core's: the record was believed as one of a run that asks (`asked`).
    let (session, start) = (record.session.clone().unwrap_or_default(), record.start.clone().unwrap_or_default());
    let expect = (lined(&mut command, &copy.folder, session, Session::Kept), Live::of(home, &run, &copy.folder));
    let limit = or_test(LIMIT, "JELLYMOON_TEST_RUN_LIMIT_MS");
    let (told, stop) = (|pid| record::child(home, &run, pid), || stop::asked(home, &run));
    let ended = child::run(command, format!("{answer}\n"), limit, expect, told, || record::began(home, &run), stop);
    let never = match &ended {
        Err(e) => Some(format!("{}: {e}", never::UNSTARTABLE)),
        Ok(ended) => ended.old.map(|option| {
            format!("nothing was started: this Claude Code is too old to take an answer, as it does not know {option}")
        }),
    };
    let ran = match (ended, never) {
        (Ok(ended), None) => settle(home, (&day, id), (&copy, &start), &ended, limit),
        (_, never) => {
            put_back(home, on, &record, wish);
            Err(refused(format!("{}; run {run} still asks its question", never.unwrap_or_default())))
        }
    };
    // A test's build can wait here, with the answered run ended and its lock still held.
    busy::pause("JELLYMOON_TEST_ANSWERED_WAIT_MS");
    drop(held);
    ran
}

/// The record of the wish's run, when it is core's record of a run that asks and that nothing was done with
/// since: no keep, no toss, no end. `working` is whether the wish says its run is going.
fn asked(home: &Path, (day, id, run): (&str, &str, &str), working: bool) -> Result<Record, Error> {
    let Some(record) = record::kept(home, run) else {
        return Err(refused(format!("the record of run {run} cannot be read, so nothing was started")));
    };
    if of_another(home, run, Some(&record), day, id) {
        return Err(refused(format!("wish {id} names run {run}, which is another wish's run: nothing was started")));
    }
    if working || record.outcome.is_none() {
        return Err(refused(format!("run {run} is going and asks nothing by now: nothing was started")));
    }
    let (outcome, _) = recorded(home, day, id, run, &record).ok_or_else(|| not_cores(run))?;
    if outcome != RunOutcome::Asks || record.work.is_some() {
        return Err(refused(format!("run {run} does not ask a question, so there is nothing to answer")));
    }
    let untouched = record.kept.is_none() && record.onto.is_none() && record.tossed.is_none();
    if !untouched || !matches!(record.began_at(), Ok(Some(_))) {
        return Err(not_cores(run));
    }
    Ok(record)
}

/// Makes the run this answer's to go on with: behind the keeps' lock and the lock runs are counted behind, the
/// run's own lock is taken, the record is read again and the copy looked at; then the wish says the run is going
/// and keeps the answer, a stop nothing read is taken away, and the record says the run goes, with no end.
fn take(home: &Path, on: (&str, &str, &str), answer: &str) -> Result<Taken, Error> {
    let (day, id, run) = on;
    // A test's build can wait here, with the record read once and no lock asked for yet.
    busy::pause("JELLYMOON_TEST_ANSWER_LOCK_WAIT_MS");
    let _alone = copy::alone(home).map_err(|unheld| {
        let why = if unheld == copy::Unkept::Busy { BUSY } else { NO_LOCK };
        refused(format!("{why}: nothing of run {run} was answered"))
    })?;
    cap::behind(home, "answer again", || {
        let held = asks::take(home, run).map_err(|going| {
            let why = if going { "another command holds run" } else { "the lock of run" };
            let then = if going { "" } else { " could not be made or taken" };
            refused(format!("{why} {run}{then}: nothing was started"))
        })?;
        let record = asked(home, on, false)?;
        let copy = copy::left_asking(home, run).ok_or_else(|| {
            refused(format!(
                "the copy of run {run} is not kept as a run that asks leaves it, so it cannot be answered: `run toss` \
                 lets the wish go"
            ))
        })?;
        let mut before = None;
        let told = wish::change(home, day, id, Duration::ZERO, |wish| {
            if wish.run.as_deref() != Some(run) || wish.state != WishState::NeedsYou || wish.question.is_none() {
                return Err(refused(format!("the wish no longer asks for run {run}: nothing was started")));
            }
            before = Some(wish.clone());
            (wish.state, wish.why, wish.question) = (WishState::Working, Some(going(run)), None);
            wish.answer = Some(answer.to_string());
            Ok(())
        })?;
        // A test's build can wait here, with the wish told and the record still asking.
        busy::pause("JELLYMOON_TEST_ANSWER_WAIT_MS");
        stop::forget(home, run);
        let (outcome, why, question, pid, mark) = (None, None, None, None, None);
        let goes = Record { outcome, why, question, pid, mark, began: false, denied: 0, skipped: 0, ..record.clone() };
        record::write(home, run, &goes)?;
        Ok(Taken { held, copy, record, wish: before.unwrap_or(told) })
    })
}

/// The run asks again as it did, after an answer no harness could be started for: the record first, as an end
/// is written, then the wish, when it still says this run is going.
fn put_back(home: &Path, (day, id, run): (&str, &str, &str), record: &Record, before: MadeWish) {
    let _ = record::write(home, run, record);
    let _ = wish::change(home, day, id, end::wait(), |wish| {
        if wish.run.as_deref() == Some(run) && wish.state == WishState::Working {
            *wish = before;
        }
        Ok(())
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_answer_is_text_cleaned_on_one_line_of_at_most_600_characters_and_never_cut() {
        let key = format!("AI{}{}", "za", "kLmNo".repeat(7));
        let said = |typed: &[u8]| cleaned(typed).map_err(|e| e.to_string());
        assert_eq!(said(b"the old one,\n in \"src\"\r\n"), Ok("the old one, in 'src'".to_string()));
        assert_eq!(said(format!("use {key}").as_bytes()), Ok("use [redacted]".to_string()));
        assert_eq!(said("w".repeat(ANSWER_MAX).as_bytes()).map(|kept| kept.len()), Ok(ANSWER_MAX));
        let long = "an answer is at most 600 characters, and this one is longer";
        for over in ["w".repeat(ANSWER_MAX + 1), " ".repeat(LOOKED + 1)] {
            assert!(said(over.as_bytes()).is_err_and(|why| why.ends_with(long)));
        }
        assert!(
            said(b"the old \xff one").is_err_and(|why| why.ends_with("an answer is text, and these bytes are not"))
        );
        for empty in [&b""[..], b" \n\t", "\u{1b}\u{200b}".as_bytes()] {
            assert!(said(empty).is_err_and(|why| why.ends_with("nothing is left of these once cleaned")));
        }
        assert!(said(key.repeat(9).as_bytes()).is_err_and(|why| why.ends_with("after 8 times")));
        // What the door left is kept as it is, and nothing else read from a file is.
        assert!(kept("the old one, in 'src'") && kept("use [redacted]") && kept(&"w".repeat(ANSWER_MAX)));
        for odd in ["", " the old one", "the old one\n", "in \"src\"", key.as_str(), &"w".repeat(ANSWER_MAX + 1)] {
            assert!(!kept(odd), "{odd:?}");
        }
    }
}
