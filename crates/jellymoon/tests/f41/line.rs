//! The command a run is started with (`docs/development/run.md`, "A run started apart"): named by a full path, in
//! a process group of its own, with the arguments its asker was given and no others. And what an asker is told of
//! a command that started no run and said why: one clean line.

use std::ffi::OsString;
use std::fs;
use std::process::Command;

use super::{World, found_world, silent_world, wait_for};
use crate::support::{hub_key, shows, skipped};

/// The pid, the process group and the arguments of the `jellymoon run` that started the made-up agent, as the
/// system's own `ps` has them while the agent waits.
fn command_of(w: &World) -> (String, String, String) {
    wait_for(&w.at("rec").join("parent"));
    let pid = fs::read_to_string(w.at("rec").join("parent")).unwrap();
    let ps = Command::new("ps").args(["-o", "pid=,pgid=,args=", "-p", &pid]).output().unwrap();
    let line = String::from_utf8(ps.stdout).unwrap();
    let mut words = line.split_whitespace().map(str::to_string);
    let (pid, group) = (words.next().unwrap(), words.next().unwrap());
    (pid, group, words.collect::<Vec<_>>().join(" "))
}

/// A world where `wish --run` has started a run whose agent waits, with more arguments: the started command.
fn started_with(more: &[&str]) -> (String, String, String) {
    let w = silent_world();
    w.typed_in("-work-alpha", &w.repo("alpha"));
    w.play("slow", 0);
    let said = w.wish_run("alpha", more);
    let run = said["run"].as_str().unwrap_or_else(|| panic!("no run was started: {said}")).to_string();
    let command = command_of(&w);
    w.let_end("go");
    assert_eq!(w.ended_as(&run)["outcome"], "completed");
    w.command_gone(&run);
    command
}

#[test]
fn the_started_command_has_a_group_of_its_own_and_takes_skip_only_when_a_name_is_switched_off() {
    if !cfg!(unix) {
        return skipped("the command's line and its group are read with ps");
    }
    let (pid, group, args) = started_with(&[]);
    assert_eq!(group, pid, "the started command is in its asker's process group: {args}");
    assert!(args.contains(" run --wish m1 ") && !args.contains("--skip"), "{args}");

    let (_, _, args) = started_with(&["--skip", "beta,gamma"]);
    assert!(args.ends_with(" --skip beta,gamma"), "{args}");
}

#[test]
fn a_jellymoon_command_not_named_by_a_full_path_is_never_started() {
    let w = found_world();
    w.typed_in("-work-alpha", &w.repo("alpha"));
    let wish = w.wish("alpha");
    let asked = w.asked_with(&[("FAKE_ASKER_PROGRAM", OsString::from("fake-asker"))], &wish);
    let why = "nothing was started: the jellymoon command is not named by a full path";
    assert_eq!((asked.stdout.as_str(), asked.code), (format!("refused\n{why}\n").as_str(), 0), "{}", asked.stderr);
    assert!(!w.at("rec").join("exit").exists(), "a program found by its name alone was started");
    w.nothing_was_started(&wish, "alpha");
}

#[test]
fn what_a_command_that_started_no_run_said_reaches_its_asker_as_one_clean_line() {
    let w = found_world();
    w.typed_in("-work-alpha", &w.repo("alpha"));
    let wish = w.wish("alpha");
    let key = hub_key();
    let says = format!("jellymoon: \u{1b}[31mno luck\u{1b}[0m with {key} {}\nand a second line", "so ".repeat(200));
    let asked = w.asked_with(&[("FAKE_ASKER_SAYS", OsString::from(says))], &wish);
    let (run, why) = asked.stdout.split_once('\n').unwrap();
    let why = why.strip_suffix('\n').unwrap();
    assert_eq!((run, asked.code), ("run none", 0), "{}", asked.stderr);
    assert!(why.starts_with("no luck with [redacted] so so ") && why.ends_with('…'), "{why}");
    assert!(!why.contains(['\u{1b}', '\n']) && !shows(why, &key) && !why.contains("second"), "{why}");
    assert_eq!(why.chars().count(), 300, "the line is cut to 300 characters, its mark with them: {why}");
    w.nothing_was_started(&wish, "alpha");
}
