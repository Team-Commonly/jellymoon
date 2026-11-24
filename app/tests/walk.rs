//! The walk example (`examples/walk.rs`) on a made-up home: a new project named, a wish typed and entered, looked at
//! until its work shows itself, and `Keep it` tapped, with the made-up agent of F39's tests in place of Claude Code, which
//! ends with a result. It is started through the door, and core starts the `jellymoon` command put where
//! `install.sh` puts it. The example needs the kit's windowless drawing, which is the Mac's: `cargo test` builds it
//! with `--features sticker-kit/headless`, and `cargo build --example walk` with the same features does too.
#![cfg(target_os = "macos")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;
use std::process::Stdio;
use std::thread::sleep;
use std::time::{Duration, Instant};

use support::door;
use support::wished::{SAID, World, quiet, wait_until};

/// What the run says it did, which no line of the walk may hold.
const DID: &str = "It made hello.txt with one word in it.";

/// The built example, beside the test programs of this package.
fn example() -> PathBuf {
    let built = std::env::current_exe().unwrap().parent().unwrap().parent().unwrap().join("examples").join("walk");
    assert!(
        built.is_file(),
        "the walk example is not built: build it with --example walk --features sticker-kit/headless"
    );
    built
}

/// The world's variables, as a person's terminal has them: Jellymoon's home, the made-up agent as the harness,
/// no session logs, and the command where `install.sh` puts it.
fn vars(w: &World) -> Vec<(&'static str, OsString)> {
    let path = std::env::join_paths([w.at("agent"), w.at("git")]).unwrap();
    let mut vars: Vec<(&'static str, OsString)> = vec![
        ("HOME", w.at("user").into()),
        ("JELLYMOON_HOME", w.at("jm").into()),
        ("JELLYMOON_CLAUDE", w.at("agent").join("claude").into()),
        ("CLAUDE_CONFIG_DIR", w.at("harness").into()),
        ("JELLYMOON_SYSTEM_BIN", OsString::new()),
        ("PATH", path),
        ("GIT_CONFIG_GLOBAL", w.at("gitconfig").into()),
        ("GIT_CONFIG_NOSYSTEM", "1".into()),
        ("GIT_CEILING_DIRECTORIES", w.day.dir.path().into()),
        ("TMPDIR", w.at("tmp").into()),
        ("LC_ALL", "C".into()),
    ];
    vars.extend(support::day::zone());
    vars
}

/// The walk's exit code, its lines and what it said on stderr, within `limit` seconds.
fn walk(w: &World, steps: &[&str], limit: u64) -> (Option<i32>, Vec<String>, String) {
    let (out, err) = (w.at("tmp").join("walk.out"), w.at("tmp").join("walk.err"));
    let pictures = w.at("pictures");
    fs::create_dir_all(&pictures).unwrap();
    let args: Vec<OsString> =
        [pictures.into_os_string(), "1440x900".into()].into_iter().chain(steps.iter().map(OsString::from)).collect();
    let mut walking = door::command(example(), vars(w), &args);
    walking.current_dir(w.day.dir.path()).stdin(Stdio::null());
    walking.stdout(fs::File::create(&out).unwrap()).stderr(fs::File::create(&err).unwrap());
    let mut child = walking.spawn().unwrap();
    let deadline = Instant::now() + Duration::from_secs(limit);
    let ended = loop {
        if let Some(ended) = child.try_wait().unwrap() {
            break ended;
        }
        if Instant::now() > deadline {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("the walk was still going after {limit} seconds: {}", fs::read_to_string(&err).unwrap());
        }
        sleep(Duration::from_millis(20));
    };
    let lines = fs::read_to_string(&out).unwrap().lines().map(str::to_string).collect();
    (ended.code(), lines, fs::read_to_string(&err).unwrap())
}

/// The made-up agent leaves `hello.txt` and ends with a result that says so.
fn told(w: &World) {
    let rec = w.at("rec");
    fs::create_dir_all(rec.join("work")).unwrap();
    fs::write(rec.join("work").join("hello.txt"), "Hello\n").unwrap();
    let pieces = serde_json::json!([
        {"type": "did", "text": DID},
        {"type": "change", "file": "hello.txt", "after": ["Hello"]}
    ]);
    fs::write(rec.join("word"), serde_json::json!({"state": "done", "result": {"pieces": pieces}}).to_string())
        .unwrap();
    w.play("told");
}

fn first(lines: &[String], said: &str) -> usize {
    lines.iter().position(|line| line.contains(said)).unwrap_or_else(|| panic!("no line holds {said}: {lines:#?}"))
}

