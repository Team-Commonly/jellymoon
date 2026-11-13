//! A step git stops part way, a file that only looks changed, and what a question writes
//! (`docs/development/keep.md`, "A step git stops", "What is in the way" and "Asked first").

use std::fs;
use std::path::Path;

use serde_json::json;

use super::support::skipped;
use super::{Entry, FORWARD, JOINED, Made, STEPPED, finished, kept, printed, touch, worked};

/// Whether a folder may be written to, set as the system's own tools set it. False where it could not be set,
/// or does not hold for the one who runs the test (root).
fn writable(folder: &Path, may: bool) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let set = fs::set_permissions(folder, fs::Permissions::from_mode(if may { 0o755 } else { 0o555 }));
        let probe = folder.join("probe-of-the-test");
        let could = fs::write(&probe, "").is_ok();
        let _ = fs::remove_file(&probe);
        set.is_ok() && could == may
    }
    #[cfg(not(unix))]
    {
        let _ = (folder, may);
        false
    }
}

/// What the keep's own files in the run's folder are called: none is left by a keep or by a question.
fn left(made: &Made) -> Vec<String> {
    let own = fs::read_dir(made.w.jm("runs").join(&made.run)).unwrap();
    let names = own.map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned());
    names.filter(|name| name.starts_with("keep.")).collect()
}

// git checks every file before it writes one, and cannot check that a folder lets it write. It stops part way:
// some files of the work are written, the branch is where it was, and the keep says exactly that.
#[test]
fn a_step_git_stops_at_a_folder_it_may_not_write_to_is_said_as_that_and_the_next_keep_begins_again() {
    let made = super::settings::finished_over(&[("shut/kept.txt", "as it was\n")]);
    let entries: [Entry; 2] = [("100644", "aaa.txt", b"of the work\n"), ("100644", "shut/new.txt", b"of the work\n")];
    let (end, start) = (made.rework(&entries), made.w.commit(&made.repo, "main"));
    if !writable(&made.repo.join("shut"), false) {
        return skipped("a folder git may not write to: this system, or this user, has no such folder");
    }
    let before = made.w.persons(&made.repo);
    assert_eq!(printed(&made.keep(&["--ask"]))["fits"], json!(true), "nothing git checks says the folder is shut");
    let ran = made.keep(&[]);
    assert!(writable(&made.repo.join("shut"), true));
    assert_eq!((ran.code, ran.stdout.as_str()), (1, ""), "{}", ran.stderr);
    assert_eq!(super::refused(&ran), super::said(&STEPPED.replace("{run}", &made.run)));
    // The branch, `HEAD`, the index and every ref of the person's are as they were; the record names what was
    // meant, and the run's branch is left. Files of the work may be there, and nothing else is.
    let after = made.w.persons(&made.repo);
    assert_eq!((&before.1, &before.2, &before.3), (&after.1, &after.2, &after.3));
    assert_eq!((made.w.commit(&made.repo, "main"), made.w.commit(&made.repo, &made.branch())), (start, end.clone()));
    let record = made.record();
    assert_eq!(
        (&record["work"], &record["kept"], &record["onto"]),
        (&json!(null), &json!(end), &json!("refs/heads/main"))
    );
    let more: Vec<_> = after.0.iter().filter(|file| !before.0.contains(file)).map(|file| file.0.clone()).collect();
    assert!(more.iter().all(|file| file.ends_with("aaa.txt") || file.ends_with("new.txt")), "{more:?}");
    assert!(before.0.iter().all(|file| after.0.contains(file)), "a file of the person's changed");
    // The next keep begins again: what the stopped step wrote is in the way until the person clears it.
    for written in more {
        fs::remove_file(written).unwrap();
    }
    kept(&made, &made.keep(&[]), &end, "main", FORWARD);
    assert_eq!(made.status(), "");
}

