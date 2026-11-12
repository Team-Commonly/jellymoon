//! A look that comes after a finished run's copy went and before the end is in the run's record
//! (`docs/development/run.md`, "The end is written in the record first"). The run is made to wait there.

use std::time::{Duration, Instant};
use std::{fs, thread};

use serde_json::json;

use super::closed::closed;
use super::{DAY, STAR, World, ended, ms, printed, refused, until, world};
use crate::support::skipped;

/// How long a build with debug assertions waits between the end of its copy and the end in its record.
const NOTE: &str = "JELLYMOON_TEST_NOTE_WAIT_MS";

/// Lets the agent of a run finish, and returns once the run's copy is gone: the run waits then, its end unwritten.
fn copy_gone(w: &World, run: &str) {
    fs::write(w.at("rec").join("go"), "").unwrap();
    let from = Instant::now();
    while w.jm("runs").join(run).join("copy.json").exists() || w.jm("copies").join(run).exists() {
        assert!(from.elapsed() < Duration::from_secs(20), "the copy of the run never went");
        thread::sleep(Duration::from_millis(5));
    }
    assert!(w.record(run)["outcome"].is_null(), "the run did not wait before it wrote its end");
}

// The run's lock is closed by hand, so a look takes the run for one nothing holds while it still goes.
#[test]
fn a_look_that_finds_the_copy_gone_and_no_end_yet_waits_for_the_end_the_run_writes() {
    let w = world();
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let (going, run) = w.run_going(("slow", 0), &[ms(NOTE, 60_000), until(&w, "noted")], &wish, &repo);
    if !closed(&w.jm("runs").join(&run).join("run.lock")) {
        skipped("no mode keeps this user from opening a file here: the system has none, or the user is root");
        ["go", "noted"].iter().for_each(|f| fs::write(w.at("rec").join(f), "").unwrap()); // `noted` ends the hold
        ended(going);
        return;
    }
    copy_gone(&w, &run);
    let look = w.going(&[], &["page", "--date", DAY, "--logs", w.at("logs").to_str().unwrap()]);
    assert!(w.record(&run)["outcome"].is_null(), "the run had written its end when the look was started");
    // The run writes its end a little after the look was started: a look that has found no end by then waits.
    thread::sleep(Duration::from_millis(300));
    fs::write(w.at("rec").join("noted"), "").unwrap();
    let drawn = printed(&ended(look))["wishes"][0].clone();
    assert_eq!((&drawn["state"], &drawn["stars"]), (&json!("done"), &json!([STAR])), "the look drew {drawn}");
    ended(going); // it or the look told the wish, whichever came first: the other changed nothing
    let (kept, record) = (w.kept(&wish), w.record(&run));
    assert_eq!((&kept["state"], &kept["stars"], &kept["why"]), (&json!("done"), &json!([STAR]), &record["why"]));
    assert_eq!(record["outcome"], "completed");
}

// The run writes its end later than a look waits for it: the look told the wish, and the run says so.
#[test]
fn a_run_that_ends_after_a_look_told_its_wish_says_that_a_look_told_it_first() {
    let w = world();
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    // The run waits until the look has told the wish, however long a busy machine takes to draw the page.
    let (going, run) = w.run_going(("slow", 0), &[ms(NOTE, 60_000), until(&w, "noted")], &wish, &repo);
    if !closed(&w.jm("runs").join(&run).join("run.lock")) {
        skipped("no mode keeps this user from opening a file here: the system has none, or the user is root");
        ["go", "noted"].iter().for_each(|f| fs::write(w.at("rec").join(f), "").unwrap()); // `noted` ends the hold
        ended(going);
        return;
    }
    copy_gone(&w, &run);
    assert_eq!(w.on_page(&wish)["state"], "needs_you", "the look waited longer than a second for the end");
    let kept = w.kept(&wish);
    fs::write(w.at("rec").join("noted"), "").unwrap();
    let said = format!(
        "jellymoon: run {run} has ended; a look had taken it for cut off and told its wish first, so the wish \
         shows what that look wrote\n"
    );
    assert_eq!(refused(&ended(going)), said);
    assert_eq!(w.kept(&wish), kept, "the run changed a wish a look had told");
}

#[test]
fn a_run_cut_off_after_its_copy_went_is_written_down_in_a_line_that_does_not_say_the_copy_is_kept() {
    let w = world();
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let (mut going, run) = w.run_going(("slow", 0), &[ms(NOTE, 60_000)], &wish, &repo);
    copy_gone(&w, &run);
    going.kill().unwrap();
    going.wait().unwrap();
    let why = format!(
        "run {run} was cut off before it ended: no copy of it is kept, what was committed of it was on the branch \
         jellymoon/run-{run}, and no star was added"
    );
    let drawn = w.on_page(&wish);
    assert_eq!((&drawn["state"], &drawn["stars"]), (&json!("needs_you"), &json!([])), "the look drew {drawn}");
    let (kept, record) = (w.kept(&wish), w.record(&run));
    assert_eq!((&kept["state"], &kept["why"]), (&json!("needs_you"), &json!(why)));
    assert_eq!((&record["outcome"], &record["why"]), (&json!("failed"), &json!(why)));
    assert!(w.copies().is_empty() && w.branches(&repo) == [format!("jellymoon/run-{run}")]);
    assert_eq!((&w.on_page(&wish)["state"], w.kept(&wish)), (&json!("needs_you"), kept), "on {DAY}'s next look");
}
