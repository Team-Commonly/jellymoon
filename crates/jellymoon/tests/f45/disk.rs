//! What a disk and a history make of the work's names (`docs/development/keep.md`, "How the working tree and the
//! index are written"): a file the work swaps for a folder and a folder for a file, a name a disk takes for
//! another's, and a branch set back behind the run's start.

use std::fs;

use super::refused::{HISTORY, IN_WAY, unfit};
use super::settings::finished_over;
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

#[test]
fn a_branch_set_back_behind_the_runs_start_does_not_hold_it() {
    let made = finished_over(&[("x.txt", "x\n")]);
    made.git(&["reset", "-q", "--hard", "HEAD~1"]);
    unfit(&made, HISTORY);
}
