//! One making at a time (what F65's gate found): the whole of a `project new`, from its look to its line on the
//! list, is behind the list's lock, so no other `project new`, `add` or `forget` meets a making under way. What
//! a making takes back is the folder it made and nothing else: a folder put in its place by another hand is
//! left, and never listed. The made folder is known by a file of the making's own in it, which a folder put in
//! its place does not hold and a made project never keeps. And a home a made project could not be listed in is
//! refused before anything is made.

use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use serde_json::json;

use super::half::{BUSY, failing_at, held_before_the_list, world_of};
use super::refused::{NO_FOLDER, NO_GIT, THERE, new_refused};
use super::*;

pub const MOVED: &str = "the new project's folder was removed, replaced or changed while it was being made, so nothing was listed, and \
                     what is there now was left as it is";
#[cfg(unix)]
const NOT_OWN: &str = "Jellymoon's folder is not a folder of your own, and a project is listed only in a folder of \
                       your own, so nothing was made";
#[cfg(unix)]
const NOT_TEXT: &str = "the full path of Jellymoon's folder is not text, or a project's in it would be over 4096 \
                        bytes, so a new project could not be listed and nothing was made";

/// What follows a refusal that came after the folder was made, when its own words do not say so.
#[cfg(unix)]
const BACK: &str = "; the folder made for it meanwhile was taken back";

/// The names in a made project's folder, in order.
pub fn inside(w: &World, name: &str) -> Vec<String> {
    let mut names: Vec<String> =
        fs::read_dir(w.made(name)).unwrap().map(|entry| entry.unwrap().file_name().into_string().unwrap()).collect();
    names.sort();
    names
}

/// A list of the made projects named, as a hand writes one.
fn list_of(w: &World, names: &[&str]) -> Vec<u8> {
    let lines: Vec<_> =
        names.iter().map(|name| json!({"name": name, "folder": w.real(&w.made(name)), "used": DAY})).collect();
    format!("{}\n", json!({ "projects": lines })).into_bytes()
}

/// Makes Jellymoon's folder one nothing can be written in, or back. False where that stops nobody (root).
#[cfg(unix)]
fn closed(w: &World, closed: bool) -> bool {
    use std::os::unix::fs::PermissionsExt;
    let mode = if closed { 0o500 } else { 0o700 };
    fs::set_permissions(w.at("jm"), fs::Permissions::from_mode(mode)).unwrap();
    !closed || fs::write(w.jm("probe"), "").is_err()
}

/// The whole test folder but the list's lock, which a making under way holds, and the file a take-over is behind.
pub fn but_the_lock(w: &World) -> Vec<(PathBuf, Vec<u8>)> {
    let locks = [w.jm("projects.lock"), w.jm("take-over.lock")]; // the second is made by a take-over and stays
    w.all().into_iter().filter(|(path, _)| !locks.contains(path)).collect()
}

#[test]
fn a_folder_put_in_place_of_the_one_being_made_is_left_with_what_is_in_it_and_never_listed() {
    let w = heard_world();
    printed(&w.new_project("beta"));
    // By hand, while the command stands before the list: its folder is removed, as its own sentence advises of
    // one left behind, and another is put there with a file in it. The second time that folder is listed too.
    for listed in [false, true] {
        let (going, go) = held_before_the_list(&w, "alpha");
        fs::remove_dir_all(w.made("alpha")).unwrap();
        fs::create_dir(w.made("alpha")).unwrap();
        fs::write(w.made("alpha").join("mine.txt"), "the person's own\n").unwrap();
        if listed {
            let beta = json!({"name": "beta", "folder": w.real(&w.made("beta")), "used": DAY});
            let alpha = json!({"name": "alpha", "folder": w.real(&w.made("alpha")), "used": DAY});
            w.write_list(format!("{}\n", json!({"projects": [beta, alpha]})).as_bytes());
        }
        let before = but_the_lock(&w);
        fs::write(&go, "").unwrap();
        let ran = ended(going);
        assert_eq!(refused(&ran), format!("jellymoon: {MOVED}\n"), "listed: {listed}");
        names_no_path(&w, &ran);
        assert!(w.all() == before, "a making took back, or listed, a folder it did not make (listed: {listed})");
        assert_eq!(fs::read_to_string(w.made("alpha").join("mine.txt")).unwrap(), "the person's own\n");
        fs::remove_dir_all(w.made("alpha")).unwrap();
    }
    assert_eq!(w.names_in("projects"), ["beta"]);
}

