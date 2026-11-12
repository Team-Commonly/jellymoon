//! The folder a session log names is a word of the log. It is resolved once, and the folder it leads to must carry
//! the wish's project as its own last name: a link named after one repo that leads to another starts no run.

use std::fs;
use std::path::Path;

use serde_json::json;

use super::{DAY, ended, found_world, made, printed, wait_for};
use crate::support::skipped;

/// A link to a folder. False where the account may not make one (Windows).
pub(super) fn link(to: &Path, at: &Path) -> bool {
    #[cfg(unix)]
    return std::os::unix::fs::symlink(to, at).is_ok();
    #[cfg(windows)]
    return std::os::windows::fs::symlink_dir(to, at).is_ok();
}

#[test]
fn a_link_of_the_project_s_name_that_leads_to_a_repo_of_another_name_starts_no_run() {
    let w = found_world();
    let beta = w.repo("beta");
    let alpha = w.at("work").join("alpha");
    if !link(&beta, &alpha) {
        return skipped("no link to a folder can be made here");
    }
    w.typed_in("-work-alpha", &alpha);
    let said = w.wish_run("alpha", &[]);
    let why = format!(
        "the folder of that name typed in on {DAY} leads to a folder of another name; name the folder with --repo"
    );
    assert_eq!(said, json!({"wish": made("m1", "alpha"), "run": null, "why": why}));
    w.nothing_was_started("m1", "alpha");
    assert!(w.branches(&beta).is_empty(), "a branch was left in the repo the link leads to");
}

// The log's folder is looked up once. The command is held with the lookup done and nothing yet asked of git, and
// the link is led to another repo of the same name: the copy is made of the folder the lookup led to.
#[test]
fn a_link_led_elsewhere_after_the_folder_was_looked_up_is_not_followed_a_second_time() {
    let w = found_world();
    let (first, second) = (w.repo("first/alpha"), w.repo("second/alpha"));
    let alpha = w.at("work").join("alpha");
    if !link(&first, &alpha) {
        return skipped("no link to a folder can be made here");
    }
    w.typed_in("-work-alpha", &alpha);
    let (wish, go) = (w.wish("alpha"), w.at("rec").join("placed"));
    let held = [("JELLYMOON_TEST_PLACE_WAIT_MS", "60000".into()), ("JELLYMOON_TEST_GO", go.clone().into())];
    let going = w.going(&held, &["run", "--wish", &wish, "--date", DAY]);
    wait_for(&go.with_extension("at"));
    fs::remove_file(&alpha).or_else(|_| fs::remove_dir(&alpha)).unwrap();
    assert!(link(&second, &alpha));
    fs::write(&go, "").unwrap();
    let said = printed(&ended(going));
    let branch = format!("jellymoon/run-{}", said["run"].as_str().unwrap());
    assert_eq!((&said["outcome"], w.branches(&first)), (&json!("completed"), vec![branch]));
    assert!(w.branches(&second).is_empty(), "the copy was made of the folder the link was led to afterwards");
}

// A folder typed in another case, or another Unicode form, than it is stored in is the same folder wherever the
// file system takes the two spellings for one name: its last name is no link, so it is the folder's own.
#[test]
fn a_folder_typed_in_another_spelling_of_its_own_name_is_the_folder_a_run_copies() {
    for (stored, typed) in [("Alpha", "alpha"), ("caf\u{e9}", "cafe\u{301}")] {
        let w = found_world();
        let repo = w.repo(stored);
        let named = w.at("work").join(typed);
        if !named.is_dir() {
            skipped(&format!("this file system does not take {typed:?} and {stored:?} for one name"));
            continue;
        }
        w.typed_in("-work-typed", &named);
        let said = w.wish_run(typed, &[]);
        let run = said["run"].as_str().unwrap_or_else(|| panic!("no run was started: {said}")).to_string();
        assert_eq!(w.ended_as(&run)["outcome"], "completed");
        w.command_gone(&run);
        assert_eq!(w.branches(&repo), [format!("jellymoon/run-{run}")], "{typed:?}");
    }
}

// The folder the link leads to is the one that is checked and the one that is copied: a link whose folder has the
// project's name runs there, and one that leads nowhere is refused before anything is asked of git.
#[test]
fn a_link_to_a_repo_of_the_project_s_name_runs_there_and_one_that_leads_nowhere_starts_no_run() {
    let w = found_world();
    let real = w.repo("real/alpha");
    let alpha = w.at("work").join("alpha");
    if !link(&real, &alpha) {
        return skipped("no link to a folder can be made here");
    }
    w.typed_in("-work-alpha", &alpha);
    let run = w.wish_run("alpha", &[])["run"].as_str().expect("no run was started").to_string();
    assert_eq!(w.ended_as(&run)["outcome"], "completed");
    w.command_gone(&run);
    assert_eq!(w.branches(&real), [format!("jellymoon/run-{run}")]);

    let w = found_world();
    let alpha = w.at("work").join("alpha");
    assert!(link(&w.at("work").join("nothing-here"), &alpha));
    w.typed_in("-work-alpha", &alpha);
    let said = w.wish_run("alpha", &[]);
    let why = format!(
        "the folder of that name typed in on {DAY} is not one a run can copy: it is not there, or cannot be reached"
    );
    assert_eq!(said, json!({"wish": made("m1", "alpha"), "run": null, "why": why}));
    w.nothing_was_started("m1", "alpha");
    assert!(fs::symlink_metadata(&alpha).is_ok(), "the link itself was removed");
}
