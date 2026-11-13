//! The records and the branches a keep does not go by: work that was tossed, a record from before a record named
//! its commits, a record that is not core's, a run's branch that is not as the run left it, and work the branch
//! holds already (`docs/development/keep.md`, "What is refused" and "Work the branch holds already").

use std::fs;

use serde_json::json;

use super::{BEFORE, FORWARD, Made, finished, kept, printed};

const HELD: &str = "run {run} was kept: the branch held the run's work already, and nothing was landed";
const NOT_CORES: &str = "the record of run {run} is not one Jellymoon wrote for this wish";

/// Fails unless a keep is refused in these words with nothing changed, and asked first says the same.
fn unread(made: &Made, why: &str, asked_is_refused: bool) {
    let before = made.held();
    let asked = made.keep(&["--ask"]);
    if asked_is_refused {
        made.refuses(&asked, &before, why);
    } else {
        let asked = printed(&asked);
        assert_eq!((&asked["fits"], &asked["why"]), (&json!(false), &json!(why.replace("{run}", &made.run))));
        assert!(before == made.held());
    }
    made.refuses(&made.keep(&[]), &before, why);
    made.refuses(&made.keep(&["--with-settings"]), &before, why);
}

#[test]
fn work_that_was_tossed_is_not_kept_and_a_keep_says_so() {
    let made = finished();
    let mut record = made.record();
    record["work"] = json!("tossed");
    made.write(&record);
    unread(&made, "the work of run {run} was tossed, so nothing of it was kept", true);
}

#[test]
fn a_record_from_before_a_record_named_its_commits_is_shown_and_never_kept() {
    let made = finished();
    made.write(&made.as_before());
    let before = "the record of run {run} is from before a run's record named its commits: its work is shown, and is \
                  not kept by Jellymoon";
    unread(&made, before, true);
}

#[test]
fn a_record_whose_words_of_a_keep_are_not_a_keeps_is_not_cores() {
    let made = finished();
    let (record, end) = (made.record(), made.end());
    let words = [
        json!({"kept": end}),
        json!({"onto": "refs/heads/main"}),
        json!({"kept": end, "onto": "main"}),
        json!({"kept": end, "onto": "refs/heads/*"}),
        json!({"kept": end, "onto": "refs/tags/v1"}),
        json!({"kept": "main", "onto": "refs/heads/main"}),
        json!({"kept": "--all", "onto": "refs/heads/main"}),
        json!({"work": "kept"}),
    ];
    for words in words {
        let mut wrong = record.clone();
        wrong.as_object_mut().unwrap().extend(words.as_object().unwrap().clone());
        made.write(&wrong);
        let before = made.held();
        let ran = made.keep(&[]);
        assert_eq!((ran.code, ran.stdout.as_str()), (1, ""), "{words}: {}", ran.stderr);
        assert!(before == made.held(), "{words}");
        made.refuses(&ran, &before, NOT_CORES);
    }
    made.write(&record);
    kept(&made, &made.keep(&[]), &end, "main", FORWARD);
}

#[test]
fn a_runs_branch_that_is_not_as_the_run_left_it_is_not_kept() {
    let moved = "the branch of run {run} is not as the run left it, so what is on it is not kept as the run's";
    // Moved to another commit: the person's own, and a commit someone made to look like a run's.
    let made = finished();
    made.git(&["update-ref", &made.branch(), "main"]);
    unread(&made, moved, false);
    let made = finished();
    let tree = format!("{}^{{tree}}", made.end());
    let other = made.git(&["commit-tree", "-p", "main", "-m", &format!("Jellymoon run {}", made.run), &tree]);
    made.git(&["update-ref", &made.branch(), &other]);
    unread(&made, moved, false);
    // A name for another branch, and gone.
    let made = finished();
    made.git(&["symbolic-ref", &made.branch(), "refs/heads/main"]);
    unread(&made, "the branch of run {run} is a name for another branch by now, so its work is not kept", false);
    let made = finished();
    made.git(&["update-ref", "-d", &made.branch()]);
    unread(&made, "the branch of run {run} is not in the repo, so there is no work of it to keep", false);
    // The record names a start the run's commit does not stand on.
    let made = finished();
    let mut record = made.record();
    record["start"] = json!(made.git(&["commit-tree", "-m", "a start of its own", "main^{tree}"]));
    made.write(&record);
    let start = "the work of run {run} does not start where its record says the run started, so it is not kept as \
                 the run's";
    unread(&made, start, false);
}

#[test]
fn a_keep_in_a_repo_the_record_does_not_name_is_refused() {
    let made = finished();
    let other = made.w.repo("elsewhere/alpha");
    let before = (made.held(), made.w.persons(&other));
    let why = "the repo found for run {run} is not the one its record names, so its work is not kept";
    let ran = made.keep_in(&other, &[], &[]);
    made.refuses(&ran, &before.0, why);
    assert!(before.1 == made.w.persons(&other));
    // And a folder below the top of the checkout is not the checkout, whatever its name.
    let below = made.repo.join("alpha");
    fs::create_dir(&below).unwrap();
    let before = made.held();
    let why = "the work of run {run} could not be read from the repo: the folder is not the top of a checkout git \
               opens, or git did not answer";
    made.refuses(&made.keep_in(&below, &[], &[]), &before, why);
}

#[test]
fn work_the_branch_holds_already_is_marked_kept_and_nothing_is_landed() {
    // The person moved their branch forward to the run's commit themselves.
    let made = finished();
    let end = made.end();
    made.git(&["merge", "-q", "--ff-only", &made.branch()]);
    let person = made.w.persons(&made.repo);
    assert_eq!(
        printed(&made.keep(&["--ask"]))["why"],
        json!(format!("a keep of run {} would land nothing: the branch holds the run's work already", made.run))
    );
    kept(&made, &made.keep(&[]), &end, "main", HELD);
    assert!(person == made.w.persons(&made.repo));
    assert!(made.w.branches(&made.repo).is_empty());
    assert_eq!(made.record()["work"], json!("kept"));
    kept(&made, &made.keep(&[]), &end, "main", BEFORE);
    // They moved it forward and went on: the commit that was kept is the run's, which their branch holds.
    let made = finished();
    let end = made.end();
    made.git(&["merge", "-q", "--ff-only", &made.branch()]);
    made.moved_on();
    kept(&made, &made.keep(&[]), &end, "main", HELD);
    // The person picked the work onto commits of their own: their last commit holds everything a keep would land.
    let made = finished();
    let theirs = made.moved_on();
    made.git(&["cherry-pick", "--allow-empty", &made.end()]);
    let tip = made.w.commit(&made.repo, "main");
    assert_ne!(tip, theirs);
    let person = made.w.persons(&made.repo);
    kept(&made, &made.keep(&[]), &tip, "main", HELD);
    assert!(person == made.w.persons(&made.repo));
    assert!(made.w.branches(&made.repo).is_empty());
}