#[test]
fn a_folder_removed_while_it_was_being_made_is_said_so_and_nothing_is_listed() {
    let w = heard_world();
    printed(&w.new_project("beta"));
    let (going, go) = held_before_the_list(&w, "alpha");
    // The folder under making holds the repo and the making's own file, and nothing else.
    let held = inside(&w, "alpha");
    assert!(held.len() == 2 && held[0] == ".git" && held[1].starts_with(".jellymoon-making-"), "{held:?}");
    fs::remove_dir_all(w.made("alpha")).unwrap();
    let before = but_the_lock(&w);
    fs::write(&go, "").unwrap();
    assert_eq!(refused(&ended(going)), format!("jellymoon: {MOVED}\n"));
    assert!(w.all() == before);
}

#[test]
fn a_line_a_hand_wrote_for_the_folder_being_made_is_the_same_line_and_the_project_is_made() {
    let w = heard_world();
    printed(&w.new_project("beta"));
    let (going, go) = held_before_the_list(&w, "alpha");
    w.write_list(&list_of(&w, &["beta", "alpha"]));
    fs::write(&go, "").unwrap();
    assert_eq!(printed(&ended(going)), json!({"name": "alpha", "used": DAY}));
    let list = |bytes: &[u8]| serde_json::from_slice::<serde_json::Value>(bytes).unwrap();
    assert_eq!(list(&fs::read(w.list_file()).unwrap()), list(&list_of(&w, &["beta", "alpha"])));
    assert_eq!(inside(&w, "alpha"), [".git"], "the making's own file was left in a made project");
}

#[cfg(unix)]
#[test]
fn a_list_that_cannot_be_written_takes_back_the_folder_and_says_so_but_never_a_folder_the_list_names() {
    let w = heard_world();
    printed(&w.new_project("beta"));
    let files = "jellymoon: jellymoon's own files: ";
    // During the hold a hand makes Jellymoon's folder one nothing is written in; the second time it has also
    // written the line for this very name and folder.
    for listed in [false, true] {
        let (going, go) = held_before_the_list(&w, "alpha");
        if listed {
            w.write_list(&list_of(&w, &["beta", "alpha"]));
        }
        if !closed(&w, true) {
            closed(&w, false);
            return crate::support::skipped("this user writes where nobody may");
        }
        let list = fs::read(w.list_file()).unwrap();
        fs::write(&go, "").unwrap();
        let ran = ended(going);
        closed(&w, false);
        fs::remove_file(w.jm("projects.lock")).unwrap();
        assert!(ran.code == 1 && ran.stderr.starts_with(files), "listed: {listed}: {}", ran.stderr);
        names_no_path(&w, &ran);
        assert_eq!(ran.stderr.ends_with(&format!("{BACK}\n")), !listed, "{}", ran.stderr);
        assert!(fs::read(w.list_file()).unwrap() == list);
        if listed {
            assert_eq!(inside(&w, "alpha"), [".git"], "a folder the list names was taken back, or kept the file");
        } else {
            assert_eq!(w.names_in("projects"), ["beta"]);
        }
    }
}

