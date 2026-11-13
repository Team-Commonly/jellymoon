//! A keep cut off after its step, whose work git landed elsewhere than the keep meant (`docs/development/toss.md`,
//! "Work that waits"; `docs/development/keep.md`, "A checkout moved just before the step"). The record holds
//! what the keep meant and no more, so a toss asks the repo where the commit is: held by any branch but the
//! run's own, or by a `HEAD` that names no branch, the work is kept and is not tossed. The made-up `fake-git`
//! moves the checkout just before the step, as `tests/f45/raced.rs` has it do. A branch named as a run's is, under
//! any spelling, is never a branch of the person's that holds the work, and a checkout that stands on the run's
//! branch under another spelling stands on it.

use std::fs;

use serde_json::{Value, json};

use super::support::Ran;
use super::{DELETED, Made, REFS, WORK, finished, printed, tossed};

const LANDED: &str = "a keep of run {run} that was cut off had landed its work, so it is kept and not tossed: `run \
                      keep` finishes that keep, and nothing was changed";

/// A keep with `first` done in the repo by the real git just before `git merge`, cut off right after that step.
fn moved_and_cut(made: &Made, first: &[&str]) {
    let ran = moved(made, first, &[("JELLYMOON_TEST_KEEP_CUT", "3".into())]);
    assert_eq!((ran.code, ran.stdout.as_str()), (70, ""), "{}", ran.stderr);
}

