//! The review of F75, findings 2 and 6: a world's `PATH` that holds the person's own harness, and a value Windows
//! reads from another drive's folder. A folder that stands for a person's own is one of the test's, judged with
//! temporary folders that leave it outside: nothing real is named, and nothing is started.

use std::env;
use std::ffi::OsStr;
use std::fs;
use std::panic::catch_unwind;
use std::path::Path;
use std::slice::from_ref;

use super::{NO_COMMAND, NONE, linked, outside, root, said};
use crate::support::door::{self, REFUSED};
use crate::support::{install, scratch, skipped};

/// Every variable the door holds to the temporary folders: the folders, git's config file and the named harness.
pub const HELD: [&str; 14] = [
    "HOME",
    "USERPROFILE",
    "JELLYMOON_HOME",
    "JELLYMOON_APP_DIR",
    "JELLYMOON_BIN_DIR",
    "CLAUDE_CONFIG_DIR",
    "TMPDIR",
    "TEMP",
    "TMP",
    "XDG_CONFIG_HOME",
    "APPDATA",
    "LOCALAPPDATA",
    "GIT_CONFIG_GLOBAL",
    "JELLYMOON_CLAUDE",
];

const LISTS: [&str; 2] = ["PATH", "JELLYMOON_SYSTEM_BIN"];

// The rule alone, with what a folder holds given as an answer: no folder outside the temporary folders is asked.
#[test]
fn a_list_of_folders_reaches_a_harness_only_in_a_full_path_outside_the_temporary_folders_that_holds_one() {
    let (dir, roots) = (scratch(), door::roots());
    let (inside, real) = (dir.path().join("bin"), outside("bin"));
    let list = |folders: &[&Path]| env::join_paths(folders).unwrap();
    let (both, all) = (list(&[&inside, &real]), |_: &Path| true);
    assert!(door::reaches(&roots, &both, |folder| folder == real), "after a folder of the test's own");
    assert!(door::reaches(&roots, &list(&[&real]), all), "alone");
    assert!(!door::reaches(&roots, &both, |folder| folder == inside), "the one that holds a harness is the test's");
    assert!(!door::reaches(&roots, &both, |_| false), "no folder holds one");
    assert!(!door::reaches(&roots, &list(&[&inside, Path::new("bin"), Path::new("")]), all), "no full path outside");
    assert!(!door::reaches(&roots, OsStr::new(""), all), "no folder at all");
    // A folder under a temporary folder that is a link out of it is outside.
    if linked(&root(), &dir.path().join("out")) {
        assert!(door::reaches(&roots, &list(&[&dir.path().join("out")]), all));
    }
}

#[test]
fn a_world_whose_path_or_system_folders_hold_a_harness_outside_the_temporary_folders_is_refused() {
    let dir = scratch();
    let (bin, sys) = (dir.path().join("bin"), dir.path().join("sys"));
    install(&bin, "claude");
    fs::create_dir(&sys).unwrap();
    let both = env::join_paths([&bin, &sys]).unwrap();
    // Every folder is temporary: the made-up harness is the world's own.
    let world = [("PATH", &both), ("JELLYMOON_SYSTEM_BIN", &both)];
    assert_eq!(door::refusal(&door::roots(), &world, NO_COMMAND), None);
    for name in LISTS {
        let world = [(name, &both)];
        // The same world where `sys` alone is temporary: the folder that holds a harness is outside.
        assert_eq!(door::refusal(from_ref(&sys), &world, NO_COMMAND), Some(name.to_string()));
        assert_eq!(door::refusal(from_ref(&bin), &world, NO_COMMAND), None, "{name}: the folder outside holds none");
        // Each of the harness's names is one, whatever the system.
        for harness in ["claude.exe", "claude.cmd"] {
            fs::write(sys.join(harness), "").unwrap();
            assert_eq!(door::refusal(from_ref(&bin), &world, NO_COMMAND), Some(name.to_string()), "{harness}");
            fs::remove_file(sys.join(harness)).unwrap();
        }
        fs::write(sys.join("claude-notes"), "").unwrap();
        assert_eq!(door::refusal(from_ref(&bin), &world, NO_COMMAND), None, "a longer name is no harness");
    }
    // As Windows reads a world the name is the same in any case.
    let lower = [("Path", &both)];
    assert_eq!(door::refusal_as(true, from_ref(&sys), &lower, NO_COMMAND), Some("Path".to_string()));
    assert_eq!(door::refusal_as(false, from_ref(&sys), &lower, NO_COMMAND), None);
}