#[test]
fn a_command_that_meets_a_making_waits_is_told_one_is_under_way_and_changes_nothing() {
    let w = heard_world();
    let elsewhere = w.repo("gamma");
    printed(&w.new_project("beta"));
    let (going, go) = held_before_the_list(&w, "alpha");
    let under_way = w.real(&w.made("alpha"));
    // Another making of the name, a listing of the folder under making, a listing of any other, and a forgetting.
    let met: [&[&str]; 4] = [
        &["project", "new", "alpha", "--date", DAY],
        &["project", "add", "--repo", &under_way, "--date", DAY],
        &["project", "add", "--repo", elsewhere.to_str().unwrap(), "--date", DAY],
        &["project", "forget", "beta"],
    ];
    for args in met {
        let before = w.all();
        let ran = w.run(args);
        assert_eq!(refused(&ran), format!("jellymoon: {BUSY}\n"), "{args:?}");
        names_no_path(&w, &ran);
        assert!(w.all() == before, "a command that met a making changed something: {args:?}");
    }
    fs::write(&go, "").unwrap();
    assert_eq!(printed(&ended(going)), json!({"name": "alpha", "used": DAY}));
    let lines = [("beta", w.real(&w.made("beta"))), ("alpha", under_way)];
    let lines: Vec<_> = lines.iter().map(|(name, at)| json!({"name": name, "folder": at, "used": DAY})).collect();
    assert_eq!(w.on_disk(), json!({ "projects": lines }));
    assert!(w.made("alpha").join(".git").is_dir() && !w.jm("projects.lock").exists());
}

#[test]
fn a_making_that_is_killed_is_waited_for_half_a_minute_and_then_its_folder_is_said_to_be_there() {
    let w = heard_world();
    let (going, _go) = held_before_the_list(&w, "alpha");
    drop(going); // killed, as a power cut would end it: its lock is left
    let lock = w.jm("projects.lock");
    assert!(lock.is_file() && w.made("alpha").join(".git").is_dir());
    assert_eq!(inside(&w, "alpha").len(), 2, "a killed making leaves its own file beside the repo");
    // Another name is waited with, as the lock's holder may still be at work.
    new_refused(&w, "beta", BUSY);
    // Half a minute on, the lock is nobody's: the next command takes it over, and says the folder is there.
    let aged = SystemTime::now() - Duration::from_secs(40);
    fs::File::options().write(true).open(&lock).unwrap().set_modified(aged).unwrap();
    let before = but_the_lock(&w);
    let ran = w.new_project("alpha");
    assert_eq!(refused(&ran), format!("jellymoon: {THERE}\n"));
    assert!(but_the_lock(&w) == before, "a refusal changed something but the lock nobody held");
    assert_eq!(printed(&w.new_project("beta"))["name"], "beta");
}

#[test]
fn what_is_looked_at_before_the_lock_is_looked_at_again_behind_it() {
    let w = heard_world();
    fs::create_dir_all(w.at("jm")).unwrap();
    let lock = w.jm("projects.lock");
    // The command has looked and waits for the lock; a file is put where `projects` would be; the lock is let go.
    fs::write(&lock, "").unwrap();
    let going = w.going(&[], &["project", "new", "--date", DAY, "alpha"]);
    std::thread::sleep(Duration::from_millis(700));
    fs::write(w.jm("projects"), "a file\n").unwrap();
    fs::remove_file(&lock).unwrap();
    let ran = ended(going);
    assert_eq!(refused(&ran), format!("jellymoon: {NO_FOLDER}\n"));
    assert_eq!(fs::read(w.jm("projects")).unwrap(), b"a file\n");
    assert!(!lock.exists() && !w.list_file().exists());
}

#[test]
fn something_at_the_name_is_said_before_git_is_looked_for() {
    let w = heard_world();
    fs::create_dir_all(w.made("alpha")).unwrap();
    let before = w.all();
    let ran = w.run_with(&[("PATH", w.at("bin").into())], &["project", "new", "Alpha", "--date", DAY]);
    assert_eq!(refused(&ran), format!("jellymoon: {THERE}\n"));
    let ran = w.run_with(&[("PATH", w.at("bin").into())], &["project", "new", "beta", "--date", DAY]);
    assert_eq!(refused(&ran), format!("jellymoon: {NO_GIT}\n"));
    assert!(w.all() == before);
}

