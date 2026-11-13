//! A run's own record, `runs/<id>/run.json`: how the run ended, which is written here before the wish is told,
//! and the harness child's pid with what tells that process from a later one of the same pid (`dead.rs`). The
//! repo's folder is not in it: the copy's record names the repo (`copy/record.rs`).

use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::{RunOutcome, WorkState, dead};
use crate::hook::put;
use crate::tap::plain;
use crate::{Error, copy};

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
    text.and_then(|text| put(&file(home, run), &format!("{text}\n"), false)).map_err(Error::Store)
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
