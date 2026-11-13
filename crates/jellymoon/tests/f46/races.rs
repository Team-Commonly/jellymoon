//! A branch that is changed between the toss's question and its delete (`docs/development/toss.md`, "The
//! delete"). The made-up git (`support/fake_git.rs`) stands first on `PATH` and does something with the real git
//! just before each `update-ref` of the toss's. The delete names the run's own ref and the commit it was asked
//! about, so git refuses it when either is no longer so, and no other name is ever written.

use std::fs;

use serde_json::json;

use super::{BEFORE, Made, WORK, finished, tossed};
use crate::support::Ran;

const RACED: &str = "its branch is left, as it was changed while the toss was at work";

/// A toss of finished work with `first` done in the repo just before its `update-ref`.
fn raced(first: &[&str]) -> (Made, Ran, Vec<String>) {
    raced_on(finished(), first)
}

fn raced_on(made: Made, first: &[&str]) -> (Made, Ran, Vec<String>) {
    let (vars, fake, others) = (made.w.fake_git(), made.w.at("fake"), made.others());
    let git = format!("--git-dir={}", made.repo.join(".git").display());
    let lines = [&[git.as_str()][..], first].concat().join("\n").replace("{name}", &made.branch());
    fs::write(fake.join("first"), lines).unwrap();
    fs::write(fake.join("on"), "update-ref").unwrap();
    fs::write(fake.join("returns"), "").unwrap();
    let ran = made.toss_in(&made.repo, &vars);
    (made, ran, others)
}

/// Whether the run's name is a symbolic ref.
fn symbolic(made: &Made) -> bool {
    made.w.git_ran(&made.repo, &["symbolic-ref", "-q", &made.branch()]).status.success()
}

#[test]
fn a_branch_moved_or_made_a_name_for_another_just_before_the_delete_is_left_as_it_is() {
    for first in [&["update-ref", "{name}", "main"][..], &["symbolic-ref", "{name}", "refs/heads/main"]] {
        let (made, ran, others) = raced(first);
        let end = json!(made.end());
        tossed(&made, &ran, true, end.clone(), &format!("{WORK}: {RACED}, and the wish is let go"));
        // The name is as the other hand left it, and the person's branch, which no delete went through to, is
        // where it was with every other ref.
        assert_eq!(made.w.commit(&made.repo, &made.branch()), made.w.commit(&made.repo, "main"));
        assert_eq!(symbolic(&made), first[0] == "symbolic-ref");
        assert_eq!(made.others(), others);
        made.is_tossed(&end, 1);
        // The next toss finds a branch that is not the run's own at that commit, and leaves it.
        let before = made.held();
        let ran = made.toss();
        let left = if first[0] == "update-ref" {
            "its branch is left, as it is not at the commit the record names for it"
        } else {
            "its branch is left, as it is a name for another branch by now"
        };
        tossed(&made, &ran, false, end, &format!("{BEFORE}: {left}"));
        assert!(before == made.held());
    }
}

// The person moved `main` forward to the run's commit themselves, and the run's name is made a name for `main`
// just before the delete. The old value holds through such a name, so only `--no-deref` stands between the
// delete and the person's branch: the name goes, as itself, and `main` is where it was.
#[test]
fn a_name_made_a_name_for_a_branch_at_the_same_commit_is_deleted_as_itself_and_the_branch_stays() {
    let made = finished();
    let end = made.end();
    made.git(&["merge", "-q", "--ff-only", &end]);
    let (made, ran, others) = raced_on(made, &["symbolic-ref", "{name}", "refs/heads/main"]);
    tossed(&made, &ran, true, json!(end), &format!("{WORK}: {}", super::DELETED));
    assert_eq!(
        (made.w.commit(&made.repo, "main"), made.git(&["symbolic-ref", "HEAD"])),
        (end, "refs/heads/main".into())
    );
    assert_eq!((made.others(), made.w.branches(&made.repo)), (others, vec![]));
    // The control: the same delete by plain git, without `--no-deref`, takes the person's branch away.
    made.git(&["symbolic-ref", &made.branch(), "refs/heads/main"]);
    made.git(&["update-ref", "-d", &made.branch(), &made.end()]);
    assert!(!made.w.git_ran(&made.repo, &["rev-parse", "--verify", "-q", "refs/heads/main"]).status.success());
}

#[test]
fn a_branch_removed_just_before_the_delete_is_said_to_be_gone() {
    let (made, ran, others) = raced(&["update-ref", "-d", "{name}"]);
    let end = json!(made.end());
    tossed(&made, &ran, true, end.clone(), &format!("{WORK}: its branch was not in the repo, and the wish is let go"));
    assert_eq!(made.others(), others);
    made.is_tossed(&end, 1);
}

// git deletes a branch by `update-ref` whoever stands on it. A checkout that came to stand on the run's branch
// in that moment would be left on a branch that is not there: the name is put back where it was.
#[test]
fn a_checkout_that_came_to_stand_on_the_branch_just_before_the_delete_stands_on_it_still() {
    let (made, ran, others) = raced(&["symbolic-ref", "HEAD", "{name}"]);
    let end = made.end();
    let stood = "its branch is left, as a checkout stands on it";
    tossed(&made, &ran, true, json!(end), &format!("{WORK}: {stood}, and the wish is let go"));
    assert_eq!((made.git(&["symbolic-ref", "HEAD"]), made.w.commit(&made.repo, "HEAD")), (made.branch(), end.clone()));
    assert!(!symbolic(&made), "it is a plain branch again");
    assert_eq!(made.others(), others);
    made.is_tossed(&json!(end), 1);
    let before = made.held();
    tossed(&made, &made.toss(), false, json!(end), &format!("{BEFORE}: {stood}"));
    assert!(before == made.held());
}
