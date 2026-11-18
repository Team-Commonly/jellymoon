//! F78: Claude Code is found when the app is opened from the Finder, and `jellymoon harness` says how (ADR 0021,
//! decision 1; `docs/development/page.md`, "Where the harness is looked for"). `jellymoon` is run as a black box
//! in F39's test folder, through the door, with no harness on `PATH` or in a fixed place. The login shell is the
//! made-up one of `support/fake_shell.rs`, named with `JELLYMOON_SHELL`: no real login shell is started and no
//! record of a user is read, here or anywhere in the tests. This file holds a harness that is found and when no
//! login shell is started; `f78/refused.rs` a login shell that is refused, or does not end; `f78/checks.rs` the
//! checks of a harness in a folder it named. A login shell that is not named by a full path is refused by the
//! door itself, so that one rule is held by a test in core (`find/shell/ask.rs`).
#![allow(clippy::unwrap_used, clippy::expect_used)]

#[cfg(unix)]
#[path = "f78/checks.rs"]
mod checks;
#[cfg(unix)]
#[path = "f55/heard.rs"]
mod heard;
#[cfg(unix)]
#[path = "f78/refused.rs"]
mod refused;
#[path = "f78/shell.rs"]
mod shell;
mod support;
#[path = "f39/world.rs"]
mod world;

#[cfg(unix)]
pub use heard::*;
pub use shell::*;
pub use world::*;

#[cfg(unix)]
const BY_SHELL: &str =
    "Claude Code was found in a folder of the login shell's PATH, as PATH and the fixed places have none";
const NOT_ASKED: &str = "Claude Code was not found on PATH or in a fixed place, and the login shell was not asked, \
                         as JELLYMOON_SHELL is set to nothing";
const NOT_ON_WINDOWS: &str =
    "Claude Code was not found on PATH or in a fixed place, and on Windows the login shell is not asked";

#[cfg(unix)]
#[test]
fn page_starts_the_harness_in_a_folder_the_login_shell_names_and_nothing_outside_the_marks_counts() {
    use std::fs;
    for name in ["zsh", "bash"] {
        let mut w = finder();
        let shell = w.shell(name);
        let (empty, held, outside) = (w.made("empty"), w.at("held"), w.at("outside"));
        // A folder that is not named by a full path is not looked in, wherever the command is started.
        for folder in [&held, &outside, &w.at("not").join("full")] {
            w.reader_in(folder);
        }
        shell.names(&[std::path::Path::new("not/full"), &empty, &held]);
        fs::write(shell.rec.join("before"), format!("Last login: a while ago\n{}\n", outside.display())).unwrap();
        fs::write(shell.rec.join("after"), format!("{}\n", outside.display())).unwrap();
        let ran = w.page();
        assert_eq!((ran.code, ran.stderr.as_str()), (0, ""), "{name}: jellymoon page");
        let ran_in = |folder: &std::path::Path| folder.join("arg0").exists();
        assert!(ran_in(&held) && !ran_in(&outside) && !ran_in(&w.at("not").join("full")), "{name}");

        // The login shell was asked once: interactive, as a login shell, with the one fixed command, nothing on
        // its input, the variables a harness is started with, and in no folder of the person's.
        assert_eq!(shell.said("args"), format!("-l\n-i\n-c\n{COMMAND}"), "{name}");
        assert_eq!((shell.said("stdin").as_str(), shell.said("cwd").as_str(), shell.starts()), ("", "/", 1));
        let allowed = ["PATH", "HOME", "TMPDIR", "USER", "LANG", "LC_ALL", "TERM", "CLAUDE_CONFIG_DIR"];
        assert!(shell.said("env").lines().all(|name| allowed.contains(&name)), "{}", shell.said("env"));
        // The harness is started with this process's PATH, then the folder it was found in, then the fixed places.
        let user = w.at("user");
        let want = [w.at("git"), held.clone(), user.join(".local").join("bin"), user.join(".claude").join("local")];
        assert_eq!(path_of(&fs::read_to_string(held.join("env")).unwrap()), want, "{name}");

        // `jellymoon harness` says how, and names no folder; it is a new command, so it asks again.
        let said = w.harness();
        assert_eq!(said, found(true, "login_shell", BY_SHELL), "{name}");
        assert!(!said.contains("held") && !said.contains(w.dir.path().to_str().unwrap()), "{said}");
        assert_eq!(shell.starts(), 2, "{name}: the answer is kept in no file");
    }
}

#[cfg(unix)]
#[test]
fn run_starts_the_harness_in_a_folder_the_login_shell_names() {
    let mut w = finder();
    let shell = w.shell("zsh");
    let held = w.at("held");
    w.agent_in(&held);
    shell.names(&[&w.made("empty"), &held]);
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    assert_eq!(shell.starts(), 0, "a wish looks for no harness");
    let (_, said) = ran(&w.start(&wish, &repo));
    assert_eq!((&said["outcome"], w.starts(), shell.starts()), (&serde_json::json!("completed"), 1, 1));
}

