//! A login shell that is refused starts no harness (F78, the row's second step): the look ends as it ends with no
//! login shell, in the same words, with the login shell and why among the places looked in. Nothing is written,
//! a second command asks again, and whether it answered or not the login shell and what it left in its group are
//! gone when the command returns.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::time::{Duration, Instant};

use crate::shell::{INSTALL, NONE, Shell, finder, found};
use crate::world::World;

const NO_FILE: &str = "it is not a file that can be run";
const OPEN: &str = "others can write to it, or everyone can write to a folder it is in";
const UNLISTED: &str = "it is not zsh or bash";
const NO_MARKS: &str = "it answered with no pair of marks on one line";
const TWO_PAIRS: &str = "it answered with more than one pair of marks";
const TOO_LONG: &str = "it did not answer in 5 seconds";

fn mode(path: &std::path::Path, mode: u32) {
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
}

/// A world whose login shell names a folder with a harness in it, which is the only harness there is.
fn named(name: &str) -> (World, Shell) {
    let mut w = finder();
    let shell = w.shell(name);
    w.reader_in(&w.at("held"));
    shell.names(&[&w.at("held")]);
    (w, shell)
}

/// Fails unless the login shell was refused in these words, by `jellymoon page` and by `jellymoon harness`, and
/// the harness it would have named was never started.
fn refused(w: &World, why: &str, case: &str) {
    assert_eq!(w.no_page(), format!("{NONE}, the login shell ({why}){INSTALL}"), "{case}");
    let says =
        format!("Claude Code was not found on PATH or in a fixed place, and none came of the login shell: {why}");
    assert_eq!(w.harness(), found(false, "none", &says), "{case}");
    assert!(!w.at("held").join("arg0").exists() && !w.at("jm").exists(), "{case}: a harness was started");
}

#[test]
fn a_login_shell_others_can_write_to_or_that_is_no_program_or_not_on_the_list_is_never_started() {
    type Spoil = fn(&World, &Shell);
    let cases: [(&str, &str, Spoil, &str); 8] = [
        ("the group can write to it", "zsh", |_, shell| mode(&shell.program, 0o775), OPEN),
        ("everyone can write to it", "bash", |_, shell| mode(&shell.program, 0o757), OPEN),
        (
            // Its file is in a folder only the test's user can write to, and the link that names it is not.
            "everyone can write to its folder",
            "zsh",
            |w, shell| {
                let real = w.made("closed").join("zsh");
                fs::rename(&shell.program, &real).unwrap();
                std::os::unix::fs::symlink(&real, &shell.program).unwrap();
                mode(&shell.rec, 0o777);
            },
            OPEN,
        ),
        (
            "everyone can write to the folder its file is in",
            "zsh",
            |w, shell| {
                let real = w.made("open").join("zsh");
                fs::rename(&shell.program, &real).unwrap();
                std::os::unix::fs::symlink(&real, &shell.program).unwrap();
                mode(&w.at("open"), 0o777);
            },
            OPEN,
        ),
        ("it cannot be run", "zsh", |_, shell| mode(&shell.program, 0o644), NO_FILE),
        ("it is not there", "zsh", |_, shell| fs::remove_file(&shell.program).unwrap(), NO_FILE),
        (
            "it is a folder",
            "bash",
            |_, shell| {
                fs::remove_file(&shell.program).unwrap();
                fs::create_dir(&shell.program).unwrap();
            },
            NO_FILE,
        ),
        ("it is not on the list", "fish", |_, _| {}, UNLISTED),
    ];
    for (case, name, spoil, why) in cases {
        let (w, shell) = named(name);
        spoil(&w, &shell);
        refused(&w, why, case);
        assert_eq!(shell.starts(), 0, "{case}: the login shell was started");
        // A folder everyone can write to is closed again, so that the world can be removed.
        for folder in [&shell.rec, &w.made("open")] {
            mode(folder, 0o755);
        }
    }
}

#[test]
fn a_login_shell_that_answers_with_no_pair_of_marks_or_more_than_one_names_no_folder_and_is_asked_again() {
    for (script, why) in [("none", NO_MARKS), ("one", NO_MARKS), ("big", NO_MARKS), ("two", TWO_PAIRS)] {
        let (w, shell) = named("zsh");
        shell.play(script);
        refused(&w, why, script);
        // Two commands, `page` and `harness`, and each asked: a failed ask is kept nowhere.
        assert_eq!(shell.starts(), 2, "{script}");
    }
}

#[test]
fn a_login_shell_that_has_not_answered_in_5_seconds_is_killed_with_its_group_and_asked_again() {
    let (w, shell) = named("bash");
    shell.play("silent");
    shell.stays();
    let from = Instant::now();
    assert_eq!(w.no_page(), format!("{NONE}, the login shell ({TOO_LONG}){INSTALL}"));
    let took = from.elapsed();
    assert!(took >= Duration::from_secs(5) && took < Duration::from_secs(15), "the wait was {took:?}");
    assert!(shell.gone(), "the login shell or the program it left in its group outlived the command");
    assert!(!w.at("held").join("arg0").exists(), "a harness was started");
    let says =
        format!("Claude Code was not found on PATH or in a fixed place, and none came of the login shell: {TOO_LONG}");
    assert_eq!(w.harness(), found(false, "none", &says));
    assert!(shell.starts() == 2 && shell.gone());
}

#[test]
fn a_login_shell_that_answers_and_does_not_end_is_killed_with_its_group_as_soon_as_its_line_is_read() {
    let (w, shell) = named("zsh");
    shell.stays();
    let from = Instant::now();
    let ran = w.page();
    assert_eq!((ran.code, ran.stderr.as_str()), (0, ""), "jellymoon page");
    assert!(from.elapsed() < Duration::from_secs(4), "the login shell was waited for: {:?}", from.elapsed());
    assert!(w.at("held").join("arg0").exists(), "the harness it named was not started");
    assert!(shell.gone(), "the login shell or the program it left in its group outlived the command");
}