/// A keep with `first` done in the repo by the real git just before `git merge`, and `more` variables.
fn moved(made: &Made, first: &[&str], more: &[(&'static str, std::ffi::OsString)]) -> Ran {
    let (mut vars, fake) = (made.w.fake_git(), made.w.at("fake"));
    fs::write(fake.join("on"), "merge").unwrap();
    fs::write(fake.join("first"), [&["-C", made.repo.to_str().unwrap()], first].concat().join("\n")).unwrap();
    fs::write(fake.join("returns"), "").unwrap();
    vars.extend(more.iter().cloned());
    let ran = made.keep_in(&made.repo, &[], &vars);
    assert!(fake.join("held").exists(), "the made-up git was never at the step: {}", ran.stderr);
    fs::remove_file(fake.join("on")).unwrap();
    ran
}

/// The record's three words of a keep.
fn words(made: &Made) -> (Value, Value, Value) {
    let record = made.record();
    (record["work"].clone(), record["kept"].clone(), record["onto"].clone())
}

#[test]
fn a_cut_off_keep_whose_work_landed_on_a_branch_the_checkout_was_moved_to_is_not_tossed() {
    let made = finished();
    let (start, end) = (made.w.commit(&made.repo, "main"), made.end());
    moved_and_cut(&made, &["checkout", "-q", "-b", "moved"]);
    // The record says what the keep meant, `main`, and git landed the work on `moved`, a name git lists after
    // the run's own.
    assert_eq!(words(&made), (Value::Null, json!(end), json!("refs/heads/main")));
    assert_eq!((made.w.commit(&made.repo, "main"), made.w.commit(&made.repo, "moved")), (start, end.clone()));
    let before = made.held();
    made.refuses(&made.toss(), &before, LANDED);
    assert_eq!((made.w.branches(&made.repo).len(), &made.w.kept(&made.wish)["state"]), (1, &json!("done")));
    // The next keep finishes it where the work is, and then a toss says it was kept.
    let kept = printed(&made.keep(&[]));
    assert_eq!((&kept["kept"], &kept["onto"]), (&json!(end), &json!("moved")), "{kept}");
    assert_eq!(words(&made), (json!("kept"), json!(end), json!("refs/heads/moved")));
    assert!(made.w.branches(&made.repo).is_empty());
    let before = made.held();
    made.refuses(&made.toss(), &before, "the work of run {run} was kept, so it is not tossed: nothing was changed");
}

#[test]
fn a_cut_off_keep_whose_work_landed_on_a_detached_head_is_not_tossed() {
    let made = finished();
    let (start, end) = (made.w.commit(&made.repo, "main"), made.end());
    moved_and_cut(&made, &["checkout", "-q", "--detach"]);
    assert_eq!(words(&made), (Value::Null, json!(end), json!("refs/heads/main")));
    assert_eq!((made.w.commit(&made.repo, "main"), made.git(&["rev-parse", "HEAD"])), (start, end.clone()));
    let before = made.held();
    made.refuses(&made.toss(), &before, LANDED);
    assert_eq!(made.w.commit(&made.repo, &made.branch()), end, "the one name the commit has stays");
    // Back on a branch, the next keep lands the work there as any keep does.
    made.git(&["checkout", "-q", "main"]);
    let kept = printed(&made.keep(&[]));
    assert_eq!((&kept["kept"], &kept["onto"]), (&json!(end), &json!("main")), "{kept}");
    assert_eq!((words(&made).0, made.w.commit(&made.repo, "main")), (json!("kept"), end));
}

// The control: other branches and a detached `HEAD` that do not hold the commit are no landing. A keep cut off
// before its step, in such a repo, is tossed, and the run's branch, which does hold the commit, is deleted.
#[test]
fn a_cut_off_keep_whose_step_never_ran_is_tossed_whatever_other_branches_and_head_hold() {
    for (step, detached) in [(1, false), (2, false), (2, true)] {
        let made = finished();
        let (start, end) = (made.w.commit(&made.repo, "main"), json!(made.end()));
        made.git(&["branch", "feature", "main"]);
        let ran = made.keep_in(&made.repo, &[], &[("JELLYMOON_TEST_KEEP_CUT", step.to_string().into())]);
        assert_eq!(ran.code, 70, "{}", ran.stderr);
        let meant = if step == 1 { Value::Null } else { end.clone() };
        assert_eq!(words(&made).1, meant, "step {step}");
        if detached {
            made.git(&["checkout", "-q", "--detach"]);
        }
        let person = made.w.persons(&made.repo);
        tossed(&made, &made.toss(), true, end.clone(), &format!("{WORK}: {DELETED}"));
        made.is_tossed(&end, 1);
        assert!(made.w.branches(&made.repo).is_empty(), "step {step}");
        assert_eq!((made.w.commit(&made.repo, "main"), made.w.commit(&made.repo, "feature")), (start.clone(), start));
        assert!(person == made.w.persons(&made.repo), "step {step}");
    }
    // An id no commit has is in no history: a record that holds one is tossed as one whose keep never landed.
    let made = finished();
    let (end, mut record) = (json!(made.end()), made.record());
    (record["kept"], record["onto"]) = (json!("1".repeat(40)), json!("refs/heads/main"));
    made.write(&record);
    tossed(&made, &made.toss(), true, end.clone(), &format!("{WORK}: {DELETED}"));
    made.is_tossed(&end, 1);
}

// F45's last fix: a branch named as a run's is, in any case of its letters, is no branch of the person's. One
// that holds the commit a cut-off keep meant is no landing, and the toss deletes the run's own name and no other.
#[test]
fn a_branch_named_as_a_runs_is_that_holds_the_commit_is_no_landing_and_is_left() {
    let made = finished();
    let end = json!(made.end());
    let ran = made.keep_in(&made.repo, &[], &[("JELLYMOON_TEST_KEEP_CUT", "2".into())]);
    assert_eq!((ran.code, words(&made).1), (70, end.clone()), "{}", ran.stderr);
    // Packed first, so the repo lists each spelling as it was given, on a disk that folds case too.
    made.git(&["pack-refs", "--all"]);
    for named in ["Jellymoon/run-zz", "JELLYMOON/RUN-yy/deep", "jellymoon/run-other"] {
        made.git(&["update-ref", &format!("refs/heads/{named}"), end.as_str().unwrap()]);
    }
    // Nor does a checkout that stands on one of them hold the work as a person's branch would.
    made.git(&["symbolic-ref", "HEAD", "refs/heads/jellymoon/run-other"]);
    let (others, all) = (made.others(), made.git(&REFS).lines().count());
    tossed(&made, &made.toss(), true, end.clone(), &format!("{WORK}: {DELETED}"));
    made.is_tossed(&end, 1);
    assert_eq!((made.others(), made.git(&REFS).lines().count()), (others, all - 1), "one name went: the run's own");
}

// A disk that folds case takes `Jellymoon/run-<id>` for the run's branch itself: a checkout whose `HEAD` names
// the branch so stands on it, and the branch is not deleted from under it.
#[test]
fn a_checkout_that_stands_on_the_runs_branch_under_another_spelling_of_its_name_is_stood_on() {
    let made = finished();
    made.git(&["symbolic-ref", "HEAD", &format!("refs/heads/Jellymoon/run-{}", made.run)]);
    let before = made.held();
    let stood = "a checkout stands on the branch of run {run}: its work was not tossed, and nothing was changed";
    made.refuses(&made.toss(), &before, stood);
    assert_eq!(made.w.commit(&made.repo, &made.branch()), made.end());
}

// A keep that found the checkout's `HEAD` detached at the step wrote `kept` with the one word `HEAD` and left the
// run's branch on purpose, as the one name the commit has. That is kept work: a toss says so and changes nothing.
#[test]
fn work_kept_on_no_branch_whose_runs_branch_the_keep_left_is_kept_and_not_tossed() {
    let made = finished();
    let end = made.end();
    let kept = printed(&moved(&made, &["checkout", "-q", "--detach"], &[]));
    assert_eq!((&kept["kept"], &kept["onto"]), (&json!(end), &Value::Null), "{kept}");
    assert_eq!(words(&made), (json!("kept"), json!(end), json!("HEAD")));
    assert_eq!(made.w.commit(&made.repo, &made.branch()), end, "the keep left the run's branch");
    let before = made.held();
    made.refuses(&made.toss(), &before, "the work of run {run} was kept, so it is not tossed: nothing was changed");
    assert_eq!((made.w.commit(&made.repo, &made.branch()), &made.w.kept(&made.wish)["state"]), (end, &json!("done")));
}

// A rebase or a bisect that goes back to the run's branch stands on it too, and git keeps that name in a file:
// it is read in either case of its letters, by its whole name and its short one. The files are typed here as git
// writes them, so this holds on a disk that keeps case apart as well.
#[test]
fn a_rebase_or_a_bisect_that_goes_back_to_the_runs_branch_under_another_spelling_stands_on_it() {
    let made = finished();
    let (git, short) = (made.repo.join(".git"), format!("Jellymoon/run-{}", made.run));
    let whole = format!("refs/heads/{short}");
    let stood = "a checkout stands on the branch of run {run}: its work was not tossed, and nothing was changed";
    for (folder, file, held) in [
        (Some("rebase-merge"), "head-name", &whole),
        (Some("rebase-apply"), "head-name", &whole),
        (None, "BISECT_START", &short),
    ] {
        let at = folder.map_or(git.clone(), |folder| git.join(folder));
        fs::create_dir_all(&at).unwrap();
        fs::write(at.join(file), format!("{held}\n")).unwrap();
        let before = made.held();
        made.refuses(&made.toss(), &before, stood);
        fs::remove_file(at.join(file)).unwrap();
        if folder.is_some() {
            fs::remove_dir(&at).unwrap();
        }
    }
    // The control: with none of the files there, the same toss goes through.
    tossed(&made, &made.toss(), true, json!(made.end()), &format!("{WORK}: {DELETED}"));
}
