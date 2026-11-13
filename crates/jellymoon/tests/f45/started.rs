//! What a keep never starts, proved beside a control (`docs/development/keep.md`, "Why nothing of the repo's or
//! the person's config can run"): in each scene the planted program is one git does start when the person does
//! the same thing with plain git, and a keep does not start it. A linked worktree has a config of its own
//! (`config.worktree`), which git reads only through that worktree's git folder: what it names is switched off
//! too, and who the person is there is read from it.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::json;

use super::settings::finished_over;
use super::{Entry, FORWARD, JOINED, Made, UNCHANGED, finished, kept, planted, printed, started, touch};

const FILTER: &str = "a file of the work would be passed through a filter this repo sets for it, and Jellymoon \
                      starts no filter";

/// A second checkout of the repo at `work/linked/alpha`, on a new branch `feat` at `main`, in a repo whose
/// worktrees each have a config of their own.
fn linked(made: &Made) -> PathBuf {
    let feat = made.w.at("work").join("linked").join("alpha");
    made.git(&["config", "extensions.worktreeConfig", "true"]);
    made.git(&["worktree", "add", "-q", "-b", "feat", feat.to_str().unwrap()]);
    feat
}

/// Names the planted filter in a config: the checkout's own (`--worktree`), the repo's (`--local`) or the
/// person's (`--global`, which in a test is a file of the test's).
fn filtered(made: &Made, checkout: &Path, scope: &str, filter: &Path) {
    for step in ["clean", "smudge"] {
        made.w.git(checkout, &["config", scope, &format!("filter.planted.{step}"), filter.to_str().unwrap()]);
    }
}

type Files = Vec<(PathBuf, Vec<u8>)>;

/// Everything of a linked checkout that a keep could change, with the repo's own (`Made::held`).
fn held_in(made: &Made, feat: &Path) -> (super::Held, Files, Vec<u8>) {
    let files = super::support::tree(feat);
    let own = PathBuf::from(made.w.git(feat, &["rev-parse", "--absolute-git-dir"]));
    (made.held(), files, fs::read(own.join("index")).unwrap())
}

// The gate's first finding. The filter is named in the linked worktree's own config, and two attribute files of
// the person's hide it from every question but git's own at the moment it writes the file: an untracked
// `.gitattributes` sets it, and an unsaved edit to a tracked one below seems to take it off again. git writes
// by the tracked file as the commit has it, so the filter is on.
#[test]
fn a_file_git_would_filter_when_it_writes_it_is_refused_whatever_the_persons_unsaved_attributes_say() {
    for scope in ["--worktree", "--local"] {
        let made = finished_over(&[("sub/deep/.gitattributes", "# nothing yet\n")]);
        let (feat, folder) = (linked(&made), made.w.at("tmp"));
        filtered(&made, &feat, scope, &planted(&folder, "filter"));
        made.w.git(&feat, &["config", scope, "filter.planted.required", "true"]);
        fs::write(feat.join("sub").join(".gitattributes"), "*.dat filter=planted\n").unwrap();
        fs::write(feat.join("sub").join("deep").join(".gitattributes"), "# nothing yet\n*.dat -filter\n").unwrap();
        let end = made.rework(&[("100644", "sub/deep/big.dat", b"what the store holds\n")]);
        let (before, why) = (held_in(&made, &feat), format!("{FILTER}: {UNCHANGED}").replace("{run}", &made.run));
        let asked = printed(&made.keep_in(&feat, &["--ask"], &[]));
        assert_eq!((&asked["fits"], &asked["why"]), (&json!(false), &json!(why)), "{scope}");
        let ran = made.keep_in(&feat, &[], &[]);
        assert_eq!(super::refused(&ran), super::said(&why), "{scope}");
        assert!(before == held_in(&made, &feat), "a refused keep changed something ({scope})");
        assert_eq!(started(&folder), Vec::<String>::new(), "{scope}");
        // The control: the person lands the same commit themselves, and git starts the filter for that file.
        made.w.git_ran(&feat, &["merge", "-q", "--ff-only", &end]);
        assert_eq!(started(&folder), ["ran-filter"], "{scope}: git starts this filter for this file");
    }
}

