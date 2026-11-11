//! A run (F39, ADR 0014, `docs/development/run.md`): the person's own harness, started headless on one made wish, in
//! a copy of the wish's repo (`copy.rs`). [`run`] finds the harness, then the repo, makes the copy, starts the
//! harness there with a fixed command line and the wish's kept words on its input, and waits. A run has finished
//! only when its stream starts as a run's does and gives one result, which says success and no error, with nothing
//! unread after it, and the harness exited with 0 (`stream.rs`, `end.rs`); then its work is one commit on its
//! branch and the wish has one more star. Every other end is a failed run: the copy is kept, no star. A run the
//! person stopped with [`stop_run`] is a cancelled one: the copy is kept, no star (`stop.rs`).
//!
//! A made wish gets one run. Its id and how it stands are kept with the wish, in fixed words; the run's own record
//! is `runs/<id>/run.json`. While `jellymoon run` lives it holds the system's lock on `runs/<id>/run.lock`, which
//! the system drops when the process ends, however it ends: a wish whose run is not locked, and whose record has
//! no end, was cut off. The next `jellymoon run` or page that finds it kills the harness child it left, if that is
//! still the same process, and writes the run down as failed ([`bury`]).

mod child;
mod dead;
mod end;
mod record;
mod stop;
mod stream;

use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub(crate) use end::bury;
use end::{lock_file, settle};
use record::Record;
use serde::{Deserialize, Serialize};
pub use stop::stop_run;

use crate::busy::or_test;
use crate::call::harness_of;
use crate::copy::{self, RunCopy};
use crate::wish::{self, MadeWish};
use crate::{DayOptions, Error, find, reader};

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

/// How long a run may take. One still going then is stopped, and has failed. A test's build reads a shorter one.
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
    /// The person stopped the run: its copy is kept, no star is added, and the wish needs the person.
    Cancelled,
    /// No run was started and nothing was made: there is no harness, or it is too old.
    NotStarted,
}

/// What `jellymoon run` and `jellymoon run stop` print. No word here is the agent's, a session log's or the wish's.
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

fn cancelled(run: &str) -> String {
    format!("run {run} was cancelled by a stop: its copy is kept and no star was added")
}

fn no_wish(day: &str) -> Error {
    refused(format!("the page of {day} has no made wish of that id; a made wish's id is m1 to m8"))
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

/// Starts the one run of a made wish and waits for it to end. Refused, with no copy made and the wish as it was: a
/// wish that is not there or has had its run, a `PATH` with no full path in it, no one folder for the wish, a
/// folder that is not a checkout git can copy. With no harness, or one too old, nothing is started either.
pub fn run(home: &Path, options: &RunOptions) -> Result<Ran, Error> {
    let day = wish::day_of(options.day.as_deref())?;
    let wish = wish::kept_of(home, &day)?.into_iter().find(|wish| wish.id == options.wish);
    let wish = wish.ok_or_else(|| no_wish(&day))?;
    // A wish whose run was never started comes back with no run, and this call goes on to start one.
    let wish = bury(home, &day, wish);
    if let Some(run) = &wish.run {
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
    // Before anything is made: a harness that would look for its programs in the copy is not started.
    let mut command = harness_of(harness).map_err(|e| refused(format!("nothing was started: {e}")))?;
    let repo = folder(home, &day, &wish, options)?;
    // A folder read from the logs that is not a repo's top is refused with copy's own reason, in fixed words: no
    // folder above it is tried, and nothing git says of it is repeated.
    let copy = copy::copy_of(home, &repo, options.repo.is_some()).map_err(|e| match (e, &options.repo) {
        (Error::Copy(why), None) => {
            let why = why.replacen("--repo", "it", 1);
            refused(format!("the folder of that name typed in on {day} is not one a run can copy: {why}"))
        }
        (e, _) => e,
    })?;
    // The lock is taken before the wish names the run, so a wish never names a run that nothing holds.
    let taken = start(home, &day, &wish.id, &copy.run);
    let lock = match taken {
        Ok(lock) => lock,
        Err(e) => {
            let _ = copy::take_back(home, &copy.run);
            return Err(e);
        }
    };
    command.current_dir(&copy.folder).args(ARGS).arg(PROMPT);
    let limit = or_test(LIMIT, "JELLYMOON_TEST_RUN_LIMIT_MS");
    let (told, stop) = (|pid| record::child(home, &copy.run, pid), || stop::asked(home, &copy.run));
    let ended = child::run(command, format!("{}\n", wish.said), limit, told, stop);
    let ended = ended.map_err(|e| refused(format!("nothing was started: Claude Code could not be started: {e}")));
    if !ended.as_ref().is_ok_and(|ended| !ended.old) {
        // Nothing ran: the wish is as it was, and can be run again. The day is waited for as a run's end waits.
        // Whatever came of that, the record says the run was never started and the copy is taken back, both
        // while this run's lock is held: so no look ever finds a run that nothing holds and that has no end. A
        // wish that still names the run is made as it was by the next look (`end::bury`).
        let undone = wish::change(home, &day, &wish.id, end::wait(), |wish| {
            (wish.run, wish.why) = (None, None);
            Ok(())
        });
        record::unstarted(home, &copy.run);
        let taken = copy::take_back(home, &copy.run);
        drop(lock);
        // Where a file that is open cannot be removed, it can be now.
        taken.or_else(|_| copy::take_back(home, &copy.run))?;
        undone?;
        return ended.and_then(|_| not_started(OLD.to_string()));
    }
    let ran = ended.and_then(|ended| settle(home, (&day, &wish.id), &copy, &ended, limit));
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
fn start(home: &Path, day: &str, wish: &str, run: &str) -> Result<File, Error> {
    let lock = OpenOptions::new().write(true).create_new(true).open(lock_file(home, run)).map_err(Error::Store)?;
    lock.lock().map_err(Error::Store)?;
    let started = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |since| since.as_secs());
    let record = Record {
        wish: wish.to_string(),
        day: day.to_string(),
        started,
        outcome: None,
        why: None,
        denied: 0,
        skipped: 0,
        pid: None,
        mark: None,
    };
    record::write(home, run, &record)?;
    wish::change(home, day, wish, Duration::ZERO, |wish| match &wish.run {
        Some(first) => Err(had(&wish.id, first)),
        None => {
            (wish.run, wish.why) = (Some(run.to_string()), Some(going(run)));
            Ok(())
        }
    })?;
    Ok(lock)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::redact::line;

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
        let long = failed(run, "it did not start with the tools, servers and permission mode of a run");
        let texts = [going(run), cut(run), long, finished(&copy, true), finished(&copy, false), STAR.to_string()];
        for text in texts.into_iter().chain([cancelled(run)]) {
            assert_eq!(line(&text, 200), text);
            assert!(copy::is_run_id(run) && text.chars().count() <= 200);
        }
    }
}
