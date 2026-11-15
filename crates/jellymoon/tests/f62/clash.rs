//! A name both typed in that day and listed. Two folders that are not one repo are not offered and start
//! nothing, in words that name `--repo` and `project forget`; one folder under both is the project it was.

use serde_json::{Value, json};

use super::*;

fn clash() -> String {
    format!(
        "a folder of the name the wish gives was typed in on {DAY} and another folder is listed under that name; \
         {WAY_OUT}"
    )
}

#[test]
fn a_name_typed_in_and_listed_with_two_folders_is_not_offered_and_starts_nothing() {
    let w = heard_world();
    let (typed, listed) = (w.repo("alpha"), w.repo("other/alpha"));
    printed(&w.add(&listed));
    w.typed_at("-work-alpha", &typed);
    assert!(clash().contains("--repo") && clash().contains("`jellymoon project forget`"));

    assert_eq!(w.heard_for(&[]), cannot(NO_PROJECT), "a name with two folders was offered");
    let ran = w.run_found("alpha", &[]);
    assert_eq!(refused(&ran), format!("jellymoon: {}\n", clash()));
    names_no_path(&w, &ran);
    let said = printed(&w.run(&["wish", "--date", DAY, "--say", SAID, "--project", "alpha", "--run"]));
    assert_eq!((&said["run"], &said["why"]), (&Value::Null, &json!(clash())));
    w.started_nothing(&typed);
    w.started_nothing(&listed);

    // Another project typed in that day is still offered, and the name with two folders is not chosen.
    w.typed("beta");
    assert_eq!(w.heard_for(&[]), heard(SAID, "beta", 1));
}

#[test]
fn repo_settles_a_name_with_two_folders_at_the_command_line_and_forget_settles_it_for_good() {
    let w = heard_world();
    let (typed, listed) = (w.repo("alpha"), w.repo("other/alpha"));
    printed(&w.add(&listed));
    w.typed_at("-work-alpha", &typed);
    let wish = w.wish("alpha");
    assert_eq!(printed(&w.start(&wish, &typed))["outcome"], "completed");
    assert_eq!((w.branches(&typed).len(), w.branches(&listed).len()), (1, 0));
    // The list is as it was: the folder a run was named is not put where another is listed.
    assert_eq!(w.on_disk(), json!({"projects": [w.line("alpha", &w.real(&listed))]}));

    printed(&w.run(&["project", "forget", "alpha"]));
    assert_eq!(w.heard_for(&[]), heard(SAID, "alpha", 1));
    assert_eq!(printed(&w.run_found("alpha", &[]))["outcome"], "completed");
    assert_eq!((w.branches(&typed).len(), w.branches(&listed).len()), (2, 0));
    // The run that found it by the day's typing listed it.
    assert_eq!(w.on_disk(), json!({"projects": [w.line("alpha", &w.real(&typed))]}));
}

#[test]
fn a_name_typed_in_and_listed_with_one_folder_is_offered_and_runs_there() {
    let w = heard_world();
    let repo = w.repo("alpha");
    printed(&w.add_on(&repo, "2026-02-01"));
    // The log names the folder as it was typed, which is not always its real path.
    w.typed_at("-work-alpha", &repo);
    assert_eq!(w.heard_for(&[]), heard(SAID, "alpha", 1));
    assert_eq!(printed(&w.run_found("alpha", &[]))["outcome"], "completed");
    assert_eq!(w.listed(), json!([{"name": "alpha", "used": DAY}]));
    assert_eq!(w.branches(&repo).len(), 1);
}

#[test]
fn two_folders_typed_in_under_a_listed_name_are_refused_as_two_typed_in_folders_are() {
    let w = heard_world();
    let (one, two) = (w.repo("one/alpha"), w.repo("two/alpha"));
    printed(&w.add(&one));
    w.typed_at("-work-one-alpha", &one);
    w.typed_at("-work-two-alpha", &two);
    let both =
        format!("jellymoon: 2 folders of the name the wish gives were typed in on {DAY}; name the one with --repo\n");
    assert_eq!(refused(&w.run_found("alpha", &[])), both);
    w.started_nothing(&one);
    w.started_nothing(&two);
}
