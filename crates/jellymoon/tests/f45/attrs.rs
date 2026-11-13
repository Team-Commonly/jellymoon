//! Which attributes git goes by when it writes the work's files (`docs/development/keep.md`, "A file with a
//! filter"): the ones in the index the step leaves, and a file of the disk only where that index has none and
//! the step has not removed it. So a `.gitattributes` the work removes is gone before a file is written, one it
//! changes or adds is read as the work has it, and one the person staged is read as staged. Each scene has a
//! control: the person lands the same commit with plain git, and the planted filter is started or is not.

use std::fs;
use std::path::Path;

use serde_json::json;

use super::late::writable;
use super::raced::told;
use super::settings::finished_over;
use super::support::skipped;
use super::{Entry, FORWARD, JOINED, Made, UNCHANGED, finished, planted, printed, shell, started};

const FILTER: &str = "a file of the work would be passed through a filter this repo sets for it, and Jellymoon \
                      starts no filter";
const SET: &str = "*.dat filter=planted\n";
const STAYED: &str = "; not every `.gitattributes` the work removes is gone: each one `stayed` names is still at \
                      that path in the checkout, and a file of the work it sets a filter for was written as the \
                      store holds it, not as that filter would have made it";
const MORE: &str = ", and more of them are still there than `stayed` names";
/// Two files a filter would be set for: git writes the first before a `.gitattributes` beside it, the second after.
const DATA: [Entry; 2] =
    [("100644", "sub/+first.dat", b"what the store holds\n"), ("100644", "sub/new.dat", b"what the store holds\n")];

