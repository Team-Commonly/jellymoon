//! F26: `jellymoon day` prints one local day of what a person typed to Claude Code, and nothing else (ADR 0011,
//! section 1). The built binary is run as a black box over session logs these tests write. The day under test is
//! 2026-03-10 in America/Los_Angeles, whatever zone the machine is in. "LEAK" marks text no person typed.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use serde_json::{Value, json};
use support::{A, B, C, G, Ran, ZONE, line, reply, said, scratch, write_log};
use tempfile::TempDir;

/// A typed line that a resumed session repeats in a second file, under the same uuid.
fn first() -> String {
    line("user", "2026-03-10T16:00:00.000Z", A, "human", json!("add a retry to the uploader"), json!({"uuid": "u1"}))
}

/// The start of the alpha session: a turn the evening before, then one turn with everything in it that is not typed.
fn alpha_untyped() -> Vec<String> {
    let tool_result = json!([{"type": "tool_result", "tool_use_id": "t1", "content": format!("LEAK tool result {}", "x".repeat(3 << 20))}]);
    let tool_use = json!([{"type": "tool_use", "id": "t1", "name": "Bash", "input": {}}]);
    let side = json!({"isSidechain": true});
    vec![
        said("2026-03-10T06:30:00.000Z", A, "the evening before"),
        reply("2026-03-10T06:30:05.000Z", A, "LEAK reply from the evening before"),
        first(),
        line("assistant", "2026-03-10T16:00:01.000Z", A, "", tool_use, json!({})),
        line("user", "2026-03-10T16:00:02.000Z", A, "", tool_result, json!({})),
        reply("2026-03-10T16:00:03.000Z", A, "LEAK an earlier reply in the same turn"),
        line("user", "2026-03-10T16:00:04.000Z", A, "task-notification", json!("LEAK task notice"), json!({"promptSource": "system"})),
        reply("2026-03-10T16:00:05.000Z", A, "retry added, tests pass"),
        line("assistant", "2026-03-10T16:00:06.000Z", A, "", json!([{"type": "text", "text": "LEAK sidechain reply"}]), side.clone()),
        line("user", "2026-03-10T16:00:07.000Z", A, "human", json!("LEAK sidechain prompt"), side),
        line("user", "2026-03-10T16:00:08.000Z", A, "human", json!("LEAK meta"), json!({"isMeta": true})),
        line("user", "2026-03-10T16:00:09.000Z", A, "", json!("LEAK compaction summary"), json!({"isCompactSummary": true})),
        line("user", "2026-03-10T16:00:09.500Z", A, "human", json!("LEAK summary marked as typed"), json!({"isCompactSummary": true})),
        line("user", "2026-03-10T16:00:10.000Z", A, "", json!("LEAK prompt from another program"), json!({"promptSource": "sdk"})),
        reply("2026-03-10T16:00:11.000Z", A, "LEAK reply to another program's prompt"),
        r#"{"type":"user","timestamp":"2026-03-10T16:30:00.000Z","origin":{"kind":"human"},"message":{"content":"LEAK broken line"#.to_string(),
    ]
}

/// The rest of the alpha session: harness blocks and credentials inside a prompt, prompts that are no turn, the day's edges.
fn alpha_typed() -> Vec<String> {
    let blocks = "<system-reminder>\nLEAK reminder\n</system-reminder>\n<artifact-view-context id=\"a1\">\n{\"LEAK\":\"view\"}\n\
                  (The JSON line above is LEAK explainer.)\n</artifact-view-context>\n\n<artifact-content-authored-by-others/>\n\
                  (The JSON line above is LEAK explainer\nover two lines.)\n";
    let typed = format!(
        "{blocks}deploy with {} and {} then {} {}",
        support::hub_key(),
        support::run_token(),
        support::unknown_key(),
        "long ".repeat(200)
    );
    vec![
        said("2026-03-10T17:30:00.000Z", A, &typed),
        reply("2026-03-10T17:30:05.000Z", A, &format!("used {} {}", support::model_key(), "done ".repeat(200))),
        said("2026-03-10T18:00:00.000Z", A, "<system-reminder>LEAK only a reminder</system-reminder>\n"),
        said("2026-03-10T18:01:00.000Z", A, "<command-name>/LEAK</command-name>"),
        said(
            "2026-03-10T18:02:00.000Z",
            A,
            "<command-message>LEAK</command-message>\n<command-name>/LEAK</command-name>",
        ),
        said("2026-03-10T18:03:00.000Z", A, "<local-command-stdout>LEAK</local-command-stdout>"),
        reply("2026-03-10T18:03:05.000Z", A, "LEAK reply to a command"),
        said("2026-03-11T06:59:00.000Z", A, "one minute before midnight"),
        reply("2026-03-11T06:59:30.000Z", A, "noted before midnight"),
        said("2026-03-11T07:01:00.000Z", A, "one minute after midnight"),
        reply("2026-03-11T07:01:30.000Z", A, "noted after midnight"),
    ]
}

