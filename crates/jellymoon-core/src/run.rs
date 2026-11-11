//! A run (F39, ADR 0014, `docs/development/run.md`): the person's own harness, started headless on one made wish, in
//! a copy of the wish's repo (`copy.rs`). [`run`] finds the harness, then the repo, makes the copy, starts the
//! harness there with a fixed command line and the wish's kept words on its input, and waits. A run has finished
//! only when its result line says success, says no error, and the harness exited with 0; then its work is one
//! commit on its branch and the wish has one more star. Every other end is a failed run: the copy is kept, no star.
//!
//! A made wish gets one run. Its id and how it stands are kept with the wish, in fixed words; the run's own record
//! is `runs/<id>/run.json`. While `jellymoon run` lives it holds the system's lock on `runs/<id>/run.lock`, which
//! the system drops when the process ends, however it ends: a wish whose run is not locked and not ended was cut
//! off. The next `jellymoon run` or page that finds it stops the harness child it left, if that is still the same
//! process, and writes the run down as failed ([`bury`]).

mod child;
mod dead;
mod record;

use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::{env, fmt};

use child::{End, Ended};
use record::Record;
use serde::{Deserialize, Serialize};

use crate::call::harness_in;
use crate::copy::{self, Outcome, RunCopy};
use crate::reading::STARS_MAX;
use crate::wish::{self, MadeWish};
use crate::{DayOptions, Error, WishState, find, reader};

/// The command line of a run, as measured on Claude Code 2.1.322 (ADR 0014, "Measured"): no shell, no web, no MCP
/// server, no question asked, edits inside the folder it is started in, no session kept. [`PROMPT`] follows it.
const ARGS: &[&str] = &[
    "-p",
    "--restricted",
    "--strict-mcp-config",
    "--permission-prompts",
    "none",
    "--permission-mode",
    "acceptEdits",
    "--tools",
    "Read",
    "Edit",
    "Write",
    "Glob",
    "Grep",
    "--no-session-persistence",
    "--output-format",
    "stream-json",
    "--verbose",
    "--append-system-prompt",
];
/// What is added to the harness's own system prompt: fixed words, so nothing of a wish or a repo can change them.
const PROMPT: &str = "You are working in a copy of a repo that was made for this one wish. You can read and edit \
                      files in this folder only. You cannot run commands. Make the change the wish asks for, then stop.";

/// How long a run may take. One still going then is stopped, and has failed.
const LIMIT: Duration = Duration::from_secs(30 * 60);

/// The one star a finished run adds.
const STAR: &str = "a run reported it finished";
const OLD: &str = "nothing was started: this Claude Code is too old for runs, as it does not know --restricted; \
                   update Claude Code and run the wish again";

/// One `jellymoon run`.
#[derive(Debug, Clone, Default)]
pub struct RunOptions {
    /// The made wish's id, `m1` to `m8`.
    pub wish: String,
    /// The day the wish was made on, as YYYY-MM-DD. None is today.
    pub day: Option<String>,
    /// The repo's folder. None: the one folder of the wish's project that was typed in that day.
    pub repo: Option<PathBuf>,
    /// The Claude Code projects directory, read only to find that folder.
    pub logs: Option<PathBuf>,
    /// The harness program. None: the one found as the reading call finds it.
    pub harness: Option<PathBuf>,
}

/// How a `jellymoon run` ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunOutcome {
    /// The run reported it finished: its work is on its branch, and the wish has its star.
    Completed,
    /// The run ended any other way: its copy is kept, and the wish needs the person.
    Failed,
    /// No run was started and nothing was made: there is no harness, or it is too old.
    NotStarted,
}

/// What `jellymoon run` prints. No word here is the agent's, a session log's or the wish's.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ran {
    /// The run's id. None when no run was started.
    pub run: Option<String>,
    pub wish: String,
    pub outcome: RunOutcome,
    /// The star the run added to the wish.
    pub star: Option<String>,
    /// How many things the harness says the agent asked for and was denied.
    pub denied: usize,
    /// How the run stands, in fixed words.
    pub why: String,
}

impl fmt::Display for Ran {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&serde_json::to_string(self).map_err(|_| fmt::Error)?)
    }
}

