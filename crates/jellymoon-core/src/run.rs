//! A run (F39, ADR 0014, `docs/development/run.md`): the person's own harness, started headless on one made wish, in
//! a copy of the wish's repo (`copy.rs`). [`run`] finds the harness, then the repo, makes the copy, starts the
//! harness there with a fixed command line and the wish's kept words on its input, and waits. A run has finished
//! only when its stream starts as a run's does and gives one result, which says success and no error, with nothing
//! unread after it, and the harness exited with 0 (`stream.rs`, `end.rs`); then its work is one commit on its
//! branch and the wish has one more star. A run whose last word is `asks`, with a question that passes the door,
//! asks: its copy is left as it is, with nothing committed, and the wish carries the question (`asks.rs`). Every
//! other end is a failed run: the copy is kept, no star. A run the person stopped with [`stop_run`] is a cancelled
//! one: the copy is kept, no star (`stop.rs`).
//!
//! A made wish gets one run. Its id and how it stands are kept with the wish, in fixed words; the run's own record
//! is `runs/<id>/run.json`. While `jellymoon run` lives it holds the system's lock on `runs/<id>/run.lock`, which
//! the system drops when the process ends, however it ends: a wish whose run is not locked, and whose record has
//! no end, was cut off. The next `jellymoon run` or page that finds it kills the harness child it left, if that is
//! still the same process, and writes the run down as failed ([`bury`]).

mod answer;
mod apart;
mod asks;
mod cap;
mod child;
mod dead;
mod end;
mod follow;
mod keep;
mod left;
mod never;
mod obeyed;
mod place;
mod record;
mod shape;
mod show;
mod steps;
mod stop;
mod stream;
mod told;
mod toss;
mod undo;
mod unrun;
mod waits;
mod words;

use std::fmt;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub(crate) use answer::kept as kept_answer;
pub use answer::{ANSWER_READ, answer_run};
pub(crate) use apart::asked_apart;
pub use apart::{Started, run_apart};
pub(crate) use asks::kept as kept_question;
pub(crate) use end::bury;
use end::{lock_file, settle};
pub use follow::{followable, wish_after};
pub use keep::{KeepOptions, Kept, keep_run};
pub(crate) use left::tidy;
pub(crate) use place::{plain, theirs};
use record::Record;
pub(crate) use record::work_waits;
use serde::{Deserialize, Serialize};
pub(crate) use shape::kept as kept_shaped;
pub use shape::{SHAPE_MODEL, Shape, ShapeOutcome, Shaped, shape, wish_of_shape, wish_shaped};
pub use show::{
    Alone, FileChange, FileKind, Shown, ShownEntry, ShownFile, WorkState, ended_alone, show_file, show_run,
};
pub use steps::{Step, Steps, Tool, doing};
pub use stop::stop_run;
pub use told::{Piece, Told};
pub use toss::{Tossed, toss_run};
pub use undo::{Undone, undo_run};
pub use waits::{KeptItself, Waiting, Waits, WorkDone, WorkTap, kept_itself, stands, waiting, waits, work_tapped};
use words::{asking, by_now, cancelled, cut, cut_late, failed, finished, going, had, no_wish};

use crate::busy::{self, or_test};
use crate::call::harness_of;
use crate::copy::{self, MadeFrom};
use crate::{Error, WishState, find, project, wish};

/// The command line of a run, as measured on Claude Code 2.1.322 (ADR 0014, "Measured") and 2.1.323 (ADR 0018,
/// "Measured"): no shell, no web, no MCP server, no permission asked, edits inside the folder it is started in.
/// After these come `--session-id` with the id core made for the run, under which the harness keeps the session
/// in its own store, and `--json-schema` with the schema of its last word (`asks.rs`); then [`AFTER`] and [`PROMPT`].
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
];
const AFTER: &[&str] = &["--output-format", "stream-json", "--verbose", "--append-system-prompt"];
/// What is added to the harness's own system prompt: fixed words, so nothing of a wish or a repo can change them.
/// Since F87 they ask for a result a person who does not program can read, with quotes core can check (`told.rs`).
const PROMPT: &str = "You are working in a copy of a repo that was made for this one wish. You can read and edit files \
                      in this folder only. You cannot run commands. Make the change the wish asks for, then stop with \
                      the state `done` and a `result` for the person who made the wish. If you cannot make it without \
                      an answer that only the person can give, stop with the state `asks` and one question for them, \
                      in plain words, and no `result`. A `result` is a short list of pieces: a `did`, `decided` or \
                      `unsure` piece has only `text`, and a `change` piece only `file`, `before` and `after`. First \
                      one `did`: one or two plain sentences that a person who does not program can read, saying what \
                      changed for them, not how. Then, for the changed files that matter most, at most two `change` \
                      pieces: `file` is the file's path relative to this folder, with no leading `./`, `before` holds \
                      only lines that are gone or read differently now, and `after` only lines that are new or read \
                      differently now, at most three on each side. Quote only lines that changed: never repeat an \
                      unchanged line on either side for context. For lines that were only added, `before` is empty; \
                      for lines that were only removed, `after` is empty. Each line is whole and copied exactly as it \
                      is in the file, character for character, with nothing left out and no `...`, and under 100 \
                      characters: a piece with a line that is not exact, or did not change, is dropped. Then a \
                      `decided` only for a choice the person might have made differently, and an `unsure` only for \
                      something the person should look at, each one short sentence under 160 characters, at most three \
                      of each. You ran nothing, so never say that anything was run or tested.";

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
    /// The run asks a question: its copy is left as it is, nothing is committed, and the wish needs the person.
    Asks,
}