/// A session with prompts whose content is a list: beside a tool result, an image alone, and text typed with an image.
fn gamma() -> Vec<String> {
    let image = json!({"type": "image", "source": {"type": "base64", "data": "LEAK"}});
    let text = |t: &str| json!({"type": "text", "text": t});
    let beside =
        json!([text("LEAK beside a tool result"), {"type": "tool_result", "tool_use_id": "t2", "content": "LEAK"}]);
    let around = json!([text("<system-reminder>LEAK</system-reminder>look at "), image, text("this screenshot")]);
    vec![
        said("2026-03-10T21:00:00.000Z", G, "first gamma ask"),
        line("user", "2026-03-10T21:00:30.000Z", G, "human", beside, json!({})),
        reply("2026-03-10T21:00:40.000Z", G, "gamma reply one"),
        line("user", "2026-03-10T21:01:00.000Z", G, "human", json!([image]), json!({})),
        reply("2026-03-10T21:01:30.000Z", G, "LEAK reply to an image alone"),
        line("user", "2026-03-10T21:02:00.000Z", G, "human", around, json!({})),
        reply("2026-03-10T21:02:30.000Z", G, "seen"),
    ]
}

/// Writes the projects directory, in a folder of its own inside the one returned with it.
fn logs() -> (TempDir, PathBuf) {
    let dir = scratch();
    let logs = dir.path().join("logs");
    let at = logs.as_path();
    write_log(at, "-work-gamma/s4.jsonl", &gamma());
    write_log(
        at,
        "-work-alpha/old.jsonl",
        &[said("2026-03-10T22:00:00.000Z", A, "LEAK from a file last written before the day")],
    );
    write_log(at, "-work-alpha/s1.jsonl", &[alpha_untyped(), alpha_typed()].concat());
    let private =
        [said("2026-03-10T19:00:00.000Z", B, "a private note"), reply("2026-03-10T19:00:05.000Z", B, "kept private")];
    write_log(at, "-work-private-notes/s2.jsonl", &private);
    let beta = [
        said("2026-03-10T20:15:00.000Z", C, "fix the login page"),
        line("user", "2026-03-10T20:15:30.000Z", C, "peer", json!("LEAK message from another session"), json!({})),
        reply("2026-03-10T20:15:40.000Z", C, "LEAK reply to a peer"),
    ];
    write_log(at, "-work-beta/s3.jsonl", &beta);
    write_log(
        at,
        "-work-alpha/s9.jsonl",
        &[first(), reply("2026-03-10T16:00:20.000Z", A, "LEAK reply in the resumed copy")],
    );
    write_log(
        at,
        "-work-private-notes/s6.jsonl",
        &[said("2026-03-10T22:30:00.000Z", "", "a note typed with no folder on its line")],
    );
    write_log(at, "-work-alpha/readme.txt", &[said("2026-03-10T20:00:00.000Z", A, "LEAK not a session log")]);
    traps(at);
    // A file last written before the day began is not opened, whatever its lines say: 2026-03-09T12:00Z.
    let old = fs::File::options().write(true).open(at.join("-work-alpha/old.jsonl")).unwrap();
    old.set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(1_773_057_600)).unwrap();
    (dir, logs)
}

