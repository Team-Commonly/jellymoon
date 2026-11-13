//! Which attributes git goes by when it writes the work's files (`docs/development/keep.md`, "A file with a
//! filter"): the ones in the index the step leaves, and a file of the disk only where that index has none and
//! the step has not removed it. So a `.gitattributes` the work removes is gone before a file is written, one it
//! changes or adds is read as the work has it, and one the person staged is read as staged. Each scene has a
//! control: the person lands the same commit with plain git, and the planted filter is started or is not.

use std::fs;
use std::path::Path;

use serde_json::json;

use super::raced::told;
use super::settings::finished_over;
use super::support::skipped;
use super::{Entry, FORWARD, Made, UNCHANGED, finished, planted, printed, shell, started};

const FILTER: &str = "a file of the work would be passed through a filter this repo sets for it, and Jellymoon \
                      starts no filter";
const SET: &str = "*.dat filter=planted\n";
/// Two files a filter would be set for: git writes the first before a `.gitattributes` beside it, the second after.
const DATA: [Entry; 2] =
    [("100644", "sub/+first.dat", b"what the store holds\n"), ("100644", "sub/new.dat", b"what the store holds\n")];

/// Names the planted filter in the repo's config, for a file git writes (`smudge`) and one it takes in (`clean`).
fn filter(made: &Made) -> std::path::PathBuf {
    let folder = made.w.at("tmp");
    for step in ["clean", "smudge"] {
        made.git(&["config", &format!("filter.planted.{step}"), &shell(&planted(&folder, "filter"))]);
    }
    folder
}

/// Fails unless the work is refused for a filter, asked or not, with nothing changed and nothing started.
fn refused(made: &Made, folder: &Path, scene: &str) {
    let (before, why) = (made.held(), format!("{FILTER}: {UNCHANGED}").replace("{run}", &made.run));
    let asked = printed(&made.keep(&["--ask", "--with-settings"]));
    assert_eq!((&asked["fits"], &asked["why"]), (&json!(false), &json!(why)), "{scene}");
    made.refuses(&made.keep(&["--with-settings"]), &before, &why);
    assert_eq!(started(folder), Vec::<String>::new(), "{scene}");
}

/// The person lands what a keep would land, with plain git: the run's commit, or, with commits of their own, one
/// new commit on top of theirs that holds both.
fn by_hand(made: &Made, end: &str, joined: bool) {
    let mut commit = end.to_string();
    if joined {
        let tree = made.git(&["merge-tree", "--write-tree", "main", end]);
        commit = made.git(&["commit-tree", "-p", "main", "-m", "the two by hand", &tree]);
    }
    made.w.git_ran(&made.repo, &["merge", "-q", "--ff-only", &commit]);
    assert_eq!(made.w.commit(&made.repo, "main"), commit, "the person's own step landed the commit");
}

// The gate's first finding of its second pass. A tracked `.gitattributes` takes the filter off below it, and the
// work removes that file and makes a file there. git removes before it writes, so by the time the new file is
// written only the person's own attributes file above is left, which sets the filter: untracked beside the
// folder, untracked at the top, or staged and edited since (git reads what is staged).
#[test]
fn an_attributes_file_the_work_removes_is_not_read_as_if_it_were_still_there() {
    for (theirs, joined) in
        [("sub", false), ("sub", true), ("top", false), ("top", true), ("staged", false), ("staged", true)]
    {
        let made = finished_over(&[("sub/deep/.gitattributes", "*.dat -filter\n")]);
        let (folder, scene) = (filter(&made), format!("{theirs}, joined {joined}"));
        made.git(&["config", "filter.planted.required", "true"]);
        let work: [Entry; 2] =
            [("0", "sub/deep/.gitattributes", b""), ("100644", "sub/deep/new.dat", b"what the store holds\n")];
        let end = made.rework(&work);
        if joined {
            made.moved_on();
        }
        let above = if theirs == "top" { made.repo.clone() } else { made.repo.join("sub") };
        fs::write(above.join(".gitattributes"), SET).unwrap();
        if theirs == "staged" {
            made.git(&["add", "sub/.gitattributes"]);
            fs::write(above.join(".gitattributes"), "# nothing by now\n").unwrap();
        }
        refused(&made, &folder, &scene);
        by_hand(&made, &end, joined);
        assert_eq!(started(&folder), ["ran-filter"], "{scene}: git starts this filter for this file");
    }
}

// The mirror: a `.gitattributes` the work changes. git reads it from the index the step leaves, so every file
// is written by the new one, the file that is written before it too.
#[test]
fn an_attributes_file_the_work_changes_is_read_as_the_work_leaves_it_whatever_git_writes_first() {
    // The change takes away the line that took the filter off: the person's untracked file above sets it.
    let made = finished_over(&[("sub/.gitattributes", "*.dat -filter\n")]);
    let folder = filter(&made);
    let end = made.rework(&[&[("100644", "sub/.gitattributes", &b"# nothing by now\n"[..])][..], &DATA[..]].concat());
    fs::write(made.repo.join(".gitattributes"), SET).unwrap();
    refused(&made, &folder, "the change sets the filter free");
    by_hand(&made, &end, false);
    assert_eq!(started(&folder), ["ran-filter"], "git starts this filter for these files");
    // The change takes the filter off. git would start nothing, and the keep is refused all the same: the
    // checkout's file as it is now still sets the filter, and any reading that says `filter` refuses.
    let made = finished_over(&[("sub/.gitattributes", SET)]);
    let folder = filter(&made);
    let end = made.rework(&[&[("100644", "sub/.gitattributes", &b"*.dat -filter\n"[..])][..], &DATA[..]].concat());
    refused(&made, &folder, "the change takes the filter off");
    by_hand(&made, &end, false);
    assert_eq!(started(&folder), Vec::<String>::new(), "git reads the changed file as the commit has it");
}

