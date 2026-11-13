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

mod apart;
mod cap;
mod child;
mod dead;
mod end;
mod keep;
mod left;
mod never;
mod obeyed;
mod place;
mod record;
mod show;
mod stop;
mod stream;
mod toss;

use std::fmt;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub use apart::{Started, run_apart};
pub(crate) use end::bury;
use end::{lock_file, settle};
pub use keep::{KeepOptions, Kept, keep_run};
pub(crate) use left::tidy;
use record::Record;
use serde::{Deserialize, Serialize};
pub use show::{FileChange, FileKind, Shown, ShownEntry, ShownFile, WorkState, show_file, show_run};
pub use stop::stop_run;
pub use toss::{Tossed, toss_run};

use crate::busy::{self, or_test};
use crate::call::harness_of;
use crate::copy::{self, MadeFrom};
use crate::wish;
use crate::{Error, WishState, find};

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
    /// Folder names switched off, as the reader's are: a folder of such a name is not found in the logs.
    pub skip: Vec<String>,
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

/// How the line of a run that did not finish ends when it is written, and how it ends when it is printed after
/// the copy has gone: a failed copy past the newest five is removed, and so is the copy of a run that was tossed.
const COPY_KEPT: &str = ": its copy is kept and no star was added";
const COPY_GONE: &str = ": its copy is no longer kept, and no star was added";

/// A run's line as it holds now: of a copy whose folder is not standing it is not said that it is kept.
fn by_now(home: &Path, run: &str, why: String) -> String {
    match why.strip_suffix(COPY_KEPT).filter(|_| copy::no_folder(home, run)) {
        Some(head) => format!("{head}{COPY_GONE}"),
        None => why,
    }
}

fn failed(run: &str, kind: &str) -> String {
    format!("run {run} did not finish ({kind}){COPY_KEPT}")
}

fn cancelled(run: &str) -> String {
    format!("run {run} was cancelled by a stop{COPY_KEPT}")
}

fn no_wish(day: &str) -> Error {
    refused(format!("the page of {day} has no made wish of that id; a made wish's id is m1 to m8"))
}

fn cut(run: &str) -> String {
    format!("run {run} was cut off before it ended{COPY_KEPT}")
}

/// The line of a run that was cut off with its copy gone: after it was committed and before the end was in the
/// record, or removed by hand, with nothing committed. It says no more than holds for both, and of the branch
/// only where the work was put: a branch can be removed by hand, and a look asks git nothing.
fn cut_late(run: &str) -> String {
    let branch = copy::branch(run);
    format!(
        "run {run} was cut off before it ended: no copy of it is kept, what was committed of it was on the branch \
         {branch}, and no star was added"
    )
}

