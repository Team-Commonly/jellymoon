//! Whether what a keep would land fits the person's checkout, asked before anything is written (`keep.rs`). The
//! step that lands it is asked again by git itself; these checks are how a refusal can say why, and how `--ask`
//! can answer at all.

use std::fs;
use std::path::{Component, Path, PathBuf};

use super::super::git::Repo;
use super::super::work::LIST_MAX;
use super::{SAID_MAX, Unkept, Unread};

/// One file a commit touches, from git's list of the change.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Touched {
    /// The path as git keeps it: any bytes but a zero.
    pub path: Vec<u8>,
    /// Another repo's entry, before the change or after it.
    pub repo: bool,
    made: bool,
    removed: bool,
}

#[cfg(test)]
impl Touched {
    pub(crate) fn of(path: &[u8], repo: bool) -> Touched {
        Touched { path: path.to_vec(), repo, made: false, removed: false }
    }
}

/// Every file that differs between two commits or trees, with no rename ever found (`work.rs`).
pub(super) fn touched(repo: &Repo, from: &str, to: &str) -> Result<Vec<Touched>, Unkept> {
    let list = ["diff-tree", "-r", "--no-abbrev", "-z", from, to];
    let list = repo.bytes(&list, &[], LIST_MAX).ok_or(Unread::Repo)?;
    if list.len() >= LIST_MAX {
        return Err(Unread::Many.into());
    }
    Ok(listed(&list).ok_or(Unread::Repo)?)
}

/// The files of `diff-tree -r -z`: a line of two modes, two ids and a letter, then the path, each ended by a zero
/// byte. None when the list is not in that form.
fn listed(list: &[u8]) -> Option<Vec<Touched>> {
    let (mut files, mut parts) = (Vec::new(), list.split(|byte| *byte == 0));
    while let Some(line) = parts.next().filter(|line| !line.is_empty()) {
        let words: Vec<&str> = std::str::from_utf8(line).ok()?.strip_prefix(':')?.split(' ').collect();
        let [was, now, _, _, letter] = words[..] else { return None };
        if !matches!(letter, "A" | "M" | "T" | "D") {
            return None;
        }
        let path = parts.next().filter(|path| !path.is_empty())?.to_vec();
        let repo = was == "160000" || now == "160000";
        files.push(Touched { path, repo, made: letter == "A", removed: letter == "D" });
    }
    Some(files)
}

/// Whether git's own line of its version names 2.38 or later, the first that merges in the store.
pub(super) fn new_enough(said: &str) -> bool {
    let mut numbers = said.strip_prefix("git version ").unwrap_or("").split(['.', ' ']).map(str::parse::<u32>);
    match (numbers.next(), numbers.next()) {
        (Some(Ok(major)), Some(Ok(minor))) => (major, minor) >= (2, 38),
        _ => false,
    }
}

/// Whether the index of a checkout is locked: git is writing it, or a git that was cut off left the file.
pub(super) fn locked(own: &Path) -> bool {
    fs::symlink_metadata(own.join("index.lock")).is_ok()
}

