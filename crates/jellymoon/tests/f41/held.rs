//! The one rule for whether a left copy's branch goes, once nothing stands on it and no ref names it
//! (`docs/development/run.md`, "What is kept"): every commit on it is also on another branch, a tag or a
//! remote-tracking branch. No `HEAD` counts, wherever it stands, and no other ref does.

use std::fs;
use std::path::Path;

use super::shapes::NONE;
use super::{World, found_world};

impl World {
    /// A checkout detached at a commit made there, which no ref has. Returns the commit.
    fn detached_at_its_own(&self, repo: &Path) -> String {
        self.git(repo, &["checkout", "-q", "--detach"]);
        self.git(repo, &["commit", "-q", "--allow-empty", "-m", "no ref's"]);
        self.commit(repo, "HEAD")
    }
}

// The third gate's first finding. The person commits on a detached `HEAD`, a run is started there and killed,
// and they switch away: the run's branch is the commit's only ref, and git said nothing at the switch because
// of it.
#[test]
fn a_commit_made_on_a_detached_head_keeps_its_only_ref_when_the_checkout_has_left_it() {
    let w = found_world();
    let repo = w.repo("alpha");
    let mine = w.detached_at_its_own(&repo);
    let (branch, run) = w.left_copy(&repo);
    w.git(&repo, &["checkout", "-q", "main"]);
    assert_eq!(w.look(&[]), 0);
    let lost = w.git(&repo, &["fsck", "--unreachable", "--no-reflogs"]);
    assert_eq!((w.holders(&repo, &mine), lost.contains(&mine)), (branch, false), "the commit lost its ref");
    assert_eq!((w.listed(&repo), w.home_of(&run)), (1, NONE));
}

// Its sibling: the commit the copy was made at is amended away in the person's checkout.
#[test]
fn a_commit_amended_away_after_the_copy_was_made_stays_on_the_run_s_branch() {
    let w = found_world();
    let repo = w.repo("alpha");
    let was = w.commit(&repo, "main");
    let (branch, run) = w.left_copy(&repo);
    w.git(&repo, &["commit", "-q", "--amend", "-m", "said again"]);
    assert_eq!(w.look(&[]), 0);
    assert_eq!((w.holders(&repo, &was), w.home_of(&run)), (branch, NONE));
}

// A stash made on the run's branch reaches its commit by `refs/stash`, which goes when the stash is taken out.
#[test]
fn a_stash_made_on_the_run_s_branch_does_not_stand_for_its_commit() {
    let w = found_world();
    let repo = w.repo("alpha");
    let (branch, run) = w.left_copy(&repo);
    w.git(&repo, &["checkout", "-q", "--ignore-other-worktrees", &branch]);
    w.git(&repo, &["commit", "-q", "--allow-empty", "-m", "the person's own"]);
    let mine = w.commit(&repo, "HEAD");
    fs::write(repo.join("a.txt"), "half done\n").unwrap();
    w.git(&repo, &["stash", "-q"]);
    w.git(&repo, &["checkout", "-q", "main"]);
    assert_eq!(w.look(&[]), 0);
    w.git(&repo, &["stash", "drop", "-q"]);
    assert_eq!((w.holders(&repo, &mine), w.home_of(&run)), (branch, NONE));
}

// A bisect marks the run's commit bad from a checkout detached at it: `refs/bisect/bad` goes at the reset, and
// `BISECT_START` names no branch.
#[test]
fn a_bisect_s_mark_on_the_run_s_commit_does_not_stand_for_it() {
    let w = found_world();
    let repo = w.repo("alpha");
    let (branch, run) = w.left_copy(&repo);
    let mine = w.commit_of_its_own(&repo, &run);
    w.git(&repo, &["checkout", "-q", "--detach", &mine]);
    w.git(&repo, &["bisect", "start", &mine]);
    assert_eq!(w.git(&repo, &["rev-parse", "refs/bisect/bad"]), mine);
    assert_eq!(w.look(&[]), 0);
    w.git(&repo, &["bisect", "reset"]);
    w.git(&repo, &["checkout", "-q", "main"]);
    assert_eq!((w.holders(&repo, &mine), w.home_of(&run)), (branch, NONE));
}

