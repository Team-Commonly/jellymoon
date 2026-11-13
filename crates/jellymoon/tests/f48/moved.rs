//! A run whose branch is not at the run's start when the run ends (`docs/development/run.md`, "What is kept"):
//! whoever moved it, the run has failed in fixed words, nothing is committed onto the moved branch and the record
//! names no end. So an end is only ever the start itself, or the one commit core made on top of it.

use std::ffi::OsString;
use std::fs;

use serde_json::json;

use super::{DAY, Going, Made, World, ended, ran, refused, wait_for, world};

/// A run whose made-up agent waits for the test to leave `go`: `slow` has its work still to do then, `worked`
/// has done it, `gone` will remove its own copy and `idle` changes nothing.
fn waiting(script: &str) -> (Made, Going) {
    waiting_in(world(), script, &[])
}

/// [`waiting`] in a world of the caller's, with its variables after the world's.
pub(super) fn waiting_in(w: World, script: &str, vars: &[(&'static str, OsString)]) -> (Made, Going) {
    w.play(script, 0);
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let going = w.going(vars, &["run", "--wish", &wish, "--date", DAY, "--repo", repo.to_str().unwrap()]);
    wait_for(&w.at("rec").join("started"));
    let run = w.kept(&wish)["run"].as_str().unwrap().to_string();
    (Made { w, repo, wish, run }, going)
}

/// A commit made by hand of the files a run of the made-up agent leaves, on `start`, with `message`: typed and
/// committed in a checkout of the test's own, which is then removed. The run's copy is not touched.
fn the_work_by_hand(made: &Made, start: &str, message: &str) -> String {
    let own = made.w.at("work").join("by-hand");
    made.git(&["worktree", "add", "-q", "--detach", own.to_str().unwrap(), start]);
    fs::write(own.join("a.txt"), "edited by the run\n").unwrap();
    fs::write(own.join("n.txt"), "new from the run\n").unwrap();
    fs::remove_file(own.join("b.txt")).unwrap();
    made.w.git(&own, &["add", "-A"]);
    made.w.git(&own, &["commit", "-q", "-m", message]);
    let by_hand = made.w.commit(&own, "HEAD");
    made.git(&["worktree", "remove", "--force", own.to_str().unwrap()]);
    by_hand
}

/// Lets the agent end, and holds the run to what a moved branch leaves: it failed in the fixed words, its record
/// names its start and no end, its branch is where it was moved to (gone, with none), and `run show` refuses it
/// as any failed run.
fn failed_as_moved(made: &Made, going: Going, moved_to: Option<&str>) {
    let (w, main) = (&made.w, made.w.commit(&made.repo, "main"));
    fs::write(w.at("rec").join("go"), "").unwrap();
    let (run, said) = ran(&ended(going));
    let why = format!(
        "run {run} did not finish (its branch was moved while it worked): its copy is kept and no star was added"
    );
    assert_eq!(run, made.run);
    assert_eq!(said, json!({"wish": made.wish, "outcome": "failed", "star": null, "denied": 0, "why": why}));
    let record = made.record();
    assert_eq!((&record["outcome"], &record["why"], &record["start"]), (&json!("failed"), &json!(why), &json!(main)));
    assert!(record.get("end").is_none(), "a run whose branch was moved ended as no commit: {record}");
    match moved_to {
        Some(to) => assert_eq!(w.commit(&made.repo, &made.branch()), to, "nothing was committed onto the branch"),
        None => assert_eq!(w.branches(&made.repo), Vec::<String>::new(), "the branch was not made again"),
    }
    assert_eq!(w.commit(&made.repo, "main"), main);
    let kept = w.kept(&made.wish);
    assert_eq!((&kept["state"], &kept["stars"], &kept["why"]), (&json!("needs_you"), &json!([]), &json!(why)));
    let no = format!("jellymoon: run {run} did not finish: there is no work of it to show\n");
    assert_eq!(refused(&made.show()), no);
    assert_eq!(refused(&made.show_in(&made.repo, &["--file", "1"])), no);
    assert_eq!(w.on_page(&made.wish)["state"], "needs_you");
    assert!(made.record().get("end").is_none(), "and no look adds one");
}

// The gate's scene: one parent, the run's own message, the very files the copy ends with. By every sign F48 had
// this commit was the run's end, and core wrote it down as one.
#[test]
fn a_branch_moved_to_a_commit_in_a_runs_own_shape_while_the_run_goes_fails_the_run_and_names_no_end() {
    let (made, going) = waiting("slow");
    let main = made.w.commit(&made.repo, "main");
    let by_hand = the_work_by_hand(&made, &main, &made.message());
    assert_eq!(made.git(&["log", "-1", "--format=%P %s", &by_hand]), format!("{main} {}", made.message()));
    made.git(&["update-ref", &made.branch(), &by_hand, &main]);
    failed_as_moved(&made, going, Some(&by_hand));
    // The copy is kept as a failed run's is, with the agent's files in it, which are the commit's own.
    let copy = made.w.jm("copies").join(&made.run);
    assert_eq!(made.w.copies(), std::slice::from_ref(&made.run));
    assert_eq!(fs::read_to_string(copy.join("n.txt")).unwrap(), "new from the run\n");
    assert_eq!(made.w.git(&copy, &["diff", "--cached", "--name-only", &main]), "", "nothing was staged in the copy");
}

#[test]
fn the_persons_own_commit_inside_the_copy_while_the_run_goes_fails_the_run_and_names_no_end() {
    let (made, going) = waiting("worked");
    let copy = made.w.jm("copies").join(&made.run);
    made.w.git(&copy, &["add", "-A"]);
    made.w.git(&copy, &["commit", "-q", "-m", "the person's own"]);
    let theirs = made.w.commit(&made.repo, &made.branch());
    failed_as_moved(&made, going, Some(&theirs));
    assert_eq!(made.w.copies(), std::slice::from_ref(&made.run), "the copy is kept, on their commit");
    assert_eq!(made.w.commit(&copy, "HEAD"), theirs);
}

// The moved branch holds the very files the run started from, so its tree says nothing; and the copy has work
// that a commit could be made of, on top of a commit no run started from.
#[test]
fn a_branch_moved_under_a_run_whose_copy_changed_too_fails_the_run_and_takes_no_commit() {
    let (made, going) = waiting("slow");
    let main = made.w.commit(&made.repo, "main");
    let by_hand = made.by_hand(&main, "moved by hand", &main);
    made.git(&["update-ref", &made.branch(), &by_hand, &main]);
    let before = made.git(&["rev-list", "--all", "--count"]);
    failed_as_moved(&made, going, Some(&by_hand));
    assert_eq!(made.git(&["rev-list", "--all", "--count"]), before, "no commit was put under any name");
    let copy = made.w.jm("copies").join(&made.run);
    assert_eq!(fs::read_to_string(copy.join("a.txt")).unwrap(), "edited by the run\n", "the work is in the copy");
}

// A run whose copy's folder is gone at its end had nothing to commit. It ended as its start when its branch is
// still there, so its record names all three as every finished run's does; and a moved branch fails it too.
#[test]
fn a_run_whose_copy_is_gone_at_its_end_ended_as_its_start_or_failed_when_its_branch_was_moved() {
    let (made, going) = waiting("gone");
    let main = made.w.commit(&made.repo, "main");
    fs::write(made.w.at("rec").join("go"), "").unwrap();
    let (run, said) = ran(&ended(going));
    let branch = format!("jellymoon/run-{run}");
    let why = format!("run {run} finished and changed nothing: the branch {branch} is where the checkout was");
    assert_eq!((&said["outcome"], &said["why"]), (&json!("completed"), &json!(why)), "{said}");
    let record = made.record();
    assert_eq!((&record["start"], &record["end"]), (&json!(main), &json!(main)), "{record}");
    assert_eq!(made.w.commit(&made.repo, &made.branch()), main);
    let shown = super::printed(&made.show());
    assert_eq!((&shown["work"], &shown["files"]), (&json!("waits"), &json!([])), "{shown}");

    let (made, going) = waiting("gone");
    let main = made.w.commit(&made.repo, "main");
    let by_hand = made.by_hand(&main, "moved by hand", &main);
    made.git(&["update-ref", &made.branch(), &by_hand, &main]);
    failed_as_moved(&made, going, Some(&by_hand));
}

// A name for another branch is not the run's branch, though it leads to the very commit the run started from:
// read through, it would look unmoved, and the run's commit would land on the branch it leads to. And a branch
// that is gone is not made again.
#[test]
fn a_branch_made_a_name_for_another_or_deleted_while_the_run_goes_fails_the_run_and_moves_nothing() {
    let (made, going) = waiting("slow");
    let main = made.w.commit(&made.repo, "main");
    made.git(&["update-ref", "refs/heads/theirs", &main]);
    made.git(&["symbolic-ref", &made.branch(), "refs/heads/theirs"]);
    let before = made.git(&["rev-list", "--all", "--count"]);
    failed_as_moved(&made, going, Some(&main));
    assert_eq!(made.git(&["symbolic-ref", &made.branch()]), "refs/heads/theirs", "the name is left as it was made");
    assert_eq!(made.w.commit(&made.repo, "refs/heads/theirs"), main, "the branch it leads to took no commit");
    assert_eq!(made.git(&["rev-list", "--all", "--count"]), before, "no commit was put under any name");

    let (made, going) = waiting("slow");
    made.git(&["update-ref", "-d", &made.branch()]);
    failed_as_moved(&made, going, None);
    assert_eq!(made.w.copies(), std::slice::from_ref(&made.run), "the copy is kept, with the work in it");
}