/// Puts things in the logs directory that are not session files in real folders: a link to a file and a link to a folder,
/// both outside it, and a pipe nobody writes to. None may be opened. Unix only: Windows has no such pipe, and making a
/// link there needs a privilege.
#[cfg(unix)]
fn traps(logs: &Path) {
    use std::os::unix::fs::symlink;
    let outside = logs.parent().unwrap().join("outside");
    write_log(&outside, "kept/s.jsonl", &[said("2026-03-10T20:30:00.000Z", A, "LEAK from outside the logs")]);
    symlink(outside.join("kept/s.jsonl"), logs.join("-work-alpha/link.jsonl")).unwrap();
    symlink(outside.join("kept"), logs.join("-work-linked")).unwrap();
    let made = std::process::Command::new("mkfifo").arg(logs.join("-work-alpha/pipe.jsonl")).status().unwrap();
    assert!(made.success(), "mkfifo failed");
}

#[cfg(not(unix))]
fn traps(_: &Path) {}

/// Runs `jellymoon day` in the tests' time zone with an empty home.
fn run(args: &[&str]) -> Ran {
    let home = scratch();
    let home = home.path().as_os_str();
    support::jellymoon(&[("TZ", OsStr::new(ZONE)), ("HOME", home), ("USERPROFILE", home)], &[&["day"], args].concat())
}

/// Runs `jellymoon day` and decodes the day it printed: the turns, and the whole output.
fn day(logs: &Path, args: &[&str]) -> (Vec<Value>, String) {
    let ran = run(&[&["--logs", logs.to_str().unwrap()], args].concat());
    assert_eq!((ran.code, ran.stderr.as_str()), (0, ""), "jellymoon day {args:?}");
    let day: Value = serde_json::from_str(&ran.stdout).expect("stdout is the day as JSON");
    (day["turns"].as_array().expect("turns is a list").clone(), ran.stdout)
}

fn turn(at: &str, project: &str, said: &str, reply: &str) -> Value {
    json!({"at": at, "tool": "claude-code", "project": project, "said": said, "reply": reply})
}

const SKIP: &[&str] = &["--date", "2026-03-10", "--skip", "unrelated, private-notes,"];