/// A whole loop: a new project named, a wish entered, `shows` until its work is on the page, and `Keep it`.
fn whole_loop(shows: &[&str]) {
    let w = quiet();
    w.install();
    told(&w);
    let typed = format!("type:{SAID}");
    let steps = [&["type:garden", "enter", "picture:named", typed.as_str(), "enter"][..], shows].concat();
    let steps = [&steps[..], &["picture:waits", "tap:work-m1:Keep it", "picture:kept"][..]].concat();
    let (code, lines, said) = walk(&w, &steps, 120);
    println!("{}", lines.join("\n"));
    assert_eq!(code, Some(0), "{said}");
    assert_eq!(lines.len(), steps.len() + 1, "one line for the opening and one a step: {lines:#?}");
    assert!(lines[0].starts_with("open | ") && lines[1].starts_with("type (6 characters) | "), "{lines:#?}");
    assert!(lines[2].contains("\"Your new project is ready.\""), "{}", lines[2]);
    let working = first(&lines, "wishes: m1 working");
    let waits = first(&lines, "\"Your folder is not changed yet.\"");
    let kept = first(&lines, "\"The change was kept.\"");
    assert!(working < waits && waits < kept, "{lines:#?}");
    assert!(lines[waits].contains("\"What it says it did\""), "the result's note is on the page: {}", lines[waits]);
    let all = lines.join("\n");
    for word in [SAID, DID, "garden", "Hello"] {
        assert!(!all.contains(word), "a line holds {word:?}: {all}");
    }
    let picture = |name: &str| fs::read(w.at("pictures").join(format!("{name}.png"))).unwrap();
    let pictures = ["named", "waits", "kept"].map(picture);
    assert!(pictures.iter().all(|png| png.starts_with(b"\x89PNG")), "every picture step wrote a picture");
    assert!(pictures[0] != pictures[1] && pictures[1] != pictures[2], "each picture is the page as it is then");
}

/// No step taps the wish's character: the look that finds its run ended shows its work, with what the run says.
#[test]
fn a_whole_loop_walks_from_a_new_project_to_kept_work_with_no_tap_on_the_wish() {
    whole_loop(&["until:Your folder is not changed yet.:30"]);
}

/// A tap on the character after the run's end, as a person may still make it: the loop is the same.
#[test]
fn a_whole_loop_with_a_tap_on_the_wish_says_titles_and_states_only() {
    whole_loop(&["until:done:30", "poke:m1"]);
}

/// Enter in the field under a kept wish's note: a follow-up is made and its run asked for, and the words are counted.
#[test]
fn a_say_step_enters_a_follow_up_under_a_note_and_its_words_are_never_printed() {
    let w = quiet();
    w.install();
    told(&w);
    let typed = format!("type:{SAID}");
    let steps = ["type:garden", "enter", typed.as_str(), "enter", "until:Your folder is not changed yet.:30"];
    let steps = [&steps[..], &["tap:work-m1:Keep it", "poke:m1", "say:wish-m1:and once more"][..]].concat();
    let (code, lines, said) = walk(&w, &steps, 120);
    let runs = w.at("jm").join("runs");
    let free = || fs::read_dir(&runs).into_iter().flatten().flatten().all(|run| !run.path().join("run.lock").exists());
    wait_until("the end of the follow-up's run", free);
    assert_eq!(code, Some(0), "{said}");
    let last = &lines[steps.len()];
    assert!(last.starts_with("say:wish-m1 (13 characters) | ") && last.contains("m2 working"), "{last}");
    assert!(!lines.join("\n").contains("once more"), "{lines:#?}");
    let (code, _, said) = walk(&w, &["say:wish-m9:and once more"], 20);
    assert_eq!((code, said.as_str()), (Some(1), "walk: \"say:wish-m9 (13 characters)\": no note \"wish-m9\"\n"));
}

/// A picture taken while the made-up run waits between its two steps: the page has the note of what it is doing
/// (live work), and the walk names no file.
#[test]
fn a_picture_taken_while_a_run_waits_between_its_steps_has_what_it_is_doing() {
    let w = quiet();
    w.install();
    w.play("steps");
    let typed = format!("type:{SAID}");
    let steps = ["type:garden", "enter", typed.as_str(), "enter", "until:steps-m1:30", "picture:midway"];
    let (code, lines, said) = walk(&w, &steps, 120);
    fs::write(w.at("rec").join("go"), "").unwrap();
    // The walk's wish is of today, not the world's day: its run's end is waited for by its lock.
    let runs = w.at("jm").join("runs");
    let free = || fs::read_dir(&runs).into_iter().flatten().flatten().all(|run| !run.path().join("run.lock").exists());
    wait_until("the end of the run", free);
    assert_eq!(code, Some(0), "{said}");
    let midway = &lines[steps.len()];
    assert!(midway.contains("\"Your wish is being worked on.\"") && midway.contains("wishes: m1 working"), "{midway}");
    assert!(fs::read(w.at("pictures").join("midway.png")).unwrap().starts_with(b"\x89PNG"));
    assert!(!lines.join("\n").contains("b.txt"), "{lines:#?}");
}

#[test]
fn a_step_whose_bound_passes_fails_with_a_plain_line() {
    let w = quiet();
    let started = Instant::now();
    let (code, lines, said) = walk(&w, &["until:Nothing like this:1", "picture:never"], 20);
    assert_eq!(code, Some(1), "{said}");
    assert_eq!(
        said,
        "walk: \"until:Nothing like this:1\": \"Nothing like this\" was not on the page after 1 seconds\n"
    );
    assert!(started.elapsed() >= Duration::from_secs(1));
    assert_eq!(lines.len(), 1, "the opening's line only: {lines:#?}");
    assert!(!w.at("pictures").join("never.png").exists(), "no step runs after the one that failed");
}
