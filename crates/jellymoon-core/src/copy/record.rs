//! A copy's record, `runs/<id>/copy.json`: what core knows of a copy, written when it is made and outside it.

use std::fs::{self, File, OpenOptions};
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::refused;
use crate::Error;

/// The git folder the repo's checkouts share, the copy's own git folder inside it, when the run failed, in
/// nanoseconds, if it did, and whether the repo's folder was read from a session log: then nothing git says of it
/// is repeated when the copy is ended either (`git.rs`).
#[derive(Serialize, Deserialize)]
pub(super) struct Record {
    pub repo: PathBuf,
    pub own: PathBuf,
    pub failed: Option<u64>,
    #[serde(default)]
    pub found: bool,
}

/// Whether something that is not a plain file is at the name. Such a thing is never opened: a named pipe would
/// keep whoever opens it waiting for ever.
fn not_plain(file: &Path) -> bool {
    fs::symlink_metadata(file).is_ok_and(|meta| !meta.is_file())
}

/// Opens the file a clean holds the system's lock on, making it first when `make` says so.
pub(super) fn lock_of(file: &Path, make: bool) -> io::Result<File> {
    if not_plain(file) {
        return Err(io::Error::new(ErrorKind::InvalidData, "the lock of the copies is not a plain file"));
    }
    OpenOptions::new().append(true).create(make).open(file)
}

pub(super) fn read(file: &Path) -> Result<Option<Record>, Error> {
    if not_plain(file) {
        return Err(refused("the record of that run's copy cannot be read; nothing was done"));
    }
    match fs::read(file) {
        Ok(text) => {
            let record = serde_json::from_slice(&text);
            record.map(Some).map_err(|_| refused("the record of that run's copy cannot be read; nothing was done"))
        }
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
        Err(e) => Err(Error::Store(e)),
    }
}

/// Writes the record whole: beside its place first, then moved there.
pub(super) fn write(file: &Path, record: &Record) -> Result<(), Error> {
    let text = serde_json::to_string(record).map_err(|_| refused("the repo's path is not text"))?;
    let new = file.with_extension("new");
    fs::write(&new, text + "\n").and_then(|()| fs::rename(&new, file)).map_err(Error::Store)
}

/// The copy is gone, so its record goes, and the run's folder in `runs` when nothing else is kept there.
pub(super) fn forget(file: &Path) {
    let _ = fs::remove_file(file);
    let _ = file.parent().map(fs::remove_dir);
}
