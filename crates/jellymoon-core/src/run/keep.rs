//! `jellymoon run keep` (F45, ADR 0017, decisions 4 to 6 and 8; `docs/development/keep.md`): the work of a made
//! wish's finished run is landed on the branch the person stands on. Which run, and whether there is work to keep,
//! is read from the home as a look reads it (`show.rs`), and the record must name the run's commits: only a look
//! goes by a branch's shape. What is done in the repo is `copy/keep.rs`. Here are the record's part and every word
//! that is printed: no refusal holds a word of git's, of the agent's or of the repo's, and a name is printed
//! only as data, cleaned at the door every kept text is cleaned at.

use std::fmt;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::obeyed::{modules, obeyed};
use super::show::{Asked, NAME_MAX, found, not_cores, unread_words};
use super::{RunOptions, WorkState, finished, place, record, refused};
use crate::Error;
use crate::copy::{self, How, Keep, LISTED, Named, Note, Touched, Unkept};
use crate::still::statement;

const KEPT: Asked = Asked { to: "keep", done: "kept", going: "is going: its work can be kept once it has finished" };

/// What `jellymoon run keep` is asked beside the wish.
#[derive(Debug, Clone, Copy, Default)]
pub struct KeepOptions {
    /// Print what a keep would do, or why it would be refused, and change nothing.
    pub ask: bool,
    /// Keep work that touches files the person's tools obey.
    pub with_settings: bool,
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
    /// With `--ask` only: whether a keep would be done.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fits: Option<bool>,
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
        Err(busy) if how.ask => return Ok(unfit(run, wish, busy, Vec::new())),
        Err(busy) => return Err(refused(words(&run, busy))),
    };
    let named = Named { repo: &repo, start: &start, end: &end };
    let was = was.as_ref().map(|was| (was.0.as_str(), was.1.as_str()));
    let of = |folder| Keep { home, folder, run: &run, named: &named, was, ask: how.ask };
    if state == WorkState::Kept {
        let Some((commit, onto)) = was else { return Err(not_cores(&run)) };
        // A keep that was cut off after the record said kept left the run's branch: it goes now, unless the
        // work is on no branch, where the run's branch is the one name it has.
        if let (Ok(folder), false, true) = (&folder, how.ask, onto != copy::LOOSE) {
            copy::swept(&of(folder));
        }
        let why = format!("the work of run {run} was kept before: nothing was changed");
        let (kept, onto, fits) = (Some(commit.to_string()), short(onto), how.ask.then_some(false));
        return Ok(Kept { run, wish, kept, onto, why, settings: Vec::new(), fits });
    }
    // Only a record that says kept names no branch: a keep writes the branch it means before it lands anything.
    if was.is_some_and(|was| was.1 == copy::LOOSE) {
        return Err(not_cores(&run));
    }
    let folder = folder?;
    let (mut settings, mut record) = (Vec::new(), found.record);
    let mut note = |note: Note| {
        let (commit, onto, done) = match note {
            Note::Intent(commit, onto) => (commit, onto, false),
            Note::Kept(commit, onto) => (commit, onto, true),
        };
        (record.kept, record.onto) = (Some(commit.to_string()), Some(onto.to_string()));
        record.work = done.then_some(WorkState::Kept);
        record::write(home, &run, &record).is_ok()
    };
    let landed = copy::keep(&of(&folder), &mut |files| judge(files, how.with_settings, &mut settings), &mut note);
    match landed {
        Ok(landed) => {
            let (why, onto, fits) = (done(&run, landed.how, how.ask), short(&landed.onto), how.ask.then_some(true));
            Ok(Kept { run, wish, kept: landed.commit, onto, why, settings, fits })
        }
        Err(unkept) if how.ask => Ok(unfit(run, wish, unkept, settings)),
        Err(unkept) => Err(refused(words(&run, unkept))),
    }
}

