//! Something done to a run's branch between two git steps of the run's end (`docs/development/run.md`, "What is
//! kept"): after core read where the branch is, and before core puts its commit there. The made-up `fake-git`
//! stands first on the run's `PATH` and does one thing by hand, with the real git, just before a named step
//! (`support/fake_git.rs`, its `first`, `on` and `returns` files). The person's branch never takes a run's commit,
//! and a run whose branch was moved in that gap has failed and names no end.

use std::fs;

use serde_json::json;

use super::moved::waiting_in;
use super::shown::names;
use super::{Made, ended, printed, ran, refused, world};
use crate::support::Ran;

/// A run of the made-up agent's `script` to its end, with `first` done in the repo just before the git step that
/// has `step` among its words: `first` is given the run and returns the words of one git command. With no words
/// the made-up git never returns from that step, and is stopped at its limit.
fn raced(script: &str, step: &str, first: impl Fn(&Made) -> Vec<String>) -> (Made, Ran) {
    let w = world();
    let (vars, fake) = (w.fake_git(), w.at("fake"));
    fs::write(fake.join("on"), "no step yet").unwrap();
    let (made, going) = waiting_in(w, script, &vars);
    let words = first(&made);
    if !words.is_empty() {
        let git = format!("--git-dir={}", made.repo.join(".git").display());
        fs::write(fake.join("first"), format!("{git}\n{}", words.join("\n"))).unwrap();
        fs::write(fake.join("returns"), "").unwrap();
    }
    fs::write(fake.join("on"), step).unwrap();
    fs::write(made.w.at("rec").join("go"), "").unwrap();
    let ran = ended(going);
    // It takes its lock on `held` at that step and at no other: a run that never came to the step raced nothing.
    assert!(fake.join("held").exists(), "the made-up git was never at the step {step}");
    assert_eq!(fake.join("stalled").exists(), words.is_empty(), "and stalled there only when told nothing to do");
    (made, ran)
}

/// The words that move the run's branch to a commit made by hand on `main`, of `main`'s own files.
fn moved_by_hand(made: &Made) -> Vec<String> {
    let main = made.w.commit(&made.repo, "main");
    vec!["update-ref".into(), made.branch(), made.by_hand(&main, "moved by hand", &main)]
}

/// Holds a run to what a commit that could not be made leaves: it failed in the fixed words and never as one
/// whose branch was moved, its record names its start and no end, its copy is kept and `main` is where it was.
fn could_not_be_committed(made: &Made, said: &Ran) {
    let (w, main) = (&made.w, made.w.commit(&made.repo, "main"));
    let (run, said) = ran(said);
    let why =
        format!("run {run} did not finish (its work could not be committed): its copy is kept and no star was added");
    assert_eq!(said, json!({"wish": made.wish, "outcome": "failed", "star": null, "denied": 0, "why": why}));
    let record = made.record();
    assert_eq!((&record["outcome"], &record["why"], &record["start"]), (&json!("failed"), &json!(why), &json!(main)));
    assert!(record.get("end").is_none(), "a run that could not be committed ended as no commit: {record}");
    assert_eq!(w.copies(), std::slice::from_ref(&made.run), "its copy is kept");
    assert_eq!(made.git(&["rev-parse", "HEAD"]), main, "the person's checkout is where it was");
    assert!(refused(&made.show()).contains("did not finish: there is no work of it to show"));
}

// The gate's first finding. The tip read refuses a name for another branch, and then the name is made one, for
// `main`, which is still at the run's start: git's check of the old value passes, and a step that followed the
// name would move the person's `main`. The run's name is written as itself.
#[test]
fn a_run_branch_made_a_name_for_main_just_before_the_commit_is_put_there_does_not_move_main() {
    let (made, said) =
        raced("slow", "update-ref", |made| vec!["symbolic-ref".into(), made.branch(), "refs/heads/main".into()]);
    let (main, end) = (made.git(&["rev-parse", "HEAD"]), made.w.commit(&made.repo, &made.branch()));
    assert_eq!(made.w.commit(&made.repo, "refs/heads/main"), main, "the person's branch took no commit");
    assert_eq!(made.git(&["status", "--porcelain", "--untracked-files=no"]), "", "and their checkout shows none");
    // The run finished: its name is a plain branch again, at core's one commit on the run's start.
    let (run, said) = ran(&said);
    let why = format!("run {run} finished: its work is one commit on the branch jellymoon/run-{run}");
    assert_eq!((&said["outcome"], &said["why"]), (&json!("completed"), &json!(why)), "{said}");
    assert!(!made.w.git_ran(&made.repo, &["symbolic-ref", "-q", &made.branch()]).status.success(), "a plain branch");
    assert_eq!(made.git(&["rev-list", "--parents", "-n", "1", &end]), format!("{end} {main}"));
    let record = made.record();
    assert_eq!((&record["start"], &record["end"]), (&json!(main), &json!(end)), "{record}");
    assert_eq!(names(&printed(&made.show())), ["a.txt", "b.txt", "n.txt"]);
}

