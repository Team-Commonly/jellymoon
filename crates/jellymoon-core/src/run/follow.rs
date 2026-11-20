//! A follow-up (F53, ADR 0018, decisions 8 and 9; `docs/development/wish.md`, "A follow-up"): a made wish that
//! names the wish it follows. Here is what is read of the followed wish and its run, by the wish that is made
//! ([`project`]), by its run ([`on`]), by a keep ([`keep_first`]) and by a toss ([`toss_first`]), and the one
//! start a follow-up's run has more than any other: a fork of the followed run's session, and a new session when
//! the harness keeps that one no more ([`Go::started`]).
//!
//! Which run a wish follows is read from the home as a look reads it (`show.rs`), and refused in a look's words.
//! What a keep or a toss changes of that run is read behind the lock both hold (`copy::alone`): a follow-up's run
//! holds it from before it reads the followed run's record until its wish names its own run.

use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use super::child::{self, Ended};
use super::keep::{BUSY, NO_LOCK, intent};
use super::record::{self, Record};
use super::show::{Asked, found, not_cores, unchanged};
use super::steps::Live;
use super::words::unchanged_on;
use super::{RunOptions, RunOutcome, Session, WorkState, asks, bury, finished, lined, refused, stop};
use crate::call::harness_of;
use crate::copy::{self, Base, Unkept};
use crate::wish::{self, MadeWish};
use crate::{Error, find};

const FOLLOWED: Asked =
    Asked { to: "follow", done: "followed", going: "is going: its work can be followed once it has finished" };

/// The work a wish follows, as the home has it: nothing of a repo is asked yet.
struct Followed {
    project: String,
    run: String,
    repo: PathBuf,
    end: String,
    /// Whether the work waits. It was kept when it does not, as the commit `kept`.
    waits: bool,
    kept: Option<String>,
    /// The id the harness keeps the run's session under, when the record names one of core's.
    session: Option<String>,
}

/// Finds the work of the wish `after`. Refused as a look is, and: work that was tossed, a run that changed
/// nothing in a copy that held work it follows and the checkout does not hold, and a record that names no
/// commits. A run that changed nothing is followed with no work: neither waits nor kept, so the follow-up's copy
/// is made where the person's checkout stands, as one of kept work's is, and sees the files as they are now.
fn followed(home: &Path, day: &str, after: &str) -> Result<Followed, Error> {
    let options = RunOptions { wish: after.to_string(), day: Some(day.to_string()), ..RunOptions::default() };
    let found = found(home, &options, &FOLLOWED)?;
    let run = found.run.clone();
    let state = found.record.work.unwrap_or(WorkState::Waits);
    if state == WorkState::Tossed {
        return Err(refused(format!("the work of run {run} was tossed, so it cannot be followed")));
    }
    let changed = found.why == finished(&run, true);
    if !changed && (!unchanged(&found) || on_unkept(home, &found.record)) {
        return Err(refused(format!(
            "run {run} changed nothing on work that is not kept: follow the wish of that work instead"
        )));
    }
    let Some(named) = found.named()? else {
        return Err(refused(format!(
            "the record of run {run} is from before a run's record named its commits: its work is not followed by \
             Jellymoon"
        )));
    };
    let (repo, end) = (named.repo.to_path_buf(), named.end.to_string());
    // Work the record says was kept names where: a record that does not is not core's.
    let kept = intent(&found.record.kept, &found.record.onto).map_err(|()| not_cores(&run))?;
    let kept = kept.filter(|_| state == WorkState::Kept).map(|(commit, _)| commit.to_string());
    if state == WorkState::Kept && kept.is_none() {
        return Err(not_cores(&run));
    }
    let session = found.record.session.clone().filter(|session| asks::is_session(session));
    let waits = changed && state == WorkState::Waits;
    Ok(Followed { project: found.made.project, run, repo, end, waits, kept, session })
}