// And one the work adds: it is in the commit, so git goes by it for the files beside it and below, and the
// person's untracked file above is read only where the added one says nothing.
#[test]
fn an_attributes_file_the_work_adds_is_read_from_the_work() {
    let made = finished();
    let folder = filter(&made);
    made.rework(&[&[("100644", "sub/.gitattributes", &b"*.dat -filter\n"[..])][..], &DATA[..]].concat());
    fs::write(made.repo.join(".gitattributes"), SET).unwrap();
    assert_eq!(printed(&made.keep(&["--ask", "--with-settings"]))["fits"], json!(true));
    let said = printed(&made.keep(&["--with-settings"]));
    assert_eq!(said["why"], json!(FORWARD.replace("{run}", &made.run)));
    assert_eq!(started(&folder), Vec::<String>::new());
    for file in ["+first.dat", "new.dat"] {
        assert_eq!(fs::read_to_string(made.repo.join("sub").join(file)).unwrap(), "what the store holds\n");
    }
    // The control: the person's file above is one git goes by, for a file the added one does not cover.
    fs::write(made.repo.join("top.dat"), "the person's\n").unwrap();
    made.w.git_ran(&made.repo, &["add", "top.dat"]);
    assert_eq!(started(&folder), ["ran-filter"], "git starts this filter for a file at the top");
}

/// Makes a link to a file. False where the system does not let this user make one.
fn linked(to: &Path, link: &Path) -> bool {
    #[cfg(unix)]
    let made = std::os::unix::fs::symlink(to, link);
    #[cfg(windows)]
    let made = std::os::windows::fs::symlink_file(to, link);
    made.is_ok()
}

// git does not follow a `.gitattributes` that is a link (2.32 and later), and neither does a keep: what the link
// leads to sets no filter for the work.
#[test]
fn an_attributes_file_that_is_a_link_is_not_followed_as_git_does_not_follow_one() {
    let made = finished();
    let folder = filter(&made);
    let end = made.rework(&[("100644", "sub/new.dat", b"what the store holds\n")]);
    fs::write(folder.join("attributes"), SET).unwrap();
    fs::create_dir(made.repo.join("sub")).unwrap();
    if !linked(&folder.join("attributes"), &made.repo.join("sub").join(".gitattributes")) {
        return skipped("a `.gitattributes` that is a link: this system let the test make no link");
    }
    // The control: git itself says the file has no filter, with the link there.
    let said = made.w.git_ran(&made.repo, &["check-attr", "filter", "sub/new.dat"]);
    if !String::from_utf8_lossy(&said.stdout).trim_end().ends_with("filter: unspecified") {
        return skipped("a `.gitattributes` that is a link: this git follows one (it is from before 2.32)");
    }
    super::kept(&made, &made.keep(&[]), &end, "main", FORWARD);
    assert_eq!(started(&folder), Vec::<String>::new());
    assert_eq!(fs::read_to_string(made.repo.join("sub").join("new.dat")).unwrap(), "what the store holds\n");
}

// A git that does not answer the question has not said that there is no filter: the keep is refused as one whose
// repo could not be read, with nothing changed. Told to answer, the same git says `filter`.
#[test]
fn a_git_that_does_not_answer_about_attributes_has_not_said_that_no_filter_is_set() {
    let made = finished();
    let folder = filter(&made);
    made.rework(&[("100644", "big.dat", b"what the store holds\n")]);
    fs::write(made.repo.join(".gitattributes"), SET).unwrap();
    let before = made.held();
    let ran = told(&made, &made.repo, "check-attr", &[("fails", &["1"])]);
    let unread = "the work of run {run} could not be read from the repo: the folder is not the top of a checkout git \
                  opens, or git did not answer";
    made.refuses(&ran, &before, unread);
    assert_eq!(started(&folder), Vec::<String>::new());
    refused(&made, &folder, "git answers");
}

// The index the step would leave, asked alone. What the person staged at the top sets the filter, and an
// untracked file of theirs below takes it off again: git would start nothing, and the keep is refused all the
// same, since any reading that says `filter` refuses.
#[test]
fn a_filter_only_what_is_staged_sets_is_refused_though_an_untracked_file_below_takes_it_off() {
    let made = finished();
    let folder = filter(&made);
    let end = made.rework(&[("100644", "sub/new.dat", b"what the store holds\n")]);
    fs::write(made.repo.join(".gitattributes"), SET).unwrap();
    made.git(&["add", ".gitattributes"]);
    fs::create_dir(made.repo.join("sub")).unwrap();
    fs::write(made.repo.join("sub").join(".gitattributes"), "*.dat -filter\n").unwrap();
    refused(&made, &folder, "staged above, taken off below");
    by_hand(&made, &end, false);
    assert_eq!(started(&folder), Vec::<String>::new(), "git goes by the untracked file below");
}