// The same hand, with the name made one for a branch that is not at the run's start: the old value does not
// hold, the commit is put nowhere, and that branch is left alone.
#[test]
fn a_run_branch_made_a_name_for_a_branch_elsewhere_just_before_the_commit_fails_the_run() {
    let (made, said) = raced("slow", "update-ref", |made| {
        let main = made.w.commit(&made.repo, "main");
        made.git(&["update-ref", "refs/heads/theirs", &made.by_hand(&main, "their own", &main)]);
        vec!["symbolic-ref".into(), made.branch(), "refs/heads/theirs".into()]
    });
    could_not_be_committed(&made, &said);
    let theirs = made.w.commit(&made.repo, "refs/heads/theirs");
    assert_eq!(made.git(&["log", "-1", "--format=%s", &theirs]), "their own", "their branch took no commit");
    assert_eq!(made.git(&["symbolic-ref", &made.branch()]), "refs/heads/theirs", "the name is left as it was made");
}

// The commit is put on the branch only while the branch still names the commit it was made on: git is told
// the old value. Without it this run would finish, with its commit over the one made by hand.
#[test]
fn a_branch_moved_between_the_tip_read_and_the_commit_fails_the_run_as_one_that_could_not_be_committed() {
    let (made, said) = raced("slow", "update-ref", moved_by_hand);
    could_not_be_committed(&made, &said);
    let at = made.w.commit(&made.repo, &made.branch());
    assert_eq!(made.git(&["log", "-1", "--format=%s", &at]), "moved by hand", "the branch is where it was put");
}

// The gate's second finding. A copy that changed nothing ends as the commit its branch was read at, and the
// words say the branch is there: git is asked once more that it is, after everything else of the end.
#[test]
fn a_branch_moved_under_a_copy_that_changed_nothing_after_the_tip_read_fails_the_run() {
    let (made, said) = raced("idle", "write-tree", moved_by_hand);
    could_not_be_committed(&made, &said);
    let at = made.w.commit(&made.repo, &made.branch());
    assert_eq!(made.git(&["log", "-1", "--format=%s", &at]), "moved by hand", "the branch is where it was put");
}

// The same last check, with the run's name made a name for `main` just before it. The check moves nothing: its
// old value and its new are the run's start. It is still a write of the run's own name, which is a plain branch
// again afterwards, at the start, and `main` is where it was.
#[test]
fn a_run_that_changed_nothing_whose_name_was_made_a_name_for_main_leaves_a_plain_branch_at_its_start() {
    let (made, said) =
        raced("idle", "update-ref", |made| vec!["symbolic-ref".into(), made.branch(), "refs/heads/main".into()]);
    let (run, said) = ran(&said);
    assert_eq!(said["outcome"], "completed", "{said}");
    assert!(said["why"].as_str().unwrap().contains("changed nothing"), "{said}");
    let main = made.git(&["rev-parse", "HEAD"]);
    assert_eq!(made.w.commit(&made.repo, "refs/heads/main"), main, "the person's branch is where it was");
    assert_eq!(made.git(&["log", "-1", "--format=%s", &main]), "the first commit");
    assert!(!made.w.git_ran(&made.repo, &["symbolic-ref", "-q", &made.branch()]).status.success(), "a plain branch");
    assert_eq!(made.w.commit(&made.repo, &made.branch()), main, "at the commit the run started from");
    let record = made.record();
    assert_eq!((&record["start"], &record["end"]), (&json!(main), &json!(main)), "run {run}: {record}");
}

// A git that does not answer where the branch is has not said the branch was moved.
#[test]
fn a_git_that_does_not_answer_at_the_tip_read_is_a_commit_that_could_not_be_made_and_never_a_moved_branch() {
    let (made, said) = raced("slow", "for-each-ref", |_| Vec::new());
    could_not_be_committed(&made, &said);
    assert!(fs::File::open(made.w.at("fake").join("held")).unwrap().try_lock().is_ok(), "the made-up git is gone");
    assert_eq!(made.w.commit(&made.repo, &made.branch()), made.w.commit(&made.repo, "main"));
}
