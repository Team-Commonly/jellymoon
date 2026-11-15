//! A making that does not get to its end. A step that fails (git, a list written over by hand under it) takes
//! back the folder that was made and lists nothing; a lock that is held is waited for, and nothing is made. A
//! command that is killed between its steps leaves the folder, unlisted: once its lock is nobody's, the next
//! `project new` of that name says what is there, and the person mends it. And two at once of one name make
//! one project. The made-up `fake-git` fails a git step
//! (`support/fake_git.rs`); the command is held just before it writes the list by a test build's own wait.

use std::env;
use std::env::consts::EXE_SUFFIX;
use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;

use serde_json::json;

use super::refused::{LISTED, THERE, new_refused};
use super::*;
use crate::support::{plant, tree};

const GIT_FAILED: &str = "git could not make the new project's repo (what git said is not repeated, as it names the \
                          folder), so what was made was taken back and nothing was listed";
pub const BUSY: &str = "another change to the list of projects, or the making of a new project, has not finished, \
                        so nothing was changed; if it was cut off, ask again in half a minute";

/// The variables that put the made-up git first on `PATH`, told to end with 3 at the step that has `step`
/// among its words and to take no other step amiss.
pub fn failing_at(w: &World, step: &str) -> Vec<(&'static str, OsString)> {
    let (folder, name) = (w.at("fake"), format!("git{EXE_SUFFIX}"));
    if !folder.exists() {
        fs::create_dir(&folder).unwrap();
        plant(env!("CARGO_BIN_EXE_fake-git"), &folder.join(&name));
    }
    let path = &w.vars.iter().find(|(name, _)| *name == "PATH").unwrap().1;
    let real = env::split_paths(path).map(|dir| dir.join(&name)).find(|git| git.is_file()).unwrap();
    for (file, holds) in [("real", real.to_str().unwrap()), ("on", step), ("fails", "3"), ("returns", "")] {
        fs::write(folder.join(file), holds).unwrap();
    }
    let _ = fs::remove_file(folder.join("held"));
    vec![("PATH", env::join_paths([folder].into_iter().chain(env::split_paths(path))).unwrap())]
}

/// The whole test folder but the made-up git's own.
pub fn world_of(w: &World) -> Vec<(PathBuf, Vec<u8>)> {
    w.all().into_iter().filter(|(path, _)| !path.starts_with(w.at("fake"))).collect()
}

/// `project new` of a name, held just before it writes the list: the command, and the file that lets it go.
pub fn held_before_the_list(w: &World, name: &str) -> (Going, PathBuf) {
    let go = w.at("rec").join(format!("go-{name}"));
    for left in [go.clone(), go.with_extension("at")] {
        let _ = fs::remove_file(left);
    }
    let hold = [("JELLYMOON_TEST_NEW_WAIT_MS", "60000".into()), ("JELLYMOON_TEST_GO", go.clone().into())];
    let going = w.going(&hold, &["project", "new", "--date", DAY, name]);
    wait_for(&go.with_extension("at"));
    (going, go)
}

#[test]
fn a_git_step_that_fails_takes_back_what_was_made_and_lists_nothing() {
    let steps = ["init", "config", "symbolic-ref", "write-tree", "commit-tree", "update-ref"];
    // In a home that was not there: nothing is left but the folder git is started in, which Jellymoon keeps empty.
    let w = heard_world();
    for step in steps {
        let ran = w.run_with(&failing_at(&w, step), &["project", "new", "alpha", "--date", DAY]);
        assert_eq!(refused(&ran), format!("jellymoon: {GIT_FAILED}\n"), "{step}");
        names_no_path(&w, &ran);
        assert!(w.at("fake").join("held").exists(), "the made-up git was never at the step {step}");
        let left: Vec<PathBuf> = tree(&w.at("jm")).into_iter().map(|(path, _)| path).collect();
        assert_eq!(left, [w.jm("runs"), w.jm("runs").join("no-hooks")], "{step}");
    }
    // Beside a project that is there: the whole test folder is as it was.
    printed(&w.new_project("beta"));
    for step in steps {
        let vars = failing_at(&w, step);
        let before = world_of(&w);
        let ran = w.run_with(&vars, &["project", "new", "alpha", "--date", DAY]);
        assert_eq!(refused(&ran), format!("jellymoon: {GIT_FAILED}\n"), "{step}");
        assert!(world_of(&w) == before, "a making that failed at {step} left something");
    }
    assert_eq!(w.listed(), json!([{"name": "beta", "used": DAY}]));
    // The same git, failing nowhere, makes it.
    let vars = failing_at(&w, "update-ref");
    fs::remove_file(w.at("fake").join("fails")).unwrap();
    assert_eq!(printed(&w.run_with(&vars, &["project", "new", "alpha", "--date", DAY]))["name"], "alpha");
}

#[test]
fn a_list_whose_lock_is_held_is_waited_for_and_nothing_is_made() {
    let w = heard_world();
    printed(&w.new_project("beta"));
    // A folder at the lock's name is never taken for stale: the wait ends with the lock still held.
    fs::create_dir(w.jm("projects.lock")).unwrap();
    new_refused(&w, "alpha", BUSY);
    assert_eq!(w.names_in("projects"), ["beta"]);
    fs::remove_dir(w.jm("projects.lock")).unwrap();
    assert_eq!(printed(&w.new_project("alpha"))["name"], "alpha");
}

