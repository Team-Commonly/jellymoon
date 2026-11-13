//! What a finished run made, shown (F44, ADR 0017, `docs/development/show.md`): [`show_run`] lists the files of
//! the run's work and [`show_file`] prints one of them as its text. Which run, and whether there is work to show,
//! is read from the home as a stop reads it (`stop.rs`); the work itself is read from the repo's store
//! (`copy/work.rs`), as the change from the start to the end the run's record names, in the repo it names (F48).
//! Nothing of the work is written anywhere, and nothing in the repo or of the day; the home gets only what any
//! command that asks git makes there, empty (`show.md`, "What a look makes"). Every name and every text is an
//! agent's, and is cleaned before it is printed; no refusal holds either, or the repo's path.

use std::fmt;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::end::{held, of_another, recorded};
use super::obeyed::obeyed;
use super::record::Record;
use super::{RunOptions, RunOutcome, cut, cut_late, finished, no_wish, place, record, refused};
use crate::copy::{self, Done, Entry, LISTED, TEXT_MAX, Unread, WorkFile};
use crate::still::{statement, still_text};
use crate::wish::MadeWish;
use crate::{Error, wish};

/// The most characters of a file's name that are printed.
pub(super) const NAME_MAX: usize = 200;

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
    /// The file as it is in the work, cleaned: what was read of it, up to the last whole word when `cut`.
    pub text: String,
    /// Whether the file was not read whole (reading stopped at `most` bytes) or the cleaned text was cut at
    /// `most` characters. A text that cleaning only shortens, at its end, is not cut.
    pub cut: bool,
    /// The most bytes of a file that are read. It is no count of what is printed: cleaning can make a text
    /// longer, up to `most` characters, each of which can be several bytes.
    pub most: usize,
}

/// Each is printed as its one line of JSON.
macro_rules! printed_as_json {
    ($($shown:ty),*) => {$(
        impl fmt::Display for $shown {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&serde_json::to_string(self).map_err(|_| fmt::Error)?)
            }
        }
    )*};
}
printed_as_json!(Shown, ShownFile);

/// The run of a made wish that finished, how its work stands, and the first files of work that waits.
struct Work {
    run: String,
    state: WorkState,
    files: Vec<WorkFile>,
    more: usize,
}

/// The words of a refusal that differ between a look at a run's work and a keep of it (`keep.rs`).
pub(super) struct Asked {
    /// What is asked: `show`.
    pub to: &'static str,
    /// What is not done: `shown`.
    pub done: &'static str,
    /// What is said of a run that is going.
    pub going: &'static str,
}

const SHOWN: Asked = Asked { to: "show", done: "shown", going: "is going: its work is shown once it has finished" };

/// A made wish's run that completed, as the home has it: nothing of a repo is asked yet.
pub(super) struct Found {
    pub day: String,
    pub made: MadeWish,
    pub run: String,
    pub record: Record,
    /// How the run ended, in the record's fixed words.
    pub why: String,
}

impl Found {
    /// What the record names, as core writes it, or the record is not core's (`Record::named`): asked before
    /// git is asked anything, and before anything is printed of a run whose work is not asked for at all.
    pub(super) fn named(&self) -> Result<Option<copy::Named<'_>>, Error> {
        self.record.named(&self.run).map_err(|()| not_cores(&self.run))
    }
}

pub(super) fn not_cores(run: &str) -> Error {
    refused(format!("the record of run {run} is not one Jellymoon wrote for this wish"))
}

/// Finds the wish's run. Refused: a wish that is not there, one with no run or another wish's, a run that is
/// going or did not complete, and a record that is not core's.
pub(super) fn found(home: &Path, options: &RunOptions, asked: &Asked) -> Result<Found, Error> {
    let (to, done) = (asked.to, asked.done);
    let day = wish::day_of(options.day.as_deref())?;
    let id = options.wish.as_str();
    let made = wish::kept_of(home, &day)?.into_iter().find(|made| made.id == id).ok_or_else(|| no_wish(&day))?;
    // A run a kept wish names is a run id, or the day's file is not used at all (`wish.rs`).
    let Some(run) = made.run.clone() else {
        return Err(refused(format!("wish {id} has had no run, so there is no work to {to}")));
    };
    let no = |why: &str| refused(format!("run {run} {why}"));
    let Some(record) = record::kept(home, &run) else {
        return Err(refused(format!("the record of run {run} cannot be read, so its work is not {done}")));
    };
    if of_another(home, &run, Some(&record), &day, id) {
        return Err(refused(format!("wish {id} names run {run}, which is another wish's run: its work is not {done}")));
    }
    // The one line of a run that was cut off: before a look has written it down, and after.
    let cut_off = format!("was cut off before it ended: there is no work of it to {to}");
    if record.outcome.is_none() {
        return Err(no(if held(home, &run) { asked.going } else { &cut_off }));
    }
    let (outcome, why) = recorded(home, &day, id, &run, &record).ok_or_else(|| not_cores(&run))?;
    match outcome {
        RunOutcome::Completed => Ok(Found { day, made, run, record, why }),
        RunOutcome::Cancelled => Err(no(&format!("was cancelled by a stop: there is no work of it to {to}"))),
        _ if why == cut(&run) || why == cut_late(&run) => Err(no(&cut_off)),
        _ => Err(no(&format!("did not finish: there is no work of it to {to}"))),
    }
}

