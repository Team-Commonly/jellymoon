//! F75: no test of `jellymoon-app` can start it in a real home, or ask a real login shell (ADR 0021, decision 7).
//! The door is `support/door.rs`, and its rule is the binary's own file, whose every case `jellymoon`'s F74 test
//! holds; here is what the row says of the app. Every path that stands for a person's own is made up: a full path
//! in no temporary folder that is not there, or the root folder where one that is there is needed. Nothing is ever
//! started in either: a command the door lets through is `layout` with a flag the app does not know.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::ffi::OsStr;
use std::fs;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use support::day::{self, DAY};
use support::door::{self, BARE, REFUSED};
use support::{page, pieces, scratch, system_scratch, wish};

const MADE_UP: &str = "jellymoon-f75-made-up";
const NO_FLAG: &[&str] = &["layout", "--f75-no-such-flag"];
const NONE: &[&str] = &[];
const FOLDERS: [&str; 6] =
    ["HOME", "USERPROFILE", "JELLYMOON_HOME", "JELLYMOON_APP_DIR", "JELLYMOON_BIN_DIR", "CLAUDE_CONFIG_DIR"];

/// The root folder: a full path that is there on every system and under no temporary folder.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).ancestors().last().unwrap().to_path_buf()
}

/// A full path in no temporary folder, which is not there.
fn outside(name: &str) -> PathBuf {
    root().join(MADE_UP).join(name)
}

/// What the app's door refuses of a world, with the temporary folders it really has.
fn refused(vars: &[(&str, &Path)], args: &[&str]) -> Option<String> {
    door::refusal(&door::roots(), vars, args)
}

/// What a panic said.
fn said(panic: Box<dyn std::any::Any + Send>) -> String {
    panic.downcast_ref::<String>().cloned().unwrap_or_default()
}

/// Makes a link to a folder. On Windows only a person allowed to can: there the half that needs one says so.
fn linked(to: &Path, at: &Path) -> bool {
    #[cfg(unix)]
    let made = std::os::unix::fs::symlink(to, at);
    #[cfg(windows)]
    let made = std::os::windows::fs::symlink_dir(to, at);
    if made.is_err() {
        eprintln!("left out: the door's links, since this system let the test make none");
    }
    made.is_ok()
}

/// The value of a variable a command would be started with: none when it would not be set at all.
fn set(command: &Command, name: &str) -> Option<String> {
    let set = command.get_envs().filter(|(set, _)| *set == OsStr::new(name));
    set.last().and_then(|(_, value)| value.map(|value| value.to_string_lossy().into_owned()))
}

#[test]
fn a_home_a_folder_of_jellymoon_s_or_the_harness_s_outside_the_temporary_folders_is_refused() {
    let (dir, real) = (scratch(), outside("home"));
    // A world with each of them under Cargo's temporary folder, but the one named, which is the made-up real one.
    let with = |swap: &str| {
        let folder = |name: &str| if name == swap { real.clone() } else { dir.path().join(name) };
        let vars: Vec<(&str, PathBuf)> = FOLDERS.iter().map(|name| (*name, folder(name))).collect();
        door::refusal(&door::roots(), &vars, NO_FLAG)
    };
    assert_eq!(with("none of them"), None);
    for name in FOLDERS {
        assert_eq!(with(name), Some(name.to_string()));
    }
    // The harness a world names is a made-up one under a temporary folder, and the folders and the file the
    // programs hand on are held too (the door's review, findings 3 and 4).
    let more = ["JELLYMOON_CLAUDE", "TMPDIR", "TEMP", "TMP", "XDG_CONFIG_HOME", "APPDATA", "LOCALAPPDATA"];
    for name in more.into_iter().chain(["GIT_CONFIG_GLOBAL"]) {
        assert_eq!(refused(&[(name, real.as_path())], NO_FLAG), Some(name.to_string()));
        assert_eq!(refused(&[(name, dir.path().join(name).as_path())], NO_FLAG), None, "{name} of the test's own");
        assert_eq!(refused(&[(name, Path::new("../up"))], NO_FLAG), Some(name.to_string()), "{name} that climbs");
    }
    // Any other variable may name any path: only these are a person's own.
    assert_eq!(refused(&[("JELLYMOON_RELEASE_URL", real.as_path())], NO_FLAG), None);
    // As Windows reads a world a name is the same in any case, and the door reads it as the system does.
    let lower = [("userprofile", real.as_path())];
    assert_eq!(door::refusal_as(true, &door::roots(), &lower, NO_FLAG), Some("userprofile".to_string()));
    assert_eq!(refused(&lower, NO_FLAG), cfg!(windows).then(|| "userprofile".to_string()));
}