// Items 1 to 4, 7 and 10 to 13.
#[test]
fn the_day_is_each_typed_turn_with_the_agents_last_reply_in_time_order() {
    let (_dir, logs) = logs();
    let (turns, raw) = day(&logs, SKIP);
    assert_eq!(turns.len(), 6, "{raw:.2000}");
    assert!(
        raw.starts_with(r#"{"day":"2026-03-10","turns":[{"at":"09:00","tool":"claude-code","project":"alpha","said":"#),
        "{raw:.200}"
    );
    assert!(raw.ends_with("}]}\n") && raw.matches('\n').count() == 1, "one line of JSON");
    // 16:00Z is 09:00 there. The reply is the last text of the turn; the same prompt in the resumed copy is not a second turn.
    assert_eq!(turns[0], turn("09:00", "alpha", "add a retry to the uploader", "retry added, tests pass"));
    assert_eq!((&turns[1]["at"], &turns[1]["project"]), (&json!("10:30"), &json!("alpha")));
    // Typed in a worktree of beta: the repo is the project. The reply to a peer's message is not this turn's.
    assert_eq!(turns[2], turn("13:15", "beta", "fix the login page", ""));
    // A list with a tool result does not end the turn; an image alone does, and makes none.
    assert_eq!(turns[3], turn("14:00", "gamma", "first gamma ask", "gamma reply one"));
    // Typed with a pasted image: its text parts, joined.
    assert_eq!(turns[4], turn("14:02", "gamma", "look at this screenshot", "seen"));
    assert_eq!(turns[5], turn("23:59", "alpha", "one minute before midnight", "noted before midnight"));
}

// Items 6, 8, 9 and 10, and the unix traps of item 13.
#[test]
fn only_what_a_person_typed_is_printed_and_the_harness_blocks_are_cut_out_of_it() {
    let (_dir, logs) = logs();
    let (turns, raw) = day(&logs, SKIP);
    if let Some(at) = raw.find("LEAK") {
        panic!("the output holds text no person typed, near {:?}", &raw[at.saturating_sub(80)..at + 4]);
    }
    let said = turns[1]["said"].as_str().unwrap();
    assert!(said.starts_with("deploy with "), "the text does not start where the typing starts: {said:.80}");
}

// Items 14 and 15.
#[test]
fn credential_shapes_are_replaced_in_both_texts_and_both_are_cut_short() {
    let (_dir, logs) = logs();
    let (turns, raw) = day(&logs, SKIP);
    let (hub, key, token) = (support::hub_key(), support::model_key(), support::run_token());
    for secret in [&hub, &key, &token, &support::unknown_key(), &hub[4..20], &key[7..20], &token[4..20]] {
        assert!(!raw.contains(secret), "the output holds {secret:?}");
    }
    let (said, reply) = (turns[1]["said"].as_str().unwrap(), turns[1]["reply"].as_str().unwrap());
    assert_eq!((said.chars().count(), reply.chars().count()), (700, 500));
    assert!(said.ends_with('…') && reply.ends_with('…'), "a cut text ends in an ellipsis");
    assert!(said.starts_with("deploy with [redacted] and jmr_… then [redacted] long long"), "{said:.80}");
    assert!(reply.starts_with("used [redacted] done done"), "{reply:.40}");
}

// Item 16.
#[test]
fn a_skipped_folder_contributes_nothing_and_is_read_when_it_is_not_skipped() {
    let (_dir, logs) = logs();
    let (_, raw) = day(&logs, SKIP);
    assert!(!raw.contains("private"), "with --skip the output names the folder");
    let (all, _) = day(&logs, &["--date", "2026-03-10"]);
    assert_eq!(all.len(), 8);
    assert_eq!(all[2], turn("12:00", "private-notes", "a private note", "kept private"));
    // A line with no folder is skipped by the name of its session's folder, and its project is empty.
    assert_eq!(all[6], turn("15:30", "", "a note typed with no folder on its line", ""));
}

// Items 1 (the bytes), 5 and 17.
#[test]
fn a_turn_lands_on_the_local_day_it_was_typed_and_an_empty_day_is_an_empty_list() {
    let (_dir, logs) = logs();
    let (_, next) = day(&logs, &["--date", "2026-03-11"]);
    let want = r#"{"day":"2026-03-11","turns":[{"at":"00:01","tool":"claude-code","project":"alpha","said":"one minute after midnight","reply":"noted after midnight"}]}"#;
    assert_eq!(next, format!("{want}\n"));
    let (before, _) = day(&logs, &["--date", "2026-03-09"]);
    assert_eq!(
        (before.len(), &before[0]["at"], &before[0]["said"]),
        (1, &json!("23:30"), &json!("the evening before"))
    );
    let (_, none) = day(&logs, &["--date", "2026-03-12"]);
    assert_eq!(none, "{\"day\":\"2026-03-12\",\"turns\":[]}\n");
}

// Item 11 on Windows: the folder is written with backslashes there.
#[test]
fn a_windows_folder_is_named_by_its_last_segment_and_a_worktree_by_its_repo() {
    let logs = scratch();
    let lines = [
        said("2026-03-10T16:00:00.000Z", r"C:\work\delta", "typed on Windows"),
        said(
            "2026-03-10T17:00:00.000Z",
            r"C:\work\beta\.claude\worktrees\fix-login\web",
            "typed in a worktree on Windows",
        ),
    ];
    write_log(logs.path(), "C--work-delta/s.jsonl", &lines);
    let (turns, _) = day(logs.path(), &["--date", "2026-03-10"]);
    assert_eq!(
        turns,
        [turn("09:00", "delta", "typed on Windows", ""), turn("10:00", "beta", "typed in a worktree on Windows", "")]
    );
}

// A folder's name is a log's text like any other: cleaned, and checked for credential shapes, before it is printed.
#[test]
fn a_project_name_has_its_control_characters_removed_and_a_credential_shape_replaced() {
    let logs = scratch();
    let mixed = format!("/w/{}\u{1b}\u{9b}", "aB3x9Zq7".repeat(6));
    let lines = [
        said("2026-03-10T16:00:00.000Z", &mixed, "typed in a folder named like a key of unknown make"),
        said("2026-03-10T17:00:00.000Z", "/w/del\u{1b}[2Jta\u{202e}", "typed in a folder named with an escape"),
        said("2026-03-10T18:00:00.000Z", &format!("/w/{}", support::hub_key()), "typed in a folder named as a key"),
    ];
    write_log(logs.path(), "-w-x/s.jsonl", &lines);
    let (turns, raw) = day(logs.path(), &["--date", "2026-03-10"]);
    let projects: Vec<&str> = turns.iter().map(|turn| turn["project"].as_str().unwrap()).collect();
    assert_eq!(projects, ["[redacted]", "delta", "[redacted]"]);
    assert!(!raw.contains("aB3x9Zq7") && !raw.contains("\\u"), "{raw}");
}

#[test]
fn the_logs_default_to_the_harness_own_folder_under_home_or_its_config_dir() {
    let (home, config) = (scratch(), scratch());
    write_log(
        home.path(),
        ".claude/projects/-work-alpha/s.jsonl",
        &[said("2026-03-10T16:00:00.000Z", A, "typed under home")],
    );
    write_log(
        config.path(),
        "projects/-work-alpha/s.jsonl",
        &[said("2026-03-10T16:00:00.000Z", A, "typed under the config dir")],
    );
    let home = home.path().as_os_str();
    let mut vars = vec![("TZ", OsStr::new(ZONE)), ("HOME", home), ("USERPROFILE", home)];
    let ran = support::jellymoon(&vars, &["day", "--date", "2026-03-10"]);
    assert!(ran.code == 0 && ran.stdout.contains("typed under home"), "{} {}", ran.stdout, ran.stderr);
    vars.push(("CLAUDE_CONFIG_DIR", config.path().as_os_str()));
    let ran = support::jellymoon(&vars, &["day", "--date", "2026-03-10"]);
    assert!(ran.code == 0 && ran.stdout.contains("typed under the config dir"), "{} {}", ran.stdout, ran.stderr);
}

#[test]
fn a_command_that_cannot_run_prints_an_error_and_no_day() {
    let (_dir, logs) = logs();
    let logs = logs.to_str().unwrap();
    let missing = scratch().path().join("gone");
    // A run that fails exits 1; a command line that is wrong exits 2 and prints the usage after the reason.
    let cases: &[(i32, &[&str])] = &[
        (1, &["--logs", logs, "--date", "10-03-2026"]),
        (1, &["--logs", logs, "--date", "2026-02-30"]),
        (1, &["--logs", missing.to_str().unwrap()]),
        (2, &["--logs", logs, "--verbose"]),
        (2, &["--logs", logs, "--model", "sonnet"]),
        (2, &["--logs", logs, "extra"]),
        (2, &["--logs"]),
    ];
    for (code, args) in cases {
        let ran = run(args);
        assert!(
            ran.code == *code && ran.stdout.is_empty() && ran.stderr.starts_with("jellymoon: "),
            "{args:?}: {} {:?} {:?}",
            ran.code,
            ran.stdout,
            ran.stderr
        );
        assert_eq!(ran.stderr.contains("usage:"), *code == 2, "{args:?}: {}", ran.stderr);
    }
    assert!(run(&["--logs", logs, "--date", "tomorrow"]).stderr.contains("--date"));
}

#[test]
fn help_prints_the_usage_and_no_command_is_an_error() {
    for args in [&["help"][..], &["--help"], &["day", "-h"], &["page", "--help"]] {
        let ran = support::jellymoon(&[], args);
        assert!(
            ran.code == 0
                && ran.stdout.starts_with("usage:")
                && ran.stdout.contains("jellymoon day")
                && ran.stdout.contains("jellymoon page"),
            "{args:?}: {}",
            ran.stdout
        );
    }
    for args in [&[][..], &["days"], &["daemon", "start"]] {
        let ran = support::jellymoon(&[], args);
        assert!(
            ran.code == 2
                && ran.stdout.is_empty()
                && ran.stderr.starts_with("jellymoon: ")
                && ran.stderr.contains("usage:"),
            "{args:?}: {}",
            ran.stderr
        );
    }
}