// A file whose time changed and whose content did not is no unsaved change: git says so of it once it has read
// the file, and so does a keep, which reads it without writing the person's index.
#[test]
fn a_tracked_file_that_was_touched_and_holds_what_it_held_is_not_in_the_way() {
    for joined in [false, true] {
        let made = finished();
        if joined {
            made.moved_on();
        }
        // One the work changes, one it removes, and one it leaves alone.
        for file in ["a.txt", "b.txt", "c.txt"] {
            touch(&made.repo.join(file));
        }
        let before = made.held();
        assert_eq!(printed(&made.keep(&["--ask"]))["fits"], json!(true), "joined {joined}");
        assert!(before == made.held(), "a question wrote the index, a ref, a file or the record");
        assert_eq!(left(&made), Vec::<String>::new());
        let ran = made.keep(&[]);
        kept(&made, &ran, &made.w.commit(&made.repo, "main"), "main", if joined { JOINED } else { FORWARD });
        assert_eq!((made.status().as_str(), left(&made)), ("", Vec::new()));
        if !joined {
            assert_eq!(made.files(), worked());
        }
    }
    // A file that was touched and does hold something else is in the way as ever.
    let made = finished();
    fs::write(made.repo.join("a.txt"), "half an edit of the person's\n").unwrap();
    touch(&made.repo.join("a.txt"));
    super::refused::unfit(&made, super::refused::IN_WAY);
}

// A question takes no lock on the checkout's index and writes nothing in the checkout's git folder: here it
// may not, and it answers all the same.
#[test]
fn a_question_is_answered_in_a_checkout_whose_git_folder_takes_no_new_file() {
    let made = finished();
    let (git, before) = (made.repo.join(".git"), made.held());
    if !writable(&git, false) {
        return skipped("a git folder that takes no new file: this system, or this user, has no such folder");
    }
    let asked = printed(&made.keep(&["--ask"]));
    assert!(writable(&git, true));
    assert_eq!((&asked["fits"], &asked["kept"]), (&json!(true), &json!(made.end())), "{asked}");
    assert!(before == made.held());
    assert_eq!(left(&made), Vec::<String>::new());
    assert!(!git.join("ORIG_HEAD").exists() && !git.join("index.lock").exists());
}

// With commits of the person's own, a question has git merge the two in the store, which writes one tree there.
// In a store that takes no new object git cannot, and the answer says that, in words of its own, asked or not.
#[test]
fn a_question_that_must_merge_says_so_when_the_store_takes_no_new_object() {
    let made = finished();
    made.moved_on();
    let before = made.held();
    let mut folders = vec![made.repo.join(".git").join("objects")];
    let below = fs::read_dir(&folders[0]).unwrap().map(|entry| entry.unwrap().path());
    folders.extend(below.filter(|path| path.is_dir()));
    let shut = |may: bool| folders.iter().filter(|folder| !writable(folder, may)).count() == 0;
    if !shut(false) {
        shut(true);
        return skipped("a store that takes no new object: this system, or this user, has no such folder");
    }
    let (asked, ran) = (made.keep(&["--ask"]), made.keep(&[]));
    assert!(shut(true));
    let why = format!(
        "git could not merge the work of run {{run}} with the branch's commits since in the repo's store: the store \
         takes no new object (the merge writes one tree there, asked or not), or git did not answer: {}",
        super::UNCHANGED
    );
    let asked = printed(&asked);
    assert_eq!((&asked["fits"], &asked["why"]), (&json!(false), &json!(why.replace("{run}", &made.run))));
    made.refuses(&ran, &before, &why);
    assert_eq!(left(&made), Vec::<String>::new());
    // With the store open again the work is kept.
    kept(&made, &made.keep(&[]), &made.w.commit(&made.repo, "main"), "main", JOINED);
}

// An index kept in two parts has its shared part in the checkout's git folder. Every file is touched, so the
// look has all of the index to write: git writes the keep's own copy, and the git folder gets no new file.
#[test]
fn a_question_writes_no_part_of_a_split_index_into_the_checkouts_git_folder() {
    let made = finished();
    made.git(&["config", "core.splitIndex", "true"]);
    made.git(&["update-index", "--split-index"]);
    for file in ["a.txt", "b.txt", "c.txt"] {
        touch(&made.repo.join(file));
    }
    let names = || {
        let names = fs::read_dir(made.repo.join(".git")).unwrap().map(|entry| entry.unwrap().file_name());
        names.collect::<std::collections::BTreeSet<_>>()
    };
    let (before, there) = (made.held(), names());
    assert!(there.iter().any(|name| name.to_string_lossy().starts_with("sharedindex.")), "{there:?}");
    assert_eq!(printed(&made.keep(&["--ask"]))["fits"], json!(true));
    assert!(before == made.held());
    assert_eq!((left(&made), names()), (Vec::<String>::new(), there));
    kept(&made, &made.keep(&[]), &made.end(), "main", FORWARD);
}
