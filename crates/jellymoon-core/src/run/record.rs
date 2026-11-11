//! A run's own record, `runs/<id>/run.json`: the one place the repo's folder is written down, and where the harness
//! child's pid is kept with what tells that process from a later one of the same pid (`dead.rs`).

use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::{RunOutcome, dead};
use crate::hook::put;
use crate::tap::plain;
use crate::{Error, copy};

/// The most bytes of a run's record that are read.
const RECORD_MAX: usize = 64 * 1024;

#[derive(Serialize, Deserialize)]
pub(super) struct Record {
    pub wish: String,
    pub day: String,
    pub repo: PathBuf,
    /// When it was started, in seconds since 1970.
    pub started: u64,
    /// None while it is going.
    pub outcome: Option<RunOutcome>,
    pub denied: usize,
    pub skipped: usize,
    /// The harness child. None before it is started.
    #[serde(default)]
    pub pid: Option<u32>,
    /// When that process started, as the system says it. None where the system cannot be asked: a child with no
    /// mark is never signalled by anyone but the `jellymoon run` that started it.
    #[serde(default)]
    pub mark: Option<String>,
}

fn file(home: &Path, run: &str) -> PathBuf {
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

/// Writes down the harness child that was just started, so that a later `jellymoon` can stop it if this one is cut
/// off. A record that cannot be written leaves the child unknown, and so never signalled.
pub(super) fn child(home: &Path, run: &str, pid: u32) {
    if let Some(mut record) = kept(home, run) {
        (record.pid, record.mark) = (Some(pid), dead::mark(pid));
        let _ = write(home, run, &record);
    }
}