/// Whether a run that changed nothing stood on work it follows that changed files and is not kept: its copy held
/// that work, which the checkout does not, so its session has seen files a copy made there would not hold. A
/// followed record that is not there is held to be such work.
fn on_unkept(home: &Path, record: &Record) -> bool {
    let Some(followed) = record.follows.as_deref() else { return false };
    record::kept(home, followed).is_none_or(|was| {
        was.why.as_deref() == Some(finished(followed, true).as_str()) && was.work != Some(WorkState::Kept)
    })
}

/// The project of a follow-up of the wish `after`: the followed wish's own. Err: that wish cannot be followed.
pub(crate) fn project(home: &Path, day: &str, after: &str) -> Result<String, Error> {
    followed(home, day, after).map(|followed| followed.project)
}

/// The made wishes of `day` a follow-up of work may be made of, in their order (F54): each whose work
/// [`followed`] finds, which waits or was kept. `with_work` false: each [`followed`] finds with no work, its run
/// having changed nothing. `wish --after` refuses none of either for what it follows, and every other wish.
/// Read from the home alone, as a look reads it: nothing of a repo is asked, and nothing is written.
pub fn followable(home: &Path, day: &str, with_work: bool) -> Vec<String> {
    let made = wish::kept_of(home, day).unwrap_or_default();
    let work = |followed: Followed| followed.waits || followed.kept.is_some();
    let of = |id: &String| followed(home, day, id).is_ok_and(|followed| work(followed) == with_work);
    made.into_iter().map(|made| made.id).filter(of).collect()
}

/// What a follow-up's run is started on, with the lock a keep and a toss hold.
pub(super) struct On {
    followed: Followed,
    _alone: File,
}

/// The work the wish follows, read behind the keeps' lock, which is returned held. None: the wish follows none.
/// Refused: the lock is not to be had, and every reason a follow-up is not made for.
pub(super) fn on(home: &Path, day: &str, wish: &MadeWish) -> Result<Option<On>, Error> {
    let Some(after) = wish.after.as_deref() else { return Ok(None) };
    let alone = copy::alone(home).map_err(|unkept| {
        refused(format!("{}: nothing was started", if unkept == Unkept::Busy { BUSY } else { NO_LOCK }))
    })?;
    Ok(Some(On { followed: followed(home, day, after)?, _alone: alone }))
}

impl On {
    /// Where the follow-up's copy is made: at the followed work's commit while that work waits, and where the
    /// person's checkout stands once it was kept, when that checkout holds the commit it was kept as.
    pub(super) fn base(&self) -> Base<'_> {
        let at = self.followed.waits.then_some(self.followed.end.as_str());
        Base { repo: &self.followed.repo, run: &self.followed.run, at, kept: self.followed.kept.as_deref() }
    }

    /// The run that is followed, and whether its session is forked.
    pub(super) fn named(&self) -> (&str, bool) {
        (&self.followed.run, self.followed.session.is_some())
    }

    /// The session to fork. The keeps' lock is let go of with this: the wish names its run by now.
    pub(super) fn session(self) -> Option<String> {
        self.followed.session
    }
}

/// What one start of a run's harness is given, whatever its session.
pub(super) struct Go<'a> {
    pub home: &'a Path,
    pub run: &'a str,
    /// The copy's folder, and the commit it was made at.
    pub folder: &'a str,
    pub start: &'a str,
    /// The harness's whole input.
    pub said: String,
    pub limit: Duration,
}