/// Names the planted filter in the repo's config, for a file git writes (`smudge`) and one it takes in (`clean`).
fn filter(made: &Made) -> std::path::PathBuf {
    let folder = made.w.at("tmp");
    let program = shell(&planted(&folder, "filter"));
    for step in ["clean", "smudge"] {
        made.git(&["config", &format!("filter.planted.{step}"), &program]);
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
    // A git that will not make the index the step would leave has stopped the trial: the attributes are asked
    // of no other index, and what stops that take stops the step.
    let stopped = format!("{}: {UNCHANGED}", super::refused::IN_WAY);
    made.refuses(&told(&made, &made.repo, "core.splitIndex=false", &[("fails", &["1"])]), &before, &stopped);
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

// Readings one and three are asked of the index the step would leave, and here only that tells. What the person
// staged at the top names a filter by a word of its own, which is in that index and in no commit. Below, a file
// of theirs gives `*.dat` that word: untracked in `low`, where git goes by it and starts the filter (the top
// file on the disk no longer has the word, so the checkout's files alone say nothing); or as an unsaved edit in
// `sub`, which git does not read, and the keep is refused all the same.
#[test]
fn a_word_only_what_is_staged_gives_a_filter_to_sets_it_through_a_file_of_the_persons_below() {
    for (below, starts) in [("low", true), ("sub", false)] {
        let made = finished_over(&[("sub/.gitattributes", "# nothing yet\n")]);
        let (folder, new, top) = (filter(&made), format!("{below}/new.dat"), made.repo.join(".gitattributes"));
        let end = made.rework(&[("100644", new.as_str(), b"what the store holds\n")]);
        fs::write(&top, "[attr]big filter=planted\n").unwrap();
        made.git(&["add", ".gitattributes"]);
        if starts {
            fs::write(&top, "# nothing by now\n").unwrap()
        } else {
            fs::remove_file(&top).unwrap()
        }
        fs::create_dir_all(made.repo.join(below)).unwrap();
        fs::write(made.repo.join(below).join(".gitattributes"), "*.dat big\n").unwrap();
        refused(&made, &folder, below);
        by_hand(&made, &end, false);
        assert_eq!(started(&folder).len(), usize::from(starts), "{below}");
    }
}

// The gate's second finding of its third pass. The work removes a `.gitattributes` that sets the filter, and git
// cannot remove it: its folder is shut. git warns, goes on, and goes by the file it left, and no check before
// the step can know every reason a file stays. So the keep looks after the step, and its answer names the file.
#[test]
fn an_attributes_file_git_could_not_remove_is_named_by_the_keep_that_wrote_past_it() {
    for (keep, joined) in [(false, false), (true, false), (true, true)] {
        let deep = ("sub/deep/.gitattributes", "*.dat -filter\n");
        let made = finished_over(&[("sub/.gitattributes", SET), deep, ("sub/plain.txt", "stays too, unnamed\n")]);
        let (folder, new) = (filter(&made), made.repo.join("sub").join("deep").join("new.dat"));
        let end = made.rework(&[
            ("0", "sub/.gitattributes", b""),
            ("0", "sub/deep/.gitattributes", b""),
            ("0", "sub/plain.txt", b""),
            ("100644", "sub/deep/new.dat", b"what the store holds\n"),
        ]);
        if joined {
            made.moved_on();
        }
        if !writable(&made.repo.join("sub"), false) {
            return skipped("an attributes file git cannot remove: this system, or this user, has no shut folder");
        }
        if !keep {
            by_hand(&made, &end, false);
            assert!(writable(&made.repo.join("sub"), true));
            assert_eq!(started(&folder), ["ran-filter"], "git goes by the file it could not remove");
            continue;
        }
        assert_eq!(printed(&made.keep(&["--ask", "--with-settings"]))["fits"], json!(true), "no question sees it");
        let ran = made.keep(&["--with-settings"]);
        assert!(writable(&made.repo.join("sub"), true));
        let (said, how, kept) =
            (printed(&ran), if joined { JOINED } else { FORWARD }, made.w.commit(&made.repo, "main"));
        let why = format!("{how}{STAYED}").replace("{run}", &made.run);
        assert_eq!((ran.code, &said["kept"], &said["why"]), (0, &json!(kept), &json!(why)), "{}", ran.stderr);
        assert_eq!(kept == end, !joined);
        assert_eq!((&said["stayed"], &made.record()["work"]), (&json!(["sub/.gitattributes"]), &json!("kept")));
        assert_eq!((started(&folder), fs::read_to_string(new).unwrap().as_str()), (vec![], "what the store holds\n"));
    }
    // Moved just before the step to a branch that holds the work already and has the file again, git writes
    // nothing: the file there is that branch's own, and is not said to have stayed.
    let made = finished_over(&[("sub/.gitattributes", "# nothing\n")]);
    let (start, end) = (made.w.commit(&made.repo, "main"), made.rework(&[("0", "sub/.gitattributes", b"")]));
    made.who();
    let again = made.git(&["commit-tree", "-p", &end, "-m", "back again", &format!("{start}^{{tree}}")]);
    made.git(&["branch", "ahead", &again]);
    let said = printed(&told(&made, &made.repo, "merge", &[("first", &["checkout", "-q", "ahead"])]));
    let held = super::record::HELD.replace("{run}", &made.run);
    assert_eq!((&said["why"], &said["onto"], &said["stayed"]), (&json!(held), &json!("ahead"), &json!(null)));
    assert!(made.repo.join("sub").join(".gitattributes").exists());
}

// The gate's second finding of its fourth pass. Only a plain file at the path is one that stayed. Where the work
// puts a folder at the path, or makes the folder above it a link to one that holds such a file, git did remove
// the file, and nothing is named.
#[test]
fn an_attributes_file_the_work_swaps_for_a_folder_or_puts_behind_a_link_is_not_said_to_have_stayed() {
    let made = finished_over(&[("a/.gitattributes", "# nothing\n"), ("b/.gitattributes", "# nothing\n")]);
    let end = made.rework(&[
        ("0", "a/.gitattributes", b""),
        ("100644", "a/.gitattributes/x", b"in a folder by now\n"),
        ("0", "b/.gitattributes", b""),
        ("120000", "b", b"real"),
        ("100644", "real/.gitattributes", b"# nothing\n"),
    ]);
    let (said, why) = (printed(&made.keep(&["--with-settings"])), FORWARD.replace("{run}", &made.run));
    assert_eq!((&said["kept"], &said["why"], &said["stayed"]), (&json!(end), &json!(why), &json!(null)));
    assert!(made.repo.join("a").join(".gitattributes").is_dir());
}

// Made again by the person between the step and the look, a file is at its path all the same and is named, the
// one at the top too. Of more than a list holds, the first 200 are named and the words say that there are more.
#[test]
fn of_more_attributes_files_still_there_than_a_list_holds_the_words_say_that_there_are_more() {
    for over in [false, true] {
        let below: Vec<String> = (0..199 + usize::from(over)).map(|n| format!("d{n:03}/.gitattributes")).collect();
        let names: Vec<&str> = [".gitattributes"].into_iter().chain(below.iter().map(String::as_str)).collect();
        let made = finished_over(&names.iter().map(|name| (*name, "# nothing\n")).collect::<Vec<_>>());
        let start = made.w.commit(&made.repo, "main");
        let end = made.rework(&names.iter().map(|name| ("0", *name, &b""[..])).collect::<Vec<_>>());
        let again: [(&str, &[&str]); 1] = [("after", &["checkout", "-q", start.as_str(), "--", "."])];
        let said = printed(&told(&made, &made.repo, "merge", &again));
        let why = format!("{FORWARD}{STAYED}{}", if over { MORE } else { "" }).replace("{run}", &made.run);
        assert_eq!((&said["kept"], &said["why"]), (&json!(end), &json!(why)), "{over}");
        let stayed = said["stayed"].as_array().unwrap();
        let ends = (stayed.len(), &stayed[0], &stayed[199]);
        assert_eq!(ends, (200, &json!(".gitattributes"), &json!("d198/.gitattributes")), "{over}");
    }
}

// A checkout that leaves a path out (sparse, by patterns), with a file of the person's at that path, which the
// work removes: plain git refuses the merge, and so does the keep, in the words of what is in the way.
#[test]
fn a_file_of_the_persons_where_a_sparse_checkout_left_one_out_is_in_the_way_of_work_that_removes_it() {
    let made = finished_over(&[("sub/.gitattributes", "*.dat -filter\n")]);
    made.rework(&[("0", "sub/.gitattributes", b""), ("100644", "new.dat", b"what the store holds\n")]);
    made.git(&["config", "core.sparseCheckout", "true"]);
    fs::write(made.repo.join(".git").join("info").join("sparse-checkout"), "/*\n!sub/.gitattributes\n").unwrap();
    made.git(&["read-tree", "-m", "-u", "HEAD"]);
    assert!(!made.repo.join("sub").join(".gitattributes").exists(), "the checkout leaves the file out");
    fs::create_dir_all(made.repo.join("sub")).unwrap();
    fs::write(made.repo.join("sub").join(".gitattributes"), "the person's own\n").unwrap();
    let why = format!("{}: {UNCHANGED}", super::refused::IN_WAY).replace("{run}", &made.run);
    let before = made.held();
    assert_eq!(printed(&made.keep(&["--ask", "--with-settings"]))["why"], json!(why));
    made.refuses(&made.keep(&["--with-settings"]), &before, &why);
}
