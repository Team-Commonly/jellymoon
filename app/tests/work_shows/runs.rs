//! The day these tests start from and the wishes and runs they make: the `jellymoon` command with the made-up agent.

use std::{fs, thread};

use serde_json::Value;

use crate::support::day::DAY;
use crate::support::wished::{SAID, World, quiet, wait_until};

/// A day nothing was typed on, so no clash is on its page, with `alpha` listed as a person lists it.
pub fn day() -> World {
    let w = quiet();
    let repo = w.at("work").join("alpha");
    let added = w.jellymoon(&["project", "add", "--repo", repo.to_str().unwrap(), "--date", DAY]);
    assert_eq!(added["name"], "alpha", "{added}");
    fs::create_dir(w.at("harness")).unwrap();
    w
}

/// A wish made for `alpha`, as a person types it: its id.
pub fn wish(w: &World) -> String {
    let made = w.jellymoon(&["wish", "--date", DAY, "--say", SAID, "--project", "alpha"]);
    made["wish"]["id"].as_str().unwrap().to_string()
}

/// The wish's run, from its start to its end: what it ended as.
pub fn run(w: &World, id: &str) -> String {
    let logs = w.at("logs");
    let ran = w.jellymoon(&["run", "--wish", id, "--date", DAY, "--logs", logs.to_str().unwrap()]);
    ran["outcome"].as_str().unwrap().to_string()
}

/// A wish whose run ended with the made-up agent playing `script`.
pub fn ended(w: &World, script: &str, outcome: &str) -> String {
    w.play(script);
    let id = wish(w);
    assert_eq!(run(w, &id), outcome);
    id
}

/// How core keeps the made wish `id`.
pub fn state(w: &World, id: &str) -> String {
    let kept = fs::read_to_string(w.at("jm").join("pages").join(format!("{DAY}.wishes.json"))).unwrap_or_default();
    let kept: Value = serde_json::from_str(&kept).unwrap_or_default();
    let wishes = kept["wishes"].as_array().cloned().unwrap_or_default();
    wishes.iter().find(|wish| wish["id"] == id).map_or(String::new(), |wish| wish["state"].as_str().unwrap().into())
}

/// Runs `then` while the runs of `ids` go, each waiting for the test's word, which is given by the `go` handed to
/// `then`: it comes back when every one of those runs has ended, each with its one commit.
pub fn going(w: &World, ids: &[&str], then: impl FnOnce(&dyn Fn())) {
    let before = w.starts();
    w.play("slow");
    thread::scope(|scope| {
        let runs: Vec<_> = ids.iter().map(|id| scope.spawn(move || run(w, id))).collect();
        wait_until("the agents", || w.starts() == before + ids.len());
        wait_until("the runs", || ids.iter().all(|id| state(w, id) == "working"));
        let runs = std::cell::RefCell::new(runs);
        then(&|| {
            fs::write(w.at("rec").join("go"), "").unwrap();
            for run in runs.borrow_mut().drain(..) {
                assert_eq!(run.join().unwrap(), "completed");
            }
        });
        assert!(runs.borrow().is_empty(), "the test let the runs end");
    });
}
