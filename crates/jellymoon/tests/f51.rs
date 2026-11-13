//! F51: a run's session is kept under an id core chose, the harness's last word is data, and a run can ask a
//! question (ADR 0018, decisions 2, 3 and 6; `docs/development/run.md`, "A run that asks"). `jellymoon run` is run
//! as a black box against the made-up `fake-agent` in F39's test folder (`f39/world.rs`), in repos made with the
//! real `git`. This file holds a run that asks and the command line it was started with; `f51/words.rs` every last
//! word and what is kept of it, `f51/start.rs` a start that is not this run's and a harness too old,
//! `f51/after.rs` what a look, a stop, a show, a keep, a second run and a toss do with a run that asks, and
//! `f51/reader.rs` the reader, which leaves the kept session of a run out of a day.
#![allow(clippy::unwrap_used, clippy::expect_used)]

#[path = "f51/after.rs"]
mod after;
#[path = "f45/made.rs"]
mod kept;
#[path = "f51/reader.rs"]
mod reader;
#[path = "f51/start.rs"]
mod start;
mod support;
#[path = "f51/words.rs"]
mod words;
#[path = "f39/world.rs"]
mod world;

use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

pub use kept::{Held, said};
use serde_json::{Value, json};
use support::tree;
pub use world::*;

/// What the made-up agent asks when its question is a plain one.
pub const QUESTION: &str = "Which of the two uploaders did you mean?";
pub const ASKS: &str = "run {run} asks a question: its copy is kept as it is, and nothing was committed";
/// The line of a run that did not finish, around why.
pub const FAILED: (&str, &str) = ("run {run} did not finish (", "): its copy is kept and no star was added");

/// One run of the made-up agent's script, and what `jellymoon run` printed of it.
pub fn played(script: &str, number: usize) -> (Made, Value) {
    let w = world();
    w.play(script, number);
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let ran = w.start(&wish, &repo);
    let said = printed(&ran);
    w.never_says_the_agent(&ran);
    let run = said["run"].as_str().unwrap().to_string();
    (Made { w, repo, wish, run }, said)
}

/// A run that asks the plain question, with the work done in its copy first.
pub fn asking() -> Made {
    let (made, said) = played("asks", 1);
    assert_eq!(said["outcome"], "asks", "{said}");
    made
}

/// What a run that asks prints: its seven names and no other.
pub fn asks(made: &Made, question: &str) -> Value {
    let why = ASKS.replace("{run}", &made.run);
    let (run, wish) = (&made.run, &made.wish);
    json!({"run": run, "wish": wish, "outcome": "asks", "star": null, "denied": 0, "why": why, "question": question})
}

/// What a run that failed for `kind` prints: six names, and none for a question.
pub fn failed(made: &Made, kind: &str) -> Value {
    let why = format!("{}{kind}{}", FAILED.0.replace("{run}", &made.run), FAILED.1);
    json!({"run": made.run, "wish": made.wish, "outcome": "failed", "star": null, "denied": 0, "why": why})
}

impl Made {
    pub fn copy(&self) -> PathBuf {
        self.w.jm("copies").join(&self.run)
    }

    /// The file in the home that holds `words`, if any does. The made-up agent's own folders are not the home.
    pub fn keeps(&self, words: &str) -> Option<PathBuf> {
        let found = |bytes: &[u8]| bytes.windows(words.len()).any(|window| window == words.as_bytes());
        tree(&self.w.at("jm")).into_iter().find(|(_, bytes)| found(bytes)).map(|(path, _)| path)
    }
}

