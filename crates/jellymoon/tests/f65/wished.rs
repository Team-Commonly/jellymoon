//! A wish for a made project: a run starts in it with no folder named and no session log that names it, the
//! run's copy and its commit work on a repo whose first commit holds no file, and a keep lands the work in the
//! made folder. And a made project that is forgotten: the list forgets it and the folder stays.

use std::fs;

use serde_json::json;

use super::refused::{THERE, new_refused};
use super::*;
use crate::support::tree;

#[test]
fn a_wish_for_the_new_project_starts_a_run_in_it_and_a_keep_lands_its_work_there() {
    let w = heard_world();
    w.play("hello", 0);
    printed(&w.new_project("alpha"));
    let made = w.made("alpha");
    let first = w.commit(&made, "main");

    // No folder is named and there is no session log at all: the list is where the repo is found.
    let ran = w.run_found("alpha");
    assert_eq!(printed(&ran)["outcome"], "completed");
    names_no_path(&w, &ran);
    let branches = w.branches(&made);
    assert_eq!((branches.len(), w.starts()), (1, 1));
    // The run's commit is on top of the empty first commit, and holds the one file the agent wrote.
    let end = w.commit(&made, &branches[0]);
    assert_eq!(w.git(&made, &["rev-list", &branches[0]]), format!("{end}\n{first}"));
    assert_eq!(w.git(&made, &["ls-tree", "-r", "--name-only", &end]), "HELLO.txt");
    assert_eq!(fs::read_dir(&made).unwrap().count(), 1, "the run wrote in the made folder itself");

    let shown = printed(&w.run(&["run", "show", "--wish", "m1", "--date", DAY]));
    assert_eq!(shown["work"], "waits", "{shown}");
    let kept = w.run(&["run", "keep", "--wish", "m1", "--date", DAY]);
    names_no_path(&w, &kept);
    let kept = printed(&kept);
    assert_eq!((&kept["kept"], &kept["onto"]), (&json!(end), &json!("main")), "{kept}");
    // The work is in the made folder, on `main`, and git finds nothing to say of the checkout.
    assert_eq!(fs::read_to_string(made.join("HELLO.txt")).unwrap(), "Hello\n");
    assert_eq!(w.git(&made, &["rev-list", "main"]), format!("{end}\n{first}"));
    assert_eq!(w.git(&made, &["status", "--porcelain"]), "");
}

#[test]
fn the_new_project_is_offered_to_a_wish_said_aloud_and_wish_run_starts_in_it() {
    let w = heard_world();
    w.play("hello", 0);
    // Nothing was typed and nothing is listed: no project is offered.
    assert_eq!(w.heard_for(), cannot(NO_PROJECT));
    printed(&w.new_project("alpha"));
    assert_eq!(w.heard_for(), heard(SAID, "alpha", 1));

    let said = printed(&w.run(&["wish", "--date", DAY, "--say", SAID, "--project", "alpha", "--run"]));
    assert!(said["run"].is_string(), "{said}");
    let run = w.run_ended("m1");
    assert_eq!(w.record(&run)["outcome"], "completed");
    assert_eq!(w.branches(&w.made("alpha")), [format!("jellymoon/run-{run}")]);
}

#[test]
fn a_made_project_that_is_forgotten_leaves_the_list_and_keeps_its_folder() {
    let w = heard_world();
    w.play("hello", 0);
    printed(&w.new_project("alpha"));
    let made = w.made("alpha");
    let before = tree(&made);
    assert_eq!(printed(&w.run(&["project", "forget", "alpha"])), json!({"name": "alpha", "used": DAY}));
    assert_eq!(w.listed(), json!([]));
    assert!(tree(&made) == before, "forgetting a made project touched its folder");
    // It is found no more, and its name is taken still: by the folder that is there.
    let ran = w.run_found("alpha");
    assert!(refused(&ran).starts_with("jellymoon: no Claude Code session logs were found"), "{}", ran.stderr);
    new_refused(&w, "alpha", THERE);
    // Listed again by its folder, as the sentence says, it is run in as before.
    let real = w.real(&made);
    assert_eq!(printed(&w.run(&["project", "add", "--repo", &real, "--date", DAY]))["name"], "alpha");
    assert_eq!(printed(&w.run_found("alpha"))["outcome"], "completed");
    assert_eq!(w.branches(&made).len(), 1);
}