// The world F38 had: the whole `PATH` of the person running the tests. Their harness is asked for by name, as the
// door asks, and never started.
#[test]
fn the_test_process_s_own_path_is_refused_at_the_door_where_a_claude_is_on_it() {
    let (own, roots) = (env::var_os("PATH").unwrap_or_default(), door::roots());
    let there = |program: std::path::PathBuf| program.symlink_metadata().is_ok();
    let named = |folder: &Path| ["claude", "claude.exe", "claude.cmd"].iter().any(|name| there(folder.join(name)));
    let temporary = |folder: &Path| door::refusal(&roots, &[("HOME", folder)], NONE).is_none();
    let on_it = env::split_paths(&own).any(|folder| folder.is_absolute() && named(&folder) && !temporary(&folder));
    if !on_it {
        skipped("the person's own PATH at the door: no claude is on it here");
        assert_eq!(door::refusal(&roots, &[("PATH", &own)], NO_COMMAND), None);
        return;
    }
    for name in LISTS {
        assert_eq!(door::refusal(&roots, &[(name, &own)], NO_COMMAND), Some(name.to_string()));
    }
    let made = catch_unwind(|| door::jellymoon([("PATH", &own)], NO_COMMAND));
    assert_eq!(said(made.err().unwrap()), format!("{REFUSED}; this one named PATH"));
}

// Finding 6: `D:logs` has no root and no `..`, and Windows reads it from the folder drive D was left in.
#[test]
fn as_windows_reads_a_value_one_with_a_drive_and_no_root_leaves_its_folder() {
    let roots = door::roots();
    for value in ["D:logs", "D:", "z:.claude"] {
        for name in ["CLAUDE_CONFIG_DIR", "HOME", "JELLYMOON_BIN_DIR", "GIT_CONFIG_GLOBAL"] {
            let world = [(name, value)];
            assert_eq!(door::refusal_as(true, &roots, &world, NONE), Some(name.to_string()), "{value}");
            assert_eq!(door::refusal_as(false, &roots, &world, NONE), None, "{value}: elsewhere, a name");
        }
        let logs = ["day", "--logs", value];
        assert_eq!(door::refusal_as(true, &roots, &world_of_none(), &logs), Some("--logs".to_string()), "{value}");
        assert_eq!(door::refusal_as(false, &roots, &world_of_none(), &logs), None, "{value}");
    }
    // A colon anywhere else is a letter of a name, and so is one letter alone.
    for value in ["ab:c", "logs:D", "D", ":logs"] {
        let world = [("CLAUDE_CONFIG_DIR", value), ("HOME", value)];
        assert_eq!(door::refusal_as(true, &roots, &world, &["day", "--logs", value]), None, "{value}");
    }
    // A drive with a root is a full path, which only Windows can resolve: there it is outside, and elsewhere
    // the rule for a value that is no full path leaves it be.
    for full in [r"D:\jellymoon-f74-made-up", "D:/jellymoon-f74-made-up"] {
        let world = [("HOME", full)];
        assert_eq!(door::refusal_as(true, &roots, &world, NONE), cfg!(windows).then(|| "HOME".to_string()), "{full}");
    }
}

fn world_of_none() -> [(&'static str, &'static str); 0] {
    []
}