#[test]
fn a_making_that_fails_leaves_the_folder_of_projects_when_it_was_there_and_empty() {
    let w = heard_world();
    fs::create_dir_all(w.jm("projects")).unwrap();
    let vars = failing_at(&w, "commit-tree");
    let ran = w.run_with(&vars, &["project", "new", "alpha", "--date", DAY]);
    assert!(ran.code == 1 && ran.stderr.contains("taken back"), "{}", ran.stderr);
    assert!(w.jm("projects").is_dir(), "a folder this command did not make was removed");
    assert_eq!(w.names_in("projects"), [""; 0]);
}

#[test]
fn a_branch_that_is_there_already_is_never_written_over_by_the_first_commit() {
    let w = heard_world();
    let other = w.repo("other");
    let vars = failing_at(&w, "update-ref");
    fs::remove_file(w.at("fake").join("fails")).unwrap();
    let git_dir = format!("--git-dir={}", w.made("alpha").join(".git").display());
    // Just before the branch is set, something else sets one: under another name, and then under this one.
    for (branch, made) in [("refs/heads/other", true), ("refs/heads/main", false)] {
        let fetched = [&git_dir, "fetch", "-q", "--update-head-ok", other.to_str().unwrap(), &format!("HEAD:{branch}")];
        fs::write(w.at("fake").join("first"), fetched.join("\n")).unwrap();
        let before = world_of(&w);
        let ran = w.run_with(&vars, &["project", "new", "alpha", "--date", DAY]);
        assert!(w.at("fake").join("held").exists(), "the made-up git was never at the step");
        if made {
            assert_eq!(printed(&ran)["name"], "alpha");
            assert_eq!(w.git(&w.made("alpha"), &["rev-list", "--count", "other"]), "1");
            printed(&w.run(&["project", "forget", "alpha"]));
            fs::remove_dir_all(w.made("alpha")).unwrap();
        } else {
            assert!(
                ran.code == 1 && ran.stderr.contains("git could not make the new project's repo"),
                "{}",
                ran.stderr
            );
            assert!(world_of(&w) == before, "a making whose branch was there left something");
        }
    }
}

#[cfg(unix)]
#[test]
fn a_home_a_project_could_not_be_listed_in_is_refused_before_anything_is_made() {
    use std::os::unix::ffi::OsStrExt;
    let w = heard_world();
    // The person's own folder is not there, so no folder is theirs: with Jellymoon's folder there, and not.
    let nobody = [("HOME", w.at("nowhere").into())];
    for there in [false, true] {
        if there {
            fs::create_dir(w.at("jm")).unwrap();
        }
        let before = w.all();
        let ran = w.run_with(&nobody, &["project", "new", "alpha", "--date", DAY]);
        assert_eq!(refused(&ran), format!("jellymoon: {NOT_OWN}\n"), "there: {there}");
        assert!(w.all() == before, "a refusal made something (there: {there})");
    }
    // The person's own folder is someone else's, and Jellymoon's is not there: it is made to hold the lock,
    // found not to be the person's, and taken back.
    fs::remove_dir(w.at("jm")).unwrap();
    if let Some(theirs) = crate::support::door::not_ours() {
        let before = w.all();
        let ran = w.run_with(&[("HOME", theirs.into())], &["project", "new", "alpha", "--date", DAY]);
        assert_eq!(refused(&ran), format!("jellymoon: {NOT_OWN}\n"));
        assert!(w.all() == before && !w.at("jm").exists(), "a refusal left Jellymoon's folder");
    }
    // A home whose path is not text.
    let mut odd = w.at("tmp").into_os_string();
    odd.push(std::ffi::OsStr::from_bytes(b"/home\xff\xfe"));
    let before = w.all();
    let ran = w.run_with(&[("JELLYMOON_HOME", odd)], &["project", "new", "alpha", "--date", DAY]);
    assert_eq!(refused(&ran), format!("jellymoon: {NOT_TEXT}\n"));
    assert!(w.all() == before, "a refusal made something");
}
