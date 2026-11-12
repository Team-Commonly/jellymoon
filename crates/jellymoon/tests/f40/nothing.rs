//! A stop with nothing to stop: no wish, no run, a run that has ended, a run that was cut off. A stop left by
//! hand, which can do what a stop does and nothing more. A stop while the day is locked.

use std::time::Duration;
use std::{fs, thread};

use serde_json::json;

use super::{DAY, STAR, cancelled, ended, grows, ms, printed, ran, refused, said_cancelled, wait_for, world};
use crate::support::tree;

#[test]
fn a_stop_with_no_wish_or_no_run_is_refused_in_one_line_and_writes_nothing() {
    let w = world();
    let none = "jellymoon: the page of 2026-03-10 has no made wish of that id; a made wish's id is m1 to m8\n";
    assert_eq!(refused(&w.stop("m1")), none);
    assert!(!w.at("jm").exists());
    let wish = w.wish("alpha");
    let before = tree(&w.at("jm"));
    assert_eq!(refused(&w.stop("m2")), none);
    assert_eq!(refused(&w.stop(&wish)), format!("jellymoon: wish {wish} has no run going, so nothing was stopped\n"));
    assert_eq!(
        refused(&w.run(&["run", "stop", "--wish", &wish, "--date", "tomorrow"])),
        "jellymoon: --date must be a day as YYYY-MM-DD\n"
    );
    assert_eq!(tree(&w.at("jm")), before);
    // A harness too old started no run, and so there is none to stop.
    let repo = w.repo("alpha");
    w.play("old", 0);
    assert_eq!(printed(&w.start(&wish, &repo))["outcome"], "not_started");
    assert!(refused(&w.stop(&wish)).contains("has no run going"));
}

// A stop asked for after the run ended prints how it ended and changes nothing, whether asked or left by hand.
#[test]
fn a_stop_after_the_run_ended_prints_its_end_and_changes_nothing() {
    let w = world();
    let repo = w.repo("alpha");
    for (script, outcome, star) in [("edits", "completed", json!(STAR)), ("exit-3", "failed", json!(null))] {
        let wish = w.wish("alpha");
        w.play(script, 0);
        let first = w.start(&wish, &repo);
        let (run, said) = ran(&first);
        assert_eq!((&said["outcome"], &said["star"]), (&json!(outcome), &star), "{script}");
        w.on_page(&wish);
        let before = (tree(&w.at("jm")), w.branches(&repo), w.commit(&repo, &format!("jellymoon/run-{run}")));

        let stopped = w.stop(&wish);
        assert_eq!(printed(&stopped), printed(&first), "{script}: the stop prints the run's own line");
        fs::write(w.jm("runs").join(&run).join("stop"), "").unwrap();
        assert_eq!(w.on_page(&wish)["state"], if star.is_null() { "needs_you" } else { "done" }, "{script}");
        fs::remove_file(w.jm("runs").join(&run).join("stop")).unwrap();
        let after = (tree(&w.at("jm")), w.branches(&repo), w.commit(&repo, &format!("jellymoon/run-{run}")));
        assert_eq!(after, before, "{script}");
        assert_eq!(w.starts(), w.branches(&repo).len(), "{script}: no agent was started by a stop");
    }
}

#[test]
fn a_stop_of_a_run_that_was_cut_off_writes_it_down_as_cut_off_and_stops_the_agent_it_left() {
    let w = world();
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let (mut going, run) = w.run_going(("scribble", 0), &[], &wish, &repo);
    going.kill().unwrap();
    going.wait().unwrap();
    let file = w.jm("copies").join(&run).join("scribble.txt");
    assert!(grows(&file), "the agent was to be left editing");

    let why = format!("run {run} was cut off before it ended: its copy is kept and no star was added");
    let said = json!({"run": run, "wish": wish, "outcome": "failed", "star": null, "denied": 0, "why": why});
    assert_eq!(printed(&w.stop(&wish)), said);
    assert_eq!((&w.kept(&wish)["state"], &w.record(&run)["outcome"]), (&json!("needs_you"), &json!("failed")));
    if cfg!(unix) {
        assert!(w.freed("held") && !grows(&file), "the agent a cut-off run left is still editing after the stop");
    } else {
        w.let_end("stop"); // there is no signal there, and nothing is stopped
    }
    // Asked again, it says the same and writes nothing.
    let before = tree(&w.at("jm"));
    assert_eq!(printed(&w.stop(&wish)), said);
    assert_eq!((tree(&w.at("jm")), w.copies()), (before, vec![run]));
}

