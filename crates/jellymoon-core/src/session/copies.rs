//! A run's copy is the run's place, not the person's (ADR 0018, decision 7): a session line whose working directory
//! is below the home's `copies` is left out, whatever it holds. The log's working directory is untrusted text and
//! is compared as text, by whole names; nothing of it is looked up. Only the home's side is resolved.

use std::fs;
use std::path::Path;

/// The home's `copies` folder under each spelling a harness may log: as the home is given, as the system
/// resolves the home (on a Mac a home under `/tmp` is logged under `/private`), and as it resolves `copies` itself,
/// which a person may have moved to another disk and linked. Each is a root and its names.
#[derive(Debug, Default)]
pub(crate) struct Copies {
    spelled: Vec<(usize, Vec<String>)>,
    windows: bool,
}

impl Copies {
    /// The copies of `home`, which is a full path. A home that is not there has one spelling, the one given.
    pub(crate) fn of(home: &Path) -> Self {
        let given = home.join("copies");
        let resolved = [fs::canonicalize(home).ok().map(|home| home.join("copies")), fs::canonicalize(&given).ok()];
        let folders = [Some(given)].into_iter().chain(resolved).flatten();
        Self::spelled(folders.map(|folder| folder.to_string_lossy().into_owned()), cfg!(windows))
    }

    /// The copies at `folders`, read as Windows writes a path or as every other system does.
    pub(super) fn spelled(folders: impl IntoIterator<Item = String>, windows: bool) -> Self {
        let whole = |folder: String| {
            let (root, names) = parts(&folder, windows)?;
            let names: Vec<String> = names.map(str::to_string).collect();
            (!names.iter().any(|name| name == "..")).then_some((root, names))
        };
        Self { spelled: folders.into_iter().filter_map(whole).collect(), windows }
    }

    /// Whether `folder`, a log's text, is a copy or a folder in one: a full path whose first names are a
    /// spelling's, each whole, with at least one name more. `copies` itself is no copy. A path with a `..` in it
    /// is never one: what it leads to cannot be told without looking it up. Nothing is kept of the folder, so one
    /// of any length is walked once for each spelling.
    pub(crate) fn hold(&self, folder: &str) -> bool {
        let same =
            |held: &String, name: &str| if self.windows { held.eq_ignore_ascii_case(name) } else { held == name };
        self.spelled.iter().any(|(root, copies)| {
            let Some((at, mut names)) = parts(folder, self.windows) else { return false };
            at == *root
                && copies.iter().all(|held| names.next().is_some_and(|name| same(held, name)))
                && names.try_fold(0, |below, name| (name != "..").then_some(below + 1)).is_some_and(|below| below > 0)
        })
    }
}

