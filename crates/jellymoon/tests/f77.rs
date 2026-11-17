//! F77: a harness Jellymoon starts from inside a Claude Code session is not told it is that session's child (ADR
//! 0021, decision 1, "Also here: the harness's own variables"; `docs/development/page.md`, "The call"; `run.md`,
//! "What the harness is started with"). `jellymoon` is run as a black box in F39's test folder (`f39/world.rs`)
//! with the variables a Claude Code session sets on what it starts, and with every variable of both systems'
//! lists. The made-up harness records the names it was started with: `fake-claude` for the reading call,
//! `fake-agent` for a run, an answer, a follow-up and a run started apart. Those names are this system's list and
//! no other, and both lists are held to the two docs on every system.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;
#[path = "f39/world.rs"]
mod world;

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use support::{install, said, started, write_log};
pub use world::*;

/// What a harness is started with on macOS and Linux, and on Windows, as the docs list them.
const UNIX: [&str; 8] = ["PATH", "HOME", "TMPDIR", "USER", "LANG", "LC_ALL", "TERM", "CLAUDE_CONFIG_DIR"];
const WINDOWS: [&str; 14] = [
    "PATH",
    "PATHEXT",
    "SystemRoot",
    "SystemDrive",
    "ComSpec",
    "USERPROFILE",
    "HOMEDRIVE",
    "HOMEPATH",
    "APPDATA",
    "LOCALAPPDATA",
    "TEMP",
    "TMP",
    "USERNAME",
    "CLAUDE_CONFIG_DIR",
];

/// What tells a program it is a Claude Code session's child: the first five as a session sets them, one name no
/// list of such names could hold, and two spelled as Windows would still take them for the first two.
const CHILD: [&str; 8] = [
    "CLAUDECODE",
    "CLAUDE_CODE_ENTRYPOINT",
    "CLAUDE_CODE_SSE_PORT",
    "CLAUDE_CODE_SESSION_ID",
    "CLAUDE_CODE_USE_BEDROCK",
    "CLAUDE_CODE_MADE_UP_FOR_F77",
    "claudecode",
    "Claude_Code_Entrypoint",
];

/// A world whose `jellymoon` is started as from inside a Claude Code session, with every variable of both lists
/// set: the other system's are variables like any other here, and none of them may be passed on.
fn inside_a_session(mut w: World) -> World {
    w.vars.extend(CHILD.map(|name| (name, OsString::from("made-up"))));
    for name in UNIX.into_iter().chain(WINDOWS) {
        if w.vars.iter().any(|(set, _)| *set == name) {
            continue;
        }
        let value = match name {
            "APPDATA" | "LOCALAPPDATA" | "TEMP" | "TMP" => w.at("tmp").into(),
            // Windows' own, as this test has them there. Anywhere else any words will do.
            _ => std::env::var_os(name).filter(|_| cfg!(windows)).unwrap_or_else(|| "made-up".into()),
        };
        w.vars.push((name, value));
    }
    w
}

/// A name as the system tells names apart: Windows takes `Path` and `PATH` for one variable.
fn spelled(name: &str) -> String {
    if cfg!(windows) { name.to_uppercase() } else { name.to_string() }
}

/// Fails unless what a made-up harness recorded of its variables, a name or a `name=value` a line, is this
/// system's list and no other. A line with no name is no variable a program can ask for (Windows keeps each
/// drive's folder in one).
fn only_the_listed(recorded: &str, start: &str) {
    let names = recorded.lines().map(|line| line.split_once('=').map_or(line, |(name, _)| name));
    let mut names: Vec<String> = names.filter(|name| !name.is_empty()).map(spelled).collect();
    names.sort();
    let child = |name: &&String| name.to_uppercase().replace('_', "").starts_with("CLAUDECODE");
    let told: Vec<&String> = names.iter().filter(child).collect();
    assert!(told.is_empty(), "{start}: the harness was told it is a session's child: {told:?}");
    let list: &[&str] = if cfg!(windows) { &WINDOWS } else { &UNIX };
    let mut listed: Vec<String> = list.iter().map(|name| spelled(name)).collect();
    listed.sort();
    assert_eq!(names, listed, "{start}: the variables the harness was started with");
}

/// What the made-up agent recorded at its last start, which was its start number `starts`. It is taken away, so
/// the next start's is its own.
fn recorded(w: &World, starts: usize) -> String {
    assert_eq!(w.starts(), starts, "how often the harness was started");
    let names = w.rec("env").expect("the harness recorded no variables");
    fs::remove_file(w.at("rec").join("env")).unwrap();
    names
}

/// One finished run of a wish for `alpha`, started from inside a session: the world, its repo and the wish.
fn finished() -> (World, PathBuf, String) {
    let w = inside_a_session(world());
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    assert_eq!(ran(&w.start(&wish, &repo)).1["outcome"], "completed");
    (w, repo, wish)
}

/// A follow-up of `wish` is made and run with the made-up agent playing `script`: what the run printed.
fn followed(w: &World, repo: &Path, wish: &str, script: &str) -> Value {
    let more = printed(&w.run(&["wish", "--date", DAY, "--say", "and log each retry", "--after", wish]));
    w.play(script, 0);
    printed(&w.start(more["wish"]["id"].as_str().unwrap(), repo))
}