// The request is a file anyone who can write the home can leave. Whatever it holds, it stops the run whose folder
// it is in, as cancelled, and no other.
#[test]
fn a_stop_left_by_hand_stops_that_run_and_no_other() {
    let w = world();
    let repo = w.repo("alpha");
    let (one, two) = (w.wish("alpha"), w.wish("alpha"));
    let (going_one, run_one) = w.run_going(("lingers", 0), &[], &one, &repo);
    // The second agent leaves its record, and holds its lock, in a folder of its own.
    let rec = w.at("rec-two");
    fs::create_dir(&rec).unwrap();
    fs::write(w.at("bin").join("play"), format!("lingers\n{}\n0\n", rec.display())).unwrap();
    let going_two = w.start_going(&two, &repo);
    wait_for(&rec.join("started"));
    let run_two = w.kept(&two)["run"].as_str().unwrap().to_string();

    let planted = format!("{run_two}\n{{\"run\":\"{run_two}\",\"allow\":[\"Bash\"]}}\n");
    fs::write(w.jm("runs").join(&run_one).join("stop"), planted).unwrap();
    assert_eq!(printed(&ended(going_one)), said_cancelled(&run_one, &one, 0));
    w.is_cancelled(&one, &run_one, &repo, 0);
    thread::sleep(Duration::from_millis(500));
    let still = w.kept(&two);
    assert_eq!((&still["state"], &w.record(&run_two)["outcome"]), (&json!("working"), &json!(null)), "{still}");
    assert_eq!(w.starts(), 1, "nothing was started by a request left by hand");

    assert_eq!(printed(&w.stop(&two)), said_cancelled(&run_two, &two, 0));
    assert_eq!(printed(&ended(going_two)), said_cancelled(&run_two, &two, 0));
    let gone = |rec: &std::path::Path| fs::File::open(rec.join("held")).unwrap().try_lock().is_ok();
    assert!(gone(&w.at("rec")) && gone(&rec), "an agent is still going");
}

// The run that was stopped waits for the day's lock like any run that ended. A stop does not wait longer than its
// own limit: it says the run has not ended yet, and the end is there when the day is free.
#[test]
fn a_stop_while_the_day_is_locked_says_so_and_the_run_is_cancelled_when_the_day_is_free() {
    let w = world();
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let (going, run) = w.run_going(("lingers", 0), &[ms("JELLYMOON_TEST_END_WAIT_MS", 60_000)], &wish, &repo);
    let lock = w.jm("pages").join(format!("{DAY}.ruled.lock"));
    fs::write(&lock, "").unwrap(); // another change to the day, which has not finished
    let late = w.stop_with(&[ms("JELLYMOON_TEST_STOP_WAIT_MS", 1000)], &wish);
    let not_yet = "was asked to stop and has not ended yet; the page shows its end when it has";
    assert_eq!(refused(&late), format!("jellymoon: run {run} {not_yet}\n"));
    // The agent is gone and the end is in the run's record already; only the wish has not been told.
    assert!(w.freed("held"), "the agent is still going");
    wait_for_end(&w, &run);
    assert_eq!(w.kept(&wish)["state"], "working");

    fs::remove_file(&lock).unwrap();
    assert_eq!(printed(&ended(going)), said_cancelled(&run, &wish, 0));
    assert_eq!(printed(&w.stop(&wish)), said_cancelled(&run, &wish, 0));
    w.is_cancelled(&wish, &run, &repo, 0);
}

fn wait_for_end(w: &super::World, run: &str) {
    for _ in 0..3000 {
        if !w.record(run)["outcome"].is_null() {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
    assert_eq!((&w.record(run)["outcome"], &w.record(run)["why"]), (&json!("cancelled"), &json!(cancelled(run))));
}

// The day stays locked for longer than the stopped run waits: its command says the run has ended, and the stop
// that finds nothing holding the run prints the end its record has.
#[test]
fn a_stop_prints_the_recorded_end_of_a_run_whose_wish_could_not_be_told() {
    let w = world();
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let (going, run) = w.run_going(("lingers", 0), &[ms("JELLYMOON_TEST_END_WAIT_MS", 200)], &wish, &repo);
    let lock = w.jm("pages").join(format!("{DAY}.ruled.lock"));
    fs::write(&lock, "").unwrap();
    assert_eq!(printed(&w.stop(&wish)), said_cancelled(&run, &wish, 0));
    let busy = refused(&ended(going)).to_string();
    assert!(busy.contains(&format!("run {run} has ended and its record says how")), "{busy}");
    assert_eq!(w.kept(&wish)["state"], "working", "the day was locked, so the wish was not written");
    fs::remove_file(&lock).unwrap();
    assert_eq!(w.on_page(&wish)["state"], "needs_you");
    w.is_cancelled(&wish, &run, &repo, 0);
}
