//! The runs that do not finish, and the two caps on the stream. Finished means all three of: a result that says
//! success, `is_error` false, and exit 0. Each script of the made-up agent breaks one of them.

use std::fs;

use serde_json::json;

use super::{DAY, SAID, STAR, ran, world};

/// The longest line of the stream that is read, with its line break, and the most bytes of a whole stream.
const LINE_MAX: usize = 1 << 20;
const STREAM_MAX: usize = 64 << 20;

#[test]
fn a_run_that_does_not_report_a_success_and_exit_0_adds_no_star_and_keeps_its_copy() {
    let kinds = [
        ("is-error", "its result was not a success"),
        ("not-success", "its result was not a success"),
        ("error-as-text", "Claude Code ended without a result"),
        ("no-result", "Claude Code ended without a result"),
        ("exit-3", "Claude Code exited with a failure after its result"),
    ];
    let w = world();
    let repo = w.repo("alpha");
    let (base, before) = (w.commit(&repo, "HEAD"), w.persons(&repo));
    for (script, kind) in kinds {
        let wish = w.wish("alpha");
        w.play(script, 0);
        let failed = w.start(&wish, &repo);
        let (run, said) = ran(&failed);
        let why = format!("run {run} did not finish ({kind}): its copy is kept and no star was added");
        assert_eq!(said, json!({"wish": wish, "outcome": "failed", "star": null, "denied": 0, "why": why}), "{script}");
        // The copy is kept with what the agent left in it, and nothing is committed.
        let copy = w.jm("copies").join(&run);
        assert_eq!(fs::read_to_string(copy.join("a.txt")).unwrap(), "edited by the run\n", "{script}");
        assert_eq!(w.commit(&repo, &format!("jellymoon/run-{run}")), base, "{script}");
        assert_eq!(w.persons(&repo), before, "{script}");
        let kept = json!({"id": wish, "said": SAID, "project": "alpha", "state": "needs_you", "stars": [], "run": run, "why": why});
        assert_eq!(w.kept(&wish), kept, "{script}");
        let on_page = w.on_page(&wish);
        assert_eq!((&on_page["state"], &on_page["stars"]), (&json!("needs_you"), &json!([])), "{script}");
        let record = w.record(&run);
        assert_eq!((&record["outcome"], &record["day"]), (&json!("failed"), &json!(DAY)), "{script}");
        w.never_says_the_agent(&failed);
    }
    assert_eq!(w.copies().len(), 5, "one kept copy for each failed run");
}

#[test]
fn a_line_over_the_cap_is_skipped_and_a_result_over_it_is_no_result() {
    let w = world();
    let repo = w.repo("alpha");
    w.play("long-line", LINE_MAX + 1);
    let (run, said) = ran(&w.start(&w.wish("alpha"), &repo));
    assert_eq!((&said["outcome"], &w.record(&run)["skipped"]), (&json!("completed"), &json!(1)));

    w.play("padded-result", LINE_MAX);
    let (run, said) = ran(&w.start(&w.wish("alpha"), &repo));
    assert_eq!((&said["outcome"], &said["star"]), (&json!("completed"), &json!(STAR)), "a result at the cap");
    assert_eq!(w.record(&run)["skipped"], 0);

    w.play("padded-result", LINE_MAX + 1);
    let (run, said) = ran(&w.start(&w.wish("alpha"), &repo));
    assert_eq!((&said["outcome"], &said["star"]), (&json!("failed"), &json!(null)), "a result one byte over");
    assert!(said["why"].as_str().unwrap().contains("Claude Code ended without a result"), "{said}");
    assert_eq!(w.record(&run)["skipped"], 1);
}

#[test]
fn a_stream_at_the_cap_is_read_whole_and_one_byte_more_stops_the_run() {
    let w = world();
    let repo = w.repo("alpha");
    w.play("fill", STREAM_MAX);
    let (_, said) = ran(&w.start(&w.wish("alpha"), &repo));
    assert_eq!((&said["outcome"], &said["star"]), (&json!("completed"), &json!(STAR)), "a stream at the cap: {said}");

    w.play("fill", STREAM_MAX + 1);
    let over = w.start(&w.wish("alpha"), &repo);
    let (run, said) = ran(&over);
    let why = format!(
        "run {run} did not finish (it printed more than Jellymoon reads of a run): its copy is kept and no star was added"
    );
    assert_eq!((&said["outcome"], &said["star"], &said["why"]), (&json!("failed"), &json!(null), &json!(why)));
    assert!(w.jm("copies").join(&run).join("a.txt").is_file());
}
