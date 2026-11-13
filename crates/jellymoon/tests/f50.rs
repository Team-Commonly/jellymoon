//! F50: the reader leaves out a session whose working directory is a run's copy (ADR 0018, decision 7;
//! `docs/development/reader.md`, "A run's copy is never read"). `jellymoon day`, `jellymoon page` and `jellymoon run`
//! are run as black boxes over session logs these tests write, with a scratch home. Every text typed or said in a
//! copy holds [`LEFT`], which no output may hold. The harness is the made-up program of `support/fake_claude.rs`,
//! and for the run the made-up agent of F39's test folder (`f39/world.rs`).
#![allow(clippy::unwrap_used, clippy::expect_used)]

#[path = "f50/lines.rs"]
mod lines;
mod support;
#[allow(unused_imports)] // the world names more than this file asks of it
#[path = "f39/world.rs"]
mod world;

use std::ffi::OsString;
use std::fs;
use std::path::{MAIN_SEPARATOR, Path, PathBuf};

use serde_json::{Value, json};
use support::{A, B, G, Ran, ZONE, install, reply, said, scratch, tree, write_log};
use tempfile::TempDir;

/// What every text of a copy's session holds.
const LEFT: &str = "quince-heron-typed-in-a-copy";
/// A run's id, as core makes one.
const RUN: &str = "r0123456789abcdef";
const DAY: [&str; 2] = ["--date", "2026-03-10"];