#[test]
fn a_value_of_logs_outside_the_temporary_folders_is_refused_however_it_is_written() {
    let (dir, real, logs) = (scratch(), outside("logs"), Some("--logs".to_string()));
    let (real, inside) = (real.to_str().unwrap(), dir.path().join("logs"));
    let inside = inside.to_str().unwrap();
    assert_eq!(refused(&[], &["layout", "--date", DAY, "--logs", real]), logs);
    assert_eq!(refused(&[], &["layout", &format!("--logs={real}"), "--date", DAY]), logs);
    assert_eq!(refused(&[], &["layout", "--logs", "../up"]), logs, "one that climbs out of its folder");
    assert_eq!(refused(&[], &["layout", "--logs", inside]), None, "under a temporary folder");
    assert_eq!(refused(&[], &["layout", &format!("--logs={inside}")]), None, "joined, under a temporary folder");
    assert_eq!(refused(&[], &["layout", "--logs", "logs"]), None, "one that is not a full path");
    assert_eq!(refused(&[], &["layout", "--logs"]), None, "no value");
    assert_eq!(refused(&[], &["layout", "--logs", inside, real]), None, "what follows the value is not the value");
    assert_eq!(refused(&[], &["layout", "--page", real, "--theme", real]), None, "a file a test names to be read");
}

#[test]
fn a_world_under_either_temporary_folder_passes_and_layout_with_no_home_still_starts() {
    for dir in [scratch(), system_scratch()] {
        let (there, gone) = (dir.path().join("user"), dir.path().join("not").join("there"));
        fs::create_dir(&there).unwrap();
        for home in [there.as_path(), gone.as_path(), dir.path()] {
            assert_eq!(refused(&[("HOME", home), ("JELLYMOON_HOME", home)], NO_FLAG), None, "{}", home.display());
        }
    }
    // The system's and the one Cargo gives tests, and no third: no test of the app needs a home of another owner.
    assert_eq!(door::roots().len(), 2);
    assert_eq!(door::roots()[1], Path::new(env!("CARGO_TARGET_TMPDIR")));
    for folder in door::roots() {
        assert_eq!(refused(&[("HOME", folder.as_path())], NO_FLAG), None, "a temporary folder itself");
    }
    for home in ["", "user", ".", "~"] {
        let vars = [("HOME", Path::new(home)), ("USERPROFILE", Path::new(home)), ("JELLYMOON_HOME", Path::new(home))];
        assert_eq!(refused(&vars, NO_FLAG), None, "{home:?}");
    }
    // One that climbs out of the folder it is read from does not pass (the door's review, finding 1).
    for name in FOLDERS {
        assert_eq!(refused(&[(name, Path::new(".."))], NO_FLAG), Some(name.to_string()));
    }
    assert_eq!(refused(&[], NONE), None, "no variable at all");

    // `jellymoon-app layout` with no home is started, as before, and places the page it is given.
    let placed = pieces(&page(vec![wish("w1", "Login page", "working", &[])], vec![]), &[]);
    assert!(placed.iter().any(|piece| piece.kind() == "label"), "{placed:?}");
    // So is a world under a temporary folder: the day is read with the made-up harness and placed.
    let world = day::world();
    assert!(!day::shown(&world, &[]).is_empty() && day::called(&world));
    // And a home that is not a full path: the app says there is none.
    let out = day::run(&world, &[], &[("JELLYMOON_HOME", "jm"), ("HOME", "user")]);
    assert_eq!(out.status.code(), Some(2), "{}", String::from_utf8_lossy(&out.stderr));
}

