//! `jellymoon run keep` (F45, ADR 0017, decisions 4 to 6 and 8; `docs/development/keep.md`): the work of a made
//! wish's finished run is landed on the branch the person stands on. Which run, and whether there is work to keep,
//! is read from the home as a look reads it (`show.rs`), and the record must name the run's commits: only a look
//! goes by a branch's shape. What is done in the repo is `copy/keep.rs`. Here are the record's part and every word
//! that is printed: no refusal holds a word of git's, of the agent's or of the repo's, and a name is printed
//! only as data, cleaned at the door every kept text is cleaned at.

use std::collections::HashSet;
use std::fmt;
use std::path::Path;

mod done;
mod itself;

use done::done;
pub(super) use itself::keep_itself;
use serde::{Deserialize, Serialize};

use super::obeyed::{modules, named};
use super::show::{Asked, NAME_MAX, found, not_cores, unread_words};
use super::{RunOptions, WorkState, finished, follow, place, record, refused};
use crate::Error;
use crate::copy::{self, How, Keep, LISTED, Named, Note, Touched, Unkept};
use crate::still::statement;

/// Why a keep, a toss or the start of a follow-up's run (`follow.rs`) did not have the lock all three hold
/// (`runs/keep.lock`): one of them holds it, or it is not to be had.
pub(super) const BUSY: &str = "a keep, a toss or the start of a follow-up's run is at work in this home";
pub(super) const NO_LOCK: &str =
    "the lock a keep or a toss holds (runs/keep.lock) could not be made or taken in this home";
const KEPT: Asked = Asked { to: "keep", done: "kept", going: "is going: its work can be kept once it has finished" };

/// What `jellymoon run keep` is asked beside the wish.
#[derive(Debug, Clone, Copy, Default)]
pub struct KeepOptions {
    /// Print what a keep would do, or why it would be refused, and change nothing.
    pub ask: bool,
    /// Keep work that touches files the person's tools obey.
    pub with_settings: bool,
}

/// Which files the person's tools obey a keep may land: all of them or none, as a command says, or for a page
/// the ones the person was named and said yes to, which must be the work's own, all of them and no other. A keep
/// by itself has no yes at all: it lands none of them, and no change to a `.gitattributes` (`itself.rs`).
#[derive(Clone, Copy)]
enum Yes<'a> {
    Said(bool),
    To(&'a [String]),
    Itself,
}

/// What `jellymoon run keep` prints.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Kept {
    pub run: String,
    pub wish: String,
    /// The commit on the person's branch that holds the work. None when asked, of a commit that would have to be
    /// made, and of work that would not be kept.
    pub kept: Option<String>,
    /// The branch, by its short name, cleaned, on one line. None of work that would not be kept, and of work
    /// that was kept on no branch a record names.
    pub onto: Option<String>,
    /// What was done, or with `--ask` what would be, in core's fixed words.
    pub why: String,
    /// The files of the work that the person's tools obey, cleaned, at most 200. Left out when there are none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub settings: Vec<String>,
    /// Each `.gitattributes` the work removes that was still at its path after the keep, cleaned, at most 200:
    /// `why` then ends with words that say so, and that there are more. Left out when there are none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stayed: Vec<String>,
    /// With `--ask` only: whether a keep would be done.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fits: Option<bool>,
    /// How this keep landed the work, for the words a page says of it (`waits/plain.rs`): never printed. None
    /// when asked, of work kept before and of work that would not be kept.
    #[serde(skip)]
    pub(crate) how: Option<How>,
}

impl fmt::Display for Kept {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&serde_json::to_string(self).map_err(|_| fmt::Error)?)
    }
}

/// Keeps the work of the wish's run, or with `ask` says what a keep would do. Refused as a look is, and: work
/// that was tossed, a run that changed nothing, a record that names no commits, and every reason of
/// [`Unkept`], each with nothing changed. With `ask`, a reason of [`Unkept`] is printed and not refused.
pub fn keep_run(home: &Path, options: &RunOptions, how: KeepOptions) -> Result<Kept, Error> {
    keep(home, options, how.ask, Yes::Said(how.with_settings))
}