/// Reads the work of the wish's run. git is asked only for a run that finished with a commit, whose work waits.
/// With `only`, a file's number, that file alone is read past its size.
fn work(home: &Path, options: &RunOptions, only: Option<usize>) -> Result<Work, Error> {
    let found = found(home, options, &SHOWN)?;
    let named = found.named()?;
    let run = found.run.clone();
    let state = found.record.work.unwrap_or(WorkState::Waits);
    if state != WorkState::Waits || found.why != finished(&run, true) {
        return Ok(Work { run, state, files: Vec::new(), more: 0 });
    }
    // The repo is found as a run finds it, whatever the record names: the record's folder is only compared.
    let folder = place::folder(home, &found.day, &found.made, options)?;
    let read = copy::work_of(home, &folder, &run, named.as_ref(), only);
    let (files, more) = read.map_err(|unread| refused(unread_words(&run, unread, &SHOWN)))?;
    Ok(Work { run, state, files, more })
}

/// Why a run's work could not be read from its repo, for a look or for a keep.
pub(super) fn unread_words(run: &str, unread: Unread, asked: &Asked) -> String {
    let (branch, to, done) = (format!("the branch of run {run}"), asked.to, asked.done);
    match unread {
        Unread::Home => format!(
            "the work of run {run} could not be read: no `git` program was found on PATH, or the folder Jellymoon \
             keeps empty for git (runs/no-hooks) is not an empty folder"
        ),
        Unread::Repo => format!(
            "the work of run {run} could not be read from the repo: the folder is not the top of a checkout git \
             opens, or git did not answer"
        ),
        Unread::Missing => format!("{branch} is not in the repo, so there is no work of it to {to}"),
        Unread::Symbolic => format!("{branch} is a name for another branch by now, so its work is not {done}"),
        Unread::Moved => format!("{branch} is not as the run left it, so what is on it is not {done} as the run's"),
        Unread::Another => {
            format!("the repo found for run {run} is not the one its record names, so its work is not {done}")
        }
        Unread::Start => format!(
            "the work of run {run} does not start where its record says the run started, so it is not {done} as the run's"
        ),
        Unread::Many => format!("the work of run {run} is more files than Jellymoon lists"),
    }
}

/// The files of a finished run's work. Nothing of it is written anywhere.
pub fn show_run(home: &Path, options: &RunOptions) -> Result<Shown, Error> {
    let work = work(home, options, None)?;
    let files = work.files.iter().enumerate().map(|(at, file)| entry(at + 1, file)).collect();
    Ok(Shown { run: work.run, wish: options.wish.clone(), work: work.state, files, more: work.more, most: LISTED })
}

/// File `n` of that list as its text. Refused in fixed words, with no byte of the file: a number that names no
/// file, a file the work removed, a link, another repo's entry, what is not text, a text that cleaning goes on
/// changing, and a text cut where reading stopped that has no whole word before the cut.
pub fn show_file(home: &Path, options: &RunOptions, n: usize) -> Result<ShownFile, Error> {
    let work = work(home, options, Some(n))?;
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
    let long = file.size > TEXT_MAX as u64;
    // The word the cut fell in can be the head of a key whose rest was never read: it is not shown.
    let text = if long { text.rfind(char::is_whitespace).map_or("", |at| &text[..at]) } else { text };
    let (text, shortened) = still_text(text, TEXT_MAX).map_err(|_| no("cannot be cleaned, so it is not shown"))?;
    if long && text.is_empty() {
        return Err(no("has no whole word in what is read of it, so it is not shown"));
    }
    let (run, wish, work, cut) = (work.run.clone(), options.wish.clone(), work.state, long || shortened);
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
