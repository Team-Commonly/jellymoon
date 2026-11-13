//! What `jellymoon run show` takes from a record that names its repo, its start and its end, and what it refuses
//! (`docs/development/show.md`, "Which run, and which repo"). A record is a file, and every test here that changes
//! one writes it by hand, as anyone who can write Jellymoon's home can.

use std::fs;
use std::path::Path;

use serde_json::{Value, json};

use super::{Made, NAMED, finished, printed, refused};

const MOVED: &str = "is not as the run left it, so what is on it is not shown as the run's";
const ANOTHER: &str = "is not the one its record names, so its work is not shown";
const START: &str = "does not start where its record says the run started, so it is not shown as the run's";
const NOT_CORES: &str = "is not one Jellymoon wrote for this wish";

/// A refusal of the list and of a file alike: one fixed line that names the run, with no path and no word of git's.
fn no(made: &Made, repo: &Path, why: &str) {
    for more in [&[][..], &["--file", "1"]] {
        let ran = made.show_in(repo, more);
        let said = refused(&ran);
        assert!(said.contains(why) && said.contains(&made.run), "{said}");
        assert!(!said.contains("alpha") && !said.contains("fatal") && !said.contains(".git"), "{said}");
    }
}

fn names(shown: &Value) -> Vec<&str> {
    shown["files"].as_array().unwrap().iter().map(|file| file["name"].as_str().unwrap()).collect()
}

/// The record with one of its fields changed.
fn with(record: &Value, name: &str, value: Value) -> Value {
    let mut record = record.clone();
    record[name] = value;
    record
}

#[test]
fn the_work_is_the_change_from_the_start_to_the_end_the_record_names() {
    let made = finished();
    let record = made.record();
    assert!(NAMED.iter().all(|name| record[name].is_string()), "{record}");
    assert_eq!(names(&printed(&made.show())), ["a.txt", "b.txt", "n.txt"]);
    let file = printed(&made.show_in(&made.repo, &["--file", "3"]));
    assert_eq!(file["text"], "new from the run");
}

// The gap F44 left open: by the branch's shape alone this commit is a run's.
#[test]
fn a_commit_made_by_hand_with_a_runs_message_under_the_runs_branch_is_refused() {
    let made = finished();
    let (branch, end) = (made.branch(), made.w.commit(&made.repo, &made.branch()));
    // One parent, the run's own message, the run's own files: every sign F44 had, and another commit.
    let by_hand = made.by_hand("main", &made.message(), &end);
    assert_ne!(by_hand, end);
    made.git(&["update-ref", &branch, &by_hand]);
    no(&made, &made.repo, MOVED);
    // The branch removed, or moved back past the end, no longer names the end either.
    made.git(&["update-ref", &branch, "main"]);
    no(&made, &made.repo, MOVED);
    made.git(&["update-ref", "-d", &branch]);
    no(&made, &made.repo, "is not in the repo");
    made.git(&["update-ref", &branch, &end]);
    assert_eq!(names(&printed(&made.show())), ["a.txt", "b.txt", "n.txt"], "the run's own commit is shown again");
}

#[test]
fn an_end_whose_one_parent_is_not_the_start_the_record_names_is_refused() {
    let made = finished();
    let (branch, end, kept) = (made.branch(), made.w.commit(&made.repo, &made.branch()), made.record());
    let other = made.by_hand("main", "a commit of the person's", "main");
    // The record says the run started somewhere else than where its end was made from.
    made.write(&with(&kept, "start", json!(other)));
    no(&made, &made.repo, START);
    made.write(&with(&kept, "start", kept["end"].clone()));
    no(&made, &made.repo, START);
    // The branch and the record agree on an end that was made from another commit than the start.
    let from_other = made.by_hand(&other, &made.message(), &end);
    made.git(&["update-ref", &branch, &from_other]);
    made.write(&with(&kept, "end", json!(from_other)));
    no(&made, &made.repo, START);
    // An end with two parents, or none, is not a commit a copy ended as, whatever the record says.
    for parents in [&["-p", "main", "-p", other.as_str()][..], &[]] {
        let tree = format!("{end}^{{tree}}");
        let shaped = made.git(&[&["commit-tree"][..], parents, &["-m", &made.message(), &tree]].concat());
        made.git(&["update-ref", &branch, &shaped]);
        made.write(&with(&kept, "end", json!(shaped)));
        no(&made, &made.repo, MOVED);
    }
    made.git(&["update-ref", &branch, &end]);
    made.write(&kept);
    assert_eq!(names(&printed(&made.show())), ["a.txt", "b.txt", "n.txt"]);
}

/// Copies a folder and all it holds.
fn copied(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let (path, to) = (entry.as_ref().unwrap().path(), to.join(entry.unwrap().file_name()));
        if path.is_dir() { copied(&path, &to) } else { fs::copy(&path, &to).map(|_| ()).unwrap() }
    }
}

#[test]
fn a_repo_that_is_not_the_one_the_record_names_is_refused() {
    let made = finished();
    let kept = made.record();
    // Another repo that holds the very same branch and commits: by all git says of it, it is the run's.
    let other = made.w.at("work").join("elsewhere").join("alpha");
    copied(&made.repo, &other);
    assert_eq!(made.w.commit(&other, &made.branch()), kept["end"].as_str().unwrap());
    no(&made, &other, ANOTHER);
    // A record made to name that other repo is not the record of a run in this one.
    made.write(&with(&kept, "repo", json!(other.join(".git"))));
    no(&made, &made.repo, ANOTHER);
    assert_eq!(names(&printed(&made.show_in(&other, &[]))), ["a.txt", "b.txt", "n.txt"], "and is one in that one");
    // The folder above the one git names is not it, and neither is one inside it.
    let named = Path::new(kept["repo"].as_str().unwrap());
    for near in [named.parent().unwrap().to_path_buf(), named.join("objects")] {
        made.write(&with(&kept, "repo", json!(near)));
        no(&made, &made.repo, ANOTHER);
    }
    // A repo the record names that is not there is no repo's same.
    made.write(&with(&kept, "repo", json!(made.w.at("work").join("gone").join(".git"))));
    no(&made, &made.repo, ANOTHER);
    made.write(&kept);
    assert_eq!(names(&printed(&made.show())), ["a.txt", "b.txt", "n.txt"]);
}

