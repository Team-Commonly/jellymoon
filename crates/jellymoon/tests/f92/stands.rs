//! Where a change that was kept by itself is now (`docs/development/keep.md`, "Where a kept change stands"): a
//! page asks at every look, and `jellymoon run waiting` prints the answer. The record alone says only that the
//! change was kept once.

use std::fs;

use serde_json::{Value, json};

use super::undone::stays;
use super::{Made, landed};

const GONE: &str = "the branch no longer holds this change, so nothing was undone, and nothing was changed";

/// Where the one kept change of the day stands, and whether its files were read.
fn stood(made: &Made) -> (Value, Value) {
    let kept = made.stood();
    assert_eq!(kept.as_array().map(Vec::len), Some(1), "{kept}");
    (kept[0]["stands"].clone(), kept[0]["waits"]["read"].clone())
}

#[test]
fn a_kept_change_is_here_only_while_the_folder_is_on_its_branch_at_its_commit() {
    let made = landed();
    assert_eq!(stood(&made), (json!("here"), json!(true)));
    // On another branch, and on none: the folder stands somewhere else, wherever that branch is.
    made.git(&["checkout", "-q", "-b", "other"]);
    assert_eq!(stood(&made), (json!("elsewhere"), json!(true)));
    made.git(&["checkout", "-q", "--detach"]);
    assert_eq!(stood(&made), (json!("elsewhere"), json!(true)));
    made.git(&["checkout", "-q", "main"]);
    assert_eq!(stood(&made), (json!("here"), json!(true)));

    // The person committed on top: the folder has the change, under a commit.
    fs::write(made.repo.join("mine.txt"), "the person's own\n").unwrap();
    made.git(&["add", "mine.txt"]);
    made.git(&["commit", "-q", "-m", "the person's own commit"]);
    assert_eq!(stood(&made), (json!("past"), json!(true)));

    // The person took the change out of the branch: it holds it no more, and an undo says that.
    made.git(&["reset", "-q", "--hard", "HEAD~2"]);
    assert_eq!(stood(&made), (json!("elsewhere"), json!(true)));
    stays(&made, GONE);
}

#[test]
fn a_kept_change_whose_files_cannot_be_read_is_still_said_where_it_is() {
    let made = landed();
    made.git(&["branch", "-q", "-D", &format!("jellymoon/run-{}", made.run)]);
    assert_eq!(stood(&made), (json!("here"), json!(false)));
    made.git(&["checkout", "-q", "-b", "other"]);
    assert_eq!(stood(&made), (json!("elsewhere"), json!(false)));
}
