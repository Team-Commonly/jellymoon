//! F74: no test of `jellymoon` can start it in a real home, or ask a real login shell (ADR 0021, decision 7). The
//! door is `support/door.rs`. Every path here that stands for a person's own is made up: a full path in no temporary
//! folder that is not there, or the root folder where one that is there is needed. Nothing is ever started in either.
//! A command the door lets through is given a command name `jellymoon` does not know, so it reads nothing.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::ffi::OsStr;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::{env, fs};

use support::door::{self, REFUSED};
use support::{jellymoon, jellymoon_in, plant, scratch, skipped, system_scratch};

const MADE_UP: &str = "jellymoon-f74-made-up";
const NO_COMMAND: &[&str] = &["f74-no-such-command"];
const NONE: &[&str] = &[];

/// The root folder: a full path that is there on every system and under no temporary folder.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).ancestors().last().unwrap().to_path_buf()
}

/// A full path in no temporary folder, which is not there.
fn outside(name: &str) -> PathBuf {
    root().join(MADE_UP).join(name)
}

/// What the door refuses of a world, with the temporary folders it really has.
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
        skipped("the door's links: this system let the test make none");
    }
    made.is_ok()
}

#[test]
fn a_home_a_folder_of_jellymoon_s_or_the_harness_s_outside_the_temporary_folders_is_refused() {
    let (dir, real) = (scratch(), outside("home"));
    let names = ["HOME", "USERPROFILE", "JELLYMOON_HOME", "CLAUDE_CONFIG_DIR"];
    // A world with each of them under Cargo's temporary folder, but the one named, which is the made-up real one.
    let with = |swap: &str| {
        let folder = |name: &str| if name == swap { real.clone() } else { dir.path().join(name) };
        let vars: Vec<(&str, PathBuf)> = names.iter().map(|name| (*name, folder(name))).collect();
        door::refusal(&door::roots(), &vars, NO_COMMAND)
    };
    assert_eq!(with("none of them"), None);
    for name in names {
        assert_eq!(with(name), Some(name.to_string()));
    }
    // Any other variable may name any path: only these are a person's own folders.
    assert_eq!(refused(&[("JELLYMOON_BIN_DIR", real.as_path()), ("TMPDIR", real.as_path())], NO_COMMAND), None);
}

#[test]
fn a_value_of_logs_settings_or_repo_outside_the_temporary_folders_is_refused_however_it_is_written() {
    let (dir, real) = (scratch(), outside("repo"));
    let (real, inside) = (real.to_str().unwrap(), dir.path().join("logs"));
    let inside = inside.to_str().unwrap();
    for flag in ["--logs", "--settings", "--repo"] {
        assert_eq!(refused(&[], &["day", flag, real]), Some(flag.to_string()));
        assert_eq!(refused(&[], &["day", "--date", "2026-03-10", &format!("{flag}={real}")]), Some(flag.to_string()));
        assert_eq!(refused(&[], &["day", flag, inside]), None, "{flag} under a temporary folder");
        assert_eq!(refused(&[], &["day", &format!("{flag}={inside}")]), None, "{flag}= under a temporary folder");
        assert_eq!(refused(&[], &["day", flag, "logs"]), None, "{flag} that is not a full path");
        assert_eq!(refused(&[], &["day", flag]), None, "{flag} with no value");
        assert_eq!(refused(&[], &["day", flag, inside, real]), None, "what follows the value is not the value");
    }
    assert_eq!(refused(&[], &["day", "--skip", real]), None, "a flag that names no folder of a person's");
}

