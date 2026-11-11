//! F34: opening the app shows today's page. The built `jellymoon-app` is run as a black box with `layout` and no
//! `--page`, which prints what the window would show once the day is read. The harness is the made-up program of
//! F27's tests, put on PATH as `claude`: it records what it was given and prints the reply the test chose. Every log
//! line and every reply is made up here. Nothing here runs the real Claude Code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::{env, fs};

use serde_json::{Value, json};
use support::Piece;
use tempfile::TempDir;

const DAY: &str = "2026-03-10";
const FIRST: &str = "add a retry to the uploader";
const SECOND: &str = "actually never retry uploads, fail fast";

/// One test's logs, jellymoon home and made-up harness. `rec` is the folder the harness is in, where it leaves what
/// it saw.
struct World {
    dir: TempDir,
    home: PathBuf,
    rec: PathBuf,
    vars: Vec<(&'static str, OsString)>,
}

/// One line of a session log, as Claude Code writes what a person typed.
fn said(at: &str, cwd: &str, text: &str) -> String {
    json!({"type": "user", "timestamp": at, "cwd": cwd, "sessionId": "s", "origin": {"kind": "human"},
        "message": {"role": "user", "content": text}})
    .to_string()
}

fn write_log(world: &World, name: &str, lines: &[String]) {
    let path = world.dir.path().join("logs").join(name);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, lines.join("\n") + "\n").unwrap();
}

/// The harness's reply for a model that answered with `text`.
fn result(text: &str) -> String {
    json!({"type": "result", "is_error": false, "result": text}).to_string()
}

/// A reading of the day: two wishes in the person's words and the clash between them.
fn reading() -> String {
    json!({
        "wishes": [
            {"id": "a", "label": "Retry uploads", "said": FIRST, "project": "alpha", "state": "let_go", "stars": []},
            {"id": "b", "label": "Fail fast", "said": SECOND, "project": "alpha", "state": "done", "stars": ["no retry left"]},
        ],
        "clashes": [{"a": "a", "b": "b", "quote_a": FIRST, "quote_b": "never retry uploads", "settled": true}],
    })
    .to_string()
}

/// A day of two things typed in `alpha` and one in `private-notes`, and a harness on PATH that answers [`reading`].
fn world() -> World {
    let dir = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let at = |name: &str| dir.path().join(name);
    let bin = at("bin");
    fs::create_dir_all(&bin).unwrap();
    let (built, program) = (env!("CARGO_BIN_EXE_f34-claude"), bin.join(format!("claude{}", env::consts::EXE_SUFFIX)));
    // A link on Linux and a copy elsewhere, for the reasons F27's tests give.
    if !cfg!(target_os = "linux") || fs::hard_link(built, &program).is_err() {
        fs::copy(built, &program).unwrap();
    }
    fs::create_dir(at("user")).unwrap();
    let vars = vec![
        ("TZ", "America/Los_Angeles".into()),
        ("HOME", at("user").into()),
        ("USERPROFILE", at("user").into()),
        ("JELLYMOON_HOME", at("jm").into()),
        ("PATH", bin.clone().into()),
    ];
    let world = World { home: at("jm"), rec: bin, vars, dir };
    let lines = [
        said("2026-03-10T16:00:00.000Z", "/work/alpha", FIRST),
        said("2026-03-10T17:00:00.000Z", "/work/alpha", SECOND),
    ];
    write_log(&world, "-work-alpha/s1.jsonl", &lines);
    write_log(
        &world,
        "-work-private-notes/s2.jsonl",
        &[said("2026-03-10T18:00:00.000Z", "/work/private-notes", "dear diary")],
    );
    answer(&world, &result(&reading()), 0);
    world
}

fn answer(world: &World, reply: &str, exit: i32) {
    fs::write(world.rec.join("reply"), reply).unwrap();
    fs::write(world.rec.join("exit"), exit.to_string()).unwrap();
}

/// Runs `jellymoon-app layout` on the day with the world's logs. It must exit 0 and say nothing on stderr.
fn shown(world: &World, args: &[&str]) -> Vec<Piece> {
    let logs = world.dir.path().join("logs");
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_jellymoon-app"));
    cmd.args(["layout", "--date", DAY, "--logs"]).arg(logs).args(args);
    cmd.env_clear().envs(world.vars.iter().map(|(name, value)| (name, value))).stdin(Stdio::null());
    if let Some(root) = env::var_os("SystemRoot").filter(|_| cfg!(windows)) {
        cmd.env("SystemRoot", root);
    }
    let out = cmd.output().unwrap();
    let said = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success() && said.is_empty(), "layout {args:?}: {said}");
    let lines = String::from_utf8(out.stdout).unwrap();
    lines.lines().map(|line| Piece(serde_json::from_str(line).expect(line))).collect()
}

