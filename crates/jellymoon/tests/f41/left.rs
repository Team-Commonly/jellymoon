//! What a `jellymoon run` that was killed left with no wish to name it (`docs/development/run.md`, "A copy nothing
//! names"), and the agent a killed command left at work when the next run is asked for.

use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use super::apart::exit_of;
use super::{DAY, World, ended, found_world, grows, made, printed, wait_for};
use crate::support::skipped;

impl World {
    /// Starts `jellymoon run` for a wish and holds it inside the claim for `wait` milliseconds, with its copy made
    /// and its wish not yet naming the run. Returns the command and the run.
    fn held_in_the_claim(&self, wish: &str, repo: &Path, wait: &str) -> (std::process::Child, String) {
        let args = ["run", "--wish", wish, "--date", DAY, "--repo", repo.to_str().unwrap()];
        let going = self.going(&[("JELLYMOON_TEST_CLAIM_WAIT_MS", wait.into())], &args);
        let from = Instant::now();
        while !self.jm("runs").join("going.lock").exists() || self.copies().is_empty() {
            assert!(from.elapsed() < Duration::from_secs(30), "the run never came to its claim");
            std::thread::sleep(Duration::from_millis(10));
        }
        (going, self.copies().remove(0))
    }

    /// A `jellymoon run` killed inside the claim: its copy, its branch and its folder in `runs` are left, and no
    /// wish names the run.
    fn killed_in_the_claim(&self, wish: &str, repo: &Path) -> String {
        let (mut going, run) = self.held_in_the_claim(wish, repo, "60000");
        going.kill().unwrap();
        going.wait().unwrap();
        assert_eq!((self.copies(), self.branches(repo)), (vec![run.clone()], vec![format!("jellymoon/run-{run}")]));
        assert_eq!(self.kept(wish), made(wish, "alpha"), "the wish named the run before the claim");
        run
    }

    fn kept_for(&self, run: &str) -> bool {
        self.jm("runs").join(run).exists()
    }
}

#[test]
fn a_copy_left_by_a_run_killed_before_its_wish_named_it_is_taken_back_by_the_next_look() {
    let w = found_world();
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let before = w.persons(&repo);
    let run = w.killed_in_the_claim(&wish, &repo);
    assert_eq!(w.on_page(&wish)["state"], "working");
    assert_eq!((w.copies(), w.branches(&repo), w.kept_for(&run)), (vec![], vec![], false));
    assert_eq!((w.kept(&wish), w.persons(&repo)), (made(&wish, "alpha"), before), "more than the copy was changed");
    // The wish never had a run, and gets its one run now.
    assert_eq!(printed(&w.start(&wish, &repo))["outcome"], "completed");
}

#[test]
fn a_copy_whose_command_still_lives_is_not_taken_back() {
    let w = found_world();
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let (going, run) = w.held_in_the_claim(&wish, &repo, "2500");
    assert_eq!(w.on_page(&wish)["state"], "working");
    assert_eq!((w.copies(), w.kept_for(&run)), (vec![run.clone()], true), "a look took a living command's copy");
    let said = printed(&ended(going));
    assert_eq!((&said["outcome"], &said["run"]), (&json!("completed"), &json!(run)));
}

#[test]
fn a_branch_that_holds_a_commit_of_its_own_is_kept_when_its_copy_is_taken_back() {
    let w = found_world();
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let run = w.killed_in_the_claim(&wish, &repo);
    let branch = format!("jellymoon/run-{run}");
    w.git(&w.jm("copies").join(&run), &["commit", "-q", "--allow-empty", "-m", "the person's own"]);
    let mine = w.commit(&repo, &branch);
    w.on_page(&wish);
    assert_eq!((w.copies(), w.kept_for(&run)), (vec![], false));
    assert_eq!((w.branches(&repo), w.commit(&repo, &branch)), (vec![branch.clone()], mine));
    assert_ne!(w.commit(&repo, &branch), w.commit(&repo, "HEAD"));
}

// Killed later, a command has written the run's record: before its wish named the run, or after a run that was
// never started had its wish told so. A record that names no child is of a copy nothing ran in. One that names a
// child is of a copy an agent may have written in: that copy is kept for a look, as a cut-off run's is.
#[test]
fn a_copy_no_wish_names_is_taken_back_on_a_record_with_no_child_and_kept_as_failed_on_one_with_a_child() {
    let record = |child: Value| {
        let mut record = json!({"wish": "m1", "day": DAY, "started": 1, "outcome": null, "denied": 0, "skipped": 0});
        record.as_object_mut().unwrap().extend(child.as_object().unwrap().clone());
        record.to_string()
    };
    let w = found_world();
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let run = w.killed_in_the_claim(&wish, &repo);
    fs::write(w.jm("runs").join(&run).join("run.json"), record(json!({}))).unwrap();
    w.on_page(&wish);
    assert_eq!((w.copies(), w.branches(&repo), w.kept_for(&run)), (vec![], vec![], false));

    // A pid no process of this system has, so nothing is signalled.
    let run = w.killed_in_the_claim(&wish, &repo);
    fs::write(w.jm("runs").join(&run).join("run.json"), record(json!({"pid": 4_000_000_000_u32, "mark": "none"})))
        .unwrap();
    w.on_page(&wish);
    let why = format!("run {run} was cut off before it ended: its copy is kept and no star was added");
    let kept = w.record(&run);
    assert_eq!((&kept["outcome"], &kept["why"]), (&json!("failed"), &json!(why)));
    assert_eq!((w.copies(), w.branches(&repo)), (vec![run.clone()], vec![format!("jellymoon/run-{run}")]));
    assert_eq!(w.kept(&wish), made(&wish, "alpha"), "a run no wish names changed a wish");
    // It is done with: the next look finds nothing more to do, and the copy stays one of the failed copies.
    w.on_page(&wish);
    assert!(!w.jm("runs").join(&run).join("start.lock").exists() && w.copies() == [run]);
}

// Three commands killed with their agents at work would leave three agents beside the three runs that may go. The
// next `jellymoon run` ends what killed commands left before it counts.
#[test]
fn the_agent_a_killed_command_left_is_ended_before_the_next_run_is_counted_and_started() {
    if !cfg!(unix) {
        return skipped("where there are no signals nothing is stopped, as run.md says");
    }
    let w = found_world();
    let repo = w.repo("alpha");
    w.typed_in("-work-alpha", &repo);
    let (first, second) = (w.wish("alpha"), w.wish("alpha"));
    w.play("scribble", 0);
    assert_eq!(w.asked(&first).code, 0);
    let run = w.run_of(&first);
    let file: &Path = &w.jm("copies").join(&run).join("scribble.txt");
    wait_for(file);
    fs::write(w.at("rec").join("cut"), "").unwrap(); // `fake-asker` kills the command it started
    assert_ne!(exit_of(&w), "0");
    assert!(grows(file), "the agent the killed command left was to be editing still");

    w.play("edits", 0);
    assert_eq!(printed(&w.start(&second, &repo))["outcome"], "completed");
    assert!(w.freed("held") && !grows(file), "the agent a killed command left is still at work beside the new run");
    // The first wish is told by the look, as before.
    assert_eq!(w.on_page(&first)["state"], "needs_you");
}
