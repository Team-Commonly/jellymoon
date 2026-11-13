//! Whether git would pass a file of the work through a filter when it writes it (`fits.rs`;
//! `docs/development/keep.md`, "A file with a filter"). Every filter is switched off (`git.rs`), so such a file
//! would be written as the store has it and not as the person's filter would make it: the work is refused.
//!
//! git reads attributes differently when it writes a checkout than when it is asked about one. The step first
//! removes every file its index held and no longer holds, and then writes; for each folder it takes
//! `.gitattributes` from the index it is leaving, and the file on the disk only where that index has none. So an
//! unsaved edit to a tracked `.gitattributes` is not read, an untracked one is, one the work removes is gone
//! before a file is written, and one the work changes or adds is read as the work has it. No one question of
//! `git check-attr` reads that way, so four are asked, and any `filter` in any answer refuses. Three are asked of
//! the index the step would leave, which git's trial made (`left`):
//!
//! 1. as the step will read them: that index, over a folder of Jellymoon's that holds a copy of each
//!    `.gitattributes` of the checkout which that index has none for and which the step does not remove;
//! 2. by that index alone: the commit's own, and what the person has staged;
//! 3. by the checkout's files as they are now, and by that index where the checkout has no file;
//! 4. by the commit's own alone (what `--source` asks on git 2.40 or later), in an index of Jellymoon's.
//!
//! The repo's `info/attributes` and the config's `core.attributesFile` are read in all four.

use std::collections::BTreeSet;
use std::fs::{self, File};
use std::io::Read;
use std::path::Path;

use super::super::git::Repo;
use super::super::work::LIST_MAX;
use super::fits::{Touched, on_disk};
use super::{SAID_MAX, Unkept, Unread};

/// The variables one git is given beside its own.
type Vars<'a> = [(&'a str, &'a str)];

/// The most bytes of a `.gitattributes` that git reads: a longer one it leaves out, and so does a keep.
const ATTRIBUTES_MAX: u64 = 100 * 1024 * 1024;

/// Refuses when a file the work writes has `filter` set, by any of the four readings. `folder` is the
/// checkout, `left` the index the step would leave, and `scratch` a folder of the home's for the keep's own
/// index of the commit (`keep.index`) and its own folder of attribute files (`keep.attrs`): both are removed
/// again, with what a keep that was cut off left of them.
pub(super) fn unfiltered(
    mine: &Repo,
    (folder, scratch): (&Path, &Path),
    left: &Path,
    target: &str,
    files: &[Touched],
) -> Result<(), Unkept> {
    let written: Vec<&[u8]> = files.iter().filter(|file| !file.removed).map(|file| &file.path[..]).collect();
    if written.is_empty() {
        return Ok(());
    }
    let (index, beside) = (scratch.join("keep.index"), scratch.join("keep.attrs"));
    let clear = || {
        let _ = (fs::remove_file(&index), fs::remove_file(index.with_extension("index.lock")));
        let _ = fs::remove_dir_all(&beside);
    };
    clear();
    let said = asked(mine, (folder, &index, &beside), left, target, &written);
    clear();
    if said? { Ok(()) } else { Err(Unkept::Filter) }
}