// The folder found is resolved by the system and compared with the record's word, which a run wrote resolved.
// However the person spells their repo it is that repo; the record's word is never resolved, so another spelling
// of the same folder in a record is not what a run wrote, and nothing is looked up at a path a file names.
#[test]
fn the_repo_found_is_compared_as_the_system_resolves_it_and_the_records_path_is_never_resolved() {
    let made = finished();
    let kept = made.record();
    let named = kept["repo"].as_str().unwrap();
    fs::create_dir(made.repo.join("sub")).unwrap();
    fs::create_dir(made.w.at("work").join("sub")).unwrap();
    let dotted = made.w.at("work").join("sub").join("..").join("alpha");
    assert_eq!(names(&printed(&made.show_in(&dotted, &[]))), ["a.txt", "b.txt", "n.txt"], "through `sub/..`");
    let upper = made.w.dir.path().join("WORK").join("alpha");
    if fs::metadata(&upper).is_ok() {
        // Where the system takes two cases of a name for one folder, so does the comparison.
        assert_eq!(names(&printed(&made.show_in(&upper, &[]))), ["a.txt", "b.txt", "n.txt"], "in another case");
        made.write(&with(&kept, "repo", json!(upper.join(".git"))));
        no(&made, &made.repo, ANOTHER);
    }
    made.write(&with(&kept, "repo", json!(Path::new(named).parent().unwrap().join("sub").join("..").join(".git"))));
    no(&made, &made.repo, ANOTHER);
    #[cfg(unix)]
    {
        let linked = made.w.at("linked");
        std::os::unix::fs::symlink(made.w.at("work"), &linked).unwrap();
        made.write(&with(&kept, "repo", json!(linked.join("alpha").join(".git"))));
        no(&made, &made.repo, ANOTHER);
        made.write(&kept);
        let shown = printed(&made.show_in(&linked.join("alpha"), &[]));
        assert_eq!(names(&shown), ["a.txt", "b.txt", "n.txt"], "the person names it through a link");
    }
}

#[test]
fn a_record_that_names_a_commit_by_anything_but_a_full_hex_id_or_names_only_some_is_refused() {
    let made = finished();
    let kept = made.record();
    let end = kept["end"].as_str().unwrap();
    // Each of these git would take for the run's end, or for an option, were it handed to git.
    let branch = made.branch();
    let not_ids = [
        "HEAD".to_string(),
        "main".to_string(),
        branch.clone(),
        format!("jellymoon/run-{}", made.run),
        format!("{end}^{{commit}}"),
        format!("{end}~0"),
        end[..12].to_string(),
        format!("{end}0"),
        format!(" {end}"),
        "--output=made-by-git".to_string(),
        "-h".to_string(),
        String::new(),
    ];
    for name in ["end", "start"] {
        for not_id in &not_ids {
            made.write(&with(&kept, name, json!(not_id)));
            no(&made, &made.repo, NOT_CORES);
        }
    }
    assert!(!made.w.at("made-by-git").exists() && !made.repo.join("made-by-git").exists());
    // A full id in capitals is a full id, and not what git says the branch names.
    made.write(&with(&kept, "end", json!(end.to_uppercase())));
    no(&made, &made.repo, MOVED);
    made.write(&with(&kept, "repo", json!(".git")));
    no(&made, &made.repo, NOT_CORES);
    // A run that ended with a commit names all three or, written by a build from before, none.
    for left_out in NAMED {
        let mut record = kept.clone();
        record.as_object_mut().unwrap().remove(left_out);
        made.write(&record);
        no(&made, &made.repo, NOT_CORES);
    }
    made.write(&with(&kept, "repo", json!(7)));
    no(&made, &made.repo, "cannot be read");
    made.write(&kept);
    assert_eq!(names(&printed(&made.show())), ["a.txt", "b.txt", "n.txt"]);
}

#[test]
fn a_record_from_before_is_shown_by_the_branchs_tip_and_its_one_parent() {
    let made = finished();
    let (branch, end, with_them) = (made.branch(), made.w.commit(&made.repo, &made.branch()), printed(&made.show()));
    let before = made.from_before();
    assert_eq!(before.as_object().unwrap().len(), 10, "{before}");
    made.write(&before);
    assert_eq!(printed(&made.show()), with_them);
    // The rule F44 built, gap and all: a commit of a run's shape is shown, and any other is refused.
    made.git(&["update-ref", &branch, &made.by_hand("main", &made.message(), &end)]);
    assert_eq!(printed(&made.show()), with_them);
    made.git(&["update-ref", &branch, &made.by_hand("main", "tidied by hand", &end)]);
    no(&made, &made.repo, MOVED);
    made.git(&["update-ref", &branch, &made.by_hand(&end, &made.message(), "main")]);
    assert_eq!(names(&printed(&made.show())), ["a.txt", "b.txt", "n.txt"], "against its one parent, whichever it is");
    // And it is shown in whichever repo is found: such a record names none.
    let other = made.w.at("work").join("elsewhere").join("alpha");
    copied(&made.repo, &other);
    assert_eq!(names(&printed(&made.show_in(&other, &[]))), ["a.txt", "b.txt", "n.txt"]);
}
