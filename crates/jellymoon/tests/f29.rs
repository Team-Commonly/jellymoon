//! F29: a ruling made on a page reaches a running Claude Code session once, at its next step (ADR 0011, section 5).
//! The built binary is run as a black box. The settings file, Jellymoon's home and every hook input are made up here,
//! in a folder of the test's own. Only the last test runs the real Claude Code, and only when a person asks it to.
#![allow(clippy::unwrap_used, clippy::expect_used)]

#[path = "f29/edges.rs"]
mod edges;
#[path = "f29/removal.rs"]
mod removal;
mod support;

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};
use std::{env, fs, thread};

use serde_json::{Value, json};
use support::{Ran, hub_key, jellymoon_fed, scratch};
use tempfile::TempDir;

const STANDS: &str = "never retry uploads, fail fast";
const OVER: &str = "add a retry to the uploader";

/// One test's folder: `harness/settings.json` stands in for Claude Code's settings, `jm` is Jellymoon's home.
struct World {
    dir: TempDir,
    vars: Vec<(&'static str, OsString)>,
}

fn world() -> World {
    let dir = scratch();
    let at = |name: &str| dir.path().join(name);
    fs::create_dir(at("harness")).unwrap();
    fs::create_dir(at("user")).unwrap();
    let vars =
        vec![("HOME", at("user").into()), ("USERPROFILE", at("user").into()), ("JELLYMOON_HOME", at("jm").into())];
    World { dir, vars }
}

/// What Claude Code gives a hook on its input: these fields and more.
fn step(session: &str, event: &str) -> String {
    json!({"session_id": session, "transcript_path": "/made/up.jsonl", "cwd": "/work/alpha", "hook_event_name": event,
        "tool_name": "Read", "tool_input": {"file_path": "a.txt"}, "tool_response": {}})
    .to_string()
}

impl World {
    fn settings(&self) -> PathBuf {
        self.dir.path().join("harness").join("settings.json")
    }

    fn fed(&self, args: &[&str], input: &str) -> Ran {
        let vars: Vec<_> = self.vars.iter().map(|(name, value)| (*name, value.as_os_str())).collect();
        jellymoon_fed(&vars, args, input)
    }

    /// Runs a command that must work, and decodes the one line it prints.
    fn ok(&self, args: &[&str]) -> Value {
        let ran = self.fed(args, "");
        assert_eq!((ran.code, ran.stderr.as_str()), (0, ""), "jellymoon {args:?}");
        assert!(ran.stdout.ends_with("}\n") && ran.stdout.matches('\n').count() == 1, "{:?}", ran.stdout);
        serde_json::from_str(&ran.stdout).unwrap()
    }

    fn install(&self) -> Ran {
        self.fed(&["hook", "install", "--settings", self.settings().to_str().unwrap()], "")
    }

