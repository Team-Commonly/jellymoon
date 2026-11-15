//! A listed project on a day nothing was typed in it: `jellymoon run` and `jellymoon wish --run` find its repo,
//! and core's `wishable` offers it, which `jellymoon hear` shows. A listed folder that is gone, is no repo's
//! top, is not the person's own or is switched off is not offered and starts nothing, and stays listed.

use std::fs;

use serde_json::{Value, json};

use super::refused::{link, someone_elses_home};
use super::*;
use crate::support::skipped;

/// `jellymoon wish --run` for the project, with `more` arguments: what it printed.
fn wish_run(w: &World, more: &[&str]) -> Value {
    let args = ["wish", "--date", DAY, "--say", SAID, "--project", "alpha", "--run"];
    printed(&w.run(&[&args[..], more].concat()))
}

#[test]
fn a_listed_project_is_found_by_a_run_on_a_day_nothing_was_typed_in_it() {
    // With no session logs at all, and with logs in which another folder was typed in.
    for logs in [false, true] {
        let w = heard_world();
        let repo = w.repo("alpha");
        if logs {
            w.typed("beta");
        }
        printed(&w.add_on(&repo, "2026-01-05"));
        let before = w.persons(&repo);
        let ran = w.run_found("alpha", &[]);
        assert_eq!(printed(&ran)["outcome"], "completed", "logs: {logs}");
        names_no_path(&w, &ran);
        assert_eq!((w.branches(&repo).len(), w.starts()), (1, 1));
        assert!(w.persons(&repo) == before, "the person's checkout was written");
        assert_eq!(w.listed(), json!([{"name": "alpha", "used": DAY}]));
    }
}

#[test]
fn a_listed_project_is_found_by_wish_run_and_offered_to_a_wish_said_aloud() {
    let w = heard_world();
    let repo = w.repo("alpha");
    // Not listed, it is not offered: nothing was typed.
    assert_eq!(w.heard_for(&[]), cannot(NO_PROJECT));
    printed(&w.add(&repo));
    assert_eq!(w.heard_for(&[]), heard(SAID, "alpha", 1));
    // And the wish is made for it, by the call the page's entry makes (`wish_offered`).
    let mut events = hold(0, SAID);
    events.push(tick(5000));
    assert_eq!(w.hear(&[], &events).last(), Some(&with(quiet(), "made", json!("m1"))));

    let said = wish_run(&w, &[]);
    assert!(said["run"].is_string(), "{said}");
    let run = w.run_ended("m2");
    assert_eq!(w.record(&run)["outcome"], "completed");
    assert_eq!(w.branches(&repo), [format!("jellymoon/run-{run}")]);
}

/// The listed project of `w` is not offered and starts nothing, by `run`, by `wish --run` and by a wish said
/// aloud, with `more` arguments; and it is still listed.
fn falls(w: &World, repo: &std::path::Path, why: &str, more: &[&str]) {
    let list = fs::read(w.list_file()).unwrap();
    assert_eq!(w.heard_for(more), cannot(NO_PROJECT), "{why}");
    let wish = w.wish("alpha");
    let ran = w.run(&[&["run", "--wish", &wish, "--date", DAY][..], more].concat());
    assert_eq!(refused(&ran), format!("jellymoon: {}\n", fallen(why)));
    names_no_path(w, &ran);
    let apart = [&["wish", "--date", DAY, "--say", SAID, "--project", "alpha", "--run"][..], more].concat();
    let said = printed(&w.run(&apart));
    assert_eq!((&said["run"], &said["why"]), (&Value::Null, &json!(fallen(why))));
    w.started_nothing(repo);
    assert!(fs::read(w.list_file()).unwrap() == list, "a folder that fell was taken off the list, or changed");
    assert_eq!(w.listed(), json!([{"name": "alpha", "used": DAY}]));
}

