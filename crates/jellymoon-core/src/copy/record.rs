//! A copy's record, `runs/<id>/copy.json`: what core knows of a copy, written when it is made and outside it.

use std::fs::{self, File, OpenOptions};
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::refused;
use crate::Error;
use crate::hook::put;

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

/// Whether the repo a record names lists the copy `root/<run>` as a worktree of its own: the copy's git folder is
/// inside the repo's, and its `gitdir` file, which git wrote when it made the worktree, names the copy's `.git`.
/// A record is a file anyone who can write the home can write, so one that names another repo is not acted on
/// in that repo. The file is read only when it is a plain file no longer than a path.
pub(super) fn ours(record: &Record, root: &Path, run: &str) -> bool {
    let file = record.own.join("gitdir");
    let plain = fs::symlink_metadata(&file).is_ok_and(|meta| meta.is_file() && meta.len() <= 8192);
    let named = if plain { fs::read_to_string(&file).unwrap_or_default() } else { String::new() };
    let copy = Path::new(named.trim_end_matches(['\r', '\n'])).parent();
    let same = |one: &Path| fs::canonicalize(one).is_ok_and(|one| fs::canonicalize(root).is_ok_and(|root| one == root));
    let here = |copy: &Path| copy.file_name().is_some_and(|last| last == run) && copy.parent().is_some_and(same);
    record.own.starts_with(&record.repo) && copy.is_some_and(here)
}

/// Whether something that is not a plain file is at the name. Such a thing is never opened: a named pipe would
/// keep whoever opens it waiting for ever.
fn not_plain(file: &Path) -> bool {
    fs::symlink_metadata(file).is_ok_and(|meta| !meta.is_file())
}

/// Opens the file a clean holds the system's lock on, making it first when `make` says so.
/// For writing, never to append only: Windows gives no lock on a file opened that way ("Access is denied").
pub(super) fn lock_of(file: &Path, make: bool) -> io::Result<File> {
    if not_plain(file) {
        return Err(io::Error::new(ErrorKind::InvalidData, "the lock of the copies is not a plain file"));
    }
    OpenOptions::new().write(true).create(make).truncate(false).open(file)
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

/// Writes the record whole, as a run's record is written: into a file made new beside its place, then moved
/// there. Nothing that is at a name already is opened, so a named pipe or a link left there is not written through.
pub(super) fn write(file: &Path, record: &Record) -> Result<(), Error> {
    let text = serde_json::to_string(record).map_err(|_| refused("the repo's path is not text"))?;
    put(file, &(text + "\n"), false).map_err(Error::Store)
}

/// The copy is gone, so its record goes, and the run's folder in `runs` when nothing else is kept there.
pub(super) fn forget(file: &Path) {
    let _ = fs::remove_file(file);
    let _ = file.parent().map(fs::remove_dir);
}