#[test]
fn the_reading_call_is_started_with_the_listed_variables_and_no_other() {
    let w = inside_a_session(world());
    // The reading call's made-up harness, where a run's stood: first on `PATH`, with an empty reading to give.
    let (bin, logs) = (w.at("bin"), w.at("logs"));
    install(&bin, "claude");
    let reply = json!({"type": "result", "is_error": false, "result": r#"{"wishes":[],"clashes":[]}"#});
    fs::write(bin.join("reply"), reply.to_string()).unwrap();
    fs::write(bin.join("exit"), "0").unwrap();
    write_log(&logs, "-work-alpha/s1.jsonl", &[said("2026-03-10T16:00:00.000Z", "/work/alpha", "add a retry")]);
    let page = w.run(&["page", "--date", DAY, "--logs", logs.to_str().unwrap()]);
    assert_eq!((page.code, page.stderr.as_str()), (0, ""), "jellymoon page");
    assert_eq!(fs::read_to_string(bin.join("arg0")).unwrap(), "-p", "the reading call was made");
    only_the_listed(&fs::read_to_string(bin.join("env")).unwrap(), "the reading call");
}

#[test]
fn a_run_is_started_with_the_listed_variables_and_no_other() {
    let (w, ..) = finished();
    only_the_listed(&recorded(&w, 1), "a run");
}

#[test]
fn an_answer_is_started_with_the_listed_variables_and_no_other() {
    let w = inside_a_session(world());
    w.play("asks", 0);
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    assert_eq!(printed(&w.start(&wish, &repo))["outcome"], "asks");
    only_the_listed(&recorded(&w, 1), "a run that asks");
    w.play("edits", 0);
    let vars: Vec<_> = w.vars.iter().map(|(name, value)| (*name, value.as_os_str())).collect();
    let answered = started(w.dir.path(), &vars, &["run", "answer", "--wish", &wish, "--date", DAY], "the old one\n");
    assert_eq!(printed(&answered)["outcome"], "completed");
    let mut args = (0..).map_while(|n| w.rec(&format!("arg{n}")));
    assert!(args.any(|arg| arg == "--resume"), "the second start went on with the run's session");
    only_the_listed(&recorded(&w, 2), "an answer");
}

#[test]
fn a_follow_up_is_started_with_the_listed_variables_and_no_other() {
    let (w, repo, wish) = finished();
    recorded(&w, 1);
    assert_eq!(followed(&w, &repo, &wish, "follow")["outcome"], "completed");
    assert!(w.rec("line2").unwrap().contains("--fork-session"), "the second start was a fork");
    only_the_listed(&recorded(&w, 2), "a follow-up's fork");
}

// A harness that keeps the followed session no more is started a second time, under a new session.
#[test]
fn a_follow_up_started_again_under_a_new_session_is_started_with_them_too() {
    let (w, repo, wish) = finished();
    recorded(&w, 1);
    assert_eq!(followed(&w, &repo, &wish, "unkept")["outcome"], "completed");
    only_the_listed(&recorded(&w, 3), "a follow-up started again");
}

// `jellymoon wish --run` starts `jellymoon run` apart, with all of its own variables; that command's harness is
// started with the list all the same.
#[test]
fn a_run_started_apart_is_started_with_the_listed_variables_and_no_other() {
    let mut w = inside_a_session(world_in(support::system_scratch()));
    w.vars.push(("JELLYMOON_TEST_APART_QUIET_MS", "60000".into()));
    let folder = w.repo("alpha");
    let typed = said("2026-03-10T16:00:00.000Z", folder.to_str().unwrap(), "look at the uploader");
    write_log(&w.at("harness").join("projects"), "-work-alpha/s.jsonl", &[typed]);
    let made = printed(&w.run(&["wish", "--date", DAY, "--say", SAID, "--project", "alpha", "--run"]));
    let run = w.jm("runs").join(made["run"].as_str().expect("no run was started")).join("run.json");
    let ended = || {
        let record = serde_json::from_str::<Value>(&fs::read_to_string(&run).unwrap_or_default());
        record.is_ok_and(|record| !record["outcome"].is_null())
    };
    let from = Instant::now();
    while !ended() {
        assert!(from.elapsed() < Duration::from_secs(30), "the run started apart never ended");
        std::thread::sleep(Duration::from_millis(10));
    }
    only_the_listed(&recorded(&w, 1), "a run started apart");
}

/// The words a doc lists variables in: each in backticks, the last after "and".
fn in_words(list: &[&str]) -> String {
    let (last, rest) = list.split_last().unwrap();
    let rest: Vec<String> = rest.iter().map(|name| format!("`{name}`")).collect();
    format!("{} and `{last}`", rest.join(", "))
}

// A variable added to a list fails the tests above until it is in the lists here, and fails this one until both
// docs list it.
#[test]
fn both_docs_list_the_variables_of_each_system_and_say_what_is_never_passed() {
    let docs = [
        ("page.md", include_str!("../../../docs/development/page.md")),
        ("run.md", include_str!("../../../docs/development/run.md")),
    ];
    for (name, doc) in docs {
        let doc = doc.split_whitespace().collect::<Vec<_>>().join(" ");
        for list in [&UNIX[..], &WINDOWS[..]] {
            assert!(doc.contains(&in_words(list)), "{name} does not list {}", in_words(list));
        }
        let never = "no `CLAUDECODE` and no `CLAUDE_CODE_*` variable";
        assert!(doc.contains(never), "{name} does not say: {never}");
    }
}
