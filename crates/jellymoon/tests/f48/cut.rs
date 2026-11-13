//! A `jellymoon run` killed between its steps (`docs/development/run.md`, "What a kill leaves in the record"): the
//! record it leaves is one `jellymoon run show` refuses in one fixed line, before a look has written the run down
//! and after. It never names an end the run did not write.

use std::time::{Duration, Instant};
use std::{fs, thread};

use serde_json::{Value, json};

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

/// The one fixed line: before a look has written the run down, and after.
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
    // The words a look writes for a run that was cut off are core's own, and are known for what they say.
    assert!(made.record()["why"].as_str().unwrap().contains("was cut off before it ended"));
    assert_eq!(refused(&made.show()), cut);
    assert_eq!(refused(&made.show_in(&made.repo, &["--file", "1"])), cut);
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
    assert!(refused(&made.show()).contains("was cut off before it ended: there is no work of it to show"));
    // Its line is a cut-off run's only as core makes it for this run: with a word more it is nobody's.
    record["why"] = json!(format!("{} at last", record["why"].as_str().unwrap()));
    made.write(&record);
    assert!(refused(&made.show()).contains("is not one Jellymoon wrote for this wish"));
    assert!(fs::read_dir(made.w.jm("copies")).unwrap().next().is_none());
}

// A look's writing of a cut-off run is the record with no end, whatever stood there: an end is only ever written
// by the run that finished.
#[test]
fn an_end_put_in_the_record_of_a_cut_off_run_is_gone_once_a_look_has_written_the_run_down() {
    let made = cut_off("forever", &[], |w, _| w.at("rec").join("started").exists());
    let mut record = made.record();
    record["end"] = record["start"].clone();
    made.write(&record);
    assert_eq!(made.w.on_page(&made.wish)["state"], "needs_you");
    assert_eq!(made.record()["outcome"], "failed");
    names_no_end(&made);
}

// A run's record is there, naming its repo and its start, before its wish names the run: a kill between the two
// leaves a record no wish names, never a wish that names a run with no record.
#[test]
fn a_runs_record_is_written_before_its_wish_names_the_run() {
    let w = world();
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let hold = [("JELLYMOON_TEST_RECORD_WAIT_MS", "60000".into())];
    let mut going = w.going(&hold, &["run", "--wish", &wish, "--date", DAY, "--repo", repo.to_str().unwrap()]);
    let records = || fs::read_dir(w.jm("runs")).into_iter().flatten().flatten().map(|run| run.path().join("run.json"));
    let read = |file: std::path::PathBuf| serde_json::from_slice::<Value>(&fs::read(file).ok()?).ok();
    let mut record = None;
    until("the run never wrote its record", || {
        record = records().find_map(read);
        record.is_some()
    });
    let named = w.kept(&wish)["run"].clone();
    going.kill().unwrap();
    going.wait().unwrap();
    let record = record.unwrap();
    assert!(named.is_null(), "the wish named run {named} while the record's first writing was held");
    assert!(record["repo"].is_string() && record["start"] == json!(w.commit(&repo, "main")), "{record}");
}

// Every writing of a record is held in a test's build until the test lets it go, so each state the file ever
// has is read here, with no clock in it: none has the run's end without the commit it ended as, or that commit
// without the end.
#[test]
fn a_finished_runs_end_and_the_commit_it_ended_as_are_in_one_writing_of_its_record() {
    let w = world();
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let (go, at) = (w.at("rec").join("go.step"), w.at("rec").join("go.at"));
    let hold = [("JELLYMOON_TEST_RECORD_WAIT_MS", "60000".into()), ("JELLYMOON_TEST_GO", go.clone().into())];
    let mut going = w.going(&hold, &["run", "--wish", &wish, "--date", DAY, "--repo", repo.to_str().unwrap()]);
    let records = || fs::read_dir(w.jm("runs")).into_iter().flatten().flatten().map(|run| run.path().join("run.json"));
    let read = |file: std::path::PathBuf| serde_json::from_slice::<Value>(&fs::read(file).ok()?).ok();
    let mut seen: Vec<Value> = Vec::new();
    until("the run never ended", || {
        // The command has written its record and waits: the state is read, and only then is it let go on.
        if at.exists() && !go.exists() {
            seen.push(records().find_map(read).expect("a record is there at each of its writings"));
            fs::remove_file(&at).unwrap();
            fs::write(&go, "").unwrap();
        }
        going.try_wait().unwrap().is_some()
    });
    // Each writing, in order: the record is made, it names its child, its stream began (when the run saw it
    // begin before it ended), it ended.
    assert!(seen.len() == 3 || seen.len() == 4, "{seen:?}");
    assert!(seen[0]["pid"].is_null() && seen[1]["pid"].is_u64() && seen[1]["outcome"].is_null(), "{seen:?}");
    for record in &seen {
        assert_eq!(record["outcome"].is_null(), record.get("end").is_none(), "{record}");
    }
    assert_eq!(seen[seen.len() - 1]["outcome"], "completed");
}
