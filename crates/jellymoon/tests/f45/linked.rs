//! A repo with a second checkout, a linked worktree: a keep lands on the branch of the checkout it is asked in and
//! touches no other, and no checkout may stand on the run's branch (`docs/development/keep.md`, "Which checkout").

use std::fs;
use std::path::PathBuf;

use serde_json::json;

use super::{FORWARD, JOINED, Made, UNCHANGED, finished, kept, printed};

/// A second checkout of the repo at `work/linked/alpha`, on a new branch `feat` at `main`.
fn linked(made: &Made) -> PathBuf {
    let feat = made.w.at("work").join("linked").join("alpha");
    made.git(&["worktree", "add", "-q", "-b", "feat", feat.to_str().unwrap()]);
    feat
}

#[test]
fn a_keep_in_a_linked_worktree_lands_on_its_branch_and_touches_no_other_checkout() {
    // Forward: the worktree's branch is where the run started.
    let made = finished();
    let (feat, end) = (linked(&made), made.end());
    let main = made.w.persons(&made.repo);
    kept(&made, &made.keep_in(&feat, &[], &[]), &end, "feat", FORWARD);
    assert_eq!(made.w.commit(&made.repo, "feat"), end);
    assert_eq!(fs::read_to_string(feat.join("n.txt")).unwrap(), "new from the run\n");
    assert_eq!(made.w.git(&feat, &["status", "--porcelain"]), "");
    // The main checkout is as it was but for the one ref that moved, which is not its own.
    let after = made.w.persons(&made.repo);
    assert_eq!((&main.0, &main.1, &main.2), (&after.0, &after.1, &after.2));
    assert_eq!(made.w.commit(&made.repo, "main"), made.record()["start"].as_str().unwrap());
    assert_eq!(made.record()["onto"], json!("refs/heads/feat"));
    assert!(made.w.branches(&made.repo).is_empty());
    // Joined: the worktree's branch has a commit of its own.
    let made = finished();
    let feat = linked(&made);
    made.who();
    fs::write(feat.join("f.txt"), "only on feat\n").unwrap();
    made.w.git(&feat, &["add", "f.txt"]);
    made.w.git(&feat, &["commit", "-q", "-m", "a commit on feat"]);
    let main = made.w.persons(&made.repo);
    let ran = made.keep_in(&feat, &[], &[]);
    let tip = made.w.commit(&made.repo, "feat");
    kept(&made, &ran, &tip, "feat", JOINED);
    assert_eq!(made.w.git(&feat, &["status", "--porcelain"]), "");
    let after = made.w.persons(&made.repo);
    assert_eq!((&main.0, &main.1, &main.2), (&after.0, &after.1, &after.2));
}

#[test]
fn a_keep_is_refused_while_any_checkout_stands_on_the_runs_branch() {
    let made = finished();
    let looked = made.w.at("work").join("looked");
    made.git(&["worktree", "add", "-q", looked.to_str().unwrap(), &format!("jellymoon/run-{}", made.run)]);
    let before = made.held();
    let why = format!("a checkout stands on the branch of run {{run}}: {UNCHANGED}");
    assert_eq!(printed(&made.keep(&["--ask"]))["fits"], json!(false));
    made.refuses(&made.keep(&[]), &before, &why);
    // Something going on in another checkout is nothing to a keep in this one.
    made.w.git(&looked, &["checkout", "-q", "--detach"]);
    made.w.git(&looked, &["bisect", "start"]);
    kept(&made, &made.keep(&[]), &made.end(), "main", FORWARD);
}