// A copy made of a checkout detached at a commit no ref has: the branch is that commit's only ref, and stays.
// No later look asks again, since the record is gone. A branch or a tag that holds the commit by the time of
// the look lets the branch go.
#[test]
fn a_copy_made_of_a_detached_checkout_leaves_its_branch_unless_a_branch_or_a_tag_holds_its_commit() {
    for held in [None, Some("branch"), Some("tag")] {
        let w = found_world();
        let repo = w.repo("alpha");
        let mine = w.detached_at_its_own(&repo);
        let (branch, run) = w.left_copy(&repo);
        if let Some(kind) = held {
            w.git(&repo, &[kind, "keep", &mine]);
        }
        assert_eq!(w.look(&[]), 0);
        let left = if held.is_none() { vec![branch] } else { vec![] };
        assert_eq!((w.branches(&repo), w.listed(&repo), w.home_of(&run)), (left, 1, NONE), "{held:?}");
        assert_ne!(w.holders(&repo, &mine), "", "{held:?}");
    }
}

// Another left copy's branch is another branch. Two copies of one detached commit: the first asked goes, since
// the other holds the commit, and the last name stays.
#[test]
fn of_two_copies_made_of_one_detached_commit_the_last_branch_stays() {
    let w = found_world();
    let repo = w.repo("alpha");
    let mine = w.detached_at_its_own(&repo);
    let (first, one) = w.left_copy(&repo);
    let (second, other) = w.left_copy(&repo);
    assert_eq!(w.look(&[]), 0);
    let left = w.branches(&repo);
    assert!(left == [first] || left == [second], "{left:?}");
    assert_eq!((w.holders(&repo, &mine), w.listed(&repo)), (left[0].clone(), 1));
    assert_eq!((w.home_of(&one), w.home_of(&other)), (NONE, NONE));
}

// Which refs stand for a commit made on the run's branch. A ref that comes and goes, and any ref git or a tool
// keeps elsewhere, does not.
#[test]
fn only_a_branch_a_tag_or_a_remote_tracking_branch_stands_for_a_commit() {
    let refs = [
        ("refs/heads/keep", true),
        ("refs/tags/keep", true),
        ("refs/remotes/origin/keep", true),
        ("refs/stash", false),
        ("refs/bisect/bad", false),
        ("refs/notes/keep", false),
        ("refs/kept/keep", false),
    ];
    for (name, stands) in refs {
        let w = found_world();
        let repo = w.repo("alpha");
        let (branch, run) = w.left_copy(&repo);
        let mine = w.commit_of_its_own(&repo, &run);
        w.git(&repo, &["update-ref", name, &mine]);
        assert_eq!(w.look(&[]), 0);
        let left = if stands { vec![] } else { vec![branch] };
        assert_eq!((w.branches(&repo), w.listed(&repo), w.home_of(&run)), (left, 1, NONE), "{name}");
        assert_eq!(w.commit(&repo, name), mine, "{name}");
    }
}

// A record of an earlier build of this branch names the commit its copy was made at, `base`. It is still read,
// whatever is there, and that word decides nothing: a branch at that very commit, which no other ref has, stays.
#[test]
fn a_record_that_names_the_commit_its_copy_was_made_at_is_read_and_the_word_is_not_used() {
    for number in [false, true] {
        let w = found_world();
        let repo = w.repo("alpha");
        let (branch, run) = w.left_copy(&repo);
        let mine = w.commit_of_its_own(&repo, &run);
        w.noted(&run, "base", if number { 7.into() } else { mine.clone().into() });
        assert_eq!(w.look(&[]), 0);
        assert_eq!((w.holders(&repo, &mine), w.listed(&repo), w.home_of(&run)), (branch, 1, NONE), "{number}");
    }
}