/// Whether `target` can be landed on `tip` in the person's checkout: `places` are the checkout, its git folder
/// and a file of the home's for an index of the keep's own; `files` are what differs between the two.
///
/// - The index is not locked.
/// - No file that would be written has a `filter` set, by the attributes the checkout would have then or has
///   now. Every filter is switched off (`git.rs`), so such a file would be written as the store has it and not
///   as the person's filter would make it: it is refused, and no filter is ever started.
/// - git's own trial of the move (`read-tree -n`) passes: no unsaved change and no untracked file in the way.
/// - Nothing is on the disk where a file the work makes would go ([`taken`]). The trial takes an ignored file as
///   nobody's; here it is the person's. A keep that goes on after one that was cut off (`again`) skips this,
///   since the files there may be the ones that keep wrote, and leaves it to the step itself, which stops for an
///   ignored file too.
pub(super) fn fits(
    mine: &Repo,
    places: (&Path, &Path, &Path),
    (tip, target): (&str, &str),
    files: &[Touched],
    again: bool,
) -> Result<(), Unkept> {
    let (folder, own, index) = places;
    if locked(own) {
        return Err(Unkept::Locked);
    }
    let written = || files.iter().filter(|file| !file.removed);
    let names: Vec<u8> = written().flat_map(|file| [&file.path[..], b"\0"].concat()).collect();
    if !names.is_empty() {
        let own_index = [("GIT_INDEX_FILE", index.to_str().ok_or(Unread::Home)?)];
        let asked = ["check-attr", "-z", "--stdin", "filter"];
        let most = names.len() + files.len() * 64 + 1;
        // The index file is Jellymoon's own, and this keep is the only one of the home: a lock on it is one a
        // keep that was cut off left.
        let lock = index.with_extension("index.lock");
        let _ = fs::remove_file(&lock);
        let read = mine.answer(None, &own_index, &["read-tree", target], &[], SAID_MAX);
        let then = mine.answer(None, &own_index, &[&asked[..1], &["--cached"], &asked[1..]].concat(), &names, most);
        let _ = (fs::remove_file(index), fs::remove_file(&lock));
        let now = mine.answer(Some(folder), &[], &asked, &names, most);
        if !matches!(read, Some((Some(0), _))) {
            return Err(Unread::Repo.into());
        }
        for said in [then, now] {
            let Some((Some(0), said)) = said.filter(|said| said.1.len() < most) else {
                return Err(Unread::Repo.into());
            };
            if !unfiltered(&said) {
                return Err(Unkept::Filter);
            }
        }
    }
    let trial = mine.answer(Some(folder), &[], &["read-tree", "-n", "-m", "-u", tip, target], &[], SAID_MAX);
    if !matches!(trial, Some((Some(0), _))) {
        return Err(if locked(own) { Unkept::Locked } else { Unkept::InWay });
    }
    let removed: Vec<&[u8]> = files.iter().filter(|file| file.removed).map(|file| &file.path[..]).collect();
    let in_way = !again && files.iter().any(|file| file.made && taken(folder, &file.path, &removed));
    if in_way { Err(Unkept::InWay) } else { Ok(()) }
}

/// Whether something is on the disk where a file the work makes would go: at its path, or a file or a link where
/// a folder of its path would be. On a disk that takes two spellings for one name, a file of the person's under
/// the other spelling is there too. What the work itself removes from there is not in the way: git's trial has
/// asked of it. Nothing is followed and nothing is opened.
fn taken(folder: &Path, path: &[u8], removed: &[&[u8]]) -> bool {
    let there = |path: &[u8]| on_disk(folder, path).and_then(|path| fs::symlink_metadata(path).ok());
    let below = |gone: &&[u8]| gone.len() > path.len() && gone.starts_with(path) && gone[path.len()] == b'/';
    let whole = there(path).is_some_and(|there| !(there.is_dir() && removed.iter().any(below)));
    let mut above = path.iter().enumerate().filter(|(_, byte)| **byte == b'/').map(|(at, _)| &path[..at]);
    whole || above.any(|above| !removed.contains(&above) && there(above).is_some_and(|there| !there.is_dir()))
}

/// Whether `check-attr -z` says of every path that no filter is set: a path, the attribute's name and its value,
/// each ended by a zero byte.
fn unfiltered(said: &[u8]) -> bool {
    let parts: Vec<&[u8]> = said.split(|byte| *byte == 0).collect();
    let (answers, rest) = parts.split_at(parts.len() - 1);
    let unset = |answer: &[&[u8]]| answer.len() == 3 && (answer[2] == b"unspecified" || answer[2] == b"unset");
    rest.iter().all(|rest| rest.is_empty()) && answers.chunks(3).all(unset)
}