/// A keep from a page (`waits.rs`): [`keep_run`], but of the files the person's tools obey it lands only those
/// `named`, by their names as they are printed, and only when they are every such file of the work. Work that
/// would land another, or that has not one of them, is refused whole, in a page's words.
pub(super) fn keep_named(home: &Path, options: &RunOptions, named: &[String]) -> Result<Kept, Error> {
    keep(home, options, false, Yes::To(named))
}

fn keep(home: &Path, options: &RunOptions, ask: bool, yes: Yes) -> Result<Kept, Error> {
    // Before the record is read: what a keep that ended a moment ago wrote is what this one goes by.
    let alone = copy::alone(home);
    let found = found(home, options, &KEPT)?;
    let (run, wish) = (found.run.clone(), options.wish.clone());
    let named = found.named()?.map(|named| (named.repo.to_path_buf(), named.start.to_string(), named.end.to_string()));
    let was = intent(&found.record.kept, &found.record.onto).map_err(|()| not_cores(&run))?;
    let was = was.map(|(kept, onto)| (kept.to_string(), onto.to_string()));
    let state = found.record.work.unwrap_or(WorkState::Waits);
    if state == WorkState::Tossed {
        return Err(refused(format!("the work of run {run} was tossed, so nothing of it was kept")));
    }
    if found.why != finished(&run, true) {
        return Err(refused(format!("run {run} changed nothing: there is no work of it to keep")));
    }
    let Some((repo, start, end)) = named else {
        return Err(refused(format!(
            "the record of run {run} is from before a run's record named its commits: its work is shown, and is \
             not kept by Jellymoon"
        )));
    };
    let folder = place::folder(home, &found.day, &found.made, options);
    let _alone = match alone {
        Ok(alone) => alone,
        Err(busy) if ask => return Ok(unfit(run, wish, busy, Vec::new())),
        Err(busy) => return Err(refused(words(&run, busy))),
    };
    // A follow-up is kept after the work it follows (ADR 0018, decision 9).
    let held = match follow::keep_first(home, &found.made, &run, &found.record) {
        Ok(held) => held,
        Err(why) if ask => return Ok(Kept { why, ..unfit(run, wish, Unkept::Busy, Vec::new()) }),
        Err(why) => return Err(refused(why)),
    };
    let named = Named { repo: &repo, start: &start, end: &end };
    let was = was.as_ref().map(|was| (was.0.as_str(), was.1.as_str()));
    let itself = matches!(yes, Yes::Itself);
    let of = |folder| Keep { home, folder, run: &run, named: &named, was, ask, held: held.as_deref(), itself };
    if state == WorkState::Kept {
        let Some((commit, onto)) = was else { return Err(not_cores(&run)) };
        if found.record.back.is_some() {
            return Err(refused(format!("an undo of the work of run {run} was cut off: `run undo` goes on with it")));
        }
        // A keep that was cut off after the record said kept left the run's branch: it goes now, unless the
        // work is on no branch, where the run's branch is the one name it has, or was kept by itself, where an
        // undo goes by it.
        let swept = onto != copy::LOOSE && found.record.before.is_none();
        if let (Ok(folder), false, true) = (&folder, ask, swept) {
            copy::swept(&of(folder));
        }
        let why = format!("the work of run {run} was kept before: nothing was changed");
        let (kept, onto, fits) = (Some(commit.to_string()), short(onto), ask.then_some(false));
        let (settings, stayed, how) = (Vec::new(), Vec::new(), None);
        return Ok(Kept { run, wish, kept, onto, why, settings, stayed, fits, how });
    }
    // Only a record that says kept names no branch: a keep writes the branch it means before it lands anything.
    if was.is_some_and(|was| was.1 == copy::LOOSE) {
        return Err(not_cores(&run));
    }
    let (folder, day) = (folder?, found.day.clone());
    let kept = landed(&of(&folder), wish, found.record, yes)?;
    if itself && matches!(kept.how, Some(How::Forward | How::Joined)) {
        itself::sweep(&of(&folder), &day);
    }
    Ok(kept)
}

