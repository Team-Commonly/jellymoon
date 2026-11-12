//! The answers of trials that are kept, so that a picture is tried once and not at every start (`trial.rs`).
//!
//! An answer accepts a picture with no child started, so the folder is the person's alone: it is a folder and no
//! link to one, on Unix it is made and held for its owner only, and an answer in it is a plain file. Where any
//! of that does not hold, nothing is read or kept there and every picture is tried. Who can write in the folder
//! all the same can write the theme too; a forged answer is what `docs/adr/0016` says it is.

use std::fs;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use super::bounds::MAX_ART_BYTES;
use super::trial::{Answer, Said, read_said, write_said};

/// The most kept answers in a folder. One more empties it first, so the folder is at most 64 pictures' size.
const MOST_KEPT: usize = 64;

/// The ending of a kept answer's file name.
const KEPT: &str = "trial";

/// The most a kept answer is longer than its picture: a key and an answer's line.
const MOST_OVER: u64 = 2048;

/// The folder, when it is one answers may be read from and kept in. It is made when it is not there.
pub(super) fn ours(folder: &Path) -> Option<&Path> {
    if fs::symlink_metadata(folder).is_err() {
        let _ = made(folder);
    }
    // Asked of the name itself: a link to a folder is not one.
    let found = fs::symlink_metadata(folder).ok().filter(fs::Metadata::is_dir)?;
    alone(folder, &found).then_some(folder)
}

#[cfg(unix)]
fn made(folder: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::DirBuilderExt;
    fs::DirBuilder::new().recursive(true).mode(0o700).create(folder)
}

#[cfg(not(unix))]
fn made(folder: &Path) -> std::io::Result<()> {
    fs::create_dir_all(folder)
}

/// Whether nobody but its owner can read or write in the folder. One that is open to others is closed to them
/// first, which only its owner can do.
#[cfg(unix)]
fn alone(folder: &Path, found: &fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;
    found.permissions().mode() & 0o777 == 0o700
        || fs::set_permissions(folder, fs::Permissions::from_mode(0o700)).is_ok()
}

/// Windows gives a folder under a person's own folder to that person, by what it inherits.
#[cfg(not(unix))]
fn alone(_: &Path, _: &fs::Metadata) -> bool {
    true
}

fn file(folder: &Path, key: &str, svg: &str) -> PathBuf {
    let mut hasher = DefaultHasher::new();
    (key, svg).hash(&mut hasher);
    folder.join(format!("{:016x}.{KEPT}", hasher.finish()))
}

/// The answer kept for this picture under this key. The file holds the whole picture it answers for, and is the
/// answer only when that is this picture to the byte: a file that is cut short, kept under another key or about
/// another picture is no answer, and the picture is tried. Nor is a link.
pub(super) fn read(folder: &Path, key: &str, svg: &str) -> Option<Answer> {
    let file = file(folder, key, svg);
    fs::symlink_metadata(&file).ok().filter(fs::Metadata::is_file)?;
    let mut text = String::new();
    fs::File::open(file).ok()?.take(MAX_ART_BYTES + MOST_OVER).read_to_string(&mut text).ok()?;
    let (answer, picture) = text.strip_prefix(key)?.strip_prefix('\n')?.split_once('\n')?;
    match read_said(answer).filter(|_| picture == svg)? {
        Said::Is(answer) => Some(answer),
        Said::Unsure(_) => None,
    }
}

/// Keeps an answer. Whatever fails here, the answer is only not kept.
pub(super) fn write(folder: &Path, key: &str, svg: &str, answer: &Answer) {
    let ours = |name: &PathBuf| name.extension().is_some_and(|ending| ending == KEPT);
    let kept: Vec<PathBuf> = fs::read_dir(folder)
        .map(|entries| entries.flatten().map(|entry| entry.path()).filter(ours).collect())
        .unwrap_or_default();
    if kept.len() >= MOST_KEPT {
        kept.iter().for_each(|old| drop(fs::remove_file(old)));
    }
    // Written beside and renamed, so no reader finds half of one. A rename puts the file in a link's place and
    // does not follow it.
    let file = file(folder, key, svg);
    let beside = file.with_extension(format!("{}.new", std::process::id()));
    let _ = fs::remove_file(&beside);
    let text = format!("{key}\n{}\n{svg}", write_said(&Said::Is(answer.clone())));
    let written = new_file(&beside).and_then(|mut new| new.write_all(text.as_bytes()));
    if written.and_then(|()| fs::rename(&beside, &file)).is_err() {
        let _ = fs::remove_file(&beside);
    }
}

/// A file that was not there, for its owner alone.
fn new_file(path: &Path) -> std::io::Result<fs::File> {
    let mut new = fs::OpenOptions::new();
    new.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        new.mode(0o600);
    }
    new.open(path)
}
