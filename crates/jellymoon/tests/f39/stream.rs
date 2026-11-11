//! The star's rule made strict, and the stream's first line (`docs/development/run.md`, "What is read of the
//! stream"). Finished needs exactly one result, with nothing unread after it; and before anything else of the
//! stream, the line that says the harness has the five tools, no MCP server and the permission mode of a run.

use std::fs;
use std::time::{Duration, Instant};

use serde_json::json;

use super::{STAR, ran, world};

const NOT_A_RUN: &str = "it did not start with the tools, servers and permission mode of a run";
const TWICE: &str = "it gave more than one result";
const UNREAD: &str = "a line that could not be read came after its result";
const NO_SUCCESS: &str = "its result was not a success";

fn failed(run: &str, kind: &str) -> String {
    format!("run {run} did not finish ({kind}): its copy is kept and no star was added")
}

#[test]
fn a_star_needs_one_result_that_says_no_error_and_nothing_unread_after_it() {
    let kinds = [
        ("after", 0, UNREAD, "a line that is not JSON after the success"),
        ("after", 1, UNREAD, "a line over the cap after the success"),
        ("after", 4, UNREAD, "a result written as a list after the success"),
        ("after", 2, TWICE, "two successes"),
        ("after", 3, TWICE, "a success, then a result that is not one"),
        ("after", 5, TWICE, "a result that is not a success, then a success"),
        ("unsaid", 0, NO_SUCCESS, "a result with no is_error"),
        ("unsaid", 1, NO_SUCCESS, "a result whose is_error is null"),
    ];
    let w = world();
    let repo = w.repo("alpha");
    let base = w.commit(&repo, "HEAD");
    let mut runs = Vec::new();
    for (script, number, kind, case) in kinds {
        let wish = w.wish("alpha");
        w.play(script, number);
        let ended = w.start(&wish, &repo);
        let (run, said) = ran(&ended);
        let why = failed(&run, kind);
        assert_eq!(said, json!({"wish": wish, "outcome": "failed", "star": null, "denied": 0, "why": why}), "{case}");
        assert_eq!(w.commit(&repo, &format!("jellymoon/run-{run}")), base, "{case}: nothing is committed");
        let kept = w.kept(&wish);
        assert_eq!((&kept["state"], &kept["stars"]), (&json!("needs_you"), &json!([])), "{case}");
        assert_eq!(w.on_page(&wish)["stars"], json!([]), "{case}");
        assert_eq!(w.record(&run)["outcome"], "failed", "{case}");
        w.never_says_the_agent(&ended);
        runs.push(run);
    }
    // Each failed run's copy was marked failed, so of the eight only the newest five are kept.
    let mut kept = w.copies();
    kept.sort();
    runs.drain(..3);
    runs.sort();
    assert_eq!(kept, runs);
}

#[test]
fn a_stream_that_does_not_start_as_a_run_s_does_is_stopped_at_once_and_is_never_a_star() {
    // 0 to 12 never end by themselves; 13 and 14 do their work and report a success before anything can stop them.
    for number in 0..=14 {
        let w = world();
        let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
        let base = w.commit(&repo, "HEAD");
        w.play("first", number);
        let from = Instant::now();
        let ended = w.start(&wish, &repo);
        let (run, said) = ran(&ended);
        let want =
            json!({"wish": wish, "outcome": "failed", "star": null, "denied": 0, "why": failed(&run, NOT_A_RUN)});
        assert_eq!(said, want, "start {number}");
        // The agent that sleeps for a minute was not waited for, nor given the five seconds of a stop that asks.
        assert!(from.elapsed() < Duration::from_secs(5), "start {number}: {:?}", from.elapsed());
        assert!(!w.at("rec").join("held").exists() || w.free("held"), "start {number}: the agent is still going");
        assert_eq!(w.commit(&repo, &format!("jellymoon/run-{run}")), base, "start {number}: nothing is committed");
        assert_eq!((&w.kept(&wish)["state"], &w.kept(&wish)["stars"]), (&json!("needs_you"), &json!([])));
        w.never_says_the_agent(&ended);
    }
    // What is compared is the three parts and nothing else: a line of the harness's own before it, the tools in
    // another order and parts nobody measured are still the start of a run.
    let w = world();
    let repo = w.repo("alpha");
    w.play("first", 15);
    let (_, said) = ran(&w.start(&w.wish("alpha"), &repo));
    assert_eq!((&said["outcome"], &said["star"]), (&json!("completed"), &json!(STAR)));
}

// "Too old" is a harness that printed no line of a stream. One that worked and then fails in those words has
// failed: its copy, with the work in it, is kept.
#[test]
fn a_harness_that_worked_and_then_fails_in_the_words_of_one_too_old_is_a_failed_run_with_its_copy() {
    let w = world();
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    w.play("worked-old", 0);
    let (run, said) = ran(&w.start(&wish, &repo));
    let why = failed(&run, "Claude Code ended without a result");
    assert_eq!(said, json!({"wish": wish, "outcome": "failed", "star": null, "denied": 0, "why": why}));
    assert_eq!(fs::read_to_string(w.jm("copies").join(&run).join("a.txt")).unwrap(), "edited by the run\n");
    assert_eq!((w.branches(&repo), &w.kept(&wish)["run"]), (vec![format!("jellymoon/run-{run}")], &json!(run)));
}
