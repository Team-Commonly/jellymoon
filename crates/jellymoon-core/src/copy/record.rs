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
    /// Since when the taking back of this left copy is put off, in nanoseconds: the first look that could not ask
    /// its repo, or could not finish (`back.rs`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub put_off: Option<u64>,
    /// The commit the copy was made at, as git named it before it made the copy. A record of an older build
    /// has none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base: Option<String>,
}

/// What a look learns of a left copy from the repo its record names.
#[derive(PartialEq, Eq)]
pub(super) enum Listed {
    /// The repo lists the copy `root/<run>` as a worktree of its own.
    Ours,
    /// The repo answers and does not list it: nothing is done in that repo.
    Not,
    /// The repo could not be asked: it is not where the record says, or cannot be read.
    Unasked,
}

/// Asks the repo a record names whether it lists the copy `root/<run>` as a worktree of its own: the copy's git
/// folder is one the repo keeps for a worktree ([`kept_by`]), and its `gitdir` file, which git wrote when it made
/// the worktree, names the copy's `.git`. A record is a file anyone who can write the home can write, so one that
/// names another repo is not acted on in that repo. The file is read only when it is a plain file no longer than a
/// path. A path in it that does not start at a root is read from the copy's git folder, as git reads one it wrote
/// with `worktree.useRelativePaths`.
/// Where the folders are is asked of them as the system resolves them, links and `..` followed, and never of
/// their names: a name can lead out of the repo it starts in.
///
/// A repo that resolves and holds no such git folder, or no such file, has answered. One that does not resolve,
/// and anything on the way that cannot be read for another reason than not being there, has not been asked.
/// Neither has one a record names by a path that does not start at a root: this process would read it from the
/// folder it was started in, and git from another.
pub(super) fn asked(record: &Record, root: &Path, run: &str) -> Listed {
    if !(record.repo.is_absolute() && record.own.is_absolute()) {
        return Listed::Unasked;
    }
    let (Ok(repo), Ok(root)) = (fs::canonicalize(&record.repo), fs::canonicalize(root)) else {
        return Listed::Unasked;
    };
    let file = record.own.join("gitdir");
    let named = match (fs::canonicalize(&record.own), fs::symlink_metadata(&file)) {
        (Ok(own), _) if !kept_by(&own, &repo) => return Listed::Not,
        (Ok(_), Ok(meta)) if meta.is_file() && meta.len() <= 8192 => fs::read(&file),
        (Ok(_), Ok(_)) => return Listed::Not,
        (Err(e), _) | (_, Err(e)) => Err(e),
    };
    let named = match named.map(String::from_utf8) {
        Ok(Ok(named)) => record.own.join(named.trim_end_matches(['\r', '\n'])),
        Ok(Err(_)) => return Listed::Not,
        Err(e) if e.kind() == ErrorKind::NotFound => return Listed::Not,
        Err(_) => return Listed::Unasked,
    };
    let copy = named.parent().filter(|copy| copy.file_name().is_some_and(|last| last == run));
    let here = copy.and_then(Path::parent).and_then(|above| fs::canonicalize(above).ok()) == Some(root);
    if here { Listed::Ours } else { Listed::Not }
}

/// Whether a git folder is one a repo keeps for a worktree of its own: directly in the `worktrees` of the repo's
/// git folder, which is a repo's own. Anywhere inside is not enough: a worktree's git folder is inside itself,
/// and inside every repo whose git folder encloses its repo's, as a submodule's is. And a worktree's git folder
/// named as the repo is taken by git for one, with that worktree's `HEAD`: such a folder holds a `commondir`
/// file, and a repo's own holds none.
fn kept_by(own: &Path, repo: &Path) -> bool {
    let own_repo = fs::symlink_metadata(repo.join("commondir")).is_err_and(|e| e.kind() == ErrorKind::NotFound);
    own_repo && own.parent() == Some(&repo.join("worktrees"))
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
