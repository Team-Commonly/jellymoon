//! What a finished run made, shown (F44, ADR 0017, `docs/development/show.md`): [`show_run`] lists the files of
//! the run's work and [`show_file`] prints one of them as its text. Which run, and whether there is work to show,
//! is read from the home as a stop reads it (`stop.rs`); the work itself is read from the repo's store
//! (`copy/work.rs`), as the change from the start to the end the run's record names, in the repo it names (F48).
//! Nothing is written: not the home, not the repo, and no file of the work anywhere. Every name and every text
//! is an agent's, and is cleaned before it is printed; no refusal holds either, or the repo's path.

use std::fmt;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::end::{held, of_another, recorded};
use super::obeyed::obeyed;
use super::record::Record;
use super::{RunOptions, RunOutcome, finished, no_wish, place, record, refused};
use crate::copy::{self, Done, Entry, LISTED, Named, RunCopy, TEXT_MAX, Unread, WorkFile};
use crate::still::{statement, still_text};
use crate::{Error, wish};

/// The most characters of a file's name that are printed.
const NAME_MAX: usize = 200;

/// Whether a run's work waits for the person, or they kept it or tossed it. A record that does not say waits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkState {
    Waits,
    Kept,
    Tossed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileChange {
    Made,
    Changed,
    Removed,
}

/// What a file of the work is: text that `--file` prints, a link, another repo's entry (a submodule), or
/// anything else, which is named and sized and never printed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileKind {
    Text,
    Other,
    Link,
    Repo,
}

/// One file of a run's work, as the list prints it.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShownEntry {
    /// Its number in the list, from 1: what `--file` takes.
    pub n: usize,
    /// Its path in the repo, cleaned, on one line, at most 200 characters.
    pub name: String,
    pub change: FileChange,
    /// Its bytes: of a removed file, what it had; of a link, its target's length; of another repo's entry, 0.
    pub size: u64,
    pub kind: FileKind,
    /// Whether the person's tools obey a file of this name.
    pub obeyed: bool,
}

/// What `jellymoon run show` prints.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Shown {
    pub run: String,
    pub wish: String,
    pub work: WorkState,
    /// The first `most` files. None for work that is not waiting, and for a run that changed nothing.
    pub files: Vec<ShownEntry>,
    /// How many files there are after those.
    pub more: usize,
    pub most: usize,
}

/// What `jellymoon run show --file <n>` prints.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShownFile {
    pub run: String,
    pub wish: String,
    pub work: WorkState,
    pub n: usize,
    pub name: String,
    pub size: u64,
    /// The file as it is in the work, cleaned: its first `most` bytes, up to the last whole word when `cut`.
    pub text: String,
    pub cut: bool,
    pub most: usize,
}

impl fmt::Display for Shown {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&serde_json::to_string(self).map_err(|_| fmt::Error)?)
    }
}

impl fmt::Display for ShownFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&serde_json::to_string(self).map_err(|_| fmt::Error)?)
    }
}

/// The run of a made wish that finished, how its work stands, and the first files of work that waits.
struct Work {
    run: String,
    state: WorkState,
    files: Vec<WorkFile>,
    more: usize,
}

/// Finds the wish's run and reads its work. Refused: a wish that is not there, one with no run or another wish's,
/// a run that is going or did not finish, and a record that is not core's. git is asked only for a run that
/// finished with a commit, whose work waits.
fn work(home: &Path, options: &RunOptions) -> Result<Work, Error> {
    let day = wish::day_of(options.day.as_deref())?;
    let id = options.wish.as_str();
    let made = wish::kept_of(home, &day)?.into_iter().find(|made| made.id == id).ok_or_else(|| no_wish(&day))?;
    let Some(run) = made.run.clone().filter(|run| copy::is_run_id(run)) else {
        return Err(refused(format!("wish {id} has had no run, so there is no work to show")));
    };
    let no = |why: &str| refused(format!("run {run} {why}"));
    let Some(kept) = record::kept(home, &run) else {
        return Err(refused(format!("the record of run {run} cannot be read, so its work is not shown")));
    };
    if of_another(home, &run, Some(&kept), &day, id) {
        return Err(refused(format!("wish {id} names run {run}, which is another wish's run: its work is not shown")));
    }
    if kept.outcome.is_none() {
        let cut = "was cut off before it ended: there is no work of it to show";
        return Err(no(if held(home, &run) { "is going: its work is shown once it has finished" } else { cut }));
    }
    let not_cores = || refused(format!("the record of run {run} is not one Jellymoon wrote for this wish"));
    let (outcome, why) = recorded(home, &day, id, &run, &kept).ok_or_else(not_cores)?;
    match outcome {
        RunOutcome::Completed => {}
        RunOutcome::Cancelled => return Err(no("was cancelled by a stop: there is no work of it to show")),
        _ => return Err(no("did not finish: there is no work of it to show")),
    }
    let state = kept.work.unwrap_or(WorkState::Waits);
    let copy = RunCopy { run: run.clone(), folder: String::new(), branch: copy::branch(&run), kept: false };
    if state != WorkState::Waits || why != finished(&copy, true) {
        return Ok(Work { run, state, files: Vec::new(), more: 0 });
    }
    // Before git is asked anything: what the record names is in the shape core writes, or the record is not core's.
    let named = named(&kept).map_err(|()| not_cores())?;
    // The repo is found as a run finds it, whatever the record names: the record's folder is only compared.
    let folder = place::folder(home, &day, &made, options)?;
    let read = copy::work_of(home, &folder, &run, options.repo.is_none(), named.as_ref());
    let (files, more) = read.map_err(|unread| unshown(&run, unread))?;
    Ok(Work { run, state, files, more })
}