/// What `jellymoon run` and `jellymoon run stop` print. No word here is a session log's or the wish's, and only
/// `question` is the agent's.
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
    /// The question of a run that asks: the agent's text, cleaned at the door, on one line, at most 300
    /// characters. It is data to draw as text. Left out of every other run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub question: Option<String>,
    /// Whether the run's change was kept by itself, so it is in the person's folder already (F92). Left out
    /// when it was not: the change waits, or there is none.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub kept_itself: bool,
}

impl fmt::Display for Ran {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&serde_json::to_string(self).map_err(|_| fmt::Error)?)
    }
}

fn refused(why: impl Into<String>) -> Error {
    Error::Run(why.into())
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
    unrun::open(&wish)?;
    let not_started = |why: String| {
        let (run, outcome) = (None, RunOutcome::NotStarted);
        let (wish, kept_itself) = (wish.id.clone(), false);
        Ok(Ran { run, wish, outcome, star: None, denied: 0, why, question: None, kept_itself })
    };
    let harness = match find::named_or_found(options.harness.as_deref()) {
        Ok(harness) => harness,
        Err(e @ Error::NoHarness(_)) => return not_started(e.to_string()),
        Err(e) => return Err(e),
    };
    // Before anything is made: a harness that would look for its programs in the copy is not started.
    harness_of(harness.clone()).map_err(|e| refused(format!("nothing was started: {e}")))?;
    // First what killed commands left: their agents are ended before the runs going are counted, so none is at
    // work beside this run's own, and a copy no wish names is taken back (`left.rs`). It is done before the
    // folder is looked up, so none of its waits stands between that lookup and git opening the folder.
    tidy(home, &wish::kept_of(home, &day)?);
    let (repo, from) = place::found(home, &day, &wish.project, options)?;
    // A test's build can wait here, with the folder looked up and nothing yet asked of git.
    busy::pause("JELLYMOON_TEST_PLACE_WAIT_MS");
    cap::room(home, cap::AGAIN)?;
    // A follow-up: the work it follows is read behind the lock a keep and a toss hold, which is held until the
    // wish names its run, so that work is neither kept nor tossed between this look and the copy made at it.
    let on = follow::on(home, &day, &wish)?;
    let base = on.as_ref().map(follow::On::base);
    let made = copy::copy_of(home, &repo, options.repo.is_some(), base).map_err(|e| from.uncopied(e, &day))?;
    // The copy's start lock is held until this command ends: what a killed command left is known by it.
    let (copy, held, from) = made;
    // Where the run started, kept as git said it: the run's branch must still be there at the run's end. And
    // the session's id, which is core's: made here, and written in the record before the harness is started.
    let (began_at, session) = (from.commit.clone(), asks::fresh());
    // The lock is taken before the wish names the run, so a wish never names a run that nothing holds.
    let taken = start(home, (&day, &wish.id), &copy.run, from, (&session, on.as_ref().map(follow::On::named)));
    let fork = on.and_then(follow::On::session);
    let lock = match taken {
        Ok(lock) => lock,
        Err(e) => {
            drop(held);
            let _ = copy::take_back(home, &copy.run, None);
            return Err(e);
        }
    };
    // Every check a run makes of a repo is passed: it is a project Jellymoon knows from now on (`project.rs`).
    project::used(home, &repo, &wish.project, &day);
    let limit = or_test(LIMIT, "JELLYMOON_TEST_RUN_LIMIT_MS");
    // A wish kept from a shape gives the harness its shaped prompt (`shape.rs`); every other its typed words.
    let said = format!("{}\n", wish.shaped.as_ref().unwrap_or(&wish.said));
    let go = follow::Go { home, run: &copy.run, folder: &copy.folder, start: &began_at, said, limit };
    let ended = go.started(harness, session, fork.as_deref());
    let never = match &ended {
        Err(_) => Some(never::UNSTARTABLE.to_string()),
        Ok(ended) => ended.old.map(never::old),
    };
    let ended = ended.map_err(|e| refused(format!("{}: {e}", never::UNSTARTABLE)));
    if let Some(why) = never {
        never::undo(home, (&day, &wish.id), &copy.run, &why, (lock, held))?;
        return ended.and_then(|_| not_started(why));
    }
    let ran = ended.and_then(|ended| settle(home, (&day, options), (&copy, &began_at), &ended, limit));
    // The start lock goes first: while the run's own is held, nothing takes this run for one left behind.
    drop(held);
    let _ = fs::remove_file(copy::start_lock(home, &copy.run));
    let_go(home, &copy.run, lock);
    ran
}