fn refused(why: impl Into<String>) -> Error {
    Error::Run(why.into())
}

fn had(wish: &str, run: &str) -> Error {
    refused(format!("wish {wish} has had its run, {run}: a made wish gets one run"))
}

fn going(run: &str) -> String {
    format!("run {run} is going, in a copy of the repo")
}

fn failed(run: &str, kind: &str) -> String {
    format!("run {run} did not finish ({kind}): its copy is kept and no star was added")
}

fn cut(run: &str) -> String {
    format!("run {run} was cut off before it ended: its copy is kept and no star was added")
}

fn finished(copy: &RunCopy, changed: bool) -> String {
    let (run, branch) = (&copy.run, &copy.branch);
    if changed {
        format!("run {run} finished: its work is one commit on the branch {branch}")
    } else {
        format!("run {run} finished and changed nothing: the branch {branch} is where the checkout was")
    }
}

/// The limit, shorter for a test: the variable is read only by a build with debug assertions, which a release is not.
fn limit() -> Duration {
    let short = env::var("JELLYMOON_TEST_RUN_LIMIT_MS").ok().filter(|_| cfg!(debug_assertions));
    short.and_then(|ms| ms.parse().ok()).map_or(LIMIT, Duration::from_millis)
}

fn lock_file(home: &Path, run: &str) -> PathBuf {
    copy::kept_at(home, run).join("run.lock")
}

/// Whether a `jellymoon run` that still lives holds the run. A lock that cannot be asked about is no lock.
fn held(home: &Path, run: &str) -> bool {
    let free = File::open(lock_file(home, run)).is_ok_and(|lock| lock.try_lock_shared().is_ok());
    !free && lock_file(home, run).is_file()
}

/// Whether a made wish's run was cut off: the wish still says working, and nothing holds its run.
fn cut_off(home: &Path, wish: &MadeWish) -> bool {
    wish.state == WishState::Working && wish.run.as_deref().is_some_and(|run| !held(home, run))
}

/// What a `jellymoon run` or a page does with a wish whose run was cut off: the harness child it left is stopped,
/// if the pid in the run's record is still that process (`dead.rs`), and the run is written down as failed, behind
/// the day's lock and only if the wish still says working there. True when the run was cut off, written down or
/// not: such a wish is drawn as needing the person, never as working for ever.
pub(crate) fn bury(home: &Path, day: &str, wish: &MadeWish) -> bool {
    let Some(run) = wish.run.as_deref().filter(|_| cut_off(home, wish)) else { return false };
    if let Some((pid, mark)) = record::kept(home, run).and_then(|record| Some((record.pid?, record.mark?))) {
        dead::stop(pid, &mark);
    }
    let _ = end(home, day, &wish.id, run, RunOutcome::Failed, (0, 0), &cut(run));
    true
}

