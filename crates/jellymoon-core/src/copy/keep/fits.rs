//! Whether what a keep would land fits the person's checkout, asked before anything is written (`keep.rs`). The
//! step that lands it is asked again by git itself; these checks are how a refusal can say why, and how `--ask`
//! can answer at all.

use std::fs;
use std::io::ErrorKind;
use std::path::{Component, Path, PathBuf};

use super::super::git::Repo;
use super::super::work::{Done, WorkFile, changed};
use super::{SAID_MAX, Unkept, Unread, attrs, yes};

/// The most a part of a file's name is long on a disk of any of the three systems.
const PART_MAX: usize = 255;

/// One file a commit touches, from git's list of the change.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Touched {
    /// The path as git keeps it: any bytes but a zero.
    pub path: Vec<u8>,
    /// Another repo's entry, before the change or after it.
    pub repo: bool,
    made: bool,
    pub(super) removed: bool,
}

#[cfg(test)]
impl Touched {
    pub(crate) fn of(path: &[u8], repo: bool) -> Touched {
        Touched { path: path.to_vec(), repo, made: false, removed: false }
    }
}

impl From<WorkFile> for Touched {
    fn from(file: WorkFile) -> Touched {
        let (made, removed) = (file.done == Done::Made, file.done == Done::Removed);
        Touched { path: file.path, repo: file.other, made, removed }
    }
}

