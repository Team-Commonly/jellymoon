//! Three runs go at once and no more (`docs/development/run.md`, "Three runs at once").

use std::ffi::OsString;
use std::process::Child;
use std::time::{Duration, Instant};
use std::{fs, thread};

use serde_json::json;

use super::{DAY, SAID, World, ended, found_world, made, printed, refused};

const FULL: &str = "nothing was started: 3 runs are going, and no more than 3 go at once; run the wish again when \
                    one has ended";

/// How many runs a command that lives holds: the locks of `runs` that cannot be taken.
fn going(w: &World) -> usize {
    let runs = fs::read_dir(w.jm("runs")).unwrap().map(|entry| entry.unwrap().path().join("run.lock"));
    runs.filter(|lock| fs::File::open(lock).is_ok_and(|lock| lock.try_lock().is_err())).count()
}

#[test]
fn a_fourth_run_while_three_go_is_refused_with_no_copy_made_and_taken_when_one_has_ended() {
    let w = found_world();
    let repo = w.repo("alpha");
    w.typed_in("-work-alpha", &repo);
    w.play("slow", 0);
    let runs: Vec<String> =
        (0..3).map(|_| w.wish_run("alpha", &[])["run"].as_str().expect("no run was started").to_string()).collect();
    assert_eq!((going(&w), w.copies().len(), w.branches(&repo).len()), (3, 3, 3));

    let fourth = w.wish_run("alpha", &[]);
    assert_eq!(fourth, json!({"wish": made("m4", "alpha"), "run": null, "why": FULL}));
    assert_eq!((w.kept("m4"), w.copies().len(), w.branches(&repo).len()), (made("m4", "alpha"), 3, 3));
    // `jellymoon run` for it is refused in the same line, and makes no copy either.
    let waited = w.run(&["run", "--wish", "m4", "--date", DAY]);
    assert_eq!((refused(&waited), w.copies().len()), (format!("jellymoon: {FULL}\n").as_str(), 3));

    // One of the three is stopped: the fourth is taken.
    assert_eq!(printed(&w.run(&["run", "stop", "--wish", "m1", "--date", DAY]))["outcome"], "cancelled");
    w.command_gone(&runs[0]);
    let taken = w.going(&[], &["run", "--wish", "m4", "--date", DAY]);
    let from = Instant::now();
    while w.kept("m4")["run"].is_null() {
        assert!(from.elapsed() < Duration::from_secs(20), "the fourth run was not taken when one had ended");
        thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(going(&w), 3);
    fs::write(w.at("rec").join("go"), "").unwrap();
    assert_eq!(printed(&ended(taken))["outcome"], "completed");
    for run in &runs[1..] {
        assert_eq!(w.ended_as(run)["outcome"], "completed");
        w.command_gone(run);
    }
}

// The count and the claim are one step: of six `jellymoon run` started at once, three hold a run and three are
// refused. Each wish is of a day of its own, so that no two wait for the same day's lock.
#[test]
fn of_six_runs_started_at_once_exactly_three_go() {
    let w = found_world();
    let repo = w.repo("alpha");
    w.play("slow", 0);
    let days: Vec<String> = (10..16).map(|day| format!("2026-03-{day}")).collect();
    for day in &days {
        printed(&w.run(&["wish", "--date", day, "--say", SAID, "--project", "alpha"]));
    }
    // Each waits between its count and its claim, so a count that was not behind the lock would let all six in.
    let wait = [("JELLYMOON_TEST_CLAIM_WAIT_MS", OsString::from("200"))];
    let start =
        |day: &String| w.going(&wait, &["run", "--wish", "m1", "--date", day, "--repo", repo.to_str().unwrap()]);
    let mut started: Vec<Child> = days.iter().map(start).collect();
    // The three that are refused end; the three that hold a run wait with their agents.
    let from = Instant::now();
    let over = |started: &mut Vec<Child>| started.iter_mut().filter_map(|run| run.try_wait().unwrap()).count();
    while over(&mut started) < 3 || going(&w) < 3 {
        assert!(from.elapsed() < Duration::from_secs(30), "{} ended, {} go", over(&mut started), going(&w));
        thread::sleep(Duration::from_millis(10));
    }
    thread::sleep(Duration::from_millis(300));
    assert_eq!((over(&mut started), going(&w), w.copies().len()), (3, 3, 3), "more than three runs went at once");
    fs::write(w.at("rec").join("go"), "").unwrap();
    let said: Vec<_> = started.into_iter().map(ended).collect();
    let full = format!("jellymoon: {FULL}\n");
    assert_eq!(said.iter().filter(|said| said.code == 1 && said.stderr == full && said.stdout.is_empty()).count(), 3);
    let done = said.iter().filter(|said| said.code == 0).map(|said| printed(said)["outcome"].clone());
    assert_eq!(done.collect::<Vec<_>>(), [json!("completed"), json!("completed"), json!("completed")]);
    assert_eq!((w.copies().len(), w.branches(&repo).len(), w.starts()), (0, 3, 3));
}