/// Whether all four readings say that no file written has a filter.
fn asked(
    mine: &Repo,
    (folder, index, beside): (&Path, &Path, &Path),
    left: &Path,
    target: &str,
    written: &[&[u8]],
) -> Result<bool, Unkept> {
    let commit = [("GIT_INDEX_FILE", index.to_str().ok_or(Unread::Home)?)];
    let step = [("GIT_INDEX_FILE", left.to_str().ok_or(Unread::Home)?)];
    let (commit, step): (&Vars, &Vars) = (&commit, &step);
    let read = mine.answer(None, commit, &["read-tree", target], &[], SAID_MAX);
    if !matches!(read, Some((Some(0), _))) {
        return Err(Unread::Repo.into());
    }
    // What the step's index holds git reads from there. What the person's index holds and the step's does not,
    // the step removes from the disk before it writes a file.
    let (theirs, stays) = (tracked(mine, folder, &[])?, tracked(mine, folder, step)?);
    copied((folder, beside), written, |name| theirs.contains(name) || stays.contains(name))?;
    let names: Vec<u8> = written.iter().flat_map(|path| [*path, &b"\0"[..]].concat()).collect();
    let most = names.len() + written.len() * 64 + 1;
    let (by_files, by_index) =
        (["check-attr", "-z", "--stdin", "filter"], ["check-attr", "--cached", "-z", "--stdin", "filter"]);
    let readings: [(Option<&Path>, &Vars, &[&str]); 4] = [
        (Some(beside), step, &by_files),
        (None, step, &by_index),
        (Some(folder), step, &by_files),
        (None, commit, &by_index),
    ];
    for (tree, vars, words) in readings {
        let said = mine.answer(tree, vars, words, &names, most).filter(|said| said.1.len() < most);
        let Some((Some(0), said)) = said else { return Err(Unread::Repo.into()) };
        if !unset(&said) {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Every `.gitattributes` an index holds: the checkout's own, or with `vars` the one they name.
fn tracked(mine: &Repo, folder: &Path, vars: &Vars) -> Result<BTreeSet<Vec<u8>>, Unkept> {
    let tracked = ["ls-files", "-z", "--", ":(glob)**/.gitattributes"];
    match mine.answer(Some(folder), vars, &tracked, &[], LIST_MAX) {
        Some((Some(0), tracked)) if tracked.len() < LIST_MAX => {
            Ok(tracked.split(|byte| *byte == 0).map(<[u8]>::to_vec).collect())
        }
        Some((Some(0), _)) => Err(Unread::Many.into()),
        _ => Err(Unread::Repo.into()),
    }
}

/// Every folder above a path, each with its `/`.
fn above(path: &[u8]) -> impl Iterator<Item = &[u8]> {
    (0..path.len()).filter(move |at| path[*at] == b'/').map(move |at| &path[..=at])
}

/// Makes the folder the first reading is asked over: for each folder above a file the work writes, and for the
/// top, the checkout's `.gitattributes` there when it is a plain file and not one to `skip`. A link is not followed and
/// a file longer than git reads is left out, as git does (2.32 and later for the link).
fn copied((folder, beside): (&Path, &Path), written: &[&[u8]], skip: impl Fn(&[u8]) -> bool) -> Result<(), Unkept> {
    fs::create_dir_all(beside).map_err(|_| Unread::Home)?;
    let folders: BTreeSet<&[u8]> = written.iter().flat_map(|path| above(path)).chain([&b""[..]]).collect();
    for inside in folders {
        let name = [inside, b".gitattributes"].concat();
        let (Some(from), Some(to)) = (on_disk(folder, &name), on_disk(beside, &name)) else { continue };
        let plain = |meta: fs::Metadata| meta.is_file() && meta.len() <= ATTRIBUTES_MAX;
        if skip(&name) || !fs::symlink_metadata(&from).is_ok_and(plain) {
            continue;
        }
        to.parent().map_or(Ok(()), fs::create_dir_all).map_err(|_| Unread::Home)?;
        // A file of the person's that cannot be read is one git leaves out too. Only what it holds is taken:
        // a copy with the file's own mode could be one the keep cannot remove again.
        let mut holds = Vec::new();
        if File::open(&from).and_then(|file| file.take(ATTRIBUTES_MAX).read_to_end(&mut holds)).is_ok() {
            fs::write(&to, holds).map_err(|_| Unread::Home)?;
        }
    }
    Ok(())
}

/// Whether `check-attr -z` says of every path that no filter is set: a path, the attribute's name and its value,
/// each ended by a zero byte.
fn unset(said: &[u8]) -> bool {
    let parts: Vec<&[u8]> = said.split(|byte| *byte == 0).collect();
    let (answers, rest) = parts.split_at(parts.len() - 1);
    let unset = |answer: &[&[u8]]| answer.len() == 3 && (answer[2] == b"unspecified" || answer[2] == b"unset");
    rest.iter().all(|rest| rest.is_empty()) && answers.chunks(3).all(unset)
}

#[cfg(test)]
mod tests {
    use super::unset;

    #[test]
    fn a_filter_is_set_unless_git_says_unspecified_or_unset_of_every_path() {
        assert!(unset(b"a.txt\0filter\0unspecified\0b\0filter\0unset\0"));
        assert!(unset(b""));
        for set in [
            &b"a\0filter\0lfs\0"[..],
            b"a\0filter\0set\0",
            b"a\0filter\0unspecified\0b\0filter\0x\0",
            b"a\0filter\0",
            b"a\0filter\0unset",
        ] {
            assert!(!unset(set), "{set:?}");
        }
    }
}