/// A full path's root and its names; none for a path that is not full. A separator is `/`, and on Windows `\` too.
/// An empty name and `.` are no names, so a separator that is doubled or ends the path changes nothing. The root is
/// how many separators the path starts with: one, or on Windows two for a shared folder (`\\server\share`) and none
/// before a drive (`C:`). Windows' long spelling of either (`\\?\C:\…`, `\\?\UNC\server\share\…`), which is what the
/// system resolves a path to there, is read as the short one.
fn parts(path: &str, windows: bool) -> Option<(usize, impl Iterator<Item = &str>)> {
    let sep = move |c: char| c == '/' || windows && c == '\\';
    let long = path.strip_prefix(r"\\?\"); // off Windows what is left has no root, so it is no full path
    let (root, path) = match (long, long.and_then(|rest| rest.strip_prefix(r"UNC\"))) {
        (_, Some(shared)) => (2, shared),
        (Some(drive), None) => (0, drive),
        (None, _) => (path.chars().take_while(|c| sep(*c)).count().min(if windows { 2 } else { 1 }), path),
    };
    let mut names = path.split(sep).filter(|name| !name.is_empty() && *name != ".").peekable();
    let drive = names.peek().is_some_and(|name| name.len() == 2 && name.ends_with(':'));
    let full = if windows { root == 2 || root == 0 && drive } else { root == 1 };
    full.then_some((root, names))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn copies(folders: &[&str], windows: bool) -> Copies {
        Copies::spelled(folders.iter().map(|folder| folder.to_string()), windows)
    }

    #[test]
    fn a_folder_below_copies_is_held_by_whole_names_and_nothing_else_is() {
        let held = copies(&["/work/ada/jm/copies", "/private/work/ada/jm/copies/"], false);
        for folder in [
            "/work/ada/jm/copies/r1",
            "/work/ada/jm/copies/r1/src/web",
            "/work/ada/jm/copies/r1/",
            "//work/ada//jm/./copies//r1/.",
            "/private/work/ada/jm/copies/r1",
            "/work/ada/jm/copies/a\\b", // one name here: only Windows parts names at a `\`
        ] {
            assert!(held.hold(folder), "{folder}");
        }
        for folder in [
            "",
            "/",
            "/work/ada/jm/copies",
            "/work/ada/jm/copies/",
            "/work/ada/jm/copies/.",
            "/work/ada/jm/copies-old/r1",
            "/work/ada/jm/copiesx",
            "/work/ada/jm/copie/s/r1",
            "/work/ada/jm/old-copies/r1",
            "/work/ada/xjm/copies/r1",
            "\\\\?\\/work/ada/jm/copies/r1",
            "/work/ada/jm/Copies/r1",
            "/work/ada/jm/runs/r1",
            "/work/ada/jm/kept/copies/r1",
            "/work/ada/copies/r1",
            "/work/ada/jm-old/copies/r1",
            "/work/ada/jm/r1",
            "/outer/work/ada/jm/copies/r1",
            "work/ada/jm/copies/r1",
            "/private/work/ada/jm/r1",
            "/work/ada/jm/copies/..",
            "/work/ada/jm/copies/../copies/r1",
            "/work/ada/jm/copies/r1/../../pages",
            "/work/ada/jm/copies/r1/src/..",
            "/work/ada/jm\\copies\\r1",
            r"C:\work\ada\jm\copies\r1",
        ] {
            assert!(!held.hold(folder), "{folder}");
        }
        // A home that is not a full path, or climbs, names no copies: only its resolved spelling can.
        let none = copies(&["jm/copies", "/work/ada/../bea/copies", ""], false);
        assert!(!none.hold("jm/copies/r1") && !none.hold("/work/ada/../bea/copies/r1") && !none.hold("/r1"));
        assert!(!Copies::default().hold("/work/ada/jm/copies/r1"));
    }

    #[test]
    fn a_folder_of_any_length_is_walked_with_no_list_of_its_names_made() {
        let held = copies(&["/jm/copies"], false);
        assert!(held.hold(&format!("/jm/copies/{}", "a/".repeat(1 << 20))));
        assert!(!held.hold(&format!("/jm/copies/{}..", "a/".repeat(1 << 20))));
        assert!(!held.hold(&"/".repeat(1 << 20)));
    }

    #[test]
    fn on_windows_both_separators_part_names_case_is_not_told_apart_and_the_long_spelling_is_the_short_one() {
        let held = copies(&[r"C:\work\ada\jm\copies", r"\\?\UNC\files\homes\ada\jm\copies"], true);
        for folder in [
            r"C:\work\ada\jm\copies\r1",
            r"c:\WORK\ADA\Jm\COPIES\r1\src",
            "C:/work/ada/jm/copies/r1",
            r"C:\work\ada/jm\\copies\.\r1\",
            r"\\?\C:\work\ada\jm\copies\r1",
            r"\\files\homes\ada\jm\copies\r1",
            r"\\?\UNC\files\homes\ada\jm\copies\r1",
            "//files/homes/ada/jm/copies/r1",
        ] {
            assert!(held.hold(folder), "{folder}");
        }
        for folder in [
            r"C:\work\ada\jm\copies",
            r"C:\work\ada\jm\copies-old\r1",
            r"D:\work\ada\jm\copies\r1",
            r"\work\ada\jm\copies\r1",
            r"C:work\ada\jm\copies\r1",
            r"work\ada\jm\copies\r1",
            r"\\C:\work\ada\jm\copies\r1",
            r"C:\work\ada\jm\copies\..\skip",
            r"\files\homes\ada\jm\copies\r1",
            r"files\homes\ada\jm\copies\r1",
            r"\\?\files\homes\ada\jm\copies\r1",
            r"\\other\homes\ada\jm\copies\r1",
            "/c/work/ada/jm/copies/r1",
        ] {
            assert!(!held.hold(folder), "{folder}");
        }
        // A home with no drive and no shared folder is no full path there, and names no copies.
        let none = copies(&[r"work\jm\copies", r"\work\jm\copies", r"C:work\jm\copies", r"\\?\work\jm\copies"], true);
        for folder in [r"work\jm\copies\r1", r"\work\jm\copies\r1", r"C:work\jm\copies\r1", r"\\?\work\jm\copies\r1"] {
            assert!(!none.hold(folder), "{folder}");
        }
        // A first name that only ends in `:` is no drive: a drive is one letter and the `:`.
        assert!(!copies(&[r"CD:\work\jm\copies"], true).hold(r"CD:\work\jm\copies\r1"));
        // Only letters of ASCII are taken for one another: a home written with another `É` is another home.
        let accented = copies(&["C:\\work\\\u{c9}da\\jm\\copies"], true);
        assert!(
            accented.hold("c:\\work\\\u{c9}da\\JM\\copies\\r1") && !accented.hold("C:\\work\\\u{e9}da\\jm\\copies\\r1")
        );
        // Elsewhere a `\` is a letter of a name, and so is a drive.
        assert!(!copies(&[r"C:\work\ada\jm\copies"], false).hold(r"C:\work\ada\jm\copies\r1"));
    }

    #[test]
    fn the_home_is_spelled_as_given_and_as_the_system_resolves_it() {
        let dir = tempfile::tempdir().unwrap();
        let (given, resolved) = (dir.path().join("jm"), fs::canonicalize(dir.path()).unwrap().join("jm"));
        let run = |home: &Path| home.join("copies").join("r1").to_string_lossy().into_owned();
        assert!(Copies::of(&given).hold(&run(&given)), "a home that is not there is spelled as it is given");
        fs::create_dir(&given).unwrap();
        let held = Copies::of(&given);
        assert!(held.hold(&run(&given)) && held.hold(&run(&resolved)));
        assert!(!held.hold(&run(dir.path())) && !held.hold(&given.to_string_lossy()));
    }

    // Unix only: on Windows making a link needs a right a test cannot count on.
    #[cfg(unix)]
    #[test]
    fn a_copies_that_is_a_link_is_also_spelled_as_the_system_resolves_it() {
        let dir = tempfile::tempdir().unwrap();
        let (home, far) = (dir.path().join("jm"), dir.path().join("far"));
        fs::create_dir(&home).unwrap();
        std::os::unix::fs::symlink(&far, home.join("copies")).unwrap();
        let run = |copies: &Path| copies.join("r1").to_string_lossy().into_owned();
        let resolved = fs::canonicalize(dir.path()).unwrap().join("far");
        assert!(!Copies::of(&home).hold(&run(&resolved)), "a link to nothing resolves to nothing");
        fs::create_dir(&far).unwrap();
        let held = Copies::of(&home);
        assert!(held.hold(&run(&resolved)) && held.hold(&run(&home.join("copies"))));
        assert!(!held.hold(&resolved.to_string_lossy()) && !held.hold(&run(dir.path())));
    }
}
