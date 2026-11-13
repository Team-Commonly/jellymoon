//! Whether git would pass a file of the work through a filter when it writes it (`fits.rs`;
//! `docs/development/keep.md`, "A file with a filter"). Every filter is switched off (`git.rs`), so such a file
//! would be written as the store has it and not as the person's filter would make it: the work is refused.
//!
//! git reads attributes differently when it writes a checkout than when it is asked about one: for each folder
//! it takes `.gitattributes` from the commit it is checking out, and the file on the disk only where the commit
//! has none. So an unsaved edit to a tracked `.gitattributes` is not read, and an untracked one is. No one
//! question of `git check-attr` reads that way, so four are asked, and any `filter` in any answer refuses:
//!
//! 1. as the step will read them: the commit's own, in an index of Jellymoon's, over a folder of Jellymoon's
//!    that holds a copy of each `.gitattributes` of the checkout the commit has none for;
//! 2. by the commit's own alone (what `--source` asks on git 2.40 or later, asked here of that index);
//! 3. by the checkout's files as they are now;
//! 4. by the checkout's index as it is now, which holds what the person has staged.
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
/// checkout, and `scratch` a folder of the home's for the keep's own index (`keep.index`) and its own folder of
/// attribute files (`keep.attrs`): both are removed again, with what a keep that was cut off left of them.
pub(super) fn unfiltered(
    mine: &Repo,
    (folder, scratch): (&Path, &Path),
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
    let said = asked(mine, (folder, &index, &beside), target, &written);
    clear();
    if said? { Ok(()) } else { Err(Unkept::Filter) }
}

/// Whether all four readings say that no file written has a filter.
fn asked(mine: &Repo, places: (&Path, &Path, &Path), target: &str, written: &[&[u8]]) -> Result<bool, Unkept> {
    let (folder, index, beside) = places;
    let own = [("GIT_INDEX_FILE", index.to_str().ok_or(Unread::Home)?)];
    let (own, theirs): (&Vars, &Vars) = (&own, &[]);
    let read = mine.answer(None, own, &["read-tree", target], &[], SAID_MAX);
    if !matches!(read, Some((Some(0), _))) {
        return Err(Unread::Repo.into());
    }
    copied(mine, (folder, beside), own, written)?;
    let names: Vec<u8> = written.iter().flat_map(|path| [*path, &b"\0"[..]].concat()).collect();
    let most = names.len() + written.len() * 64 + 1;
    let (by_files, by_index) =
        (["check-attr", "-z", "--stdin", "filter"], ["check-attr", "--cached", "-z", "--stdin", "filter"]);
    let readings: [(Option<&Path>, &Vars, &[&str]); 4] = [
        (Some(beside), own, &by_files),
        (None, own, &by_index),
        (Some(folder), theirs, &by_files),
        (None, theirs, &by_index),
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

/// Every folder above a path, each with its `/`.
fn above(path: &[u8]) -> impl Iterator<Item = &[u8]> {
    (0..path.len()).filter(move |at| path[*at] == b'/').map(move |at| &path[..=at])
}

/// Makes the folder the first reading is asked over: for each folder above a file the work writes, the
/// checkout's `.gitattributes` there when it is a plain file and the commit has none at that path. A link is
/// not followed and a file longer than git reads is left out, as git does (2.32 and later for the link).
fn copied(mine: &Repo, places: (&Path, &Path), own: &[(&str, &str)], written: &[&[u8]]) -> Result<(), Unkept> {
    let (folder, beside) = places;
    let tracked = ["ls-files", "-z", "--", ":(glob)**/.gitattributes"];
    let tracked = match mine.answer(Some(folder), own, &tracked, &[], LIST_MAX) {
        Some((Some(0), tracked)) if tracked.len() < LIST_MAX => tracked,
        Some((Some(0), _)) => return Err(Unread::Many.into()),
        _ => return Err(Unread::Repo.into()),
    };
    let tracked: BTreeSet<&[u8]> = tracked.split(|byte| *byte == 0).collect();
    fs::create_dir_all(beside).map_err(|_| Unread::Home)?;
    let folders: BTreeSet<&[u8]> = written.iter().flat_map(|path| above(path)).chain([&b""[..]]).collect();
    for inside in folders {
        let name = [inside, b".gitattributes"].concat();
        let (Some(from), Some(to)) = (on_disk(folder, &name), on_disk(beside, &name)) else { continue };
        let plain = |meta: fs::Metadata| meta.is_file() && meta.len() <= ATTRIBUTES_MAX;
        if tracked.contains(&name[..]) || !fs::symlink_metadata(&from).is_ok_and(plain) {
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
