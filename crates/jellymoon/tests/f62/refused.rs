//! What `jellymoon project add` refuses, each in one line, exit 1, with nothing written; and two changes to the
//! list at once.

use std::ffi::OsString;
use std::fs;
use std::path::Path;

use serde_json::json;

use super::*;
use crate::support::{door, hub_key, shows, skipped};

/// A link at `at` that leads to the folder `to`. False where this system makes none for this user.
pub fn link(to: &Path, at: &Path) -> bool {
    #[cfg(unix)]
    let linked = std::os::unix::fs::symlink(to, at);
    #[cfg(windows)]
    let linked = std::os::windows::fs::symlink_dir(to, at);
    linked.is_ok()
}

/// A folder that is someone else's: the person is taken to be the owner of their home, so a home that is the
/// system's makes every folder of the test someone else's. It is a temporary folder of the system's, the only
/// kind the door passes (`support/door.rs`, `not_ours`). None where there are no owners, or the user is root.
pub fn someone_elses_home() -> Option<(&'static str, OsString)> {
    door::not_ours().map(|other| ("HOME", OsString::from(other)))
}

/// What `add` says of a folder whose last name holds a character that cannot be seen and that cleaning leaves.
pub const EARLIER: &str = "--repo is not a folder a project can be named after: its last name holds a character \
                           that cannot be seen; only an earlier build of Jellymoon kept such a name, and one taken \
                           off the list is not listed again, so rename the folder, then add it";

fn add_refused(w: &World, repo: &Path, why: &str) {
    let before = w.all();
    let ran = w.add(repo);
    assert_eq!(refused(&ran), format!("jellymoon: {why}\n"));
    names_no_path(w, &ran);
    assert!(w.all() == before && w.starts() == 0, "a refusal wrote something: {why}");
}

#[test]
fn add_refuses_a_folder_that_is_not_there_is_no_repo_s_top_or_is_not_named_as_a_project_is() {
    let w = heard_world();
    let repo = w.repo("alpha");
    add_refused(&w, Path::new("work/alpha"), "--repo must be a full path, from the root");
    let gone = "--repo is not a folder that is there";
    add_refused(&w, &w.at("work").join("nowhere"), gone);
    add_refused(&w, &repo.join("a.txt"), gone);
    // A folder with no repo in it, and a folder inside a repo: the repo above is not listed in its place.
    let top = "--repo is not the top of a git repo";
    add_refused(&w, &w.folder("plain"), top);
    fs::create_dir(repo.join("inside")).unwrap();
    add_refused(&w, &repo.join("inside"), top);
    // A last name cleaning would change is no project's name, and is not said back.
    let name = "--repo is not a folder a project can be named after: its last name must be one folder's name of at \
                most 100 characters, with nothing that cannot be seen and nothing shaped like a credential";
    // One that cleaning leaves and nobody sees whole is a name an earlier build kept, and is told so: U+200B is
    // none of those, as cleaning takes it out.
    let holding = |unseen: char| format!("al{unseen}pha");
    let cases = [
        (hub_key(), name),
        ("a".repeat(101), name),
        (holding('\u{200b}'), name),
        (holding('\u{34f}'), EARLIER),
        (holding('\u{115f}'), EARLIER),
        (holding('\u{3164}'), EARLIER),
        (holding('\u{2800}'), EARLIER),
        (holding('\u{fe0f}'), EARLIER),
    ];
    for (odd, why) in cases {
        // Not every file system takes every name: one it refuses is no case here.
        if fs::create_dir_all(w.at("work").join(&odd)).is_err() {
            skipped("this file system does not take one of the names");
            continue;
        }
        let ran = w.add(&w.repo(&odd));
        assert_eq!(refused(&ran), format!("jellymoon: {why}\n"), "{odd:?}");
        assert!(!shows(&ran.stderr, &odd) && !w.at("jm").exists());
    }
    assert!(!w.at("jm").exists(), "a refusal made Jellymoon's folder");
}

#[test]
fn add_refuses_a_folder_that_is_not_the_person_s_own() {
    let Some(home) = someone_elses_home() else {
        return skipped("no folder here is known to be someone else's: the system has no owners, or the user is root");
    };
    let w = heard_world();
    let repo = w.repo("alpha");
    let before = w.all();
    let ran = w.run_with(&[home], &["project", "add", "--repo", repo.to_str().unwrap()]);
    assert_eq!(refused(&ran), "jellymoon: --repo is not a folder of your own\n");
    assert!(w.all() == before);
}

#[test]
fn a_second_folder_under_a_listed_name_is_refused_in_words_that_say_how_to_let_the_first_go() {
    let w = heard_world();
    let (first, second) = (w.repo("alpha"), w.repo("other/alpha"));
    printed(&w.add(&first));
    let why = "another folder is listed under the name alpha, and a name has one folder, so nothing was changed; \
               let the listed one go with `jellymoon project forget alpha`, then add this one";
    add_refused(&w, &second, why);
    assert_eq!(w.on_disk(), json!({"projects": [w.line("alpha", &w.real(&first))]}));
    // The way out it names is the way out.
    printed(&w.run(&["project", "forget", "alpha"]));
    printed(&w.add(&second));
    assert_eq!(w.on_disk(), json!({"projects": [w.line("alpha", &w.real(&second))]}));
}