#[test]
fn a_path_is_resolved_as_far_as_it_is_there_whether_it_is_there_or_not() {
    let (dir, root) = (scratch(), root());
    let home = |path: &Path| refused(&[("HOME", path)], NO_FLAG);
    assert_eq!(home(&root), Some("HOME".to_string()), "a folder that is there");
    assert_eq!(home(&outside("home")), Some("HOME".to_string()), "one that is not");
    let (gone, up) = (dir.path().join("not-there"), "../".repeat(64));
    assert_eq!(home(&gone.join("user")), None);
    // Out of the test's folder by `..` after a name that is not there, where no folder can be asked.
    let climbed = gone.join(&up).join(MADE_UP);
    assert_eq!(home(&climbed), Some("HOME".to_string()), "{}", climbed.display());
    assert_eq!(refused(&[], &["layout", "--logs", climbed.to_str().unwrap()]), Some("--logs".to_string()));
    // And back in by `..`: the path is what it leads to, not how it is written.
    let back = gone.join("deeper").join("..").join("user");
    assert_eq!(home(&back), None, "{}", back.display());
}

#[test]
fn links_are_resolved_on_the_side_of_the_path_and_on_the_side_of_the_temporary_folder() {
    let dir = scratch();
    let at = |name: &str| dir.path().join(name);
    fs::create_dir(at("real")).unwrap();
    if !linked(&root(), &at("out")) {
        return;
    }
    let home = |path: PathBuf| refused(&[("HOME", path.as_path())], NO_FLAG);
    // A home under a temporary folder that is a link out of it, and a path through such a link that is not there.
    assert_eq!(home(at("out")), Some("HOME".to_string()));
    assert_eq!(home(at("out").join(MADE_UP).join("home")), Some("HOME".to_string()));
    // A link whose end is not there is read, not taken for a folder of the test's own.
    assert!(linked(&outside("home"), &at("dangling")));
    assert_eq!(home(at("dangling").join("deeper")), Some("HOME".to_string()));
    assert_eq!(refused(&[], &["layout", "--logs", at("dangling").to_str().unwrap()]), Some("--logs".to_string()));
    // A link that stays inside is a folder inside, and so is one whose end inside is not there.
    assert!(linked(&at("real"), &at("alias")) && linked(&at("real").join("later"), &at("soon")));
    assert_eq!((home(at("alias").join("home")), home(at("soon").join("home"))), (None, None));
    // The temporary folder's side: a folder named through a link holds what is under the folder it leads to, and
    // the other way round. Nothing else is under it.
    let under = |folder: PathBuf, path: PathBuf| door::refusal(&[folder], &[("HOME", path)], NONE).is_none();
    assert!(under(at("alias"), at("real").join("home")), "the folder is named through a link");
    assert!(under(at("real"), at("alias").join("home")), "the path goes through a link");
    assert!(!under(at("real"), at("other").join("home")) && !under(at("alias"), dir.path().to_path_buf()));
    assert!(!under(at("alias"), at("out")), "a link out is outside whatever the folder is named by");
}