/// One test's folder: `logs`, the home, and the made-up harness, alone on `PATH`.
struct World {
    dir: TempDir,
    home: PathBuf,
    vars: Vec<(&'static str, OsString)>,
}

/// A world whose home is `jm` in the test's folder. Nothing makes the home.
fn world() -> World {
    let dir = scratch();
    let home = dir.path().join("jm");
    world_at(dir, home)
}

fn world_at(dir: TempDir, home: PathBuf) -> World {
    let at = |name: &str| dir.path().join(name);
    install(&at("bin"), "claude");
    let reading = json!({"type": "result", "is_error": false, "result": r#"{"wishes":[],"clashes":[]}"#});
    fs::write(at("bin").join("reply"), reading.to_string()).unwrap();
    fs::write(at("bin").join("exit"), "0").unwrap();
    fs::create_dir(at("user")).unwrap();
    let vars = vec![
        ("TZ", ZONE.into()),
        ("HOME", at("user").into()),
        ("USERPROFILE", at("user").into()),
        ("JELLYMOON_HOME", home.clone().into()),
        ("PATH", at("bin").into()),
        ("JELLYMOON_SYSTEM_BIN", OsString::new()), // no folder of this machine is looked in for a harness
    ];
    World { dir, home, vars }
}

/// The minute of the fixture day a line is written at: lines are printed in this order.
fn at(minute: usize) -> String {
    format!("2026-03-10T16:{minute:02}:00.000Z")
}

fn text(folder: &Path) -> &str {
    folder.to_str().unwrap()
}

/// What the turns of a printed day say, each with its reply.
fn turns(day: &str) -> Vec<(String, String)> {
    let day: Value = serde_json::from_str(day).expect(day);
    let word = |turn: &Value, key: &str| turn[key].as_str().unwrap().to_string();
    day["turns"].as_array().unwrap().iter().map(|turn| (word(turn, "said"), word(turn, "reply"))).collect()
}

impl World {
    /// A run's copy in this world's home.
    fn copy(&self) -> PathBuf {
        self.home.join("copies").join(RUN)
    }

    fn log(&self, name: &str, lines: &[String]) {
        write_log(&self.dir.path().join("logs"), name, lines);
    }

    /// One session of one line for each folder: `LEFT` and its place in the list when it is typed in a copy, and
    /// `seen` and its place when it is not.
    fn typed_in(&self, folders: &[(String, bool)]) {
        let line = |(n, (folder, left)): (usize, &(String, bool))| {
            said(&at(n), folder, &if *left { format!("{LEFT} {n}") } else { format!("seen {n}") })
        };
        self.log("-many/s.jsonl", &folders.iter().enumerate().map(line).collect::<Vec<_>>());
    }

    fn run(&self, command: &str, more: &[&str]) -> Ran {
        let vars: Vec<_> = self.vars.iter().map(|(name, value)| (*name, value.as_os_str())).collect();
        let logs = self.dir.path().join("logs");
        support::jellymoon_in(self.dir.path(), &vars, &[&[command, "--logs", text(&logs)], &DAY[..], more].concat())
    }

    /// What `jellymoon day` prints of the day's turns. It worked, and printed nothing of a copy.
    fn day(&self, more: &[&str]) -> Vec<String> {
        let ran = self.run("day", more);
        assert_eq!((ran.code, ran.stderr.as_str()), (0, ""), "{}", ran.stdout);
        assert!(!ran.stdout.contains(LEFT), "{}", ran.stdout);
        turns(&ran.stdout).into_iter().map(|(said, _)| said).collect()
    }

    /// What the reading call was sent since the last look. None when no call was made.
    fn sent(&self) -> Option<String> {
        let seen = self.dir.path().join("bin").join("stdin");
        let sent = fs::read_to_string(&seen).ok()?;
        fs::remove_file(seen).unwrap();
        Some(sent)
    }
}

/// A session in `alpha`: two turns, each answered.
fn alpha() -> Vec<String> {
    vec![
        said(&at(0), A, "add a retry to the uploader"),
        reply(&at(1), A, "retry added, tests pass"),
        said(&at(30), A, "now cap it at two tries"),
        reply(&at(31), A, "capped at two"),
    ]
}

/// A run's kept session: the wish, what the agent said, an answer typed deeper in the copy, and words with a
/// pasted picture.
fn kept_session(copy: &Path) -> Vec<String> {
    let deeper = copy.join("src").join("web");
    let pasted = json!([{"type": "text", "text": format!("{LEFT} with a picture")}, {"type": "image", "source": {}}]);
    vec![
        said(&at(10), text(copy), &format!("{LEFT}: make the uploader retry twice")),
        reply(&at(11), text(copy), &format!("{LEFT} which file is the uploader?")),
        said(&at(12), text(&deeper), &format!("{LEFT}: it is upload.rs")),
        reply(&at(13), text(&deeper), &format!("{LEFT} done")),
        support::line("user", &at(14), text(copy), "human", pasted, json!({"uuid": "u-copy"})),
    ]
}

#[test]
fn a_session_wholly_in_a_copy_prints_no_turn_and_every_other_session_reads_as_before() {
    let w = world();
    w.log("-work-alpha/s1.jsonl", &alpha());
    let before = w.run("day", &[]);
    assert_eq!(turns(&before.stdout).len(), 2, "{} {}", before.stdout, before.stderr);

    w.log("-jm-copies-run/s2.jsonl", &kept_session(&w.copy()));
    let after = w.run("day", &[]);
    assert_eq!((after.code, after.stderr.as_str()), (0, ""));
    assert_eq!(after.stdout, before.stdout, "the day is what it was before the copy's session was there");
    assert!(!w.home.exists(), "reading the day made nothing in the home");
}

#[test]
fn of_a_session_with_lines_in_a_copy_and_lines_elsewhere_only_the_copy_s_lines_go() {
    let w = world();
    let copy = w.copy();
    let (copy, deeper) = (text(&copy).to_string(), copy.join("notes"));
    let lines = [
        said(&at(0), A, "first, in alpha"),
        reply(&at(1), A, "first done"),
        // Typed in the copy: no turn, and the turn before it is over, so what is said next answers nobody here.
        said(&at(2), &copy, &format!("{LEFT} typed")),
        reply(&at(3), &copy, &format!("{LEFT} said")),
        reply(&at(4), A, &format!("{LEFT} said elsewhere to what was typed in the copy")),
        // Said in the copy to a turn typed elsewhere: it is a line of the copy, and no reply. The turn stays open
        // for what is said elsewhere after it.
        said(&at(5), A, "second, in alpha"),
        reply(&at(6), text(&deeper), &format!("{LEFT} said deeper")),
        reply(&at(7), A, "second done"),
        said(&at(8), A, "third, in alpha"),
        reply(&at(9), A, "third done"),
        reply(&at(10), &copy, &format!("{LEFT} said last")),
    ];
    w.log("-work-alpha/s1.jsonl", &lines);
    let ran = w.run("day", &[]);
    assert!(!ran.stdout.contains(LEFT), "{}", ran.stdout);
    let word = |said: &str, reply: &str| (said.to_string(), reply.to_string());
    let kept = [("first", "first done"), ("second", "second done"), ("third", "third done")];
    assert_eq!(turns(&ran.stdout), kept.map(|(nth, reply)| word(&format!("{nth}, in alpha"), reply)));
}

#[test]
fn only_a_folder_below_the_home_s_copies_is_left_out_and_it_is_matched_by_whole_names() {
    let w = world();
    let (home, dir) = (&w.home, w.dir.path());
    let (copies, sep) = (home.join("copies"), MAIN_SEPARATOR);
    let read = |folder: PathBuf| (text(&folder).to_string(), false);
    let left = |folder: String| (folder, true);
    let folders = [
        // A name that only starts or ends like `copies`, the folder itself, and the home and its other folders.
        read(home.join("copies-old").join(RUN)),
        read(home.join("copiesx").join(RUN)),
        read(home.join("copie").join(RUN)),
        read(home.join("old-copies").join(RUN)),
        read(copies.clone()),
        (format!("{}{sep}", text(&copies)), false),
        (format!("{}{sep}.", text(&copies)), false),
        read(home.clone()),
        read(home.join("runs").join(RUN)),
        // A folder named `copies` somewhere else: deeper in the home, beside it, and in a folder beside the home.
        read(home.join("kept").join("copies").join(RUN)),
        read(dir.join("copies").join(RUN)),
        read(dir.join("jm-old").join("copies").join(RUN)),
        read(dir.join("j").join("copies").join(RUN)),
        // A path that is not a full one, and one that climbs back out: the log's text is never looked up.
        read(Path::new("copies").join(RUN)),
        read(Path::new("jm").join("copies").join(RUN)),
        read(copies.join("..").join("pages")),
        read(copies.join(RUN).join("..").join("..").join("pages")),
        // Another case is another name, and a `\` is a letter of one, except on Windows.
        (text(&home.join("Copies").join(RUN)).to_string(), cfg!(windows)),
        (format!("{}\\{RUN}", text(&copies)), cfg!(windows)),
        // A copy, whatever its name; anything deeper; a separator at its end, doubled, or with a `.` between.
        left(text(&copies.join(RUN)).to_string()),
        left(text(&copies.join("any-name")).to_string()),
        left(text(&copies.join(RUN).join("src").join("deep").join("er")).to_string()),
        left(format!("{}{sep}", text(&copies.join(RUN)))),
        left(format!("{}{sep}{sep}{RUN}", text(&copies))),
        left(format!("{}{sep}.{sep}copies{sep}{RUN}{sep}.", text(home))),
    ];
    w.typed_in(&folders);
    let read = folders.iter().enumerate().filter(|(_, (_, left))| !left);
    let seen: Vec<String> = read.map(|(n, _)| format!("seen {n}")).collect();
    assert_eq!(seen.len(), if cfg!(windows) { 17 } else { 19 });
    assert_eq!(w.day(&[]), seen);
}

#[test]
fn the_copies_are_compared_as_the_home_is_given_and_as_the_system_resolves_it_and_a_home_not_there_yet_is_no_error() {
    let w = world();
    w.log("-work-alpha/s1.jsonl", &alpha());
    w.log("-jm-copies-run/s2.jsonl", &kept_session(&w.copy()));
    let both = ["add a retry to the uploader", "now cap it at two tries"];
    assert_eq!(w.day(&[]), both, "the home is not there: its copies are named as it is given");
    assert!(!w.home.exists());

    // The home as the system resolves it, which on Windows is spelled `\\?\C:\…`.
    fs::create_dir(&w.home).unwrap();
    let resolved = fs::canonicalize(&w.home).unwrap().join("copies").join(RUN);
    w.log("-resolved/s3.jsonl", &kept_session(&resolved));
    assert_eq!(w.day(&[]), both);
}

// Unix only: a link to a folder is made here with no right asked for, and on Windows making one needs a right a
// test cannot count on.
#[cfg(unix)]
#[test]
fn a_copy_logged_under_the_folder_a_link_in_the_home_s_path_leads_to_is_left_out() {
    let dir = scratch();
    let (real, link) = (dir.path().join("real"), dir.path().join("link"));
    fs::create_dir_all(real.join("jm")).unwrap();
    std::os::unix::fs::symlink(&real, &link).unwrap();
    let resolved = fs::canonicalize(&real).unwrap().join("jm");
    assert_ne!(resolved, link.join("jm"));

    // The home is named through the link. A harness logs the folder it stands in as the system resolved it.
    let mut w = world_at(dir, link.join("jm"));
    w.typed_in(&[
        (A.to_string(), false),
        (text(&resolved.join("copies").join(RUN)).to_string(), true),
        (text(&w.copy()).to_string(), true),
        (text(&resolved.join("copies")).to_string(), false),
        (text(&resolved.join("copies-old").join(RUN)).to_string(), false),
    ]);
    assert_eq!(w.day(&[]), ["seen 0", "seen 3", "seen 4"]);

    // The other way round it is read: only the home's side is resolved, never the log's text.
    let through = w.copy();
    w.vars.push(("JELLYMOON_HOME", resolved.clone().into())); // the later of two is the one a command gets
    w.home = resolved;
    w.typed_in(&[(A.to_string(), false), (text(&through).to_string(), false), (text(&w.copy()).to_string(), true)]);
    assert_eq!(w.day(&[]), ["seen 0", "seen 1"]);
}

#[test]
fn skip_and_skip_txt_switch_folders_off_as_before_beside_a_copy() {
    let w = world();
    w.log("-work-alpha/s1.jsonl", &alpha());
    w.log("-work-private-notes/s2.jsonl", &[said(&at(40), B, "dear diary")]);
    w.log("-work-gamma/s3.jsonl", &[said(&at(50), G, "tidy the changelog")]);
    w.log("-jm-copies-run/s4.jsonl", &kept_session(&w.copy()));
    let alpha = ["add a retry to the uploader", "now cap it at two tries"];
    assert_eq!(w.day(&[]), [&alpha[..], &["dear diary", "tidy the changelog"]].concat());
    assert_eq!(w.day(&["--skip", "gamma"]), [&alpha[..], &["dear diary"]].concat());

    fs::create_dir(&w.home).unwrap();
    fs::write(w.home.join("skip.txt"), "private-notes\n").unwrap();
    assert_eq!(w.day(&[]), [&alpha[..], &["tidy the changelog"]].concat());
    assert_eq!(w.day(&["--skip", "gamma"]), alpha);
    assert_eq!(w.day(&["--skip", "alpha,gamma"]), [""; 0]);
    // A name that is in the copy's own path switches nothing else off, and a list that is no list still stops.
    assert_eq!(w.day(&["--skip", RUN]), [&alpha[..], &["tidy the changelog"]].concat());
    fs::write(w.home.join("skip.txt"), b"private-notes\n\xff\n").unwrap();
    let stopped = w.run("day", &[]);
    assert_eq!((stopped.code, stopped.stdout.as_str()), (1, ""), "{}", stopped.stderr);
}

#[test]
fn nothing_of_a_copy_s_session_reaches_the_reading_call_or_the_page() {
    let w = world();
    w.log("-work-alpha/s1.jsonl", &alpha());
    let day = w.run("day", &[]).stdout;
    w.log("-jm-copies-run/s2.jsonl", &kept_session(&w.copy()));
    // A folder of another name deep in a copy would be a project of its own.
    w.log("-jm-copies-zeta/s3.jsonl", &[said(&at(20), text(&w.copy().join("zeta")), &format!("{LEFT} in zeta"))]);

    let page = w.run("page", &[]);
    assert_eq!((page.code, page.stderr.as_str()), (0, ""), "{}", page.stdout);
    assert_eq!(w.sent().as_deref(), Some(day.as_str()), "the call is sent the day without the copy, byte for byte");
    for of_the_copy in [LEFT, RUN, "zeta"] {
        assert!(!page.stdout.contains(of_the_copy), "{of_the_copy}: {}", page.stdout);
    }
    let printed: Value = serde_json::from_str(&page.stdout).unwrap();
    assert_eq!(printed["read"]["projects"], json!(["alpha"]));
    // Nothing the commands left in the home holds a word of it, and neither did the harness's arguments.
    for (path, bytes) in [tree(&w.home), tree(&w.dir.path().join("bin"))].concat() {
        let held = bytes.windows(LEFT.len()).any(|window| window == LEFT.as_bytes());
        assert!(!held, "{} holds what was typed in a copy", path.display());
    }

    // A day whose only sessions are a run's: no turn, so no call is made, as on a day nothing was typed.
    fs::remove_dir_all(w.dir.path().join("logs").join("-work-alpha")).unwrap();
    assert_eq!(w.day(&[]), [""; 0]);
    let page = w.run("page", &[]);
    assert_eq!((page.code, page.stderr.as_str(), w.sent()), (0, "", None), "{}", page.stdout);
    let printed: Value = serde_json::from_str(&page.stdout).unwrap();
    assert_eq!((&printed["read"]["projects"], &printed["wishes"]), (&json!([]), &json!([])));
    assert!(!page.stdout.contains(LEFT) && !page.stdout.contains(RUN), "{}", page.stdout);
}

// In the system's own temporary folder, as F39's test of the lookup is: Cargo's may be inside a harness worktree
// of this repo, where a folder is named after that repo. On a Mac that folder is behind a link, so the home here
// has two spellings.
#[test]
fn a_run_never_finds_a_copy_as_the_folder_its_wish_s_project_was_typed_in() {
    let w = world::world_in(tempfile::tempdir().unwrap());
    let repo = w.repo("alpha");
    let (alpha, zeta) = (w.wish("alpha"), w.wish("zeta"));
    let logs = w.at("harness").join("projects");
    let typed = |session: &str, folder: &Path, words: &str| {
        write_log(&logs, &format!("{session}/s.jsonl"), &[said(&at(0), text(folder), words)]);
    };
    typed("-work-alpha", &repo, "look at the uploader");
    // A folder of the project's name in a copy, under each spelling of the home, and one only a copy holds.
    let homes = [w.at("jm"), fs::canonicalize(w.at("jm")).unwrap()];
    for (n, home) in homes.iter().enumerate() {
        let copy = home.join("copies").join(format!("r{n:016x}"));
        typed(&format!("-copy-alpha-{n}"), &copy.join("alpha"), LEFT);
        typed(&format!("-copy-zeta-{n}"), &copy.join("zeta"), LEFT);
    }

    let none =
        "jellymoon: no folder of the name the wish gives was typed in on 2026-03-10; name the folder with --repo\n";
    assert_eq!(world::refused(&w.run(&["run", "--wish", &zeta, "--date", world::DAY])), none);
    let (_, said) = world::ran(&w.run(&["run", "--wish", &alpha, "--date", world::DAY]));
    assert_eq!(said["outcome"], "completed", "{said}");
    assert_eq!(w.branches(&repo).len(), 1, "the run was started in the one folder that is the person's");
    // `run show` looks the folder up the same way, and finds the same one.
    let shown = world::printed(&w.run(&["run", "show", "--wish", &alpha, "--date", world::DAY]));
    assert_eq!(shown["wish"], alpha.as_str(), "{shown}");
}
