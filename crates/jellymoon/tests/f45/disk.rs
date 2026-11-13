//! What a disk and a history make of the work's names (`docs/development/keep.md`, "How the working tree and the
//! index are written"): a file the work swaps for a folder and a folder for a file, a name a disk takes for
//! another's, and a branch set back behind the run's start.

use std::fs;

use super::refused::{HISTORY, IN_WAY, unfit};
use super::settings::finished_over;
use super::support::skipped;
use super::{Entry, FORWARD, finished, kept};

const SWAP: [Entry; 4] = [
    ("0", "b.txt", b""),
    ("100644", "b.txt/now.txt", b"a folder now\n"),
    ("0", "dir/inner.txt", b""),
    ("100644", "dir", b"a file now\n"),
];

#[test]
fn work_that_swaps_a_file_for_a_folder_and_a_folder_for_a_file_is_kept() {
    let made = finished_over(&[("dir/inner.txt", "inside\n")]);
    let end = made.rework(&SWAP);
    kept(&made, &made.keep(&[]), &end, "main", FORWARD);
    assert_eq!(fs::read_to_string(made.repo.join("dir")).unwrap(), "a file now\n");
    assert_eq!(fs::read_to_string(made.repo.join("b.txt").join("now.txt")).unwrap(), "a folder now\n");
    assert_eq!(made.status(), "");
    // With something of the person's in the file or the folder that goes, nothing is done.
    for theirs in ["b.txt", "dir/inner.txt", "dir/untracked.txt"] {
        let made = finished_over(&[("dir/inner.txt", "inside\n")]);
        made.rework(&SWAP);
        fs::write(made.repo.join(theirs), "the person's, unsaved\n").unwrap();
        unfit(&made, IN_WAY);
        assert_eq!(fs::read_to_string(made.repo.join(theirs)).unwrap(), "the person's, unsaved\n");
    }
}

#[test]
fn a_file_of_the_persons_under_another_spelling_of_a_made_files_name_is_never_written_over() {
    for (joined, unsaved) in [(false, true), (true, true), (false, false)] {
        let made = finished();
        made.rework(&[("100644", "A.TXT", b"made by the run\n")]);
        if joined {
            made.moved_on();
        }
        // Whether this disk takes the two spellings for one name.
        let folds = made.repo.join("A.TXT").exists();
        let theirs = if unsaved { "the person's, unsaved\n" } else { "a.txt as it was\n" };
        fs::write(made.repo.join("a.txt"), theirs).unwrap();
        if folds {
            unfit(&made, IN_WAY);
        } else {
            assert_eq!(made.keep(&[]).code, 0);
            assert_eq!(fs::read_to_string(made.repo.join("A.TXT")).unwrap(), "made by the run\n");
        }
        assert_eq!(fs::read_to_string(made.repo.join("a.txt")).unwrap(), theirs);
    }
}

// A name no disk of the three systems takes: one part of it is longer than 255. git would stop at it part way;
// the keep says what it is before anything is written.
#[test]
fn a_file_of_the_work_whose_name_the_disk_does_not_take_is_said_as_that() {
    let made = finished();
    let long = format!("deep/{}.txt", "n".repeat(300));
    made.rework(&[("100644", long.as_str(), b"under a long name\n")]);
    let name = "a file of the work has a name this disk does not take: a part of it is too long, or is no name on this \
                system";
    unfit(&made, name);
    assert!(!made.repo.join("deep").exists());
}

// On a disk that takes two spellings for one name, `git checkout Main` stands the checkout on `main` under a
// name the repo does not list. The keep says that, and not that the branch has no history.
#[test]
fn a_branch_stood_on_under_another_spelling_of_its_name_is_said_as_that() {
    let made = finished();
    let moved = made.w.git_ran(&made.repo, &["checkout", "-q", "Main"]).status.success();
    if !moved || made.git(&["symbolic-ref", "HEAD"]) != "refs/heads/Main" {
        return skipped("a branch under another spelling of its name: this disk takes one spelling for a name");
    }
    let spelled = "the checkout's HEAD names a branch the repo does not list under that name: on a disk that takes \
                   two spellings for one name it is another spelling of a branch's name, and `git checkout` with \
                   the name as `git branch` prints it stands on the branch itself";
    unfit(&made, spelled);
    made.git(&["checkout", "-q", "main"]);
    kept(&made, &made.keep(&[]), &made.end(), "main", FORWARD);
}

#[test]
fn a_branch_set_back_behind_the_runs_start_does_not_hold_it() {
    let made = finished_over(&[("x.txt", "x\n")]);
    made.git(&["reset", "-q", "--hard", "HEAD~1"]);
    unfit(&made, HISTORY);
}