/// The keep itself, and what it prints. What it notes for the record is written there as it goes; a
/// `.gitattributes` it notes as still there is named in the answer, whose `why` then ends with [`STAYED`].
fn landed(of: &Keep, wish: String, mut record: record::Record, yes: Yes) -> Result<Kept, Error> {
    let (run, mut settings, mut stayed, mut pointed) = (of.run.to_string(), Vec::new(), Vec::new(), "");
    let mut note = |note: Note| {
        let (commit, onto, done) = match note {
            Note::Pointed | Note::Unasked => {
                pointed = if matches!(note, Note::Pointed) { POINTED } else { UNASKED };
                return true;
            }
            Note::Intent(commit, onto, before) => {
                record.before = before.map(str::to_string);
                (commit, onto, false)
            }
            Note::Kept(commit, onto, meant) => {
                // Only a keep by itself that landed as it meant can be undone.
                record.before = record.before.take().filter(|_| meant);
                (commit, onto, true)
            }
            Note::Stayed(path) => {
                // One past what a list holds says that there are more.
                stayed.extend((stayed.len() <= LISTED).then(|| statement(&String::from_utf8_lossy(path), NAME_MAX)));
                return true;
            }
        };
        (record.kept, record.onto) = (Some(commit.to_string()), Some(onto.to_string()));
        record.work = done.then_some(WorkState::Kept);
        record::write(of.home, &run, &record).is_ok()
    };
    // The first files asked of are the work's own; the ones after are what one new commit would land of it.
    let mut works = true;
    let mut judged = |files: &[Touched]| judge(files, yes, std::mem::take(&mut works), &mut settings);
    let landed = copy::keep(of, &mut judged, &mut note);
    match landed {
        Ok(landed) => {
            let own = landed.commit.as_deref() == Some(of.named.end);
            let more = if stayed.len() > LISTED { MORE } else { "" };
            stayed.truncate(LISTED);
            let why =
                done(&run, landed.how, of.ask, own) + if stayed.is_empty() { "" } else { STAYED } + more + pointed;
            let (onto, fits, how) = (short(&landed.onto), of.ask.then_some(true), (!of.ask).then_some(landed.how));
            Ok(Kept { run, wish, kept: landed.commit, onto, why, settings, stayed, fits, how })
        }
        Err(unkept) if of.ask => Ok(unfit(run, wish, unkept, settings)),
        Err(Unkept::Settings) if matches!(yes, Yes::To(_)) => Err(refused(format!(
            "the files of the work of run {run} that your tools obey, as it would land in your checkout, are not \
             the ones a note named, so it was not kept, and nothing was changed"
        ))),
        Err(unkept) => Err(refused(words(&run, unkept))),
    }
}

/// What `--ask` prints of work that would not be kept.
fn unfit(run: String, wish: String, unkept: Unkept, settings: Vec<String>) -> Kept {
    let (why, stayed) = (words(&run, unkept), Vec::new());
    Kept { why, run, wish, kept: None, onto: None, settings, stayed, fits: Some(false), how: None }
}

/// What `why` ends with when a `.gitattributes` the work removes is still at its path after the step, whatever
/// left it there, and after that when there are more of them than a list holds.
const STAYED: &str = "; not every `.gitattributes` the work removes is gone: each one `stayed` names is still at \
                      that path in the checkout, and a file of the work it sets a filter for was written as the \
                      store holds it, not as that filter would have made it";