/// Where a path of the store is in a checkout. None for a path that leads out of it, which git never writes, and
/// on a system whose names are text for a path that is not.
fn on_disk(folder: &Path, path: &[u8]) -> Option<PathBuf> {
    #[cfg(unix)]
    let below = Path::new(<std::ffi::OsStr as std::os::unix::ffi::OsStrExt>::from_bytes(path));
    #[cfg(not(unix))]
    let below = Path::new(std::str::from_utf8(path).ok()?);
    below.components().all(|part| matches!(part, Component::Normal(_))).then(|| folder.join(below))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_list_of_a_change_says_which_files_are_made_removed_and_another_repos() {
        let (a, none) = ("a".repeat(40), "0".repeat(40));
        let list = format!(
            ":000000 100644 {none} {a} A\0-x\ny\0:100644 000000 {a} {none} D\0gone\0:160000 100644 {a} {a} T\0was\0\
             :100644 160000 {a} {a} M\0sub\0:100644 100644 {a} {a} M\0plain\0"
        );
        let files = listed(list.as_bytes()).unwrap();
        let seen: Vec<_> = files.iter().map(|file| (&file.path[..], file.repo, file.made, file.removed)).collect();
        let want = [
            (&b"-x\ny"[..], false, true, false),
            (b"gone", false, false, true),
            (b"was", true, false, false),
            (b"sub", true, false, false),
            (b"plain", false, false, false),
        ];
        assert_eq!(seen, want);
        assert!(listed(b"").unwrap().is_empty());
        for broken in [format!(":000000 100644 {none} {a} A\0"), format!(":100644 100644 {a} {a} R100\0x\0y\0")] {
            assert!(listed(broken.as_bytes()).is_none(), "{broken:?}");
        }
    }

    #[test]
    fn only_a_git_that_says_it_is_2_38_or_later_merges_in_the_store() {
        for new in ["git version 2.38.0", "git version 2.50.1 (Apple Git-155)", "git version 3.0.0", "git version 2.38"]
        {
            assert!(new_enough(new), "{new}");
        }
        for old in ["git version 2.37.9", "git version 1.99.0", "git version", "2.50.1", "git version x.50", ""] {
            assert!(!new_enough(old), "{old}");
        }
    }

    #[test]
    fn a_filter_is_set_unless_git_says_unspecified_or_unset_of_every_path() {
        assert!(unfiltered(b"a.txt\0filter\0unspecified\0b\0filter\0unset\0"));
        assert!(unfiltered(b""));
        for set in [
            &b"a\0filter\0lfs\0"[..],
            b"a\0filter\0set\0",
            b"a\0filter\0unspecified\0b\0filter\0x\0",
            b"a\0filter\0",
            b"a\0filter\0unset",
        ] {
            assert!(!unfiltered(set), "{set:?}");
        }
    }

    #[test]
    fn what_is_on_the_disk_where_a_made_file_would_go_is_in_the_way_unless_the_work_removes_it() {
        let dir = tempfile::tempdir().unwrap();
        let folder = dir.path();
        fs::write(folder.join("f"), "").unwrap();
        fs::create_dir(folder.join("d")).unwrap();
        fs::write(folder.join("d").join("x"), "").unwrap();
        let taken = |path: &str, removed: &[&[u8]]| taken(folder, path.as_bytes(), removed);
        for free in ["new", "d/y", "new/deep/file", "../f", "d/../f"] {
            assert!(!taken(free, &[]), "{free}");
        }
        for held in ["f", "d", "d/x", "f/x", "f/deep/x"] {
            assert!(taken(held, &[]), "{held}");
        }
        // A file the work removes to make a folder there, and a folder it empties to make a file there.
        assert!(!taken("f/x", &[b"f"]) && !taken("f/deep/x", &[b"f"]));
        assert!(!taken("d", &[b"d/x"]));
        // What the work removes elsewhere changes nothing.
        assert!(taken("f/x", &[b"f2", b"d/x"]) && taken("d", &[b"dx", b"d2/x", b"f"]) && taken("f", &[b"f/x"]));
    }

    #[test]
    fn a_path_that_leads_out_of_the_checkout_is_nowhere_on_the_disk() {
        let folder = Path::new("checkout");
        assert_eq!(on_disk(folder, b"d/e.txt"), Some(folder.join("d").join("e.txt")));
        for out in [&b"../x"[..], b"d/../../x", b"/etc/x"] {
            assert_eq!(on_disk(folder, out), None, "{out:?}");
        }
    }
}