fn notes(placed: &[Piece]) -> Vec<String> {
    placed.iter().filter(|piece| piece.kind() == "note").map(Piece::said).collect()
}

fn labels(placed: &[Piece]) -> Vec<String> {
    placed.iter().filter(|piece| piece.kind() == "label").map(Piece::said).collect()
}

/// Whether the harness was called since the last look. Its record of the call is removed.
fn called(world: &World) -> bool {
    fs::remove_file(world.rec.join("stdin")).is_ok()
}

fn kept(world: &World) -> PathBuf {
    world.home.join("pages").join(format!("{DAY}.json"))
}

fn names(folder: &Path) -> Vec<String> {
    let mut names: Vec<String> =
        fs::read_dir(folder).unwrap().map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned()).collect();
    names.sort();
    names
}

#[test]
fn the_waiting_state_is_a_page_with_one_note_and_the_day_s_page_follows_it() {
    let world = world();
    let waiting = shown(&world, &["--waiting"]);
    assert_eq!(
        notes(&waiting),
        ["Reading your day… This takes a few seconds: your own Claude Code is reading what you typed today."]
    );
    assert!(waiting.iter().any(|piece| piece.kind() == "paper") && labels(&waiting).is_empty());
    assert!(!called(&world) && !world.home.exists(), "the waiting state reads nothing and keeps nothing");

    let page = shown(&world, &[]);
    assert_eq!(labels(&page), ["Fail fast", "Retry uploads"], "the let-go wish is placed last");
    let said = notes(&page);
    assert!(
        said[0].starts_with("came true today ★ no retry left") && said[1].starts_with("you changed your mind"),
        "{said:?}"
    );
    assert!(called(&world));
}

#[test]
fn a_kept_page_is_shown_without_a_call_until_the_day_has_turns_it_did_not_see() {
    let world = world();
    let first = notes(&shown(&world, &[]));
    assert!(called(&world));
    assert_eq!(notes(&shown(&world, &[])), first);
    assert!(!called(&world), "the kept page saw every turn of the day");

    let more = [said("2026-03-10T19:00:00.000Z", "/work/alpha", "and log every failure")];
    write_log(&world, "-work-alpha/s3.jsonl", &more);
    shown(&world, &[]);
    assert!(called(&world), "a turn the kept page did not see");
    let page: Value = serde_json::from_slice(&fs::read(kept(&world)).unwrap()).unwrap();
    assert_eq!(page["read"]["turns"], 4);
    shown(&world, &[]);
    assert!(!called(&world));
    shown(&world, &["--fresh"]);
    assert!(called(&world), "asked to read again");

    let other = shown(&world, &["--date", "2026-03-09"]);
    assert_eq!(
        notes(&other)[0],
        "Nothing wished for yet today. Ask one of your agents for something and it will show up here."
    );
    assert!(!called(&world), "a day with nothing typed makes no call");
    assert_eq!(other.iter().find(|piece| piece.kind() == "tape").unwrap().said(), "Mon · Mar 9");
}

#[test]
fn a_reading_that_cannot_run_is_one_plain_note_that_says_why() {
    let world = world();
    let one = |world: &World, args: &[&str]| {
        let placed = shown(world, args);
        let plain: Vec<&Piece> = placed.iter().filter(|piece| piece.kind() == "note").collect();
        assert_eq!(plain.len(), 1, "{args:?}");
        let roles = plain[0].roles();
        assert!(roles[0] == "note_title" && roles[1..].iter().all(|role| role == "body"), "a plain note: {roles:?}");
        assert!(labels(&placed).is_empty() && !kept(world).exists(), "no page is shown and none is kept");
        plain[0].said()
    };
    answer(&world, "", 1);
    let failed = one(&world, &[]);
    assert!(
        failed.starts_with(
            "Today could not be read. Claude Code could not do the reading. Check that `claude` answers in a terminal"
        ),
        "{failed}"
    );
    answer(&world, &result("Sorry, I cannot help with that."), 0);
    assert_eq!(
        one(&world, &[]),
        "Today could not be read. The model's answer was not a page, so nothing from it is shown. Open Jellymoon again to try once more."
    );
    let nowhere = world.dir.path().join("nowhere");
    let none = one(&world, &["--logs", nowhere.to_str().unwrap()]);
    assert!(none.starts_with("Today could not be read. No Claude Code sessions were found on this computer"), "{none}");

    fs::remove_file(world.rec.join(format!("claude{}", env::consts::EXE_SUFFIX))).unwrap();
    let missing = one(&world, &[]);
    assert!(missing.starts_with("Today could not be read. Claude Code was not found on this computer."), "{missing}");
}

