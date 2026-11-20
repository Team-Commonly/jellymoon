//! What a run is doing, kept while it works (live work; `docs/development/run.md`, "What is read of the stream"):
//! the made-up agent reads one file, waits, and changes another. Core says the first step while it waits and both
//! once the run has ended, and keeps nothing of the agent's text and no path of the machine.

use std::fs;
use std::time::{Duration, Instant};

use jellymoon_core::{Step, Steps, Tool, doing};
use serde_json::json;

use super::{DAY, MARKER, ended, ran, wait_for, world};

fn step(tool: Tool, file: &str) -> Step {
    Step { tool, file: Some(file.to_string()) }
}

#[test]
fn a_run_s_steps_are_what_core_says_while_it_works_and_are_kept_when_it_ends() {
    let w = world();
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let home = w.at("jm");
    assert!(doing(&home, DAY).is_empty(), "no run, no steps");
    w.play("steps", 0);
    let going = w.start_going(&wish, &repo);
    wait_for(&w.at("rec").join("started"));
    let first = vec![(wish.clone(), Steps { count: 1, last: vec![step(Tool::Read, "b.txt")] })];
    let deadline = Instant::now() + Duration::from_secs(30);
    while doing(&home, DAY) != first {
        assert!(Instant::now() < deadline, "the first step never came: {:?}", doing(&home, DAY));
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(w.kept(&wish)["state"], "working", "the run is held mid-way");
    fs::write(w.at("rec").join("go"), "").unwrap();
    let (run, said) = ran(&ended(going));
    assert_eq!(said["outcome"], json!("completed"));
    let both = Steps { count: 2, last: vec![step(Tool::Read, "b.txt"), step(Tool::Edit, "a.txt")] };
    assert_eq!(doing(&home, DAY), [(wish.clone(), both)], "kept as they were at the end");
    // The whole file: no word of the agent's (`MARKER`) and no path of the machine.
    let kept = fs::read_to_string(w.jm("runs").join(&run).join("steps.json")).unwrap();
    let (read, edit) = (r#"{"tool":"read","file":"b.txt"}"#, r#"{"tool":"edit","file":"a.txt"}"#);
    assert_eq!(kept, format!("{{\"count\":2,\"last\":[{read},{edit}]}}\n"));
    assert!(!kept.contains(MARKER));
    // Steps are a wish's only when its run's record is of that wish and that day.
    let record = w.jm("runs").join(&run).join("run.json");
    let written = fs::read_to_string(&record).unwrap();
    for (part, other) in [("wish", "m9"), ("day", "2026-03-11")] {
        let mut changed: serde_json::Value = serde_json::from_str(&written).unwrap();
        changed[part] = json!(other);
        fs::write(&record, changed.to_string()).unwrap();
        assert!(doing(&home, DAY).is_empty(), "a record of another {part}");
    }
    fs::write(&record, &written).unwrap();
    assert_eq!(doing(&home, DAY).len(), 1);
}
