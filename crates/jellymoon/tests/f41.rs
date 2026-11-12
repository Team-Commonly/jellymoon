//! F41: a wish made with `--run` starts its own run, which outlives the command that asked for it
//! (`docs/development/run.md`, "A run started apart"). `jellymoon wish --run` is run as a black box in F39's test
//! folder (`f39/world.rs`), with the made-up `fake-agent` as the harness, so the real Claude Code is never started.
//! The repo is found by its name in a session log typed here, so every world is in the system's own temporary
//! folder: Cargo's may be inside a harness worktree of this repo, and a folder there is read as the repo's own.
//! `f41/apart.rs` holds the run that outlives its asker, its end, a stop and a run cut off; `f41/refused.rs` what
//! starts no run; `f41/cap.rs` the three runs that may go at once; `f41/named.rs` the folder a name leads to;
//! `f41/never.rs` a harness that never begins a run; `f41/left.rs` what a killed command left; `f41/line.rs`
//! the command a run is started with, and what an asker is told of one that started none.
#![allow(clippy::unwrap_used, clippy::expect_used)]

#[path = "f41/apart.rs"]
mod apart;
#[path = "f41/cap.rs"]
mod cap;
#[path = "f41/left.rs"]
mod left;
#[path = "f41/line.rs"]
mod line;
#[path = "f41/named.rs"]
mod named;
#[path = "f41/never.rs"]
mod never;
#[path = "f41/refused.rs"]
mod refused;
mod support;
#[path = "f39/world.rs"]
mod world;

use std::path::Path;
use std::time::{Duration, Instant};
use std::{fs, thread};

use serde_json::{Value, json};
use support::{said, write_log};
pub use world::*;

/// A world whose repos a session log can name. Whoever asks for a run apart there waits a minute for a harness
/// that has said nothing (`JELLYMOON_TEST_APART_QUIET_MS`), where a release waits three seconds: no test is told
/// that a run is going only because the machine was slow to say that it never started.
pub fn found_world() -> World {
    quiet_for("60000")
}

/// A world for an agent that says nothing before it waits: its asker is told that the run is going after a
/// tenth of a second of that silence.
pub fn silent_world() -> World {
    quiet_for("100")
}

fn quiet_for(ms: &str) -> World {
    let mut world = world_in(tempfile::tempdir().unwrap());
    world.vars.push(("JELLYMOON_TEST_APART_QUIET_MS", ms.into()));
    world
}

/// The wish as `jellymoon wish` prints it when it is made.
pub fn made(id: &str, project: &str) -> Value {
    json!({"id": id, "said": SAID, "project": project, "state": "working", "stars": []})
}

impl World {
    /// A session log, in the harness's own folder, of one thing typed in `folder` on the day.
    pub fn typed_in(&self, session: &str, folder: &Path) {
        let line = said("2026-03-10T16:00:00.000Z", folder.to_str().unwrap(), "look at the uploader");
        write_log(&self.at("harness").join("projects"), &format!("{session}/s.jsonl"), &[line]);
    }

    /// `jellymoon wish --run` for a project on the day, with more arguments: what it printed.
    pub fn wish_run(&self, project: &str, more: &[&str]) -> Value {
        let args = ["wish", "--date", DAY, "--say", SAID, "--project", project, "--run"];
        printed(&self.run(&[&args[..], more].concat()))
    }

    /// Waits for a run's record to have its end, half a minute at most, and returns the record.
    pub fn ended_as(&self, run: &str) -> Value {
        let from = Instant::now();
        loop {
            let record = fs::read_to_string(self.jm("runs").join(run).join("run.json")).unwrap_or_default();
            let record = serde_json::from_str::<Value>(&record).unwrap_or_default();
            if !record["outcome"].is_null() {
                return record;
            }
            assert!(from.elapsed() < Duration::from_secs(30), "run {run} never ended");
            thread::sleep(Duration::from_millis(10));
        }
    }

    /// Waits until nothing is at a run's lock: its command took it away as it ended.
    pub fn command_gone(&self, run: &str) {
        let from = Instant::now();
        while self.jm("runs").join(run).join("run.lock").exists() {
            assert!(from.elapsed() < Duration::from_secs(30), "the command of run {run} never ended");
            thread::sleep(Duration::from_millis(10));
        }
    }

    /// Nothing of a run is in Jellymoon's home, and the agent was never started.
    pub fn nothing_was_started(&self, wish: &str, project: &str) {
        let runs = fs::read_dir(self.jm("runs")).into_iter().flatten();
        let runs = runs.map(|entry| entry.unwrap().file_name().into_string().unwrap());
        let runs: Vec<String> = runs.filter(|name| name != "no-hooks" && !name.contains("lock")).collect();
        assert_eq!((self.copies(), runs, self.starts()), (vec![], vec![], 0));
        assert_eq!(self.kept(wish), made(wish, project), "the wish is kept as it was made");
    }
}

#[test]
fn wish_takes_logs_and_skip_only_with_run_and_never_a_repo() {
    let w = world();
    let cases: [&[&str]; 3] = [&["--logs", "x"], &["--skip", "x"], &["--run", "--repo", "x"]];
    for more in cases {
        let ran = w.run(&[&["wish", "--date", DAY, "--say", SAID, "--project", "alpha"][..], more].concat());
        assert_eq!((ran.code, ran.stdout.as_str()), (2, ""), "{more:?}: {}", ran.stderr);
        assert!(ran.stderr.contains("jellymoon wish --say <your words> --project <folder name>"), "{more:?}");
    }
    assert!(!w.at("jm").exists(), "a command that is not one makes nothing");
}

// A wish core refuses is refused as it is with no `--run`, and nothing is started.
#[test]
fn a_wish_core_refuses_is_refused_with_run_too_and_starts_nothing() {
    let w = found_world();
    w.typed_in("-work-alpha", &w.repo("alpha"));
    let ran = w.run(&["wish", "--date", DAY, "--say", " ", "--project", "alpha", "--run"]);
    assert_eq!(refused(&ran), "jellymoon: a wish needs words, and nothing is left of these once cleaned\n");
    assert!(!w.at("jm").exists() && w.starts() == 0, "a refused wish made something");
}