// With a harness on `PATH`, in a fixed place or named, the login shell that would name another is never started.
#[cfg(unix)]
#[test]
fn no_login_shell_is_started_when_a_harness_is_named_on_path_or_in_a_fixed_place() {
    let places = |w: &World| [w.at("named"), w.at("git"), w.at("user").join(".local").join("bin")];
    let says = [
        ("named", "Claude Code is the program JELLYMOON_CLAUDE names, and was not looked for"),
        ("path", "Claude Code was found on PATH"),
        ("fixed_place", "Claude Code was found in a fixed place, as PATH has none"),
    ];
    for (at, (how, says)) in says.into_iter().enumerate() {
        let mut w = finder();
        let shell = w.shell("zsh");
        let (place, held) = (places(&w)[at].clone(), w.at("held"));
        w.reader_in(&held);
        shell.names(&[&held]);
        let program = w.reader_in(&place);
        if how == "named" {
            w.vars.push(("JELLYMOON_CLAUDE", program.into()));
        }
        let ran = w.page();
        assert_eq!((ran.code, ran.stderr.as_str()), (0, ""), "{how}: jellymoon page");
        assert!(place.join("arg0").exists() && !held.join("arg0").exists(), "{how}");
        assert_eq!(w.harness(), found(true, how, says));
        assert_eq!(shell.starts(), 0, "{how}: the login shell was started");
    }
}

// On every system: with `JELLYMOON_SHELL` set to nothing, as the door sets it, the look ends as it did before
// there was a login shell to ask, in the same words. On Windows it is not asked though one is named.
#[test]
fn with_no_login_shell_to_ask_the_look_ends_as_it_did_and_harness_says_it_was_not_asked() {
    let mut w = finder();
    assert_eq!(w.no_page(), format!("{NONE}{INSTALL}"));
    assert_eq!(w.harness(), found(false, "none", if cfg!(windows) { NOT_ON_WINDOWS } else { NOT_ASKED }));
    if cfg!(windows) {
        let shell = w.shell("zsh");
        w.reader_in(&w.at("held"));
        shell.names(&[&w.at("held")]);
        assert_eq!(w.no_page(), format!("{NONE}{INSTALL}"));
        assert_eq!((w.harness(), shell.starts()), (found(false, "none", NOT_ON_WINDOWS), 0));
    }
}

// What is remembered is in the process's memory: a hearing that makes two wishes, each with its run asked for,
// looks for the harness twice. An answer with a harness in it is kept for the second look: the hearing asks once,
// and each of the two runs it starts, a command of its own, asks once. An answer with no harness in it is not
// kept, so a harness installed while the command goes is found by its next look; and an ask that failed is made
// again.
#[cfg(unix)]
#[test]
fn one_command_asks_once_when_the_answer_held_a_harness_and_again_when_it_held_none_or_failed() {
    for (script, harness, asked) in [("answer", true, 3), ("answer", false, 2), ("none", false, 2)] {
        let mut w = heard_world();
        let repo = w.repo("alpha");
        w.typed_at("-work-alpha", &repo);
        w.vars.push(("PATH", w.at("git").into()));
        let shell = w.shell("bash");
        shell.play(script);
        shell.names(&[&w.made("empty"), &w.at("held")]);
        if harness {
            w.agent_in(&w.at("held"));
        }
        let events = [hold(0, SAID), vec![tick(5000)], hold(6000, "and log each retry"), vec![tick(11_000)]].concat();
        let printed = w.hear_ran(&["--run"], &events);
        assert_eq!(printed.code, 0, "{}", printed.stderr);
        assert_eq!(printed.stdout.matches("\"made\":").count(), 2, "{script}: {}", printed.stdout);
        if harness {
            for wish in ["m1", "m2"] {
                w.run_ended(wish);
            }
            assert_eq!(w.starts(), 2, "a run of each wish");
        }
        assert_eq!(shell.starts(), asked, "{script}, a harness there: {harness}");
    }
}

// The gate of F78, finding 1: on Linux a planted program is a hard link to the built one, which every test starts.
// So a test that wants a mode of its own for a file makes a copy of it first. Held on every Unix, with a link
// made here: on a Mac `plant` copies, and the mistake would not show.
#[cfg(unix)]
#[test]
fn a_mode_set_for_a_test_is_set_on_a_copy_and_never_on_the_file_a_link_shares() {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    let w = finder();
    let (built, planted) = (w.reader_in(&w.at("built")), w.made("planted").join("claude"));
    std::fs::hard_link(&built, &planted).unwrap();
    let before = std::fs::read(&built).unwrap();
    moded(&planted, 0o757);
    let (built, planted) = (std::fs::metadata(&built).unwrap(), std::fs::metadata(&planted).unwrap());
    assert_eq!((built.permissions().mode() & 0o777, planted.permissions().mode() & 0o777), (0o755, 0o757));
    assert!(built.ino() != planted.ino() && std::fs::read(w.at("planted").join("claude")).unwrap() == before);
}