const MORE: &str = ", and more of them are still there than `stayed` names";
/// What `why` ends with when the run's branch was not deleted because a name of the person's is a name for it.
const POINTED: &str = "; the run's branch is left, as a name of the person's points at it";
/// And when the repo did not say what stands on the branch or points at it: a toss's words for a branch it leaves.
const UNASKED: &str = "; the run's branch is left, as the repo could not be found or asked";

/// Whether a record's `onto` is a branch a keep writes there: by the rule the keep itself writes a name by
/// (`copy::nameable`), and no run's branch, where no work is ever kept (`copy::runs`).
fn branch(onto: &str) -> bool {
    copy::nameable(onto) && !copy::runs(onto)
}

/// The commit and the branch a keep wrote in the record, both or neither, as a keep writes them: a whole object
/// id, and a branch's whole name ([`branch`]), or the one word for work that landed on no branch a record names.
pub(super) fn intent<'a>(kept: &'a Option<String>, onto: &'a Option<String>) -> Result<Option<(&'a str, &'a str)>, ()> {
    match (kept.as_deref(), onto.as_deref()) {
        (None, None) => Ok(None),
        (Some(kept), Some(onto)) if copy::is_id(kept) && (branch(onto) || onto == copy::LOOSE) => {
            Ok(Some((kept, onto)))
        }
        _ => Err(()),
    }
}

/// A branch's short name as it is printed. None of work on no branch a record names.
fn short(onto: &str) -> Option<String> {
    (onto != copy::LOOSE).then(|| statement(onto.strip_prefix("refs/heads/").unwrap_or(onto), NAME_MAX))
}

/// Whether work that touches these files may be kept. Each name is asked as git keeps it and as it is printed
/// (`show.rs`). Another repo's entry, or `.gitmodules`, is never kept. A file the person's tools obey is kept
/// only when `yes` says so, and its cleaned name is added to `settings` either way, up to the most a list has.
/// A page's yes is to names, held as a set: each such file must be among them, and when the files are the
/// `works` own, each name must be one of theirs.
fn judge(files: &[Touched], yes: Yes, works: bool, settings: &mut Vec<String>) -> Result<(), Unkept> {
    let asked: HashSet<&str> = match yes {
        Yes::To(names) => names.iter().map(String::as_str).collect(),
        Yes::Said(_) | Yes::Itself => HashSet::new(),
    };
    let (mut unasked, mut found) = (false, HashSet::new());
    for file in files {
        let raw = String::from_utf8_lossy(&file.path);
        let (name, obeys) = named(&file.path);
        if file.repo || modules(&raw) || modules(&name) {
            return Err(Unkept::Repos);
        }
        unasked |= matches!(yes, Yes::Itself) && itself::attributes(&raw);
        if obeys {
            unasked |= match yes {
                Yes::Said(all) => !all,
                Yes::To(_) => !asked.contains(name.as_str()),
                Yes::Itself => true,
            };
            if settings.len() < LISTED && !settings.contains(&name) {
                settings.push(name.clone());
            }
            found.insert(name);
        }
    }
    let other = works && matches!(yes, Yes::To(_)) && asked.iter().any(|name| !found.contains(*name));
    if unasked || other { Err(Unkept::Settings) } else { Ok(()) }
}

/// The words of the refusals of the home, the config, the branch's name and a file's name: no run's id in them.
fn before_the_work(unkept: &Unkept) -> &'static str {
    match unkept {
        Unkept::Spelled => {
            "the checkout's HEAD names a branch the repo does not list under that name: on a disk that takes two \
             spellings for one name it is another spelling of a branch's name, and `git checkout` with the name as \
             `git branch` prints it stands on the branch itself"
        }
        Unkept::Name => {
            "a file of the work has a name this disk does not take: a part of it is too long, or is no name on \
             this system"
        }
        Unkept::NoLock => NO_LOCK,
        Unkept::Config => {
            "the repo's git config names a filter, a merge driver or a transport that cannot be switched off on \
             git's command line: there are too many of them, or one has `=` in its name"
        }
        _ => {
            "the checkout stands on a branch whose name Jellymoon does not write in a run's record: the name holds \
             a space of some kind, a line break or a control character, or is longer than 1024 bytes"
        }
    }
}