#[test]
fn a_list_that_changed_while_the_project_was_made_takes_back_what_was_made() {
    let w = heard_world();
    printed(&w.new_project("beta"));
    let (root, sep) = (w.real(&w.at("work")), std::path::MAIN_SEPARATOR);
    let line = |n: usize| json!({"name": format!("p{n}"), "folder": format!("{root}{sep}p{n}"), "used": DAY});
    let full = json!({ "projects": (0..100).map(line).collect::<Vec<_>>() }).to_string();
    let was = fs::read(w.list_file()).unwrap();
    let named = json!({ "projects": [{"name": "Alpha", "folder": format!("{root}{sep}Alpha"), "used": DAY}] });
    // While the command stands before the list, the list is written over by hand, as no command can change it
    // then: with the name for another folder, with a full list, and with no list at all.
    let changes: [(&dyn Fn(), &str); 3] = [
        (&|| w.write_list(format!("{named}\n").as_bytes()), LISTED),
        (
            &|| w.write_list(full.as_bytes()),
            "the list has its 100 projects already, so this one was not added; let one go with `jellymoon project \
             forget`",
        ),
        (
            &|| w.write_list(b"not a list\n"),
            "the list of projects (projects.json in Jellymoon's folder) is not used: it is not a list as Jellymoon \
             writes one; move the file away and add your projects again",
        ),
    ];
    for (change, why) in changes {
        let (going, go) = held_before_the_list(&w, "alpha");
        assert!(w.made("alpha").join(".git").is_dir(), "the command was held before the repo was made");
        change();
        let list = fs::read(w.list_file()).unwrap();
        fs::write(&go, "").unwrap();
        let ran = ended(going);
        assert_eq!(refused(&ran), format!("jellymoon: {why}\n"));
        names_no_path(&w, &ran);
        assert_eq!(w.names_in("projects"), ["beta"], "what was made was not taken back: {why}");
        assert!(fs::read(w.list_file()).unwrap() == list, "the list was written: {why}");
        w.write_list(&was);
    }
}

#[test]
fn a_making_that_is_killed_leaves_a_folder_the_next_one_says_is_there_and_the_person_mends() {
    let w = heard_world();
    w.play("hello", 0);
    for name in ["alpha", "beta"] {
        let (going, _go) = held_before_the_list(&w, name);
        drop(going); // killed, as a power cut would end it
        assert!(w.made(name).join(".git").is_dir() && !w.list_file().exists());
        // Its lock is left, and is nobody's half a minute on.
        let aged = std::time::SystemTime::now() - std::time::Duration::from_secs(40);
        fs::File::options().write(true).open(w.jm("projects.lock")).unwrap().set_modified(aged).unwrap();
        // The next one takes that lock over, looks behind it, and leaves it to nobody.
        let before = w.all();
        let ran = w.new_project(name);
        assert_eq!(refused(&ran), format!("jellymoon: {THERE}\n"));
        let lock = w.jm("projects.lock");
        assert!(!lock.exists() && w.all() == before.into_iter().filter(|(path, _)| *path != lock).collect::<Vec<_>>());
    }
    // Mended by listing what is there: the repo was whole, and a run starts in it.
    let real = w.real(&w.made("alpha"));
    assert_eq!(printed(&w.run(&["project", "add", "--repo", &real, "--date", DAY]))["name"], "alpha");
    assert_eq!(printed(&w.run_found("alpha"))["outcome"], "completed");
    // Mended by removing it and asking again.
    fs::remove_dir_all(w.made("beta")).unwrap();
    assert_eq!(printed(&w.new_project("beta"))["name"], "beta");
    assert_eq!(w.listed(), json!([{"name": "alpha", "used": DAY}, {"name": "beta", "used": DAY}]));
}

#[test]
fn two_at_once_of_one_name_make_one_project() {
    let w = heard_world();
    // The first stands with its folder made and nothing listed: the second waits for it, and makes nothing.
    let (first, go) = held_before_the_list(&w, "alpha");
    new_refused(&w, "Alpha", BUSY);
    fs::write(&go, "").unwrap();
    assert_eq!(printed(&ended(first)), json!({"name": "alpha", "used": DAY}));

    // Six started together, three of each of two names: one of each name is made, and the others are refused
    // as taken, by the folder or by the list.
    let all: Vec<Going> =
        (0..6).map(|n| w.going(&[], &["project", "new", "--date", DAY, ["gamma", "delta"][n % 2]])).collect();
    let all: Vec<Ran> = all.into_iter().map(ended).collect();
    for name in ["gamma", "delta"] {
        let made: Vec<&Ran> = all.iter().filter(|ran| ran.stdout.contains(name)).collect();
        assert_eq!(made.len(), 1, "{name}");
        assert_eq!(printed(made[0]), json!({"name": name, "used": DAY}));
        assert_eq!(w.first_commit(name).split('|').nth(2), Some(format!("Jellymoon project {name}").as_str()));
    }
    for ran in all.iter().filter(|ran| ran.code != 0) {
        let said = refused(ran);
        assert!(said == format!("jellymoon: {THERE}\n") || said == format!("jellymoon: {LISTED}\n"), "{said}");
    }
    assert_eq!(w.names_in("projects"), ["alpha", "delta", "gamma"]);
    let listed = w.listed();
    let mut names: Vec<&str> = listed.as_array().unwrap().iter().map(|line| line["name"].as_str().unwrap()).collect();
    names.sort_unstable();
    assert_eq!(names, ["alpha", "delta", "gamma"]);
    assert!(!w.jm("projects.lock").exists(), "the lock was left");
}