/// Lets go of a run's own lock at the end of its command. The lock's file goes with it, but for a run that asks:
/// an answer or a toss takes that lock next, and its file stays with the run's folder (`asks::Held`).
fn let_go(home: &Path, run: &str, lock: File) {
    // Read while the lock is held: an answer that took it next could write the run as going behind this read.
    let asks = record::kept(home, run).is_some_and(|record| record.outcome == Some(RunOutcome::Asks));
    drop(lock);
    if !asks {
        let _ = fs::remove_file(lock_file(home, run));
    }
}

/// The session a harness is started with: a new one, the one it kept under the run's id, which an answer goes
/// on with (`answer.rs`), or a fork of the session another run's harness kept, under the run's own new id.
#[derive(Clone, Copy)]
enum Session<'a> {
    New,
    Kept,
    Fork(&'a str),
}

/// Gives the harness a run's whole command line, in the copy's folder, and returns what its stream must then say.
/// A run that is answered has the same one, with `--resume` where `--session-id` stands; a follow-up's has
/// `--resume` with the followed run's session and `--fork-session` before its own `--session-id` (`follow.rs`).
fn lined(command: &mut Command, folder: &str, session: String, from: Session) -> stream::Expect {
    command.current_dir(folder).args(ARGS);
    match from {
        Session::New => command.args(["--session-id", &session]),
        Session::Kept => command.args(["--resume", &session]),
        Session::Fork(of) => command.args(["--resume", of, "--fork-session", "--session-id", &session]),
    };
    command.args(["--json-schema", asks::schema()]).args(AFTER).arg(PROMPT);
    stream::Expect { session, schema: true, resumed: !matches!(from, Session::New) }
}

/// The session a run is started under, and of a follow-up the run it follows and whether it forks its session.
type Under<'a> = (&'a str, Option<(&'a str, bool)>);

/// Takes the run's lock, when fewer than three runs go, writes its record, and has the wish name the run, behind
/// the day's lock: all of it behind the lock the runs are counted behind (`cap.rs`). A wish that names a run
/// already has had its run: this one is refused, and before it is counted as a run.
fn start(home: &Path, (day, wish): (&str, &str), run: &str, from: MadeFrom, under: Under) -> Result<File, Error> {
    let free = || match wish::kept_of(home, day)?.into_iter().find(|kept| kept.id == wish).and_then(|kept| kept.run) {
        Some(first) => Err(had(wish, &first)),
        None => Ok(()),
    };
    cap::claim(home, run, free, || name(home, (day, wish), run, from, under))
}

/// Writes the run's record and has the wish name the run, with the run's lock held. The record names the repo and
/// the commit the copy was made from, from its first writing: before the harness is started, and so long before
/// the copy's own record, which names the repo too, is removed with a finished run's copy. And it names the
/// session the harness is then started under (ADR 0018, decision 6), and of a follow-up the run it follows and
/// whether that session is a fork of that run's (decision 8).
fn name(home: &Path, (day, wish): (&str, &str), run: &str, from: MadeFrom, under: Under) -> Result<(), Error> {
    let (session, follows) = under;
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
        before: None,
        back: None,
        tossed: None,
        session: Some(session.to_string()),
        question: None,
        follows: follows.map(|(followed, _)| followed.to_string()),
        forked: follows.map(|(_, forked)| forked),
    };
    record::write(home, run, &record)?;
    wish::change(home, day, wish, Duration::ZERO, |wish| match &wish.run {
        Some(first) => Err(had(&wish.id, first)),
        None => {
            // A toss that had the day's lock first let the wish go: no run is its run (`unrun.rs`).
            unrun::open(wish)?;
            (wish.run, wish.why, wish.question) = (Some(run.to_string()), Some(going(run)), None);
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
        let unchanged = [finished(run, false), words::unchanged_on(run)];
        let texts = [going(run), cut(run), long, finished(run, true), STAR.to_string()].into_iter().chain(unchanged);
        let never = [never::UNSTARTABLE.to_string(), never::UNTOLD.to_string(), never::old("--json-schema")];
        let never = never.into_iter().chain([unrun::LET_GO.to_string()]);
        let last = asks::KINDS.map(|kind| failed(run, kind)).into_iter().chain([asking(run)]);
        for text in texts.chain([cancelled(run), cut_late(run)]).chain(never).chain(last) {
            assert_eq!(line(&text, 200), text);
            assert!(copy::is_run_id(run) && text.chars().count() <= 200);
        }
    }
}