#[test]
fn a_kept_page_stays_on_show_when_a_later_reading_fails_and_the_first_note_says_so() {
    let world = world();
    shown(&world, &[]);
    answer(&world, "", 1);
    let placed = shown(&world, &["--fresh"]);
    assert_eq!(labels(&placed), ["Fail fast", "Retry uploads"]);
    assert!(notes(&placed)[0].starts_with("This page is from an earlier reading. Claude Code could not do"));
}

#[test]
fn only_the_day_goes_to_the_call_with_every_tool_off_and_only_the_checked_page_is_kept() {
    let world = world();
    fs::create_dir_all(&world.home).unwrap();
    fs::write(world.home.join("skip.txt"), "private-notes\n").unwrap();
    shown(&world, &[]);

    let rec = |name: &str| fs::read_to_string(world.rec.join(name)).unwrap();
    let day: Value = serde_json::from_str(&rec("stdin")).unwrap();
    let typed: Vec<&str> = day["turns"].as_array().unwrap().iter().map(|turn| turn["said"].as_str().unwrap()).collect();
    assert_eq!((day["day"].as_str(), typed), (Some(DAY), vec![FIRST, SECOND]), "the day, less the folder switched off");
    assert_eq!(day.as_object().unwrap().len(), 2);
    assert_eq!((rec("arg3"), rec("arg4")), ("--tools".to_string(), String::new()));

    let page: Value = serde_json::from_str(&fs::read_to_string(kept(&world)).unwrap()).unwrap();
    let wish = |id: &str, label: &str, said: &str, state: &str, stars: Value| json!({"id": id, "label": label, "said": said, "project": "alpha", "state": state, "stars": stars});
    let checked = json!({
        "day": DAY,
        "wishes": [wish("w1", "Retry uploads", FIRST, "let_go", json!([])), wish("w2", "Fail fast", SECOND, "done", json!(["no retry left"]))],
        "clashes": [{"a": "w1", "b": "w2", "quote_a": FIRST, "quote_b": "never retry uploads", "settled": true}],
        "read": {"turns": 2, "projects": ["alpha"]},
    });
    assert_eq!(page, checked);
    assert_eq!(names(&world.home), ["pages", "reading", "skip.txt"]);
    assert_eq!(names(&world.home.join("pages")), [format!("{DAY}.json")]);
    assert!(names(&world.home.join("reading")).is_empty(), "the call's folder is gone");
}

// With an empty environment the system could still say where the user's home is. The app does not ask it, so a run
// that names no folder stops here and never reads a real person's logs or opens a window.
#[test]
fn with_no_home_in_the_environment_the_app_stops_before_it_reads_anything() {
    for args in [&[][..], &["layout"], &["layout", "--fresh"]] {
        let out = support::run(args, None);
        let said = String::from_utf8_lossy(&out.stderr);
        assert_eq!(out.status.code(), Some(2), "{args:?}: {said}");
        assert!(said.starts_with("jellymoon-app: finding the session logs: no home directory"), "{args:?}: {said}");
        assert!(out.stdout.is_empty());
    }
}

#[test]
fn a_date_that_is_not_a_day_is_a_wrong_command_line() {
    for date in ["tomorrow", "2026-3-10", "../2026-03-10", "2026-02-30"] {
        let out = support::run(&["layout", "--date", date], None);
        let said = String::from_utf8_lossy(&out.stderr);
        assert_eq!(out.status.code(), Some(2), "{date}: {said}");
        assert!(
            said.starts_with("jellymoon-app: --date must be a day as YYYY-MM-DD") && out.stdout.is_empty(),
            "{said}"
        );
    }
}