#[test]
fn a_world_under_either_temporary_folder_passes_and_so_does_no_home_an_empty_one_or_one_that_is_no_full_path() {
    for dir in [scratch(), system_scratch()] {
        let (there, gone) = (dir.path().join("user"), dir.path().join("not").join("there"));
        fs::create_dir(&there).unwrap();
        for home in [there.as_path(), gone.as_path(), dir.path()] {
            assert_eq!(refused(&[("HOME", home), ("JELLYMOON_HOME", home)], NO_COMMAND), None, "{}", home.display());
        }
    }
    for folder in door::roots() {
        assert_eq!(refused(&[("HOME", folder.as_path())], NO_COMMAND), None, "a temporary folder itself");
    }
    for home in ["", "user", ".", "..", "~"] {
        let vars = [("HOME", Path::new(home)), ("USERPROFILE", Path::new(home)), ("JELLYMOON_HOME", Path::new(home))];
        assert_eq!(refused(&vars, &["day", "--logs", home, "--repo", home]), None, "{home:?}");
    }
    assert_eq!(refused(&[], NONE), None, "no variable at all");
    // The system's temporary folder is also the one every user shares, where there is one.
    let shared = [("HOME", Path::new("/tmp")), ("JELLYMOON_HOME", Path::new("/tmp/jellymoon-f74/jm"))];
    assert_eq!(refused(&shared, NO_COMMAND), None);
    // And such a world is started, as before: `jellymoon` answers with its usage.
    for vars in [&[][..], &[("HOME", OsStr::new(""))], &[("HOME", OsStr::new("user"))]] {
        let ran = jellymoon(vars, NO_COMMAND);
        assert_eq!((ran.code, ran.stdout.as_str()), (2, ""), "{vars:?}: {}", ran.stderr);
        assert!(ran.stderr.contains("usage:"), "{vars:?}: {}", ran.stderr);
    }
}

#[test]
fn a_path_is_resolved_as_far_as_it_is_there_whether_it_is_there_or_not() {
    let (dir, root) = (scratch(), root());
    let home = |path: &Path| refused(&[("HOME", path)], NO_COMMAND);
    assert_eq!(home(&root), Some("HOME".to_string()), "a folder that is there");
    assert_eq!(home(&outside("home")), Some("HOME".to_string()), "one that is not");
    let (gone, up) = (dir.path().join("not-there"), "../".repeat(64));
    assert_eq!(home(&gone.join("user")), None);
    // Out of the test's folder by `..` after a name that is not there, where no folder can be asked.
    let climbed = gone.join(&up).join(MADE_UP);
    assert_eq!(home(&climbed), Some("HOME".to_string()), "{}", climbed.display());
    assert_eq!(refused(&[], &["day", "--repo", climbed.to_str().unwrap()]), Some("--repo".to_string()));
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
    let home = |path: PathBuf| refused(&[("HOME", path.as_path())], NO_COMMAND);
    // A home under a temporary folder that is a link out of it, and a path through such a link that is not there.
    assert_eq!(home(at("out")), Some("HOME".to_string()));
    assert_eq!(home(at("out").join(MADE_UP).join("home")), Some("HOME".to_string()));
    // A link whose end is not there is read, not taken for a folder of the test's own.
    assert!(linked(&outside("home"), &at("dangling")));
    assert_eq!(home(at("dangling")), Some("HOME".to_string()));
    assert_eq!(home(at("dangling").join("deeper")), Some("HOME".to_string()));
    assert_eq!(refused(&[], &["day", "--logs", at("dangling").to_str().unwrap()]), Some("--logs".to_string()));
    // A link that stays inside is a folder inside, and so is one whose end inside is not there.
    assert!(linked(&at("real"), &at("alias")) && linked(&at("real").join("later"), &at("soon")));
    assert_eq!(home(at("alias").join("home")), None);
    assert_eq!(home(at("soon").join("home")), None);
    // The temporary folder's side: a folder named through a link holds what is under the folder it leads to, and
    // the other way round. Nothing else is under it.
    let under = |folder: PathBuf, path: PathBuf| door::refusal(&[folder], &[("HOME", path)], NONE).is_none();
    assert!(under(at("alias"), at("real").join("home")), "the folder is named through a link");
    assert!(under(at("real"), at("alias").join("home")), "the path goes through a link");
    assert!(under(at("alias"), at("alias").join("home")) && under(at("real"), at("real").join("home")));
    assert!(!under(at("real"), at("other").join("home")) && !under(at("alias"), dir.path().to_path_buf()));
    assert!(!under(at("alias"), at("out")), "a link out is outside whatever the folder is named by");
}