/// What the record of a run that ended with a commit names of it: its repo, its start and its end, all three, or
/// none of them in a record from before F48. Err for a record that names some and not others, a repo by a path
/// that does not start at a root (a run writes the folder as the system resolves it, from a root), or a commit by
/// anything but a whole object id. Such a word never reaches git, and neither does one that passes: an id is only
/// ever compared with what git printed (`copy/work.rs`).
fn named(record: &Record) -> Result<Option<Named<'_>>, ()> {
    match (record.repo.as_deref(), record.start.as_deref(), record.end.as_deref()) {
        (None, None, None) => Ok(None),
        (Some(repo), Some(start), Some(end)) if repo.is_absolute() && copy::is_id(start) && copy::is_id(end) => {
            Ok(Some(Named { repo, start, end }))
        }
        _ => Err(()),
    }
}

fn unshown(run: &str, unread: Unread) -> Error {
    let branch = format!("the branch of run {run}");
    refused(match unread {
        Unread::Repo => format!(
            "the work of run {run} could not be read from the repo: the folder is not the top of a checkout git \
             opens, or git did not answer"
        ),
        Unread::Missing => format!("{branch} is not in the repo, so there is no work of it to show"),
        Unread::Symbolic => format!("{branch} is a name for another branch by now, so its work is not shown"),
        Unread::Moved => format!("{branch} is not as the run left it, so what is on it is not shown as the run's"),
        Unread::Another => {
            format!("the repo found for run {run} is not the one its record names, so its work is not shown")
        }
        Unread::Start => format!(
            "the work of run {run} does not start where its record says the run started, so it is not shown as the run's"
        ),
        Unread::Many => format!("the work of run {run} is more files than Jellymoon lists"),
    })
}

/// The files of a finished run's work. Nothing is written anywhere.
pub fn show_run(home: &Path, options: &RunOptions) -> Result<Shown, Error> {
    let work = work(home, options)?;
    let files = work.files.iter().enumerate().map(|(at, file)| entry(at + 1, file)).collect();
    Ok(Shown { run: work.run, wish: options.wish.clone(), work: work.state, files, more: work.more, most: LISTED })
}

/// File `n` of that list as its text. Refused in fixed words, with no byte of the file: a number that names no
/// file, a file the work removed, a link, another repo's entry, what is not text, and a text that cleaning goes
/// on changing.
pub fn show_file(home: &Path, options: &RunOptions, n: usize) -> Result<ShownFile, Error> {
    let work = work(home, options)?;
    let no = |why: &str| refused(format!("file {n} of run {} {why}", work.run));
    let Some(file) = n.checked_sub(1).and_then(|at| work.files.get(at)) else {
        return Err(no("names no file: `run show` lists the files, each with its number"));
    };
    let shown = entry(n, file);
    let text = match (shown.change, shown.kind, head(file)) {
        (FileChange::Removed, _, _) => return Err(no("was removed by the work, so there is none of it to show")),
        (_, FileKind::Link, _) => return Err(no("is a link, and where it leads is not shown")),
        (_, FileKind::Repo, _) => return Err(no("is another repo's entry, and nothing of it is in this repo")),
        (_, _, None) => return Err(no("is not text, so it is not shown")),
        (_, _, Some(text)) => text,
    };
    let cut = file.size > TEXT_MAX as u64;
    // The word the cut fell in can be the head of a key whose rest was never read: it is not shown.
    let text = if cut { text.rfind(char::is_whitespace).map_or("", |at| &text[..at]) } else { text };
    let text = still_text(text, TEXT_MAX).map_err(|_| no("cannot be cleaned, so it is not shown"))?;
    let (run, wish, work) = (work.run.clone(), options.wish.clone(), work.state);
    Ok(ShownFile { run, wish, work, n, name: shown.name, size: file.size, text, cut, most: TEXT_MAX })
}

/// What was read of a file, as text: every byte of it is of a character, and none is a zero. Only where reading
/// stopped short of the file's end may the last character be one that was cut in two, and it is left out.
fn head(file: &WorkFile) -> Option<&str> {
    let whole = file.size <= TEXT_MAX as u64;
    let text = match std::str::from_utf8(&file.head) {
        Ok(text) => text,
        Err(e) if !whole && e.error_len().is_none() => std::str::from_utf8(&file.head[..e.valid_up_to()]).ok()?,
        Err(_) => return None,
    };
    (file.entry == Entry::File && !text.contains('\0')).then_some(text)
}

fn entry(n: usize, file: &WorkFile) -> ShownEntry {
    let raw = String::from_utf8_lossy(&file.path);
    let name = statement(&raw, NAME_MAX);
    let change = match file.done {
        Done::Made => FileChange::Made,
        Done::Changed => FileChange::Changed,
        Done::Removed => FileChange::Removed,
    };
    let kind = match file.entry {
        Entry::Link => FileKind::Link,
        Entry::Repo => FileKind::Repo,
        Entry::File if head(file).is_some() => FileKind::Text,
        Entry::File => FileKind::Other,
    };
    // Asked of the name as git keeps it and as it is printed: a character nobody sees must not hide one.
    let obeyed = obeyed(&raw) || obeyed(&name);
    ShownEntry { n, name, change, size: file.size, kind, obeyed }
}