/// Starts the one run of a made wish and waits for it to end. Refused, with nothing made: a wish that is not there
/// or has had its run, no one folder for it, a folder that is not a checkout git can copy. With no harness, or one
/// too old, nothing is started and the wish stays as it was.
pub fn run(home: &Path, options: &RunOptions) -> Result<Ran, Error> {
    let day = wish::day_of(options.day.as_deref())?;
    let none = || refused(format!("the page of {day} has no made wish of that id; a made wish's id is m1 to m8"));
    let wish = wish::kept_of(home, &day)?.into_iter().find(|wish| wish.id == options.wish).ok_or_else(none)?;
    if let Some(run) = &wish.run {
        bury(home, &day, &wish);
        return Err(had(&wish.id, run));
    }
    let not_started = |why: String| {
        Ok(Ran { run: None, wish: wish.id.clone(), outcome: RunOutcome::NotStarted, star: None, denied: 0, why })
    };
    let harness = match find::named_or_found(options.harness.as_deref()) {
        Ok(harness) => harness,
        Err(e @ Error::NoHarness(_)) => return not_started(e.to_string()),
        Err(e) => return Err(e),
    };
    let repo = folder(home, &day, &wish, options)?;
    // A folder read from the logs that is not a repo's top is refused with copy's own reason: no folder above it
    // is tried.
    let copy = copy::make_copy(home, &repo, None).map_err(|e| match (e, &options.repo) {
        (Error::Copy(why), None) => {
            let why = why.replacen("--repo", "it", 1);
            refused(format!("the folder of that name typed in on {day} is not one a run can copy: {why}"))
        }
        (e, _) => e,
    })?;
    // The lock is taken before the wish names the run, so a wish never names a run that nothing holds.
    let taken = start(home, &day, &wish.id, &copy.run, &repo);
    let lock = match taken {
        Ok(lock) => lock,
        Err(e) => {
            let _ = copy::take_back(home, &copy.run);
            return Err(e);
        }
    };
    let mut command = harness_in(harness, Path::new(&copy.folder));
    command.args(ARGS).arg(PROMPT);
    let limit = limit();
    let ended = child::run(command, format!("{}\n", wish.said), limit, |pid| record::child(home, &copy.run, pid));
    let ended = ended.map_err(|e| refused(format!("nothing was started: Claude Code could not be started: {e}")));
    if !ended.as_ref().is_ok_and(|ended| !ended.old) {
        // Nothing ran: the wish is as it was, and can be run again.
        wish::change(home, &day, &wish.id, |wish| {
            (wish.run, wish.why) = (None, None);
            Ok(())
        })?;
        drop(lock);
        copy::take_back(home, &copy.run)?;
        return ended.and_then(|_| not_started(OLD.to_string()));
    }
    let ran = ended.and_then(|ended| settle(home, &day, &wish.id, &copy, &ended, limit));
    drop(lock);
    let _ = fs::remove_file(lock_file(home, &copy.run));
    ran
}

/// The repo a wish runs in: the folder named, or the one folder of the wish's project that was typed in that day.
/// Its last name must be the wish's project. A folder read from a session log is never put in a refusal.
fn folder(home: &Path, day: &str, wish: &MadeWish, options: &RunOptions) -> Result<PathBuf, Error> {
    if let Some(repo) = &options.repo {
        if !repo.is_absolute() {
            return Err(refused("--repo must be a full path, from the root"));
        }
        let named = repo.file_name().is_some_and(|last| last == wish.project.as_str());
        let why = "--repo is not the folder the wish is for: its last name is not the wish's project";
        return if named { Ok(repo.clone()) } else { Err(refused(why)) };
    }
    let logs = options.logs.clone().ok_or_else(|| refused("no session logs are known; name the folder with --repo"))?;
    let read = DayOptions { logs, date: Some(day.to_string()), skip: Vec::new(), home: Some(home.to_path_buf()) };
    let mut found = reader::folders(&read, &wish.project)?;
    match (found.pop(), found.len()) {
        (Some(repo), 0) => Ok(repo),
        (None, _) => Err(refused(format!(
            "no folder of the name the wish gives was typed in on {day}; name the folder with --repo"
        ))),
        (Some(_), more) => Err(refused(format!(
            "{} folders of the name the wish gives were typed in on {day}; name the one with --repo",
            more + 1
        ))),
    }
}

/// Takes the run's lock, writes its record, and has the wish name the run, behind the day's lock. A wish that names
/// a run already has had its run: this one is refused.
fn start(home: &Path, day: &str, wish: &str, run: &str, repo: &Path) -> Result<File, Error> {
    let lock = OpenOptions::new().write(true).create_new(true).open(lock_file(home, run)).map_err(Error::Store)?;
    lock.lock().map_err(Error::Store)?;
    let started = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |since| since.as_secs());
    let record = Record {
        wish: wish.to_string(),
        day: day.to_string(),
        repo: repo.to_path_buf(),
        started,
        outcome: None,
        denied: 0,
        skipped: 0,
        pid: None,
        mark: None,
    };
    record::write(home, run, &record)?;
    wish::change(home, day, wish, |wish| match &wish.run {
        Some(first) => Err(had(&wish.id, first)),
        None => {
            (wish.run, wish.why) = (Some(run.to_string()), Some(going(run)));
            Ok(())
        }
    })?;
    Ok(lock)
}

