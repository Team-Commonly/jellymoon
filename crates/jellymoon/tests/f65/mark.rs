//! The file of a making (what F65's third gate found). It is the person's alone, and a folder without it is not
//! the made one, so a making whose file a hand removed lists nothing and leaves the folder. A making that was
//! cut off leaves the file, and `project add` of that folder takes it out. And a folder that could not be taken
//! back is said to have been left.

use std::fs;
use std::time::{Duration, SystemTime};

use super::alone::{MOVED, and_the_take_over, but_the_lock, inside};
#[cfg(unix)]
use super::half::failing_at;
use super::half::held_before_the_list;
use super::*;

const BY_ANOTHER: &str = "the new project's folder was listed by another command while it was being made, so the \
                          project is on the list as that command listed it, and this one changed nothing more";

#[test]
fn a_making_whose_file_a_hand_removed_lists_nothing_and_leaves_the_folder_as_it_is() {
    let w = heard_world();
    let (going, go) = held_before_the_list(&w, "alpha");
    let file = w.made("alpha").join(&inside(&w, "alpha")[1]);
    #[cfg(unix)]
    {
        let mode = std::os::unix::fs::PermissionsExt::mode(&fs::metadata(&file).unwrap().permissions());
        assert_eq!((mode & 0o777, fs::metadata(&file).unwrap().len()), (0o600, 0));
    }
    fs::remove_file(file).unwrap();
    let before = but_the_lock(&w);
    fs::write(&go, "").unwrap();
    assert_eq!(refused(&ended(going)), format!("jellymoon: {MOVED}\n"));
    // Letting go of the lock left `take-over.lock`, empty, and nothing else differs.
    let after = and_the_take_over(&w, before);
    assert!(w.all() == after && inside(&w, "alpha") == [".git"], "a folder that was changed was touched");
    assert_eq!(w.listed(), serde_json::json!([]));
}

#[test]
fn a_folder_a_making_that_was_cut_off_left_is_added_without_the_making_s_file() {
    let w = heard_world();
    let (going, _go) = held_before_the_list(&w, "alpha");
    drop(going); // killed: the folder, the making's file and the lock are left
    let aged = SystemTime::now() - Duration::from_secs(40);
    fs::File::options().write(true).open(w.jm("projects.lock")).unwrap().set_modified(aged).unwrap();
    fs::write(w.made("alpha").join("mine.txt"), "the person's own\n").unwrap();
    assert_eq!(inside(&w, "alpha").len(), 3);
    let folder = w.made("alpha");
    let ran = w.run(&["project", "add", "--repo", folder.to_str().unwrap(), "--date", DAY]);
    assert_eq!(printed(&ran)["name"], "alpha");
    assert_eq!(inside(&w, "alpha"), [".git", "mine.txt"]);
    assert_eq!(fs::read_to_string(folder.join("mine.txt")).unwrap(), "the person's own\n");
}

#[test]
fn a_making_whose_folder_another_command_listed_meanwhile_says_so_as_a_made_project_and_changes_nothing() {
    let w = heard_world();
    let (going, go) = held_before_the_list(&w, "alpha");
    // The making has gone on past half a minute: its lock is nobody's, and an `add` of its folder takes it over,
    // lists the folder on a later day, and takes the making's file out.
    let aged = SystemTime::now() - Duration::from_secs(40);
    fs::File::options().write(true).open(w.jm("projects.lock")).unwrap().set_modified(aged).unwrap();
    let folder = w.real(&w.made("alpha"));
    let later = serde_json::json!({"name": "alpha", "used": "2026-03-11"});
    assert_eq!(printed(&w.run(&["project", "add", "--repo", &folder, "--date", "2026-03-11"])), later);
    assert_eq!(inside(&w, "alpha"), [".git"]);
    // A second making holds the list's lock, under its own mark, while the first comes to its end.
    let (second, second_go) = held_before_the_list(&w, "beta");
    let before = but_the_lock(&w);
    fs::write(&go, "").unwrap();
    let ran = ended(going);
    // The project is made and listed: the line the list holds is printed, exit 0, and the sentence beside it.
    assert_eq!((ran.code, ran.stderr.as_str()), (0, format!("jellymoon: {BY_ANOTHER}\n").as_str()), "{}", ran.stdout);
    assert_eq!(serde_json::from_str::<serde_json::Value>(&ran.stdout).unwrap(), later);
    names_no_path(&w, &ran);
    // It wrote nothing and removed nothing: not the lock, which is another's now, and not `take-over.lock`.
    assert!(but_the_lock(&w) == before, "a making that found its project listed changed something");
    assert_eq!(w.names_in(""), ["projects", "projects.json", "projects.lock", "runs", "take-over.lock"]);

    // A file a hand took out, with the name listed for another folder, is no such making: nothing was listed.
    fs::remove_file(w.made("beta").join(&inside(&w, "beta")[1])).unwrap();
    let elsewhere = serde_json::json!({"name": "beta", "folder": w.real(&w.repo("beta")), "used": DAY});
    w.write_list(format!("{}\n", serde_json::json!({ "projects": [elsewhere] })).as_bytes());
    let before = but_the_lock(&w);
    fs::write(&second_go, "").unwrap();
    assert_eq!(refused(&ended(second)), format!("jellymoon: {MOVED}\n"));
    assert!(w.all() == and_the_take_over(&w, before), "a making that listed nothing changed something");
}