// A scene in which switching the filter off is all that stops it. No file of the work has a filter, so the
// keep goes on; the person's own tracked file has one, and was touched, so git passes it through the filter
// when it looks whether the file changed, as the first thing `git merge` does.
#[test]
fn a_filter_git_would_start_for_a_touched_file_beside_the_work_is_switched_off_wherever_it_is_named() {
    for scope in ["--worktree", "--local", "--global"] {
        let made = finished_over(&[(".gitattributes", "*.dat filter=planted\n"), ("big.dat", "as it was stored\n")]);
        let (feat, folder) = (linked(&made), made.w.at("tmp"));
        filtered(&made, &feat, scope, &planted(&folder, "filter"));
        touch(&feat.join("big.dat"));
        assert_eq!(printed(&made.keep_in(&feat, &["--ask"], &[]))["fits"], json!(true), "{scope}");
        assert_eq!(started(&folder), Vec::<String>::new(), "{scope}: asked");
        kept(&made, &made.keep_in(&feat, &[], &[]), &made.end(), "feat", FORWARD);
        assert_eq!(started(&folder), Vec::<String>::new(), "{scope}: kept");
        assert_eq!(fs::read_to_string(feat.join("big.dat")).unwrap(), "as it was stored\n");
        // The control: the same touched file, and the person asks git what changed.
        super::touch_back(&feat.join("big.dat"), 10);
        made.w.git_ran(&feat, &["status", "--porcelain"]);
        assert_eq!(started(&folder), ["ran-filter"], "{scope}: git starts this filter for this file");
    }
}

// The same touched file, with a filter the repo says is required. A required filter that is set to nothing is
// one git stops at, so `required` is switched off with the filter, in a linked worktree's own config too: the
// work is kept and nothing is started.
#[test]
fn a_touched_file_beside_the_work_whose_filter_is_required_is_no_reason_to_stop() {
    for scope in ["--worktree", "--local"] {
        let made = finished_over(&[(".gitattributes", "*.dat filter=planted\n"), ("big.dat", "as it was stored\n")]);
        let (feat, folder) = (linked(&made), made.w.at("tmp"));
        filtered(&made, &feat, scope, &planted(&folder, "filter"));
        made.w.git(&feat, &["config", scope, "filter.planted.required", "true"]);
        touch(&feat.join("big.dat"));
        assert_eq!(printed(&made.keep_in(&feat, &["--ask"], &[]))["fits"], json!(true), "{scope}");
        kept(&made, &made.keep_in(&feat, &[], &[]), &made.end(), "feat", FORWARD);
        assert_eq!(started(&folder), Vec::<String>::new(), "{scope}");
    }
}

// A merge driver named in the config of the repo's first checkout is one git starts for a merge in the store,
// which reads that config whichever checkout the keep is asked in.
#[test]
fn a_merge_driver_in_the_first_checkouts_own_config_is_not_started_by_a_keep_in_a_linked_one() {
    let made = finished_over(&[("long.txt", "one\ntwo\nthree\nfour\nfive\nsix\nseven\neight\n")]);
    let end = made.rework(&[("100644", "long.txt", b"one\ntwo\nthree\nfour\nfive\nsix\nseven\neight by the run\n")]);
    made.who();
    let (feat, folder) = (linked(&made), made.w.at("tmp"));
    made.git(&["config", "--worktree", "merge.planted.driver", planted(&folder, "driver").to_str().unwrap()]);
    fs::write(made.repo.join(".git").join("info").join("attributes"), "long.txt merge=planted\n").unwrap();
    fs::write(feat.join("long.txt"), "one by the person\ntwo\nthree\nfour\nfive\nsix\nseven\neight\n").unwrap();
    made.w.git(&feat, &["commit", "-q", "-am", "the person's edit of another line"]);
    let before = held_in(&made, &feat);
    assert_eq!(made.keep_in(&feat, &[], &[]).code, 1);
    assert!(before == held_in(&made, &feat));
    assert_eq!(started(&folder), Vec::<String>::new());
    // The control: the person merges the two in the store themselves, in the first checkout.
    made.w.git_ran(&made.repo, &["merge-tree", "--write-tree", "feat", &end]);
    assert_eq!(started(&folder), ["ran-driver"], "git starts this driver for this merge");
}