/// Waits for `done`, half a minute at most.
pub fn until(why: &str, mut done: impl FnMut() -> bool) {
    let from = Instant::now();
    while !done() {
        assert!(from.elapsed() < Duration::from_secs(30), "{why}");
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn a_run_that_asks_keeps_its_question_leaves_its_copy_as_it_is_and_commits_nothing() {
    let w = world();
    w.play("asks", 1);
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    w.work_in_progress(&repo);
    let (person, start) = (w.persons(&repo), w.commit(&repo, "main"));
    let ran = w.start(&wish, &repo);
    let said = printed(&ran);
    let made = Made { w, repo, wish, run: said["run"].as_str().unwrap().to_string() };
    let (w, run, line) = (&made.w, &made.run, ASKS.replace("{run}", &made.run));
    assert_eq!(said, asks(&made, QUESTION));
    w.never_says_the_agent(&ran);
    // The record says asks, with the question and the session, and names no end and no work.
    let record = made.record();
    let told = (&record["outcome"], &record["why"], &record["question"], &record["session"]);
    assert_eq!(told, (&json!("asks"), &json!(line), &json!(QUESTION), &json!(w.args().1)));
    assert_eq!((record.get("end"), record.get("work"), &record["start"]), (None, None, &json!(start)));
    // The wish needs the person and carries the question, on the page too. It got no star.
    let wish = json!({"id": made.wish, "said": SAID, "project": "alpha", "state": "needs_you", "stars": [],
        "run": run, "why": line, "question": QUESTION});
    assert_eq!(w.kept(&made.wish), wish);
    let on_page = w.on_page(&made.wish);
    assert_eq!(
        (&on_page["state"], &on_page["question"], &on_page["stars"]),
        (&wish["state"], &wish["question"], &json!([]))
    );
    // The copy is left as the agent left it: its edits are in it, and they are in no commit.
    let copy = made.copy();
    assert_eq!(w.copies(), std::slice::from_ref(run));
    assert_eq!(fs::read_to_string(copy.join("a.txt")).unwrap(), "edited by the run\n");
    assert!(copy.join("n.txt").is_file() && !copy.join("b.txt").exists());
    assert_eq!(w.git(&copy, &["status", "--porcelain"]).lines().count(), 3, "three changes, none committed");
    assert_eq!((w.commit(&made.repo, &made.branch()), w.branches(&made.repo).len()), (start, 1));
    // It is not one of the failed copies, and nothing holds the run: both its locks are gone.
    let kept = fs::read_to_string(w.jm("runs").join(run).join("copy.json")).unwrap();
    assert!(!serde_json::from_str::<Value>(&kept).unwrap()["failed"].is_u64(), "{kept}");
    for lock in ["run.lock", "start.lock"] {
        assert!(!w.jm("runs").join(run).join(lock).exists(), "{lock}");
    }
    assert!(person == w.persons(&made.repo), "nothing of the person's was changed");
    assert_eq!(w.starts(), 1);
}

// The control of the test above: the same work, ended with `done`, is committed and its copy goes.
#[test]
fn the_same_work_ended_with_done_is_one_commit_and_no_question_is_printed_or_kept() {
    let (made, said) = played("edits", 0);
    assert_eq!((&said["outcome"], said.as_object().unwrap().len()), (&json!("completed"), 6), "{said}");
    assert_ne!(made.w.commit(&made.repo, &made.branch()), made.w.commit(&made.repo, "main"));
    assert_eq!(made.w.copies(), [""; 0]);
    let (record, wish) = (made.record(), made.w.kept(&made.wish));
    assert_eq!(
        (record.get("question"), wish.get("question"), made.w.on_page(&made.wish).get("question")),
        (None, None, None)
    );
    assert_eq!(record["session"], json!(made.w.args().1));
}

// Every writing of a record is held in a test's build: at the first, the record names the session, and the
// harness has not been started yet. The id it is then started with is the one in the record.
#[test]
fn the_session_id_is_core_s_and_is_in_the_record_before_the_harness_is_started() {
    let w = world();
    w.play("asks", 0);
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let (go, at) = (w.at("rec").join("go.step"), w.at("rec").join("go.at"));
    let hold = [("JELLYMOON_TEST_RECORD_WAIT_MS", "60000".into()), ("JELLYMOON_TEST_GO", go.clone().into())];
    let mut going = w.going(&hold, &["run", "--wish", &wish, "--date", DAY, "--repo", repo.to_str().unwrap()]);
    let record = || {
        let runs = fs::read_dir(w.jm("runs")).into_iter().flatten().flatten().map(|run| run.path().join("run.json"));
        runs.filter_map(|file| serde_json::from_slice::<Value>(&fs::read(file).ok()?).ok()).next()
    };
    let mut seen: Vec<(Value, usize)> = Vec::new();
    until("the run never ended", || {
        if at.exists() && !go.exists() {
            seen.push((record().expect("a record is there at each of its writings"), w.starts()));
            fs::remove_file(&at).unwrap();
            fs::write(&go, "").unwrap();
        }
        going.try_wait().unwrap().is_some()
    });
    let (first, starts) = &seen[0];
    let id = first["session"].as_str().expect("the first writing names the session").to_string();
    assert!(is_session(&id) && first["outcome"].is_null() && first["pid"].is_null(), "{first}");
    assert_eq!(*starts, 0, "the harness was started before the record named its session");
    assert_eq!((w.args().1, w.starts()), (id.clone(), 1), "and it is the id the harness was given");
    // Every later writing names the same session, and only the last has the question.
    assert!(seen.iter().all(|(record, _)| record["session"] == json!(id)), "{seen:?}");
    let asked: Vec<bool> = seen.iter().map(|(record, _)| record.get("question").is_some()).collect();
    assert_eq!(asked.iter().filter(|asked| **asked).count(), 1, "{seen:?}");
    assert_eq!(seen.last().unwrap().0["question"], QUESTION);
    // No two runs are given one id.
    w.play("edits", 0);
    let second = w.wish("alpha");
    printed(&w.start(&second, &repo));
    assert_ne!(w.args().1, id);
}
