//! What the agent asked for outside its list is denied by the harness and counted from the stream's first result
//! line (`docs/development/run.md`, "What is denied"). F39 counts it on a run that finished. Here: on a run that
//! failed, and on one that was stopped after its result came. A denial is never an allow: a run is started with
//! the measured list (`f40.rs`), and a start that names one tool more is no run (F39, `f39/stream.rs`).

use serde_json::json;

use super::{ended, printed, ran, said_cancelled, world};

#[test]
fn what_was_denied_is_counted_on_a_run_that_failed() {
    let w = world();
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    w.play("denied-failed", 3);
    let failed = w.start(&wish, &repo);
    let (run, said) = ran(&failed);
    let why =
        format!("run {run} did not finish (its result was not a success): its copy is kept and no star was added");
    assert_eq!(said, json!({"wish": wish, "outcome": "failed", "star": null, "denied": 3, "why": why}));
    assert_eq!((&w.record(&run)["denied"], &w.kept(&wish)["stars"]), (&json!(3), &json!([])));
    // A stop asked for afterwards prints the same count, from the record.
    assert_eq!(printed(&w.stop(&wish)), printed(&failed));
    w.never_says_the_agent(&failed);
}

// The result came, said success and counted two denials, and the agent was still going when the stop was asked: the
// run is cancelled, whatever its result claimed, and the two are counted.
#[test]
fn what_was_denied_is_counted_on_a_run_that_was_stopped_after_its_result_and_no_star_is_added() {
    let w = world();
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let (going, run) = w.run_going(("lingers", 2), &[], &wish, &repo);
    let stopped = w.stop(&wish);
    assert_eq!(printed(&stopped), said_cancelled(&run, &wish, 2));
    assert_eq!(printed(&ended(going)), said_cancelled(&run, &wish, 2));
    w.is_cancelled(&wish, &run, &repo, 2);
    w.never_says_the_agent(&stopped);
}

// A stream that names a tool outside the list after its start is not a run's: no star, though its result says
// success. What is counted is the first result's, and this stream is not read that far.
#[test]
fn a_run_whose_stream_names_one_tool_more_than_the_list_earns_no_star() {
    let w = world();
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    w.play("again", 0);
    let (run, said) = ran(&w.start(&wish, &repo));
    assert_eq!((&said["outcome"], &said["star"], &said["denied"]), (&json!("failed"), &json!(null), &json!(0)));
    let kind = "it did not start with the tools, servers and permission mode of a run";
    assert!(said["why"].as_str().unwrap().contains(kind), "{said}");
    assert_eq!((&w.kept(&wish)["stars"], w.copies()), (&json!([]), vec![run]));
}
