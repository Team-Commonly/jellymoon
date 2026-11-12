//! The folder a session log names is a word of the log. It is resolved once, and the folder it leads to must carry
//! the wish's project as its own last name: a link named after one repo that leads to another starts no run.

use std::fs;
use std::path::Path;

use serde_json::json;

use super::{DAY, found_world, made};
use crate::support::skipped;

/// A link to a folder. False where the account may not make one (Windows).
fn link(to: &Path, at: &Path) -> bool {
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