/// Why a run's work was not kept, or with `--ask` would not be: fixed words, with the run's id and nothing else.
fn words(run: &str, unkept: Unkept) -> String {
    let unchanged = format!("the work of run {run} was not kept, and nothing was changed");
    match unkept {
        Unkept::Unread(unread) => unread_words(run, unread, &KEPT),
        Unkept::Busy => format!("{BUSY}: {unchanged}"),
        Unkept::NoLock | Unkept::Config | Unkept::Unnamed | Unkept::Spelled | Unkept::Name => {
            format!("{}: {unchanged}", before_the_work(&unkept))
        }
        Unkept::Unmerged => format!(
            "git could not merge the work of run {run} with the branch's commits since in the repo's store: the \
             store takes no new object (the merge writes one tree there, asked or not), or git did not answer: \
             {unchanged}"
        ),
        Unkept::Stood => format!("a checkout stands on the branch of run {run}: {unchanged}"),
        Unkept::NoBranch => format!(
            "the checkout stands on no branch of the person's: its HEAD is detached, or names a branch named as a \
             run's is: \
             {unchanged}"
        ),
        Unkept::Midway => {
            format!("a merge, a cherry-pick, a revert, a rebase or a bisect is going on in the checkout: {unchanged}")
        }
        Unkept::History => format!("the branch's history does not hold the commit run {run} started from: {unchanged}"),
        Unkept::Old => format!(
            "the branch has commits since run {run} started, and this git is too old to merge them with the work in \
             the store (it is from before 2.38): {unchanged}"
        ),
        Unkept::NoBase => format!(
            "the work run {run} follows was kept as a new commit, and this git is too old to keep a follow-up's \
             work on top of one (it is from before 2.40): {unchanged}"
        ),
        Unkept::Clash => format!(
            "the branch's commits since run {run} started and the run's work changed the same lines, or the same \
             file in a way git does not merge: {unchanged}"
        ),
        Unkept::Repos => format!(
            "the work of run {run} adds, changes or removes another repo's entry or `.gitmodules`, and Jellymoon \
             never keeps such work: nothing was changed"
        ),
        Unkept::Settings => format!(
            "the work of run {run} touches files the person's tools obey, which `run keep --ask` names: it is kept \
             only with --with-settings, and nothing was changed"
        ),
        Unkept::Filter => format!(
            "a file of the work would be passed through a filter this repo sets for it, and Jellymoon starts no \
             filter: {unchanged}"
        ),
        Unkept::InWay => {
            format!("an unsaved change, or a file that is not in the branch, is where the work would go: {unchanged}")
        }
        Unkept::Locked | Unkept::Stepped | Unkept::OnRun | Unkept::Unwritten => at_the_step(run, &unkept),
    }
}

/// The words of the refusals a keep may come to at its step, which do not say that nothing was changed.
fn at_the_step(run: &str, unkept: &Unkept) -> String {
    match unkept {
        Unkept::Locked => format!(
            "the checkout's index is locked (index.lock): another git is at work in it, or one was cut off and \
             left the file: the work of run {run} was not kept"
        ),
        Unkept::Stepped => format!(
            "git would not move the branch forward to the work of run {run}: git stopped the step, files of the \
             work may already be written to the checkout, and the next `run keep` begins again"
        ),
        Unkept::OnRun => format!(
            "while the keep was at work the checkout was moved onto a branch named as a run's is, where Jellymoon \
             keeps no work: git may have moved that branch forward and written the work's files, and nothing is \
             recorded as kept; the branch of run {run} is left, and the next `run keep` begins again"
        ),
        _ => format!(
            "the record of run {run} could not be written: the next `run keep` goes on from where this one stopped"
        ),
    }
}

#[cfg(test)]
mod tests;