#[test]
fn a_listed_folder_that_is_gone_or_is_no_repo_s_top_is_not_offered_and_starts_nothing() {
    let w = heard_world();
    let repo = w.repo("alpha");
    printed(&w.add(&repo));
    fs::rename(repo.join(".git"), w.at("work").join("put-away")).unwrap();
    falls(&w, &repo, "it is not the top of a git repo", &[]);
    fs::remove_dir_all(&repo).unwrap();
    falls(&w, &repo, "it is not there, or is reached through a link now", &[]);
    // A file where the folder was is no folder.
    fs::write(&repo, "a file now\n").unwrap();
    falls(&w, &repo, "it is not there, or is reached through a link now", &[]);
}

#[test]
fn a_link_put_where_a_listed_folder_was_leads_nowhere() {
    let w = heard_world();
    let (repo, elsewhere) = (w.repo("alpha"), w.repo("elsewhere/alpha"));
    printed(&w.add(&repo));
    fs::rename(&repo, w.at("work").join("moved")).unwrap();
    if !link(&elsewhere, &repo) {
        return skipped("this system makes no link for this user");
    }
    falls(&w, &elsewhere, "it is not there, or is reached through a link now", &[]);
}

#[test]
fn a_listed_folder_that_is_switched_off_is_not_offered_and_starts_nothing() {
    let w = heard_world();
    let repo = w.repo("alpha");
    printed(&w.add(&repo));
    // By a name given, matched against the folder's whole path as a session's folder is.
    falls(&w, &repo, "it is switched off", &["--skip", "beta,work"]);
    // And by the file of folders switched off.
    fs::write(w.jm("skip.txt"), "alpha\n").unwrap();
    falls(&w, &repo, "it is switched off", &[]);
    // A file of them that cannot be used offers nothing at all.
    fs::write(w.jm("skip.txt"), b"al\0pha\n").unwrap();
    assert_eq!(w.heard_for(&[]), cannot(NO_PROJECT));
    let ran = w.run(&["run", "--wish", "m1", "--date", DAY]);
    assert!(refused(&ran).contains("skip.txt"), "{}", ran.stderr);
    w.started_nothing(&repo);
}

#[test]
fn a_listed_folder_that_is_not_the_person_s_own_is_not_offered_and_starts_nothing() {
    let Some(home) = someone_elses_home() else {
        return skipped("no folder here is known to be someone else's: the system has no owners, or the user is root");
    };
    let w = heard_world();
    let repo = w.repo("alpha");
    printed(&w.add(&repo));
    let wish = w.wish("alpha");
    let ran = w.run_with(std::slice::from_ref(&home), &["run", "--wish", &wish, "--date", DAY]);
    assert_eq!(refused(&ran), format!("jellymoon: {}\n", fallen("it is not a folder of your own")));
    let apart = ["wish", "--date", DAY, "--say", SAID, "--project", "alpha", "--run"];
    let said = printed(&w.run_with(std::slice::from_ref(&home), &apart));
    assert_eq!((&said["run"], &said["why"]), (&Value::Null, &json!(fallen("it is not a folder of your own"))));
    w.started_nothing(&repo);
    assert_eq!(w.listed(), json!([{"name": "alpha", "used": DAY}]));
}

#[test]
fn a_listed_folder_git_cannot_copy_is_refused_when_the_run_copies_it_in_fixed_words() {
    let w = heard_world();
    let repo = w.folder("alpha");
    // A `.git` that is no repo: the look that offers a project asks git nothing, and the run's copy does.
    fs::create_dir(repo.join(".git")).unwrap();
    printed(&w.add(&repo));
    let ran = w.run_found("alpha", &[]);
    let why = refused(&ran);
    let start = "jellymoon: the folder listed under the name the wish gives is not one a run can copy: it is not a \
                 git repo git could open: ";
    assert!(why.starts_with(start), "{why}");
    names_no_path(&w, &ran);
    assert_eq!((w.copies(), w.starts()), (vec![], 0));
}