/// What a child that is gone comes to: the copy is ended as completed or failed, and the wish and the record say so.
fn settle(home: &Path, day: &str, wish: &str, copy: &RunCopy, ended: &Ended, limit: Duration) -> Result<Ran, Error> {
    let kind = match (ended.end, ended.result, ended.exit_0) {
        (End::Limit, ..) => Some(format!("it was stopped at its time limit of {} seconds", limit.as_secs())),
        (End::Over, ..) => Some("it printed more than Jellymoon reads of a run".to_string()),
        (End::Exited, None, _) => Some("Claude Code ended without a result".to_string()),
        (End::Exited, Some(false), _) => Some("its result was not a success".to_string()),
        (End::Exited, Some(true), false) => Some("Claude Code exited with a failure after its result".to_string()),
        (End::Exited, Some(true), true) => None,
    };
    let run = &copy.run;
    let committed = match kind {
        None => {
            copy::end_copy(home, run, Outcome::Completed).map_err(|_| "its work could not be committed".to_string())
        }
        Some(kind) => Err(kind),
    };
    let (outcome, why) = match committed {
        Ok((ended, changed)) => (RunOutcome::Completed, finished(&ended, changed)),
        Err(kind) => (RunOutcome::Failed, failed(run, &kind)),
    };
    let star = end(home, day, wish, run, outcome, (ended.denied, ended.skipped), &why)?;
    Ok(Ran { run: Some(run.clone()), wish: wish.to_string(), outcome, star, denied: ended.denied, why })
}

/// Writes down how a run ended. First the wish, behind the day's lock: its state, its words and, when the run
/// finished, its star, which is returned. A wish that names another run, or no longer says working, has had its
/// end written already, and nothing is changed. Then a failed run's copy is marked failed, and the record takes the
/// outcome.
fn end(
    home: &Path,
    day: &str,
    wish: &str,
    run: &str,
    outcome: RunOutcome,
    (denied, skipped): (usize, usize),
    why: &str,
) -> Result<Option<String>, Error> {
    let done = outcome == RunOutcome::Completed;
    wish::change(home, day, wish, |wish| {
        if wish.run.as_deref() != Some(run) || wish.state != WishState::Working {
            return Err(refused("the wish no longer names this run as going; it was not changed"));
        }
        wish.state = if done { WishState::Done } else { WishState::NeedsYou };
        if done && wish.stars.len() < STARS_MAX {
            wish.stars.push(STAR.to_string());
        }
        wish.why = Some(why.to_string());
        Ok(())
    })?;
    if !done {
        // A copy that is marked already, or gone, is left as it is.
        let _ = copy::clean_copy(home, run, Outcome::Failed);
    }
    if let Some(mut record) = record::kept(home, run) {
        (record.outcome, record.denied, record.skipped) = (Some(outcome), denied, skipped);
        record::write(home, run, &record)?;
    }
    Ok(done.then(|| STAR.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::redact::line;

    // A page can find a run's lock free in the moment after that run wrote its end. The end stands.
    #[test]
    fn a_run_that_wrote_its_end_is_not_written_down_again_as_cut_off() -> Result<(), Box<dyn std::error::Error>> {
        let (home, day, run) = (tempfile::tempdir()?, "2026-03-10", "r0123456789abcdef");
        wish::wish(home.path(), Some(day), "ship the changelog", "alpha")?;
        let done = wish::change(home.path(), day, "m1", |wish| {
            (wish.run, wish.state) = (Some(run.to_string()), WishState::Done);
            Ok(())
        })?;
        assert!(end(home.path(), day, "m1", run, RunOutcome::Failed, (0, 0), &cut(run)).is_err());
        assert_eq!(wish::kept_of(home.path(), day)?, [done]);
        Ok(())
    }

    // A wish's file is used only when cleaning its words changes nothing: so must every word a run writes there.
    #[test]
    fn every_word_a_run_keeps_with_a_wish_is_what_cleaning_leaves() {
        let run = "r0123456789abcdef";
        let copy = RunCopy {
            run: run.to_string(),
            folder: String::new(),
            branch: format!("jellymoon/run-{run}"),
            kept: false,
        };
        let long = failed(run, "Claude Code exited with a failure after its result");
        let texts = [going(run), cut(run), long, finished(&copy, true), finished(&copy, false), STAR.to_string()];
        for text in texts {
            assert_eq!(line(&text, 200), text);
            assert!(copy::is_run_id(run) && text.chars().count() <= 200);
        }
    }
}
