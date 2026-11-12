//! A run's lock that cannot be opened. A lock that cannot be asked about is no lock: the run is held by nothing,
//! so it was cut off (`docs/development/run.md`, "A file of the home that is not a plain file").

use std::path::Path;
use std::time::{Duration, Instant};
use std::{fs, thread};

use serde_json::json;

use super::{DAY, STAR, ended, grows, ms, printed, world};
use crate::support::skipped;

/// Takes every permission off a file. False where that does not keep this user from opening it.
#[cfg(unix)]
pub fn closed(file: &Path) -> bool {
    std::fs::set_permissions(file, std::os::unix::fs::PermissionsExt::from_mode(0o000)).unwrap();
    std::fs::File::open(file).is_err()
}

#[cfg(not(unix))]
pub fn closed(_: &Path) -> bool {
    false
}

#[test]
fn a_cut_off_run_whose_lock_cannot_be_opened_is_written_down_and_its_agent_ended() {
    let w = world();
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let (run, file) = w.cut_off(&wish, &repo);
    if !closed(&w.jm("runs").join(&run).join("run.lock")) {
        skipped("no mode keeps this user from opening a file here: the system has none, or the user is root");
        return w.let_end("stop");
    }
    assert_eq!(w.on_page(&wish)["state"], "needs_you", "a lock nobody can ask about was taken for one held");
    assert!(w.freed("held") && !grows(&file), "the agent the run left is still editing");
    let why = format!("run {run} was cut off before it ended: its copy is kept and no star was added");
    let said = json!({"run": run, "wish": wish, "outcome": "failed", "star": null, "denied": 0, "why": why});
    assert_eq!(printed(&w.stop(&wish)), said);
    assert_eq!((&w.kept(&wish)["state"], &w.record(&run)["outcome"]), (&json!("needs_you"), &json!("failed")));
}

// The lock of a run that still goes is closed by hand, and the run finishes by itself while a look, which took it
// for cut off, is on its way: the look gives the wish the end the run's record has by then. The look is made to
// wait between its first reading of the record and the rest, and the day is locked meanwhile so that it is the
// look's own drawing that is read here.
#[test]
fn a_run_that_finished_while_a_look_took_it_for_cut_off_is_drawn_with_the_end_its_record_has() {
    let w = world();
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let (going, run) = w.run_going(("slow", 0), &[], &wish, &repo);
    let (logs, day) = (w.at("logs"), w.jm("pages").join(format!("{DAY}.ruled.lock")));
    if !closed(&w.jm("runs").join(&run).join("run.lock")) {
        skipped("no mode keeps this user from opening a file here: the system has none, or the user is root");
        fs::write(w.at("rec").join("go"), "").unwrap();
        assert_eq!(printed(&ended(going))["outcome"], "completed");
        return;
    }
    fs::write(&day, "").unwrap(); // another change to the day: neither the run nor the look can tell the wish
    let waits = [ms("JELLYMOON_TEST_LOOK_WAIT_MS", 10_000)];
    let mut look = w.going(&waits, &["page", "--date", DAY, "--logs", logs.to_str().unwrap()]);
    thread::sleep(Duration::from_secs(2)); // the look has read the record, which has no end, and waits
    fs::write(w.at("rec").join("go"), "").unwrap();
    assert!(look.try_wait().unwrap().is_none(), "the look had ended before the agent was let go");
    let from = Instant::now();
    while w.record(&run)["outcome"] != "completed" {
        assert!(from.elapsed() < Duration::from_secs(7), "the run did not finish while the look waited");
        thread::sleep(Duration::from_millis(10));
    }
    let drawn = printed(&ended(look))["wishes"][0].clone();
    assert_eq!((&drawn["state"], &drawn["stars"]), (&json!("done"), &json!([STAR])), "the look drew {drawn}");
    fs::remove_file(&day).unwrap();
    assert_eq!(printed(&ended(going))["outcome"], "completed");
    assert_eq!((&w.kept(&wish)["state"], &w.kept(&wish)["stars"]), (&json!("done"), &json!([STAR])));
    assert_eq!(w.on_page(&wish)["state"], "done");
}
