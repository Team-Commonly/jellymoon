//! The file of a making (what F65's third gate found). It is the person's alone, and a folder without it is not
//! the made one, so a making whose file a hand removed lists nothing and leaves the folder. A making that was
//! cut off leaves the file, and `project add` of that folder takes it out. And a folder that could not be taken
//! back is said to have been left.

use std::fs;
use std::time::{Duration, SystemTime};

use super::alone::{MOVED, and_the_take_over, but_the_lock, inside};
use super::half::held_before_the_list;
use super::*;

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