/// Every file that differs between two commits or trees, by the one list of a change (`work.rs`).
pub(super) fn touched(repo: &Repo, from: &str, to: &str) -> Result<Vec<Touched>, Unkept> {
    Ok(changed(repo, from, to)?.into_iter().map(Touched::from).collect())
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
/// and a folder of the home's for what the keep makes of its own; `files` are what differs between the two.
///
/// - The index is not locked.
/// - Every file that would be written has a name the disk takes ([`nameless`]).
/// - git's own trial of the move passes ([`trial`]): no unsaved change and no untracked file in the way.
/// - No file that would be written has a `filter` set, by any reading of the attributes (`attrs.rs`).
/// - Nothing is on the disk where a file the work makes would go ([`taken`]). The trial takes an ignored file as
///   nobody's; here it is the person's. A keep that goes on after one that was cut off (`again`) may find there
///   the files that keep's git wrote, which are in git's index by then: so what is there is in the way only
///   when git says it ignores it ([`ignored`]), which it says of no file in its index. The step itself stops
///   for whatever else is there.
///
/// Nothing here writes to the repo or the checkout, and nothing takes the lock on the checkout's index.
pub(super) fn fits(
    mine: &Repo,
    places: (&Path, &Path, &Path),
    moved: (&str, &str),
    files: &[Touched],
    again: bool,
) -> Result<(), Unkept> {
    let (folder, own, scratch) = places;
    if locked(own) {
        return Err(Unkept::Locked);
    }
    if files.iter().any(|file| !file.removed && nameless(folder, &file.path)) {
        return Err(Unkept::Name);
    }
    let copy = scratch.join("keep.trial");
    let clear = || (fs::remove_file(&copy), fs::remove_file(copy.with_extension("trial.lock")));
    let _ = clear();
    let passed = trial(mine, places, &copy, moved, files);
    let _ = clear();
    if !passed? {
        return Err(Unkept::InWay);
    }
    let removed: Vec<&[u8]> = files.iter().filter(|file| file.removed).map(|file| &file.path[..]).collect();
    let made = files.iter().filter(|file| file.made && taken(folder, &file.path, &removed));
    let there: Vec<&[u8]> = made.map(|file| &file.path[..]).collect();
    if there.is_empty() {
        return Ok(());
    }
    if !again || ignored(mine, folder, &there)? { Err(Unkept::InWay) } else { Ok(()) }
}

/// Whether the disk does not take a file's name: a part of it is longer than any disk of the three systems
/// takes, or the disk, asked what is there, says it is no name. git would stop at such a file part way.
fn nameless(folder: &Path, path: &[u8]) -> bool {
    let units = |part: &[u8]| String::from_utf8_lossy(part).encode_utf16().count();
    let asked = || on_disk(folder, path).map(fs::symlink_metadata);
    path.split(|byte| *byte == b'/').any(|part| units(part) > PART_MAX)
        || matches!(asked(), Some(Err(e)) if e.kind() == ErrorKind::InvalidFilename)
}

/// git's own trial of the move (`read-tree -n -m -u`), on a copy of the checkout's index that is Jellymoon's
/// (`copy`, which the caller removes, with a lock a cut keep left on it): git locks the index it is given even
/// for a trial, and a question does not lock the person's. The copy is first told which files only look changed
/// (`update-index --refresh`, which reads them): a file that was touched and holds what it held is no unsaved
/// change, to the trial as to `git merge`, which does the same to the index itself. A filter is off for the look,
/// and so is its `required` (`git.rs`): git compares the file as it is and does not stop at it. A lock on the
/// checkout's own index is not seen by the trial: it is asked before ([`fits`]) and found by the step.
///
/// A trial that passed is taken once more without `-n` and without `-u`, which writes the copy and no file of
/// the checkout: the copy is then the index the step would leave, and the attributes are asked of it (`attrs.rs`).
fn trial(
    mine: &Repo,
    places: (&Path, &Path, &Path),
    copy: &Path,
    (tip, target): (&str, &str),
    files: &[Touched],
) -> Result<bool, Unkept> {
    let (folder, own, scratch) = places;
    match fs::copy(own.join("index"), copy) {
        // A checkout that never had anything staged has no index, and git reads none as an empty one.
        Err(e) if e.kind() != ErrorKind::NotFound => return Err(Unread::Repo.into()),
        _ => {}
    }
    let index = [("GIT_INDEX_FILE", copy.to_str().ok_or(Unread::Home)?)];
    let did = |words: &[&str]| matches!(mine.answer(Some(folder), &index, words, &[], SAID_MAX), Some((Some(0), _)));
    // What the look ends with is not asked: where it could not read a file, the trial stops at that file.
    let _ = did(&["update-index", "-q", "--refresh"]);
    // The second take is written whole into the copy: an index kept in two parts would get a new shared part in
    // the git folder. What stops only that take is in the way too: it stops, as the step does, for a file of the
    // person's at a path a sparse checkout leaves out, which the trial passes.
    let second = ["-c", "core.splitIndex=false", "read-tree", "-m", tip, target];
    if !did(&["read-tree", "-n", "-m", "-u", tip, target]) || !did(&second) {
        return Ok(false);
    }
    attrs::unfiltered(mine, (folder, scratch), copy, target, files)?;
    Ok(true)
}

/// Whether git ignores any of these paths of the checkout (`check-ignore`, the names on its input): it says so
/// of no file that is in its index.
fn ignored(mine: &Repo, folder: &Path, paths: &[&[u8]]) -> Result<bool, Unkept> {
    let names: Vec<u8> = paths.iter().flat_map(|path| [*path, &b"\0"[..]].concat()).collect();
    yes(mine.answer(Some(folder), &[], &["check-ignore", "-z", "--stdin"], &names, names.len() + 1))
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

/// Where a path of the store is in a checkout. None for a path that leads out of it, which git never writes, and
/// on a system whose names are text for a path that is not.
pub(super) fn on_disk(folder: &Path, path: &[u8]) -> Option<PathBuf> {
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
        let files = super::super::super::work::listed(list.as_bytes()).unwrap();
        let files: Vec<Touched> = files.into_iter().map(Touched::from).collect();
        let seen: Vec<_> = files.iter().map(|file| (&file.path[..], file.repo, file.made, file.removed)).collect();
        let want = [
            (&b"-x\ny"[..], false, true, false),
            (b"gone", false, false, true),
            (b"was", true, false, false),
            (b"sub", true, false, false),
            (b"plain", false, false, false),
        ];
        assert_eq!(seen, want);
    }

    #[test]
    fn a_name_with_a_part_longer_than_any_disk_takes_is_no_name_for_a_file() {
        let folder = Path::new("checkout");
        let (fits, long) = ("é".repeat(127), "n".repeat(256));
        assert!(!nameless(folder, format!("deep/{fits}/{}", "n".repeat(255)).as_bytes()));
        for path in [long.clone(), format!("deep/{long}/a.txt"), format!("a/{long}")] {
            assert!(nameless(folder, path.as_bytes()), "{path}");
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