/// What `--ask` prints of work that would not be kept.
fn unfit(run: String, wish: String, unkept: Unkept, settings: Vec<String>) -> Kept {
    Kept { why: words(&run, unkept), run, wish, kept: None, onto: None, settings, fits: Some(false) }
}

/// The commit and the branch a keep wrote in the record, both or neither, as a keep writes them: a whole object
/// id, and a branch's whole name by the rule the keep itself writes one by (`copy::nameable`), or the one word
/// for work that landed on no branch a record names.
fn intent<'a>(kept: &'a Option<String>, onto: &'a Option<String>) -> Result<Option<(&'a str, &'a str)>, ()> {
    match (kept.as_deref(), onto.as_deref()) {
        (None, None) => Ok(None),
        (Some(kept), Some(onto)) if copy::is_id(kept) && (copy::nameable(onto) || onto == copy::LOOSE) => {
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
/// only when that was asked for, and its cleaned name is added to `settings` either way, up to the most a list has.
fn judge(files: &[Touched], with_settings: bool, settings: &mut Vec<String>) -> Result<(), Unkept> {
    let mut obeys = false;
    for file in files {
        let raw = String::from_utf8_lossy(&file.path);
        let name = statement(&raw, NAME_MAX);
        if file.repo || modules(&raw) || modules(&name) {
            return Err(Unkept::Repos);
        }
        if obeyed(&raw) || obeyed(&name) {
            obeys = true;
            if settings.len() < LISTED && !settings.contains(&name) {
                settings.push(name);
            }
        }
    }
    if obeys && !with_settings { Err(Unkept::Settings) } else { Ok(()) }
}

/// What a keep did, or would do.
fn done(run: &str, how: How, ask: bool) -> String {
    match (how, ask) {
        (How::Forward, false) => format!("run {run} was kept: the branch was moved forward to the run's commit"),
        (How::Forward, true) => format!("a keep of run {run} would move the branch forward to the run's commit"),
        (How::Joined, false) => {
            format!("run {run} was kept: one new commit on top of the branch's own holds its work and the run's")
        }
        (How::Joined, true) => format!(
            "a keep of run {run} would land one new commit on top of the branch's own, holding its work and the run's"
        ),
        (How::Held, false) => {
            format!("run {run} was kept: the branch held the run's work already, and nothing was landed")
        }
        (How::Held, true) => {
            format!("a keep of run {run} would land nothing: the branch holds the run's work already")
        }
        (How::Finished, false) => {
            format!("run {run} was kept: a keep that was cut off had landed its work, and this one finished it")
        }
        (How::Finished, true) => {
            format!("a keep of run {run} would finish one that was cut off after it had landed the work")
        }
        // Only a keep that took the step finds the checkout moved: nothing is asked that would.
        (How::Elsewhere, _) => format!(
            "run {run} was kept, on another branch than the keep began on: the checkout was moved to the branch \
             that `onto` names while the keep was at work, and git landed the work there"
        ),
        (How::Loose, _) => format!(
            "run {run} was kept, on no branch that is named here: while the keep was at work the checkout's HEAD \
             was detached, or moved to a branch whose name Jellymoon does not write down, and git landed the work \
             there; the run's branch is left"
        ),
    }
}

/// The words of the three refusals that are of the home, the config and the branch's name, and not of the work.
fn before_the_work(unkept: &Unkept) -> &'static str {
    match unkept {
        Unkept::NoLock => "the keeps' lock (runs/keep.lock) could not be made or taken in this home",
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
        Unkept::Busy => format!("another keep is at work in this home: {unchanged}"),
        Unkept::NoLock | Unkept::Config | Unkept::Unnamed => format!("{}: {unchanged}", before_the_work(&unkept)),
        Unkept::Stood => format!("a checkout stands on the branch of run {run}: {unchanged}"),
        Unkept::NoBranch => format!(
            "the checkout stands on no branch of the person's: its HEAD is detached, or names a run's branch: \
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
        Unkept::Clash => format!(
            "the branch's commits since run {run} started and the run's work changed the same lines, or the same \
             file in a way git does not merge: {unchanged}"
        ),
        Unkept::Nobody => format!(
            "git has no name and address for this repo's commits (user.name and user.email), and Jellymoon makes \
             none up: {unchanged}"
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
        Unkept::Locked => format!(
            "the checkout's index is locked (index.lock): another git is at work in it, or one was cut off and \
             left the file: the work of run {run} was not kept"
        ),
        Unkept::Stepped => format!(
            "git would not move the branch forward to the work of run {run}: git stopped the step, files of the \
             work may already be written to the checkout, and the next `run keep` begins again"
        ),
        Unkept::Unwritten => format!(
            "the record of run {run} could not be written: the next `run keep` goes on from where this one stopped"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_a_keep_wrote_in_a_record_is_an_id_and_a_branchs_whole_name_or_the_record_is_not_cores() {
        let id = "a".repeat(40);
        let named = |kept: &str, onto: &str| intent(&Some(kept.to_string()), &Some(onto.to_string())).map(|_| ());
        assert_eq!(intent(&None, &None), Ok(None));
        assert_eq!(named(&id, "refs/heads/main"), Ok(()));
        assert_eq!(named(&id, "refs/heads/feature/x-1"), Ok(()));
        // The one word of work that landed on no branch, and names of other alphabets and of 1024 bytes.
        for onto in ["HEAD", "refs/heads/ok-ünï", "refs/heads/@", &format!("refs/heads/{}", "x".repeat(1013))] {
            assert_eq!(named(&id, onto), Ok(()), "{onto}");
        }
        for onto in [
            "refs/heads/a\u{a0}b",
            "refs/heads/a\u{3000}b",
            "refs/heads/a\u{2028}b",
            "refs/heads/a\u{85}b",
            "refs/heads/a\u{9b}b",
            &format!("refs/heads/{}", "x".repeat(1014)),
            "refs/heads/HEAD ",
            "head",
            "main",
            "refs/tags/v1",
            "refs/heads/a b",
            "refs/heads/*",
            "refs/heads/a\nb",
            "refs/heads/a?",
            "refs/heads/[a]",
        ] {
            assert_eq!(named(&id, onto), Err(()), "{onto}");
        }
        for kept in ["main", "HEAD", "--all", &id[..39]] {
            assert_eq!(named(kept, "refs/heads/main"), Err(()), "{kept}");
        }
        assert_eq!(intent(&Some(id), &None), Err(()));
        assert_eq!(intent(&None, &Some("refs/heads/main".to_string())), Err(()));
    }

    #[test]
    fn another_repos_entry_is_never_kept_and_a_file_tools_obey_only_when_asked() {
        let file = |path: &str, repo: bool| Touched::of(path.as_bytes(), repo);
        let asked = |files: &[Touched], with: bool| {
            let mut settings = Vec::new();
            (judge(files, with, &mut settings), settings)
        };
        assert_eq!(asked(&[file("src/a.rs", false)], false), (Ok(()), vec![]));
        let obeyed = [file("src/a.rs", false), file(".claude/settings.json", false), file("CLAUDE.md", false)];
        let names = vec![".claude/settings.json".to_string(), "CLAUDE.md".to_string()];
        assert_eq!(asked(&obeyed, false), (Err(Unkept::Settings), names.clone()));
        assert_eq!(asked(&obeyed, true), (Ok(()), names));
        // Another repo's entry is refused whatever was asked for, and before a file tools obey is.
        for repos in
            [file("sub", true), file(".gitmodules", false), file("x/.GITMODULES", false), file(".gitmodules ", false)]
        {
            let files = [file("CLAUDE.md", false), repos];
            assert_eq!(asked(&files, true).0, Err(Unkept::Repos));
            assert_eq!(asked(&files, false).0, Err(Unkept::Repos));
        }
        // A character nobody sees does not hide a name: the name is asked as it is printed too.
        assert_eq!(asked(&[file(".git\u{200b}modules", false)], true).0, Err(Unkept::Repos));
    }
}