/// The value of `JELLYMOON_SHELL` a command would be started with: none when it would not be set at all.
fn shell(command: &std::process::Command) -> Option<String> {
    let set = command.get_envs().filter(|(name, _)| *name == OsStr::new("JELLYMOON_SHELL"));
    set.last().and_then(|(_, value)| value.map(|value| value.to_string_lossy().into_owned()))
}

#[test]
fn every_world_that_is_started_has_the_login_shell_set_to_nothing_and_a_real_one_is_refused() {
    let dir = scratch();
    let made_up = dir.path().join("bin").join("zsh");
    assert_eq!(shell(&door::jellymoon([] as [(&str, &str); 0], NO_COMMAND)).as_deref(), Some(""));
    assert_eq!(shell(&door::jellymoon([("HOME", dir.path())], NO_COMMAND)).as_deref(), Some(""));
    assert_eq!(shell(&door::command("some-program", [("PATH", dir.path())], NONE)).as_deref(), Some(""));
    // A made-up login shell under a temporary folder may be named, and nothing else may.
    let named = door::jellymoon([("JELLYMOON_SHELL", &made_up)], NO_COMMAND);
    assert_eq!(shell(&named).map(PathBuf::from), Some(made_up.clone()));
    let (real, relative, none) = (outside("zsh"), Path::new("zsh"), Path::new(""));
    let named = |shell: &Path| refused(&[("JELLYMOON_SHELL", shell)], NO_COMMAND);
    assert_eq!(named(&real), Some("JELLYMOON_SHELL".to_string()));
    assert_eq!(named(relative), Some("JELLYMOON_SHELL".to_string()));
    assert_eq!((named(none), named(&made_up)), (None, None));

    // And a program that is started sees it: the made-up harness leaves its variables beside itself.
    let harness = dir.path().join("rec").join(format!("claude{}", env::consts::EXE_SUFFIX));
    fs::create_dir(dir.path().join("rec")).unwrap();
    plant(env!("CARGO_BIN_EXE_fake-claude"), &harness);
    fs::write(harness.with_file_name("reply"), "").unwrap();
    fs::write(harness.with_file_name("exit"), "0").unwrap();
    let mut command = door::command(&harness, [("HOME", dir.path())], NONE);
    let ran = command.current_dir(dir.path()).stdin(Stdio::null()).stdout(Stdio::null()).status().unwrap();
    assert!(ran.success());
    let seen = fs::read_to_string(harness.with_file_name("env")).unwrap();
    assert!(seen.lines().any(|line| line == "JELLYMOON_SHELL="), "{seen}");
}

#[test]
fn a_refusal_is_one_fixed_sentence_with_the_name_it_refused_and_nothing_is_started() {
    let (dir, real) = (scratch(), outside("home"));
    let hook = dir.path().join(format!("hook{}", env::consts::EXE_SUFFIX));
    let mark = PathBuf::from(format!("{}.ran", hook.display()));
    plant(env!("CARGO_BIN_EXE_fake-hook"), &hook);
    let start = |home: &Path| {
        let home = home.to_path_buf();
        catch_unwind(AssertUnwindSafe(|| door::command(&hook, [("HOME", home)], NONE).status().unwrap()))
    };
    let why = said(start(&real).expect_err("a home outside the temporary folders was let through"));
    assert_eq!(why, format!("{REFUSED}; this one named HOME"));
    assert!(!why.contains(MADE_UP) && !REFUSED.contains(['/', '\\']), "the sentence names no path: {why}");
    assert!(!mark.exists(), "the program was started");
    // The same program in a world the door passes is started: the mark is how a start is seen.
    assert!(start(dir.path()).is_ok_and(|status| status.success()) && mark.exists());

    // The support's own ways to run `jellymoon` go through the same door.
    let real = real.as_os_str();
    let by_var = catch_unwind(|| jellymoon(&[("JELLYMOON_HOME", real)], NO_COMMAND));
    assert_eq!(said(by_var.err().unwrap()), format!("{REFUSED}; this one named JELLYMOON_HOME"));
    let folder = dir.path();
    let by_flag =
        catch_unwind(|| jellymoon_in(folder, &[], &["f74-no-such-command", "--repo", real.to_str().unwrap()]));
    assert_eq!(said(by_flag.err().unwrap()), format!("{REFUSED}; this one named --repo"));
}
