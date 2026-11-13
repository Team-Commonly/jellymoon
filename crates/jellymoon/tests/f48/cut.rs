//! A `jellymoon run` killed between its steps (`docs/development/run.md`, "What a kill leaves in the record"): the
//! record it leaves is one `jellymoon run show` refuses in fixed words, before a look has written the run down and
//! after. It never names an end the run did not write.

use std::time::{Duration, Instant};
use std::{fs, thread};

use serde_json::json;

use super::{DAY, Made, World, refused, world};

fn until(what: &str, mut so: impl FnMut() -> bool) {
    let from = Instant::now();
    while !so() {
        assert!(from.elapsed() < Duration::from_secs(30), "{what}");
        thread::sleep(Duration::from_millis(5));
    }
}

/// Starts a run of the made-up agent's `script` with `held` set, waits until `there`, and kills the command.
fn cut_off(script: &str, held: &[(&'static str, std::ffi::OsString)], there: impl Fn(&World, &str) -> bool) -> Made {
    let w = world();
    w.play(script, 0);
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let mut going = w.going(held, &["run", "--wish", &wish, "--date", DAY, "--repo", repo.to_str().unwrap()]);
    until("the wish never named its run", || w.kept(&wish)["run"].is_string());
    let run = w.kept(&wish)["run"].as_str().unwrap().to_string();
    until("the run never came to where it is cut off", || there(&w, &run));
    going.kill().unwrap();
    going.wait().unwrap();
    Made { w, repo, wish, run }
}

/// What the record of a run that was cut off names: its repo and its start, and no end.
fn names_no_end(made: &Made) {
    let record = made.record();
    assert!(record["repo"].is_string(), "{record}");
    assert_eq!(record["start"], json!(made.w.commit(&made.repo, "main")), "{record}");
    assert!(record.get("end").is_none(), "{record}");
}

/// The two fixed lines: before a look has written the run down, and after.
fn refused_before_and_after_a_look(made: &Made) {
    let (run, w) = (&made.run, &made.w);
    assert!(made.record()["outcome"].is_null());
    names_no_end(made);
    let cut = format!("jellymoon: run {run} was cut off before it ended: there is no work of it to show\n");
    assert_eq!(refused(&made.show()), cut);
    assert_eq!(refused(&made.show_in(&made.repo, &["--file", "1"])), cut);
    assert_eq!(w.on_page(&made.wish)["state"], "needs_you");
    assert_eq!(made.record()["outcome"], "failed");
    names_no_end(made);
    // The words a look writes for a run that was cut off are not among those `run show` knows a failed run by
    // (as F44 built it), so the line is the one for a record whose words are not a run's own end.
    let failed = format!("jellymoon: the record of run {run} is not one Jellymoon wrote for this wish\n");
    assert_eq!(refused(&made.show()), failed);
    assert_eq!(refused(&made.show_in(&made.repo, &["--file", "1"])), failed);
}

#[test]
fn a_run_cut_off_while_its_agent_works_leaves_a_record_with_no_end_which_is_refused() {
    let made = cut_off("forever", &[], |w, _| w.at("rec").join("started").exists());
    assert_eq!(made.w.commit(&made.repo, &made.branch()), made.w.commit(&made.repo, "main"));
    refused_before_and_after_a_look(&made);
    assert_eq!(made.w.copies(), std::slice::from_ref(&made.run), "its copy is kept");
}

// The commit is made and the copy is gone, and the end is not in the record yet: the one moment at which the
// branch holds a run's commit that no record names.
#[test]
fn a_run_cut_off_after_its_commit_and_before_its_end_was_written_is_refused_and_names_no_end() {
    let hold = [("JELLYMOON_TEST_NOTE_WAIT_MS", "60000".into())];
    let made = cut_off("edits", &hold, |w, run| {
        !w.jm("runs").join(run).join("copy.json").exists() && !w.jm("copies").join(run).exists()
    });
    let (end, main) = (made.w.commit(&made.repo, &made.branch()), made.w.commit(&made.repo, "main"));
    assert_ne!(end, main, "the run's commit was made");
    refused_before_and_after_a_look(&made);
    // Nobody wrote that commit down as the run's end, and an end put in the record by hand does not make the
    // run a finished one: its outcome and its words are a cut-off run's still.
    let mut record = made.record();
    record["end"] = json!(end);
    made.write(&record);
    assert!(refused(&made.show()).contains("is not one Jellymoon wrote for this wish"));
    assert!(fs::read_dir(made.w.jm("copies")).unwrap().next().is_none());
}
