//! What `jellymoon wish --run` keeps a wish for and starts no run for. Each prints the wish with `run` null and
//! core's own line in `why`, exits 0, and leaves no copy, no branch and no record; the agent is never started.

use std::ffi::OsString;
use std::fs;

use serde_json::{Value, json};

use super::{DAY, SAID, World, found_world, made, printed};
use crate::support::skipped;

const RUN: [&str; 8] = ["wish", "--date", DAY, "--say", SAID, "--project", "alpha", "--run"];

/// The one wish of the world was printed as made, with no run and this reason, and nothing was started.
fn kept_with_no_run(w: &World, said: &Value, why: &str) {
    assert_eq!(*said, json!({"wish": made("m1", "alpha"), "run": null, "why": why}));
    w.nothing_was_started("m1", "alpha");
}

#[test]
fn with_no_harness_the_wish_is_kept_and_why_names_the_places_looked() {
    let w = found_world();
    w.typed_in("-work-alpha", &w.repo("alpha"));
    fs::create_dir(w.at("empty")).unwrap();
    let said = printed(&w.run_with(&[("PATH", OsString::from(w.at("empty")))], &RUN));
    let why = "finding Claude Code: no `claude` program in PATH, ~/.local/bin, ~/.claude/local; install Claude \
               Code, or name the program with JELLYMOON_CLAUDE";
    kept_with_no_run(&w, &said, why);
}

#[test]
fn with_no_folder_or_two_folders_of_the_name_the_wish_is_kept_and_no_run_starts() {
    let w = found_world();
    w.typed_in("-work-beta", &w.repo("beta"));
    let none = format!("no folder of the name the wish gives was typed in on {DAY}; name the folder with --repo");
    kept_with_no_run(&w, &w.wish_run("alpha", &[]), &none);

    let w = found_world();
    let (one, two) = (w.repo("one/alpha"), w.repo("two/alpha"));
    w.typed_in("-work-one-alpha", &one);
    w.typed_in("-work-two-alpha", &two);
    let both = format!("2 folders of the name the wish gives were typed in on {DAY}; name the one with --repo");
    kept_with_no_run(&w, &w.wish_run("alpha", &[]), &both);
    assert!(w.branches(&one).is_empty() && w.branches(&two).is_empty());
}

#[test]
fn a_folder_that_is_no_repo_s_top_starts_no_run_and_the_repo_above_it_is_not_used() {
    let w = found_world();
    let above = w.repo("beta");
    let inside = above.join("alpha");
    fs::create_dir(&inside).unwrap();
    w.typed_in("-work-beta-alpha", &inside);
    let said = w.wish_run("alpha", &[]);
    let why = said["why"].as_str().unwrap().to_string();
    let top = format!("the folder of that name typed in on {DAY} is not one a run can copy: it is not a git repo ");
    assert!(why.starts_with(&top) && !why.contains("beta") && !why.contains('\n'), "{why}");
    kept_with_no_run(&w, &said, &why);
    assert!(w.branches(&above).is_empty());
}

// The person is taken to be the owner of their own home folder. A home that is the system's makes every folder
// here someone else's, which is the one way to have such a folder without being another user.
#[test]
fn a_folder_found_in_a_log_that_is_not_the_person_s_own_starts_no_run() {
    #[cfg(unix)]
    let other = {
        use std::os::unix::fs::MetadataExt;
        let owner = |path: &str| fs::metadata(path).unwrap().uid();
        ["/", "/usr"].into_iter().find(|folder| owner(folder) != owner(env!("CARGO_MANIFEST_DIR")))
    };
    #[cfg(not(unix))]
    let other: Option<&str> = None;
    let Some(other) = other else {
        return skipped("no folder here is known to be someone else's: the system has no owners, or the user is root");
    };
    let w = found_world();
    let repo = w.repo("alpha");
    w.typed_in("-work-alpha", &repo);
    let said = printed(&w.run_with(&[("HOME", OsString::from(other))], &RUN));
    let why =
        format!("the folder of that name typed in on {DAY} is not a folder of your own; name the folder with --repo");
    kept_with_no_run(&w, &said, &why);
    assert!(w.branches(&repo).is_empty());
    // Named by the person, the same folder is run in: the word of a log is what was not taken.
    let ran = w.run_with(
        &[("HOME", OsString::from(other))],
        &["run", "--wish", "m1", "--date", DAY, "--repo", repo.to_str().unwrap()],
    );
    assert_eq!(printed(&ran)["outcome"], "completed");
}

// The lock runs are counted behind is opened only when it is a plain file: a folder at its name, or a link that
// would have another file opened and written, starts no run and is left as it is.
#[test]
fn a_folder_or_a_link_where_the_runs_are_counted_starts_no_run_and_is_left_as_it_is() {
    let why = "nothing was started: the lock runs are counted behind is not a plain file";
    let w = found_world();
    let repo = w.repo("alpha");
    w.typed_in("-work-alpha", &repo);
    fs::create_dir_all(w.jm("runs").join("going.lock")).unwrap();
    kept_with_no_run(&w, &printed(&w.run(&RUN)), why);
    assert!(w.jm("runs").join("going.lock").is_dir() && w.branches(&repo).is_empty());

    let w = found_world();
    let repo = w.repo("alpha");
    w.typed_in("-work-alpha", &repo);
    fs::create_dir_all(w.jm("runs")).unwrap();
    let (door, elsewhere) = (w.jm("runs").join("going.lock"), w.at("elsewhere.txt"));
    fs::write(&elsewhere, "the person's own\n").unwrap();
    #[cfg(unix)]
    let linked = std::os::unix::fs::symlink(&elsewhere, &door);
    #[cfg(windows)]
    let linked = std::os::windows::fs::symlink_file(&elsewhere, &door);
    if linked.is_err() {
        return skipped("this system makes no link for this user");
    }
    kept_with_no_run(&w, &printed(&w.run(&RUN)), why);
    assert!(fs::symlink_metadata(&door).unwrap().file_type().is_symlink() && w.branches(&repo).is_empty());
    assert_eq!(fs::read_to_string(&elsewhere).unwrap(), "the person's own\n");
}

#[test]
fn a_folder_switched_off_is_not_found_by_a_run() {
    let w = found_world();
    let repo = w.repo("alpha");
    w.typed_in("-work-alpha", &repo);
    let none = format!("no folder of the name the wish gives was typed in on {DAY}; name the folder with --repo");
    kept_with_no_run(&w, &w.wish_run("alpha", &["--skip", "beta,alpha"]), &none);
    // `jellymoon run` takes the same names, and the file of folders switched off counts as it does for a page.
    let skipped = w.run(&["run", "--wish", "m1", "--date", DAY, "--skip", "alpha"]);
    assert_eq!(super::refused(&skipped), format!("jellymoon: {none}\n"));
    fs::write(w.jm("skip.txt"), "alpha\n").unwrap();
    assert_eq!(super::refused(&w.run(&["run", "--wish", "m1", "--date", DAY])), format!("jellymoon: {none}\n"));
    assert!(w.copies().is_empty() && w.branches(&repo).is_empty() && w.starts() == 0);
}