// Who the person is in a linked worktree is what git says there: a name in that worktree's own config is theirs.
#[test]
fn the_one_new_commit_in_a_linked_worktree_is_by_who_git_says_the_person_is_there() {
    let made = finished();
    made.who();
    let feat = linked(&made);
    made.w.git(&feat, &["config", "--worktree", "user.name", "Ada In The Worktree"]);
    made.w.git(&feat, &["config", "--worktree", "user.email", "ada@worktree.example.test"]);
    fs::write(feat.join("f.txt"), "only on feat\n").unwrap();
    made.w.git(&feat, &["add", "f.txt"]);
    made.w.git(&feat, &["commit", "-q", "-m", "a commit on feat"]);
    let ran = made.keep_in(&feat, &[], &[]);
    let tip = made.w.commit(&made.repo, "feat");
    kept(&made, &ran, &tip, "feat", JOINED);
    let who = "Ada In The Worktree <ada@worktree.example.test> / Ada In The Worktree <ada@worktree.example.test>";
    assert_eq!(made.git(&["log", "-1", "--format=%an <%ae> / %cn <%ce>", &tip]), who);
}

// Work that touches a file with a filter is refused however the filter is told of: here by an untracked
// `.gitattributes` alone, in the folder of the file, with nothing of the kind in any commit.
#[test]
fn an_untracked_attributes_file_above_a_file_of_the_work_is_read_as_git_will_read_it() {
    let made = finished();
    let folder = made.w.at("tmp");
    made.git(&["config", "filter.planted.smudge", planted(&folder, "filter").to_str().unwrap()]);
    let entries: [Entry; 1] = [("100644", "sub/deep/big.dat", b"what the store holds\n")];
    made.rework(&entries);
    fs::create_dir(made.repo.join("sub")).unwrap();
    fs::write(made.repo.join("sub").join(".gitattributes"), "deep/*.dat filter=planted\n").unwrap();
    super::refused::unfit(&made, FILTER);
    assert_eq!(started(&folder), Vec::<String>::new());
    // With the file gone the work is kept.
    fs::remove_file(made.repo.join("sub").join(".gitattributes")).unwrap();
    kept(&made, &made.keep(&[]), &made.end(), "main", FORWARD);
}

// The other three readings each refuse alone. A filter that only one of them tells of is a reason to stop,
// though git at this version would not start it: the readings are asked so that no one of them has to be right.
#[test]
fn a_filter_only_the_commit_the_checkouts_files_or_the_staged_files_tell_of_is_refused() {
    let big: Entry = ("100644", "sub/deep/big.dat", b"what the store holds\n");
    let set = "*.dat filter=planted\n";
    // Only the commit's own: an untracked attributes file below takes the filter off for the checkout's files,
    // and the person has staged the removal of the file that sets it.
    let made = finished_over(&[("sub/.gitattributes", set)]);
    made.rework(&[big]);
    fs::create_dir_all(made.repo.join("sub").join("deep")).unwrap();
    fs::write(made.repo.join("sub").join("deep").join(".gitattributes"), "*.dat -filter\n").unwrap();
    made.git(&["rm", "-q", "--cached", "sub/.gitattributes"]);
    super::refused::unfit(&made, FILTER);
    // Only the checkout's files: an unsaved edit of a tracked attributes file.
    let made = finished_over(&[(".gitattributes", "# nothing yet\n")]);
    made.rework(&[big]);
    fs::write(made.repo.join(".gitattributes"), set).unwrap();
    super::refused::unfit(&made, FILTER);
    // Only what is staged: the same edit added to the index, and taken back on the disk.
    let made = finished_over(&[(".gitattributes", "# nothing yet\n")]);
    made.rework(&[big]);
    fs::write(made.repo.join(".gitattributes"), set).unwrap();
    made.git(&["add", ".gitattributes"]);
    fs::write(made.repo.join(".gitattributes"), "# nothing yet\n").unwrap();
    super::refused::unfit(&made, FILTER);
}