impl Go<'_> {
    /// One start of the harness, waited for to its end.
    pub(super) fn once(&self, mut command: Command, session: String, from: Session) -> io::Result<Ended> {
        let expect = (lined(&mut command, self.folder, session, from), Live::of(self.home, self.run, self.folder));
        let (told, stop) = (|pid| record::child(self.home, self.run, pid), || stop::asked(self.home, self.run));
        let began = || record::began(self.home, self.run);
        child::run(command, self.said.clone(), self.limit, expect, told, began, stop)
    }

    /// Starts a run's harness under `session`, as a fork of the session `fork` when there is one. A harness
    /// that says it keeps no session of that id (`asks::LOST_SIGN`) is started once more, as any run is, under a
    /// new session of core's: the record names that one, and says the session is no fork, before that start.
    /// Only in a copy that is as it was made (`copy::as_made`): a start that worked in it before it said so has
    /// failed (`asks::WORKED`), and no second start is given what it left.
    pub(super) fn started(&self, harness: find::Found, session: String, fork: Option<&str>) -> io::Result<Ended> {
        let Some(fork) = fork else { return self.once(harness_of(harness)?, session, Session::New) };
        let ended = self.once(harness_of(harness.clone())?, session, Session::Fork(fork))?;
        if !ended.lost {
            return Ok(ended);
        }
        if !copy::as_made(self.home, self.run, self.start) {
            return Ok(Ended { worked: true, ..ended });
        }
        let session = asks::fresh();
        if let Some(record) = record::kept(self.home, self.run) {
            let (session, forked, pid, mark) = (Some(session.clone()), Some(false), None, None);
            let anew = Record { session, forked, pid, mark, began: false, ..record };
            record::write(self.home, self.run, &anew).map_err(|e| io::Error::other(e.to_string()))?;
        }
        self.once(harness_of(harness)?, session, Session::New)
    }
}

/// The line of a run that finished: a run's own, and of a follow-up that changed nothing in a copy made at the
/// commit the work it follows ended as, that its branch is there.
pub(super) fn ended_as(home: &Path, run: &str, start: &str, changed: bool) -> String {
    let followed = || record::kept(home, &record::kept(home, run)?.follows?);
    match followed().filter(|_| !changed) {
        Some(followed) if followed.end.as_deref() == Some(start) => unchanged_on(run),
        _ => finished(run, changed),
    }
}

/// The order of keeps (decision 9), asked by the keep of the run `run` with the keeps' lock held: Err, in the
/// words to refuse in, while the work `record` says it follows waits. Else the commit that work was kept as, when
/// the run started at that work's own commit and the keep made another: the branch holds the followed work
/// under that commit, which stands for the run's start in the keep (`copy::Keep::held`).
pub(super) fn keep_first(home: &Path, made: &MadeWish, run: &str, record: &Record) -> Result<Option<String>, String> {
    let (Some(after), Some(followed)) = (made.after.as_deref(), record.follows.as_deref()) else { return Ok(None) };
    if record::work_waits(home, followed) {
        return Err(format!(
            "the work of run {run} follows the work of wish {after}, which still waits: keep wish {after} first, \
             and nothing was changed"
        ));
    }
    let was = record::kept(home, followed).filter(|was| was.work == Some(WorkState::Kept) && was.end == record.start);
    Ok(was.and_then(|was| was.kept).filter(|kept| copy::is_id(kept) && Some(kept) != record.start.as_ref()))
}

/// Whether a run still stands on the work it follows: it goes or asks, or its own work is neither kept nor
/// tossed. A run that did not finish, or that changed nothing, holds nothing of the work it started at.
fn stands(run: &str, record: &Record) -> bool {
    let tossed = record.work == Some(WorkState::Tossed);
    match record.outcome {
        None => true,
        Some(RunOutcome::Asks) => !tossed,
        Some(RunOutcome::Completed) => !tossed && record.why.as_deref() == Some(finished(run, true).as_str()),
        _ => false,
    }
}

/// The order of tosses (decision 9), asked by the toss of the work of `run`, the run of the wish `id`, with the
/// keeps' lock held: refused while the run of a wish that follows it stands on that work, naming that wish.
pub(super) fn toss_first(home: &Path, day: &str, id: &str, run: &str) -> Result<(), Error> {
    let follows = |wish: &MadeWish| wish.after.as_deref() == Some(id);
    for wish in wish::kept_of(home, day)?.into_iter().filter(follows) {
        let wish = bury(home, day, wish);
        let own = wish.run.as_deref().and_then(|own| Some((own, record::kept(home, own)?)));
        if let Some((own, _)) = own.filter(|(own, record)| stands(own, record)) {
            return Err(refused(format!(
                "wish {} follows the work of run {run}, and its run {own} is not tossed: toss wish {} first, and \
                 nothing was changed",
                wish.id, wish.id
            )));
        }
    }
    Ok(())
}