#[test]
fn every_world_that_is_started_has_the_login_shell_set_to_nothing_and_a_real_one_is_refused() {
    let dir = scratch();
    let home = [("HOME", dir.path())];
    let made = [
        door::app(BARE, NO_FLAG),
        door::app(home, NO_FLAG),
        door::jellymoon(home, NONE),
        door::window(false, home, NONE),
        door::window(true, home, NONE),
        door::command("some-program", [("PATH", dir.path())], NONE),
    ];
    for command in &made {
        assert_eq!(set(command, "JELLYMOON_SHELL").as_deref(), Some(""), "{command:?}");
        // No folder of this machine is looked in for a harness either, unless the test names one of its own.
        assert_eq!(set(command, "JELLYMOON_SYSTEM_BIN").as_deref(), Some(""), "{command:?}");
    }
    let own = door::app([("JELLYMOON_SYSTEM_BIN", dir.path())], NO_FLAG);
    assert_eq!(set(&own, "JELLYMOON_SYSTEM_BIN").map(PathBuf::from), Some(dir.path().to_path_buf()));
    // A login shell that is named is a made-up one under a temporary folder.
    let (real, made_up) = (outside("zsh"), dir.path().join("bin").join("zsh"));
    let named = |shell: &Path| refused(&[("JELLYMOON_SHELL", shell)], NO_FLAG);
    assert_eq!(named(&real), Some("JELLYMOON_SHELL".to_string()));
    assert_eq!(named(Path::new("zsh")), Some("JELLYMOON_SHELL".to_string()));
    assert_eq!((named(Path::new("")), named(&made_up)), (None, None));

    // And a program that is started sees it: the made-up harness leaves its variables beside itself.
    let world = day::world();
    let mut harness = door::command(world.claude(), [("HOME", world.dir.path())], NONE);
    assert!(harness.stdin(Stdio::null()).stdout(Stdio::null()).status().unwrap().success());
    let seen = fs::read_to_string(world.rec.join("env")).unwrap();
    assert!(seen.lines().any(|line| line == "JELLYMOON_SHELL="), "{seen}");
}

#[test]
fn a_refusal_is_one_fixed_sentence_with_the_name_it_refused_and_nothing_is_started() {
    let (world, real) = (day::world(), outside("home"));
    let start = |home: &Path| {
        let mut harness = door::command(world.claude(), [("HOME", home)], NONE);
        catch_unwind(AssertUnwindSafe(|| harness.stdin(Stdio::null()).stdout(Stdio::null()).status().unwrap()))
    };
    let unwound = catch_unwind(AssertUnwindSafe(|| door::command(world.claude(), [("HOME", &real)], NONE)));
    let why = said(unwound.expect_err("a home outside the temporary folders was let through"));
    assert_eq!(why, format!("{REFUSED}; this one named HOME"));
    assert!(!why.contains(MADE_UP) && !REFUSED.contains('\\'), "the sentence names no path: {why}");
    assert!(!day::called(&world), "the program was started");
    // The same program in a world the door passes is started: what it leaves is how a start is seen.
    assert!(start(world.dir.path()).is_ok_and(|status| status.success()) && day::called(&world));

    // The support's own ways to run the app go through the same door.
    let real = real.to_str().unwrap();
    let by_var = catch_unwind(AssertUnwindSafe(|| day::app(&world, NO_FLAG, &[("JELLYMOON_HOME", real)])));
    assert_eq!(said(by_var.err().unwrap()), format!("{REFUSED}; this one named JELLYMOON_HOME"));
    let by_flag = catch_unwind(|| support::run(&["layout", "--logs", real, "--f75-no-such-flag"], None));
    assert_eq!(said(by_flag.err().unwrap()), format!("{REFUSED}; this one named --logs"));
    let by_day = catch_unwind(AssertUnwindSafe(|| day::run(&world, &["--logs", real, "--f75-no-such-flag"], &[])));
    assert_eq!(said(by_day.err().unwrap()), format!("{REFUSED}; this one named --logs"));
    assert!(!day::called(&world), "the app was started and read the day");
}

// What the support starts, it starts at the door: no other file of it makes a command.
#[test]
fn the_support_makes_a_command_nowhere_but_at_the_door() {
    let support = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("support");
    let mut read = 0;
    for file in fs::read_dir(&support).unwrap().map(|entry| entry.unwrap().path()) {
        let text = fs::read_to_string(&file).unwrap();
        let code = text.lines().filter(|line| !line.trim_start().starts_with("//"));
        let makes: Vec<&str> = code.filter(|line| line.contains("Command::new")).collect();
        assert!(makes.is_empty(), "{}: {makes:?}", file.display());
        read += 1;
    }
    assert!(read >= 7, "the support's files were not read: {read}");
    // The door's own file makes none either: the rule's file does, which the binary's door takes too.
    let rule = support.join("../../../crates/jellymoon/tests/support/door/world.rs");
    assert_eq!(fs::read_to_string(rule).unwrap().matches("Command::new").count(), 1);
}