/// The line of a run that finished. Both forms say what git was last asked as the run ended (`end::settle`,
/// `copy/clean.rs`): the branch took the run's one commit, or was looked at once more, only while it still named
/// the commit the copy was made from, which is where the person's checkout was when the run started. Neither says
/// where the branch is after that check: until the copy's folder is removed, and ever after, a hand can move it,
/// and a look goes by what the repo says then (`show.rs`).
fn finished(run: &str, changed: bool) -> String {
    let branch = copy::branch(run);
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
    // Read again: a look that could not take a never-started run's copy back answers with no run and leaves the
    // wish naming that run. It has its run still, and no copy is made to find that out at the claim (`start`).
    let named = wish::kept_of(home, &day)?.into_iter().find(|kept| kept.id == wish.id).and_then(|kept| kept.run);
    if let Some(run) = wish.run.as_ref().or(named.as_ref()) {
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
    // First what killed commands left: their agents are ended before the runs going are counted, so none is at
    // work beside this run's own, and a copy no wish names is taken back (`left.rs`). It is done before the
    // folder is looked up, so none of its waits stands between that lookup and git opening the folder.
    tidy(home, &wish::kept_of(home, &day)?);
    let repo = place::folder(home, &day, &wish, options)?;
    // A test's build can wait here, with the folder looked up and nothing yet asked of git.
    busy::pause("JELLYMOON_TEST_PLACE_WAIT_MS");
    cap::room(home)?;
    // A folder read from the logs that is not a repo's top is refused with copy's own reason, in fixed words: no
    // folder above it is tried, and nothing git says of it is repeated.
    let made = copy::copy_of(home, &repo, options.repo.is_some()).map_err(|e| match (e, &options.repo) {
        (Error::Copy(why), None) => {
            let why = why.replacen("--repo", "it", 1);
            refused(format!("the folder of that name typed in on {day} is not one a run can copy: {why}"))
        }
        (e, _) => e,
    })?;
    // The copy's start lock is held until this command ends: what a killed command left is known by it.
    let (copy, held, from) = made;
    // Where the run started, kept as git said it: the run's branch must still be there at the run's end.
    let began_at = from.commit.clone();
    // The lock is taken before the wish names the run, so a wish never names a run that nothing holds.
    let taken = start(home, (&day, &wish.id), &copy.run, from);
    let lock = match taken {
        Ok(lock) => lock,
        Err(e) => {
            drop(held);
            let _ = copy::take_back(home, &copy.run, None);
            return Err(e);
        }
    };
    command.current_dir(&copy.folder).args(ARGS).arg(PROMPT);
    let limit = or_test(LIMIT, "JELLYMOON_TEST_RUN_LIMIT_MS");
    let (told, stop) = (|pid| record::child(home, &copy.run, pid), || stop::asked(home, &copy.run));
    let began = || record::began(home, &copy.run);
    let ended = child::run(command, format!("{}\n", wish.said), limit, told, began, stop);
    let never = match &ended {
        Err(_) => Some(never::UNSTARTABLE),
        Ok(ended) => ended.old.then_some(never::OLD),
    };
    let ended = ended.map_err(|e| refused(format!("{}: {e}", never::UNSTARTABLE)));
    if let Some(why) = never {
        never::undo(home, (&day, &wish.id), &copy.run, why, (lock, held))?;
        return ended.and_then(|_| not_started(never::OLD.to_string()));
    }
    let ran = ended.and_then(|ended| settle(home, (&day, &wish.id), (&copy, &began_at), &ended, limit));
    // The start lock goes first: while the run's own is held, nothing takes this run for one left behind.
    drop(held);
    let _ = fs::remove_file(copy::start_lock(home, &copy.run));
    drop(lock);
    let _ = fs::remove_file(lock_file(home, &copy.run));
    ran
}

/// Takes the run's lock, when fewer than three runs go, writes its record, and has the wish name the run, behind
/// the day's lock: all of it behind the lock the runs are counted behind (`cap.rs`). A wish that names a run
/// already has had its run: this one is refused, and before it is counted as a run.
fn start(home: &Path, (day, wish): (&str, &str), run: &str, from: MadeFrom) -> Result<File, Error> {
    let free = || match wish::kept_of(home, day)?.into_iter().find(|kept| kept.id == wish).and_then(|kept| kept.run) {
        Some(first) => Err(had(wish, &first)),
        None => Ok(()),
    };
    cap::claim(home, run, free, || name(home, (day, wish), run, from))
}

/// Writes the run's record and has the wish name the run, with the run's lock held. The record names the repo and
/// the commit the copy was made from, from its first writing: before the harness is started, and so long before
/// the copy's own record, which names the repo too, is removed with a finished run's copy.
fn name(home: &Path, (day, wish): (&str, &str), run: &str, from: MadeFrom) -> Result<(), Error> {
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
        began: false,
        repo: Some(from.repo),
        start: Some(from.commit),
        end: None,
        work: None,
        kept: None,
        onto: None,
        tossed: None,
    };
    record::write(home, run, &record)?;
    wish::change(home, day, wish, Duration::ZERO, |wish| match &wish.run {
        Some(first) => Err(had(&wish.id, first)),
        None => {
            (wish.run, wish.why) = (Some(run.to_string()), Some(going(run)));
            // A wish that an earlier run was never started for needed the person (`never.rs`): it is at work now.
            if wish.state == WishState::NeedsYou {
                wish.state = WishState::Working;
            }
            Ok(())
        }
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::redact::line;

    // A wish's file is used only when cleaning its words changes nothing: so must every word a run writes there.
    #[test]
    fn every_word_a_run_keeps_with_a_wish_is_what_cleaning_leaves() {
        let run = "r0123456789abcdef";
        let long = failed(run, "it did not start with the tools, servers and permission mode of a run");
        let texts = [going(run), cut(run), long, finished(run, true), finished(run, false), STAR.to_string()];
        let never = [never::OLD, never::UNSTARTABLE, never::UNTOLD].map(str::to_string);
        for text in texts.into_iter().chain([cancelled(run), cut_late(run)]).chain(never) {
            assert_eq!(line(&text, 200), text);
            assert!(copy::is_run_id(run) && text.chars().count() <= 200);
        }
    }
}