    /// One `PostToolUse` step of a session: what the hook hands it, "" when nothing.
    fn told(&self, session: &str) -> String {
        context(&self.fed(&["hook"], &step(session, "PostToolUse")), "PostToolUse")
    }
}

/// The context a hook run adds for the model: "" when it printed nothing. Whatever it printed, it exited 0, said
/// nothing on stderr, and printed either nothing or the one object Claude Code's hook contract names.
fn context(ran: &Ran, event: &str) -> String {
    assert_eq!((ran.code, ran.stderr.as_str()), (0, ""), "jellymoon hook");
    if ran.stdout.is_empty() {
        return String::new();
    }
    let mut printed: Value = serde_json::from_str(&ran.stdout).expect("the hook prints one JSON object");
    let context = printed["hookSpecificOutput"]["additionalContext"].take();
    assert_eq!(printed, json!({"hookSpecificOutput": {"hookEventName": event, "additionalContext": null}}));
    context.as_str().unwrap().to_string()
}

/// The entry `install` adds: this program, run with no shell, so the same entry works on Windows.
fn entry() -> Value {
    json!({"hooks": [{"type": "command", "command": env!("CARGO_BIN_EXE_jellymoon"), "args": ["hook"]}]})
}

/// The command of the entry at the head of the `PostToolUse` list is this program, wherever a link to it points.
fn is_ours(group: &Value) {
    let mut group = group.clone();
    let command = group["hooks"][0]["command"].take();
    let same = |path: &str| Path::new(path).canonicalize().unwrap();
    assert_eq!(same(command.as_str().unwrap()), same(env!("CARGO_BIN_EXE_jellymoon")));
    let mut rest = entry();
    rest["hooks"][0]["command"].take();
    assert_eq!(group, rest);
}

/// Settings a person could have, each with what `PostToolUse` held before. The first has comments, its own key order
/// and Windows line ends; the others cover each place the entry can go.
const SETTINGS: &[(&str, usize)] = &[
    (
        "// mine\r\n{\r\n  \"model\": \"opus\", /* keep \"hooks\": {} */\r\n  \"hooks\": {\r\n    \"Stop\": [],\r\n    \
         \"PostToolUse\": [\r\n      {\"matcher\": \"Edit|Write\", \"hooks\": [{\"type\": \"command\", \"command\": \
         \"fmt // not a comment\"}]}\r\n    ]\r\n  },\r\n  \"env\": {\"A\": \"1\"}\r\n}\r\n",
        1,
    ),
    ("{\n  \"permissions\": {\"allow\": [\"Read\"]},\n  \"model\": \"sonnet\"\n}", 0),
    ("{\"zeta\":1,\"hooks\":{\"PreToolUse\":[{\"hooks\":[]}]},\"alpha\":[\"{\",\"hooks\"]}\n", 0),
    ("{ \"hooks\" : { } }\n\n", 0),
    ("\t{\"hooks\":{\"PostToolUse\":[ ]}}", 0),
    ("{}\n", 0),
    ("{ }", 0),
    ("", 0),
    (" \n", 0),
];

#[test]
fn install_adds_one_entry_and_leaves_every_other_byte_and_uninstall_restores_the_file_exactly() {
    for (before, held) in SETTINGS {
        let w = world();
        fs::write(w.settings(), before).unwrap();
        assert_eq!(
            w.ok(&["hook", "install", "--settings", w.settings().to_str().unwrap()]),
            json!({"installed": true, "changed": true})
        );
        let after = fs::read_to_string(w.settings()).unwrap();
        // Every byte of the file is still there, in order, around one piece of new text.
        let head = before.bytes().zip(after.bytes()).take_while(|(a, b)| a == b).count();
        let tail = before.bytes().rev().zip(after.bytes().rev()).take_while(|(a, b)| a == b).count();
        assert!(head + tail >= before.len() && after.len() > before.len(), "{before:?} became {after:?}");
        if !before.contains("//") {
            let settings: Value = serde_json::from_str(&after).expect("the settings are still JSON");
            let groups = settings["hooks"]["PostToolUse"].as_array().unwrap();
            assert_eq!(groups.len(), held + 1, "{after}");
            is_ours(&groups[0]);
        }
        assert_eq!(after.matches("\"args\":[\"hook\"]").count(), 1);
        assert_eq!(
            w.ok(&["hook", "install", "--settings", w.settings().to_str().unwrap()]),
            json!({"installed": true, "changed": false})
        );
        assert_eq!(fs::read_to_string(w.settings()).unwrap(), after, "a second install changes nothing");
        assert_eq!(w.ok(&["hook", "uninstall"]), json!({"installed": false, "changed": true}));
        assert_eq!(fs::read_to_string(w.settings()).unwrap(), *before, "uninstall restores the file exactly");
        assert_eq!(w.ok(&["hook", "uninstall"]), json!({"installed": false, "changed": false}));
    }
}

#[test]
fn a_missing_settings_file_is_made_and_removed_again_and_the_default_is_claude_codes_own_folder() {
    let mut w = world();
    w.vars.push(("CLAUDE_CONFIG_DIR", w.dir.path().join("harness").into()));
    assert_eq!(w.ok(&["hook", "install"]), json!({"installed": true, "changed": true}));
    let made = fs::read_to_string(w.settings()).unwrap();
    let settings: Value = serde_json::from_str(&made).unwrap();
    is_ours(&settings["hooks"]["PostToolUse"][0]);
    assert!(made.ends_with("}\n") && settings.as_object().unwrap().len() == 1, "{made:?}");
    assert_eq!(w.ok(&["hook", "uninstall"]), json!({"installed": false, "changed": true}));
    assert!(!w.settings().exists(), "the file install made is removed");
}

#[test]
fn settings_that_are_not_a_json_object_are_refused_and_left_as_they_were() {
    for broken in [
        "{\"model\": \"opus\",",
        "[1, 2]",
        "nonsense",
        "{\"hooks\": []}",
        "{\"hooks\": {\"PostToolUse\": {}}}",
        "{\"a\": 1,}",
    ] {
        let w = world();
        fs::write(w.settings(), broken).unwrap();
        let ran = w.install();
        assert_eq!((ran.code, ran.stdout.as_str()), (1, ""), "{broken}");
        assert!(ran.stderr.starts_with("jellymoon: the harness settings file is not a JSON object"), "{}", ran.stderr);
        assert_eq!(fs::read_to_string(w.settings()).unwrap(), broken);
        assert_eq!(w.ok(&["hook", "uninstall"]), json!({"installed": false, "changed": false}));
    }
}

#[test]
fn uninstall_keeps_what_the_person_changed_since_and_refuses_when_they_changed_the_entry() {
    let w = world();
    fs::write(w.settings(), "{\n  \"model\": \"opus\"\n}\n").unwrap();
    assert_eq!(w.install().code, 0);
    let installed = fs::read_to_string(w.settings()).unwrap();
    fs::write(w.settings(), installed.replace("\"opus\"\n", "\"sonnet\",\n  \"env\": {}\n")).unwrap();
    w.ok(&["hook", "uninstall"]);
    assert_eq!(fs::read_to_string(w.settings()).unwrap(), "{\n  \"model\": \"sonnet\",\n  \"env\": {}\n}\n");

    assert_eq!(w.install().code, 0);
    let edited = fs::read_to_string(w.settings()).unwrap().replace("[\"hook\"]", "[\"hook\", \"-v\"]");
    fs::write(w.settings(), &edited).unwrap();
    let ran = w.fed(&["hook", "uninstall"], "");
    assert_eq!((ran.code, ran.stdout.as_str()), (1, ""));
    assert!(ran.stderr.contains("remove it by hand"), "{}", ran.stderr);
    assert_eq!(fs::read_to_string(w.settings()).unwrap(), edited);
}

#[test]
fn with_no_ruling_waiting_the_hook_prints_nothing_and_exits_0() {
    let w = world();
    for input in [step("s-1", "PostToolUse"), String::new(), "not json".to_string()] {
        let ran = w.fed(&["hook"], &input);
        assert_eq!((ran.code, ran.stdout.as_str(), ran.stderr.as_str()), (0, "", ""), "{input}");
    }
    assert!(!w.dir.path().join("jm").exists(), "a hook with nothing to do writes nothing");
}

#[test]
fn a_ruling_is_handed_to_each_running_session_exactly_once_as_the_persons_newer_decision() {
    let w = world();
    let ruling = w.ok(&["rule", "--stands", STANDS, "--over", OVER]);
    assert_eq!((ruling["stands"].as_str(), ruling["over"].as_str()), (Some(STANDS), Some(OVER)));

    let first = w.told("session-one");
    for part in ["newer decision", "replaces what they said before", STANDS, OVER] {
        assert!(first.contains(part), "{part:?} is not in {first:?}");
    }
    assert!(first.find(STANDS) < first.find(OVER), "what stands is named first");
    assert_eq!(w.told("session-one"), "", "a session is told once");
    assert_eq!(w.told("session-two"), first, "every session is told");
    assert_eq!(w.told("session-two"), "");

    thread::sleep(Duration::from_millis(5));
    w.ok(&["rule", "--stands", "keep the old file name", "--over", "rename the file"]);
    let second = w.told("session-one");
    assert!(second.contains("keep the old file name") && !second.contains(STANDS), "only the new ruling: {second}");
    let late = w.told("session-three");
    assert!(late.find(STANDS) < late.find("keep the old file name"), "oldest first: {late}");
}

#[test]
fn the_installed_entry_run_with_no_shell_hands_the_ruling_over() {
    let w = world();
    fs::write(w.settings(), "{}").unwrap();
    assert_eq!(w.install().code, 0);
    w.ok(&["rule", "--stands", STANDS, "--over", OVER]);
    let settings: Value = serde_json::from_str(&fs::read_to_string(w.settings()).unwrap()).unwrap();
    let handler = &settings["hooks"]["PostToolUse"][0]["hooks"][0];
    let args: Vec<&str> = handler["args"].as_array().unwrap().iter().map(|arg| arg.as_str().unwrap()).collect();
    let input = w.dir.path().join("input");
    fs::write(&input, step("s-1", "PostToolUse")).unwrap();
    let mut command = Command::new(handler["command"].as_str().unwrap());
    command.args(args).env_clear().envs(w.vars.iter().map(|(name, value)| (*name, value.as_os_str())));
    if let Some(root) = env::var_os("SystemRoot").filter(|_| cfg!(windows)) {
        command.env("SystemRoot", root);
    }
    let out = command.stdin(fs::File::open(&input).unwrap()).output().unwrap();
    assert!(out.status.success() && String::from_utf8(out.stdout).unwrap().contains(STANDS));
}

#[test]
fn two_hooks_of_one_session_that_race_hand_the_ruling_over_once_between_them() {
    let w = world();
    w.ok(&["rule", "--stands", STANDS, "--over", OVER]);
    let told: Vec<String> = thread::scope(|scope| {
        let racers: Vec<_> = (0..8).map(|_| scope.spawn(|| w.told("session-one"))).collect();
        racers.into_iter().map(|racer| racer.join().unwrap()).collect()
    });
    assert_eq!(told.iter().filter(|told| told.contains(STANDS)).count(), 1, "{told:?}");
    assert_eq!(told.iter().filter(|told| told.is_empty()).count(), 7, "{told:?}");
}

#[test]
fn a_credential_in_a_ruling_is_replaced_before_it_is_kept_or_handed_over() {
    let w = world();
    let stands = format!("deploy with {} from now on", hub_key());
    let ruling = w.ok(&["rule", "--stands", &stands, "--over", &"x".repeat(900)]);
    assert_eq!(ruling["stands"], "deploy with [redacted] from now on");
    assert_eq!(ruling["over"].as_str().unwrap().chars().count(), 200, "a statement is cut as a quote is");
    let kept = fs::read_dir(w.dir.path().join("jm").join("rulings")).unwrap().map(|entry| entry.unwrap().path());
    let kept: Vec<String> = kept.map(|path| fs::read_to_string(path).unwrap()).collect();
    let told = w.told("s-1");
    assert!(told.contains("deploy with [redacted] from now on"), "{told}");
    assert!(!format!("{ruling}{kept:?}{told}").contains(&hub_key()), "the key is nowhere");

    for args in [&["rule", "--stands", STANDS][..], &["rule", "--stands", " ", "--over", OVER]] {
        let ran = w.fed(args, "");
        assert_eq!((ran.code, ran.stdout.as_str()), (1, ""), "{args:?}");
        assert!(ran.stderr.starts_with("jellymoon: a ruling needs"), "{}", ran.stderr);
    }
    assert_eq!(w.fed(&["rule", "--bogus"], "").code, 2);
}

#[test]
fn input_that_is_not_a_hook_step_is_told_nothing_and_the_ruling_still_waits() {
    let w = world();
    w.ok(&["rule", "--stands", STANDS, "--over", OVER]);
    let listed = || fs::read_dir(w.dir.path().join("jm").join("rulings")).unwrap().count();
    for input in [step("../../escape", "PostToolUse"), step("", "PostToolUse"), step("s-1", "Stop"), "{".to_string()] {
        assert_eq!(context(&w.fed(&["hook"], &input), ""), "", "{input}");
    }
    assert_eq!(listed(), 1, "nothing was marked as told, and no file was made from a session's name");
    for event in ["UserPromptSubmit", "PreToolUse"] {
        let told = context(&w.fed(&["hook"], &step(&format!("s-{event}"), event)), event);
        assert!(told.contains(STANDS), "{event}: {told}");
    }
}

/// The harness-real step. It runs the real Claude Code, signed in as whoever runs it, so it is ignored unless asked for:
///
/// ```text
/// JELLYMOON_HARNESS_REAL=1 cargo test -p jellymoon --test f29 -- --ignored harness_real --nocapture
/// ```
///
/// The turn runs in a folder of the test's own, with the hook installed in a settings file there (passed with
/// `--settings`), never in the person's. It is told to read a file and then write RED; the ruling, made once the turn
/// has started, says to write BLUE. `JELLYMOON_CLAUDE` names the program when it is not `claude` on `PATH`.
#[test]
#[ignore = "runs the real Claude Code; see the comment above"]
fn harness_real_a_turn_told_to_do_one_thing_then_sent_a_ruling_to_do_another_does_the_other() {
    assert_eq!(env::var("JELLYMOON_HARNESS_REAL").as_deref(), Ok("1"), "set JELLYMOON_HARNESS_REAL=1 to run this");
    let w = world();
    let work = w.dir.path().join("work");
    fs::create_dir(&work).unwrap();
    fs::write(work.join("notes.txt"), "nothing to see\n").unwrap();
    fs::write(w.settings(), "{}\n").unwrap();
    assert_eq!(w.install().code, 0);
    let prompt = "First read the file notes.txt. After that, write the single word RED to a new file answer.txt.";
    let mut claude = Command::new(env::var_os("JELLYMOON_CLAUDE").unwrap_or_else(|| "claude".into()));
    claude.args(["-p", prompt, "--settings", w.settings().to_str().unwrap(), "--allowedTools", "Read,Write"]);
    let mut turn = claude.current_dir(&work).env("JELLYMOON_HOME", w.dir.path().join("jm")).spawn().unwrap();
    w.ok(&[
        "rule",
        "--stands",
        "write the single word BLUE to answer.txt",
        "--over",
        "write the single word RED to answer.txt",
    ]);
    let deadline = Instant::now() + Duration::from_secs(300);
    while turn.try_wait().unwrap().is_none() {
        if Instant::now() > deadline {
            turn.kill().unwrap();
            panic!("the turn was still running after five minutes");
        }
        thread::sleep(Duration::from_millis(200));
    }
    let answer = fs::read_to_string(work.join("answer.txt")).unwrap_or_default();
    assert!(answer.contains("BLUE") && !answer.contains("RED"), "the turn wrote {answer:?}");
    assert!(w.told("another-session").contains("BLUE"), "the ruling still waits for other sessions");
}