#[test]
fn a_link_is_listed_as_the_folder_it_leads_to_under_that_folder_s_name() {
    let w = heard_world();
    let repo = w.repo("alpha");
    if !link(&repo, &w.at("work").join("nick")) {
        return skipped("this system makes no link for this user");
    }
    assert_eq!(printed(&w.add(&w.at("work").join("nick"))), json!({"name": "alpha", "used": DAY}));
    assert_eq!(w.on_disk(), json!({"projects": [w.line("alpha", &w.real(&repo))]}));
}

#[test]
fn the_list_holds_a_hundred_projects_and_the_next_is_refused() {
    let w = heard_world();
    let root = w.real(&w.at("work"));
    let sep = std::path::MAIN_SEPARATOR;
    let lines: Vec<_> = (0..100).map(|n| w.line(&format!("p{n}"), &format!("{root}{sep}p{n}"))).collect();
    w.write_list(json!({ "projects": lines }).to_string().as_bytes());
    assert_eq!(w.listed().as_array().unwrap().len(), 100);
    let full = "the list has its 100 projects already, so this one was not added; let one go with `jellymoon \
                project forget`";
    add_refused(&w, &w.repo("alpha"), full);
    printed(&w.run(&["project", "forget", "p7"]));
    assert_eq!(printed(&w.add(&w.at("work").join("alpha")))["name"], "alpha");
    assert_eq!(w.listed().as_array().unwrap().len(), 100);
}

#[test]
fn changes_made_at_once_are_all_kept_and_a_lock_that_stays_refuses_the_change() {
    let w = heard_world();
    let names: Vec<String> = (0..8).map(|n| format!("p{n}")).collect();
    let repos: Vec<_> = names.iter().map(|name| w.repo(name)).collect();
    let going: Vec<_> = repos
        .iter()
        .map(|repo| w.going(&[], &["project", "add", "--repo", repo.to_str().unwrap(), "--date", DAY]))
        .collect();
    for going in going {
        printed(&ended(going));
    }
    let mut listed: Vec<String> =
        w.listed().as_array().unwrap().iter().map(|known| known["name"].as_str().unwrap().to_string()).collect();
    listed.sort();
    assert_eq!(listed, names, "a change made beside another was lost");
    assert!(!w.jm("projects.lock").exists());

    // A lock just made is another change at work: after the tap's three seconds nothing is changed.
    let late = w.repo("late");
    fs::write(w.jm("projects.lock"), "").unwrap();
    let before = w.all();
    let held = "jellymoon: another change to the list of projects, or the making of a new project, has not finished, \
                so nothing was changed; if it was cut off, ask again in half a minute\n";
    assert_eq!(refused(&w.run(&["project", "forget", "p0"])), held);
    assert_eq!(refused(&w.add(&late)), held);
    assert!(w.all() == before);
    // Reading waits for no lock.
    assert_eq!(w.listed().as_array().unwrap().len(), 8);
}

// A killed command's lock, where the lock a take-over is done behind cannot be had: the change is told which
// two files to remove, and not that another change is at work.
#[test]
fn a_lock_of_the_list_left_behind_that_cannot_be_taken_over_is_refused_with_the_names_of_the_files_to_remove() {
    let w = heard_world();
    assert_eq!(printed(&w.add(&w.repo("alpha")))["name"], "alpha");
    let (lock, over) = (w.jm("projects.lock"), w.jm("take-over.lock"));
    // The first change left the take-over's file there; a folder at its name can never be locked.
    let _ = fs::remove_file(&over);
    fs::create_dir(&over).unwrap();
    fs::write(&lock, "").unwrap();
    let a_minute_ago = std::time::SystemTime::now() - std::time::Duration::from_secs(60);
    fs::File::options().write(true).open(&lock).unwrap().set_modified(a_minute_ago).unwrap();
    let beta = w.repo("beta");
    let before = w.all();
    let ran = w.add(&beta);
    let stuck = "jellymoon: jellymoon's own files: projects.lock is a lock that was left behind, and it could not be \
                 taken over, as take-over.lock beside it could not be locked; remove both, then try again\n";
    assert_eq!(refused(&ran), stuck);
    names_no_path(&w, &ran);
    assert_eq!(refused(&w.run(&["project", "forget", "alpha"])), stuck);
    assert!(w.all() == before, "a refusal wrote something");
    // The way out the line gives.
    fs::remove_file(&lock).unwrap();
    fs::remove_dir(&over).unwrap();
    assert_eq!(printed(&w.add(&beta))["name"], "beta");
    assert!(!lock.exists(), "the change's own lock was left");
}
