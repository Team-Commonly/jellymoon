//! A run that starts writes its repo on the list, after every check a run makes of a repo. A run that is
//! refused writes nothing there, and a list that cannot take the repo never fails a run.

use std::fs;

use serde_json::json;

use super::*;

#[test]
fn a_run_that_starts_lists_its_repo_and_one_that_is_refused_does_not() {
    let w = heard_world();
    let repo = w.repo("alpha");
    // Refused at the copy: a folder of the wish's name that is no repo is never listed.
    fs::create_dir_all(w.at("bare").join("alpha")).unwrap();
    let wish = w.wish("alpha");
    refused(&w.start(&wish, &w.at("bare").join("alpha")));
    assert!(!w.list_file().exists(), "a run that was refused listed its folder");

    let ran = w.start(&wish, &repo);
    assert_eq!(printed(&ran)["outcome"], "completed");
    names_no_path(&w, &ran);
    assert_eq!(w.on_disk(), json!({"projects": [w.line("alpha", &w.real(&repo))]}));
    assert_eq!(w.listed(), json!([{"name": "alpha", "used": DAY}]));
    assert!(!w.jm("projects.lock").exists());
}

#[test]
fn a_run_named_a_folder_that_another_is_listed_for_goes_and_leaves_the_list_as_it_was() {
    let w = heard_world();
    let (listed, named) = (w.repo("alpha"), w.repo("other/alpha"));
    printed(&w.add_on(&listed, LATER));
    let before = fs::read(w.list_file()).unwrap();
    let wish = w.wish("alpha");
    assert_eq!(printed(&w.start(&wish, &named))["outcome"], "completed");
    assert!(fs::read(w.list_file()).unwrap() == before, "a run changed a line that was not its own");
    assert_eq!((w.branches(&named).len(), w.branches(&listed).len()), (1, 0));
}

#[test]
fn a_run_in_a_folder_whose_real_name_is_not_the_wish_s_project_lists_nothing() {
    let w = heard_world();
    let beta = w.repo("beta");
    // A link named for the wish's project that leads to a repo of another name: `--repo` takes it by its last name.
    let named = w.at("nick").join("alpha");
    fs::create_dir_all(named.parent().unwrap()).unwrap();
    if !crate::refused::link(&beta, &named) {
        return; // this system makes no link for this user
    }
    let wish = w.wish("alpha");
    let ran = w.start(&wish, &named);
    if ran.code == 0 {
        assert!(!w.list_file().exists(), "a folder was listed under a name that is not its own");
    }
}

#[test]
fn a_list_that_is_not_used_never_fails_a_run_and_is_not_written_over() {
    let w = heard_world();
    let repo = w.repo("alpha");
    w.write_list(b"{\"projects\": \"mine\"}\n");
    let wish = w.wish("alpha");
    assert_eq!(printed(&w.start(&wish, &repo))["outcome"], "completed");
    assert_eq!(fs::read(w.list_file()).unwrap(), b"{\"projects\": \"mine\"}\n");
}

#[test]
fn a_run_on_a_later_day_moves_the_day_the_project_was_last_used() {
    let w = heard_world();
    let repo = w.repo("alpha");
    printed(&w.add(&repo));
    let made = printed(&w.run(&["wish", "--date", LATER, "--say", SAID, "--project", "alpha"]));
    assert_eq!(made["wish"]["id"], "m1");
    // No folder is named and no session log is there: the list is where the repo is found.
    let ran = w.run(&["run", "--wish", "m1", "--date", LATER]);
    assert_eq!(printed(&ran)["outcome"], "completed");
    assert_eq!(w.listed(), json!([{"name": "alpha", "used": LATER}]));
    assert_eq!(w.branches(&repo).len(), 1);
    // The run's work is found the same way, by a look and by a toss, with no folder named.
    let shown = printed(&w.run(&["run", "show", "--wish", "m1", "--date", LATER]));
    assert_eq!(shown["work"], "waits", "{shown}");
    printed(&w.run(&["run", "toss", "--wish", "m1", "--date", LATER]));
    assert!(w.branches(&repo).is_empty(), "the toss did not find the repo");
    // Forgotten, the project is found no more, in the words a run had before there was a list.
    printed(&w.run(&["project", "forget", "alpha"]));
    printed(&w.run(&["wish", "--date", LATER, "--say", SAID, "--project", "alpha"]));
    let ran = w.run(&["run", "--wish", "m2", "--date", LATER]);
    assert!(refused(&ran).starts_with("jellymoon: no Claude Code session logs were found"), "{}", ran.stderr);
}

#[test]
fn a_run_refused_at_its_copy_lists_nothing_since_the_list_is_written_after_every_check() {
    let w = heard_world();
    // A repo with no commit passes every look at the folder, and is refused when the copy asks git.
    let empty = w.at("work").join("alpha");
    fs::create_dir_all(&empty).unwrap();
    w.git(&empty, &["init", "-q", "-b", "main"]);
    let wish = w.wish("alpha");
    let ran = w.start(&wish, &empty);
    assert_eq!(refused(&ran), "jellymoon: --repo has no commit yet, so there is nothing to copy\n");
    assert!(!w.list_file().exists(), "a run that was refused listed its folder");
}
