//! A run's own record, `runs/<id>/run.json`: how the run ended, which is written here before the wish is told,
//! and the harness child's pid with what tells that process from a later one of the same pid (`dead.rs`). It
//! names the run's repo, the commit the run started from and the commit it ended as (F48), since the copy's record,
//! which names the repo while the copy is kept (`copy/record.rs`), goes with a finished run's copy. A record is a
//! file anyone who can write the home can write: what it names is checked and compared, never trusted (`show.rs`).

use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::{RunOutcome, WorkState, dead, finished};
use crate::copy::{self, Named};
use crate::hook::put;
use crate::tap::plain;
use crate::{Error, busy};

/// The most bytes of a run's record that are read.
const RECORD_MAX: usize = 64 * 1024;

#[derive(Serialize, Deserialize)]
pub(super) struct Record {
    pub wish: String,
    pub day: String,
    /// When it was started, in seconds since 1970.
    pub started: u64,
    /// None while it is going.
    pub outcome: Option<RunOutcome>,
    /// How it ended, in the fixed words the wish is then given. None while it is going.
    #[serde(default)]
    pub why: Option<String>,
    pub denied: usize,
    pub skipped: usize,
    /// The harness child. None before it is started.
    #[serde(default)]
    pub pid: Option<u32>,
    /// When that process started, as the system says it. None where the system cannot be asked: a child with no
    /// mark is never signalled by anyone but the `jellymoon run` that started it.
    #[serde(default)]
    pub mark: Option<String>,
    /// Whether the harness's stream began as a run's does: the harness knows a run's command line, and works.
    #[serde(default)]
    pub began: bool,
    /// The git folder the checkouts of the run's repo share, as the system resolved it when the copy was made. It
    /// is the one place a finished run's repo is kept, it is printed and sent nowhere, and it never chooses which
    /// repo is read: `run show` finds the repo as a run does and compares. None in a record from before F48.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repo: Option<PathBuf>,
    /// The commit the copy was made from. None in a record from before F48.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start: Option<String>,
    /// The commit the copy ended as, written with the end of a run that finished: the one commit core made of its
    /// work on top of `start`, or `start` itself when it changed nothing. None for every run that did not finish,
    /// one whose branch was moved from `start` while it worked among them (`copy/clean.rs`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end: Option<String>,
    /// Whether the person kept or tossed the work of a run that finished. Left out while it waits: no run writes
    /// it, and `run show` reads it (`show.rs`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub work: Option<WorkState>,
}

pub(super) fn file(home: &Path, run: &str) -> PathBuf {
    copy::kept_at(home, run).join("run.json")
}

pub(super) fn write(home: &Path, run: &str, record: &Record) -> Result<(), Error> {
    let text = serde_json::to_string(record).map_err(io::Error::from);
    let written = text.and_then(|text| put(&file(home, run), &format!("{text}\n"), false)).map_err(Error::Store);
    // A test's build is held here, after each writing, until the test has read the state the record has.
    busy::step("JELLYMOON_TEST_RECORD_WAIT_MS");
    written
}

impl Record {
    /// What the record of a run that finished names of it: its repo, its start and its end, all three, or none of
    /// them in a record from before F48. It is the one place those words are read, by `run show` and by whatever
    /// comes to act on a run's work, and it is asked of every finished run, whatever the state of its work,
    /// before git is asked anything and before anything is printed. Err for a record that is not as a run writes
    /// it: one that names some and not others, a repo by a path that does not start at a root (a run writes the
    /// folder as the system resolves it, from a root), a commit by anything but a whole object id, or a start and
    /// an end that do not fit the record's own words: a run that made its one commit ended as another commit than
    /// it started from, and a run that changed nothing ended as its start. None of these words reaches git, and
    /// neither does one that passes: an id is only ever compared with what git printed (`copy/work.rs`).
    pub(super) fn named(&self, run: &str) -> Result<Option<Named<'_>>, ()> {
        let changed = self.why.as_deref() == Some(finished(run, true).as_str());
        match (self.repo.as_deref(), self.start.as_deref(), self.end.as_deref()) {
            (None, None, None) => Ok(None),
            (Some(repo), Some(start), Some(end))
                if repo.is_absolute() && copy::is_id(start) && copy::is_id(end) && (start != end) == changed =>
            {
                Ok(Some(Named { repo, start, end }))
            }
            _ => Err(()),
        }
    }
}

/// The record as it is kept. None when there is none that can be used.
pub(super) fn kept(home: &Path, run: &str) -> Option<Record> {
    let bytes = plain(&file(home, run), RECORD_MAX).ok()?;
    serde_json::from_slice(&bytes).ok()
}

/// Writes down that the run was never started: an end of its own kind, which no look takes for a run cut off.
/// It names no child: a harness that was started to find it too old is gone by now. Only so is the copy removed
/// on the record's word (`end::bury`).
pub(super) fn unstarted(home: &Path, run: &str) {
    if let Some(mut record) = kept(home, run) {
        (record.outcome, record.pid, record.mark) = (Some(RunOutcome::NotStarted), None, None);
        let _ = write(home, run, &record);
    }
}

/// Writes down that the harness's stream began as a run's does, for whoever asked for the run apart and waits to
/// say that it is going (`apart.rs`).
pub(super) fn began(home: &Path, run: &str) {
    if let Some(mut record) = kept(home, run) {
        record.began = true;
        let _ = write(home, run, &record);
    }
}

/// Writes down the harness child that was just started, so that a later `jellymoon` can stop it if this one is cut
/// off. A record that cannot be written leaves the child unknown, and so never signalled.
pub(super) fn child(home: &Path, run: &str, pid: u32) {
    if let Some(mut record) = kept(home, run) {
        (record.pid, record.mark) = (Some(pid), dead::mark(pid));
        let _ = write(home, run, &record);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXAMPLE: &str = include_str!("../../tests/fixtures/run-record.json");
    const BEFORE: &str = include_str!("../../tests/fixtures/run-record-before.json");

    // The contract is the type and its example: one changed without the other fails here.
    #[test]
    fn a_record_round_trips_byte_for_byte_and_one_from_before_is_read_and_written_as_it_was() {
        let written = |record: &Record| serde_json::to_string(record).map(|line| format!("{line}\n")).unwrap();
        let record: Record = serde_json::from_str(EXAMPLE).unwrap();
        assert_eq!(written(&record), EXAMPLE);
        let named = (record.repo.as_deref(), record.start.as_deref(), record.end.as_deref());
        let (start, end) = ("0123456789abcdef0123456789abcdef01234567", "89abcdef0123456789abcdef0123456789abcdef");
        assert_eq!(named, (Some(Path::new("/work/alpha/.git")), Some(start), Some(end)));
        // A record from before F48 names none of the three, and written again it gains no field.
        let before: Record = serde_json::from_str(BEFORE).unwrap();
        assert!(before.repo.is_none() && before.start.is_none() && before.end.is_none());
        assert_eq!(written(&before), BEFORE);
        assert_eq!(EXAMPLE.split(",\"repo\":").next(), BEFORE.strip_suffix("}\n"));
        // What is named is text, or the record is not read: nothing is made of a number or a list.
        for odd in ["\"repo\":7", "\"start\":[\"main\"]", "\"end\":{}"] {
            let changed = BEFORE.replace("\"began\":true", &format!("\"began\":true,{odd}"));
            assert!(serde_json::from_str::<Record>(&changed).is_err(), "{odd}");
        }
    }
}