#[cfg(unix)]
#[test]
fn a_git_that_fails_in_a_folder_that_cannot_be_taken_back_does_not_say_it_was_taken_back() {
    use std::os::unix::fs::PermissionsExt;
    let w = heard_world();
    let shut = |folder: &std::path::Path, mode| fs::set_permissions(folder, fs::Permissions::from_mode(mode)).unwrap();
    let probe = w.folder("probe");
    shut(&probe, 0o500);
    let writes = fs::write(probe.join("probe"), "").is_ok();
    shut(&probe, 0o700);
    if writes {
        return crate::support::skipped("this user writes where nobody may");
    }
    // Just before git's first step the new folder becomes one nothing can be removed from, and the step fails.
    let vars = failing_at(&w, "init");
    let alias = format!("alias.shut=!/bin/chmod 500 '{}'", w.made("alpha").display());
    fs::write(w.at("fake").join("first"), ["-c", &alias, "shut"].join("\n")).unwrap();
    let ran = w.run_with(&vars, &["project", "new", "alpha", "--date", DAY]);
    shut(&w.made("alpha"), 0o700);
    let left = "git could not make the new project's repo (what git said is not repeated, as it names the folder), \
                so nothing was listed; the folder made for it meanwhile could not be taken back and was left";
    assert_eq!(refused(&ran), format!("jellymoon: {left}\n"));
    names_no_path(&w, &ran);
    // What is left is the folder with the making's file, the folder git is started in, and `take-over.lock`.
    assert_eq!(w.names_in(""), ["projects", "runs", "take-over.lock"]);
    assert!(inside(&w, "alpha").len() == 1 && fs::read(w.jm("take-over.lock")).unwrap().is_empty());
}

#[cfg(unix)]
#[test]
fn a_folder_that_could_not_be_taken_back_is_said_to_have_been_left() {
    use std::os::unix::fs::PermissionsExt;
    let w = heard_world();
    let (going, go) = held_before_the_list(&w, "alpha");
    // During the hold the list fills, and the folder becomes one nothing can be removed from.
    let root = w.real(w.dir.path());
    let lines: Vec<_> = (0..100)
        .map(|n| serde_json::json!({"name": format!("p{n}"), "folder": format!("{root}/p{n}"), "used": DAY}))
        .collect();
    w.write_list(format!("{}\n", serde_json::json!({ "projects": lines })).as_bytes());
    let shut = |mode| fs::set_permissions(w.made("alpha"), fs::Permissions::from_mode(mode)).unwrap();
    shut(0o500);
    if fs::write(w.made("alpha").join("probe"), "").is_ok() {
        shut(0o700);
        return crate::support::skipped("this user writes where nobody may");
    }
    fs::write(&go, "").unwrap();
    let ran = ended(going);
    shut(0o700);
    let full = "the list has its 100 projects already, so this one was not added; let one go with `jellymoon \
                project forget`; the folder made for it meanwhile could not be taken back and was left";
    assert_eq!(refused(&ran), format!("jellymoon: {full}\n"));
    names_no_path(&w, &ran);
    assert!(w.made("alpha").is_dir() && w.listed().as_array().unwrap().len() == 100);
}
