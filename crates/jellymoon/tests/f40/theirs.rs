//! Made wishes changed by hand while runs go, so that a wish names a run its record gives to another wish. While
//! that run is held it is its own command's, whatever the day's wishes say: a stop for the wish that was changed
//! is refused, and ends nobody's agent (`docs/development/run.md`, "A wish that names another wish's run").

use std::fs;
use std::path::Path;

use serde_json::json;

use super::{World, ms, refused, wait_for, world};

impl World {
    /// A stop for a wish that names a run going for another wish: refused in one line, with no request left and
    /// the run's agent, which holds its lock in `rec`, still going.
    fn stops_nothing(&self, wish: &str, run: &str, rec: &Path) {
        let stop = self.stop_with(&[ms("JELLYMOON_TEST_STOP_WAIT_MS", 2000)], wish);
        let why =
            format!("jellymoon: wish {wish} names run {run}, which is another wish's run, so nothing was stopped\n");
        assert_eq!(refused(&stop), why);
        assert!(fs::symlink_metadata(self.jm("runs").join(run).join("stop")).is_err(), "a request was left");
        assert!(fs::File::open(rec.join("held")).unwrap().try_lock().is_err(), "the agent of run {run} was ended");
    }
}

// The second wish is made to name the first wish's run, and the first is put back as it was made, naming none.
#[test]
fn a_stop_for_a_wish_made_to_name_a_run_that_its_own_wish_no_longer_names_ends_nobodys_agent() {
    let w = world();
    let repo = w.repo("alpha");
    let (one, two) = (w.wish("alpha"), w.wish("alpha"));
    let as_made = w.kept(&one);
    let (mut going, run) = w.run_going(("slow", 0), &[], &one, &repo);
    w.edit_wish(&two, |wish| (wish["state"], wish["run"]) = (json!("working"), json!(run)));
    w.edit_wish(&one, |wish| *wish = as_made);

    w.stops_nothing(&two, &run, &w.at("rec"));
    // A page does not draw it as working, and ends nobody either.
    assert_eq!(w.on_page(&two)["state"], "needs_you");
    w.stops_nothing(&two, &run, &w.at("rec"));
    assert!(going.try_wait().unwrap().is_none(), "the first wish's `jellymoon run` ended");

    fs::write(w.at("rec").join("go"), "").unwrap();
    going.wait().unwrap();
    assert_eq!(w.starts(), 1);
}

// Each of two wishes is made to name the other's run, while both go.
#[test]
fn a_stop_for_either_of_two_wishes_made_to_name_each_others_runs_ends_nobodys_agent() {
    let w = world();
    let repo = w.repo("alpha");
    let (one, two) = (w.wish("alpha"), w.wish("alpha"));
    let (mut going_one, run_one) = w.run_going(("slow", 0), &[], &one, &repo);
    // The second agent leaves its record, and holds its lock, in a folder of its own.
    let rec = w.at("rec-two");
    fs::create_dir(&rec).unwrap();
    fs::write(w.at("bin").join("play"), format!("slow\n{}\n0\n", rec.display())).unwrap();
    let mut going_two = w.start_going(&two, &repo);
    wait_for(&rec.join("started"));
    let run_two = w.kept(&two)["run"].as_str().unwrap().to_string();
    w.edit_wish(&one, |wish| wish["run"] = json!(run_two));
    w.edit_wish(&two, |wish| wish["run"] = json!(run_one));

    w.stops_nothing(&one, &run_two, &rec);
    w.stops_nothing(&two, &run_one, &w.at("rec"));
    let both = going_one.try_wait().unwrap().is_none() && going_two.try_wait().unwrap().is_none();
    assert!(both, "a `jellymoon run` ended");

    fs::write(w.at("rec").join("go"), "").unwrap();
    fs::write(rec.join("go"), "").unwrap();
    going_one.wait().unwrap();
    going_two.wait().unwrap();
}
