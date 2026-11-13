//! Work that no longer fits is refused with the working tree, `HEAD`, the index, every ref and the run's record
//! unchanged, and the reason in fixed words (`docs/development/keep.md`, "What is refused").

use std::fs;

use serde_json::json;

use super::{DAY, Entry, FORWARD, JOINED, Made, UNCHANGED, finished, kept, printed, ran, said, world};

pub const IN_WAY: &str = "an unsaved change, or a file that is not in the branch, is where the work would go";
pub const HISTORY: &str = "the branch's history does not hold the commit run {run} started from";
const NO_BRANCH: &str =
    "the checkout stands on no branch of the person's: its HEAD is detached, or names a branch named as a run's is";
const MIDWAY: &str = "a merge, a cherry-pick, a revert, a rebase or a bisect is going on in the checkout";

/// Fails unless a keep is refused for `why` with nothing changed, asked or not.
pub fn unfit(made: &Made, why: &str) {
    let (before, why) = (made.held(), format!("{why}: {UNCHANGED}").replace("{run}", &made.run));
    let asked = printed(&made.keep(&["--ask"]));
    assert_eq!((&asked["fits"], &asked["kept"], &asked["why"]), (&json!(false), &json!(null), &json!(why)));
    assert!(before == made.held(), "a keep that only asked changed something");
    made.refuses(&made.keep(&[]), &before, &why);
    made.refuses(&made.keep(&["--with-settings"]), &before, &why);
}

#[test]
fn the_same_lines_changed_on_both_sides_are_refused() {
    let made = finished();
    made.who();
    fs::write(made.repo.join("a.txt"), "edited by the person\n").unwrap();
    made.git(&["commit", "-q", "-am", "the person's own edit of the same line"]);
    let clash = "the branch's commits since run {run} started and the run's work changed the same lines, or the same \
                 file in a way git does not merge";
    unfit(&made, clash);
    // A file the work removed, which the person changed, is one git does not merge either.
    let made = finished();
    made.who();
    fs::write(made.repo.join("b.txt"), "the person still wants this\n").unwrap();
    made.git(&["commit", "-q", "-am", "an edit of the file the work removes"]);
    unfit(&made, clash);
}

#[test]
fn an_unsaved_change_in_a_file_of_the_work_is_in_the_way() {
    // Unsaved in a file the work changes, in one it removes, and staged.
    for (file, staged) in [("a.txt", false), ("b.txt", false), ("a.txt", true)] {
        let made = finished();
        fs::write(made.repo.join(file), "half an edit of the person's\n").unwrap();
        if staged {
            made.git(&["add", file]);
        }
        unfit(&made, IN_WAY);
        // And the same with commits of the person's own since.
        made.git(&["stash", "-q"]);
        made.moved_on();
        made.git(&["stash", "pop", "-q"]);
        unfit(&made, IN_WAY);
    }
}

#[test]
fn a_file_of_the_persons_that_git_does_not_have_is_in_the_way_ignored_or_not() {
    for ignored in [false, true] {
        let made = finished();
        if ignored {
            fs::write(made.repo.join(".git").join("info").join("exclude"), "n.txt\n").unwrap();
        }
        fs::write(made.repo.join("n.txt"), "the person's own, never committed\n").unwrap();
        unfit(&made, IN_WAY);
        made.moved_on();
        unfit(&made, IN_WAY);
        assert_eq!(fs::read_to_string(made.repo.join("n.txt")).unwrap(), "the person's own, never committed\n");
    }
    // A folder of theirs where the work makes a file, and a file of theirs where the work makes a folder.
    let made = finished();
    let entries: [Entry; 2] = [("100644", "plain", b"a file\n"), ("100644", "deep/in/file.txt", b"in folders\n")];
    made.rework(&entries);
    fs::create_dir(made.repo.join("plain")).unwrap();
    fs::write(made.repo.join("plain").join("theirs.txt"), "the person's\n").unwrap();
    unfit(&made, IN_WAY);
    // The same when git was told to ignore what is theirs: an ignored folder where a file would go, and an ignored
    // file where a folder would.
    fs::write(made.repo.join(".git").join("info").join("exclude"), "plain\ndeep\n").unwrap();
    unfit(&made, IN_WAY);
    fs::remove_dir_all(made.repo.join("plain")).unwrap();
    for ignored in [false, true] {
        let exclude = if ignored { "deep\n" } else { "" };
        fs::write(made.repo.join(".git").join("info").join("exclude"), exclude).unwrap();
        fs::write(made.repo.join("deep"), "the person's file\n").unwrap();
        unfit(&made, IN_WAY);
        fs::remove_file(made.repo.join("deep")).unwrap();
    }
    kept(&made, &made.keep(&[]), &made.end(), "main", FORWARD);
}

#[test]
fn a_checkout_that_stands_on_no_branch_of_the_persons_is_refused() {
    let made = finished();
    made.git(&["checkout", "-q", "--detach"]);
    unfit(&made, NO_BRANCH);
    // On another run's branch: a second wish's run in the same repo.
    made.git(&["checkout", "-q", "main"]);
    let wish = made.w.wish("alpha");
    let (run, _) = ran(&made.w.start(&wish, &made.repo));
    made.git(&["checkout", "-q", &format!("jellymoon/run-{run}")]);
    unfit(&made, NO_BRANCH);
    // On the run's own branch.
    made.git(&["checkout", "-q", &format!("jellymoon/run-{}", made.run)]);
    unfit(&made, "a checkout stands on the branch of run {run}");
    // On a name for a branch, the checkout stands on that branch, as git itself says: the work lands on it.
    made.git(&["checkout", "-q", "main"]);
    made.git(&["symbolic-ref", "refs/heads/alias", "refs/heads/main"]);
    made.git(&["symbolic-ref", "HEAD", "refs/heads/alias"]);
    kept(&made, &made.keep(&[]), &made.end(), "main", FORWARD);
    assert_eq!(made.git(&["symbolic-ref", "refs/heads/alias"]), "refs/heads/main");
    assert_eq!((made.w.commit(&made.repo, "main"), made.status()), (made.end(), String::new()));
}

#[test]
fn a_merge_a_cherry_pick_a_revert_a_rebase_or_a_bisect_in_progress_is_refused() {
    // A bisect git itself started.
    let made = finished();
    made.git(&["bisect", "start"]);
    unfit(&made, MIDWAY);
    made.git(&["bisect", "reset"]);
    // And each thing git keeps while one of the others goes on, as git leaves it.
    let start = made.w.commit(&made.repo, "main");
    for file in ["MERGE_HEAD", "CHERRY_PICK_HEAD", "REVERT_HEAD", "BISECT_LOG"] {
        fs::write(made.repo.join(".git").join(file), format!("{start}\n")).unwrap();
        unfit(&made, MIDWAY);
        fs::remove_file(made.repo.join(".git").join(file)).unwrap();
    }
    for folder in ["rebase-merge", "rebase-apply", "sequencer"] {
        fs::create_dir(made.repo.join(".git").join(folder)).unwrap();
        unfit(&made, MIDWAY);
        fs::remove_dir(made.repo.join(".git").join(folder)).unwrap();
    }
    kept(&made, &made.keep(&[]), &made.end(), "main", FORWARD);
}

#[test]
fn a_history_that_no_longer_holds_the_runs_start_is_refused() {
    // The start was rewritten.
    let made = finished();
    made.who();
    made.git(&["commit", "-q", "--amend", "-m", "the first commit, said again"]);
    unfit(&made, HISTORY);
    // The person stands on a branch with no commit yet.
    let made = finished();
    made.git(&["checkout", "-q", "--orphan", "anew"]);
    unfit(&made, HISTORY);
    // The person stands on a branch that never had the start.
    let made = finished();
    made.who();
    made.git(&["checkout", "-q", "--orphan", "apart"]);
    made.git(&["commit", "-q", "-m", "a history of its own"]);
    unfit(&made, HISTORY);
}

#[test]
fn a_git_too_old_to_merge_in_the_store_is_refused_only_when_a_merge_is_needed() {
    let old = "the branch has commits since run {run} started, and this git is too old to merge them with the work in \
               the store (it is from before 2.38)";
    for says in ["git version 2.37.4\n", "git version 2.9.5 (Apple Git-98)\n", "a git of no version\n"] {
        let made = finished();
        made.moved_on();
        let fake = made.w.fake_git();
        fs::write(made.w.at("fake").join("says"), says).unwrap();
        let before = made.held();
        let why = format!("{old}: {UNCHANGED}");
        made.refuses(&made.keep_in(&made.repo, &[], &fake), &before, &why);
        assert_eq!(
            printed(&made.keep_in(&made.repo, &["--ask"], &fake))["why"],
            json!(why.replace("{run}", &made.run))
        );
        // The same git, saying it is new enough, keeps it.
        fs::write(made.w.at("fake").join("says"), "git version 2.38.0\n").unwrap();
        let ran = made.keep_in(&made.repo, &[], &fake);
        kept(&made, &ran, &made.w.commit(&made.repo, "main"), "main", JOINED);
    }
    // Nothing is merged to move a branch forward: the oldest git does that.
    let made = finished();
    let fake = made.w.fake_git();
    fs::write(made.w.at("fake").join("says"), "git version 2.20.0\n").unwrap();
    kept(&made, &made.keep_in(&made.repo, &[], &fake), &made.end(), "main", FORWARD);
}

#[test]
fn a_commit_is_never_made_under_a_made_up_name() {
    let nobody = "git has no name and address for this repo's commits (user.name and user.email), and Jellymoon \
                  makes none up";
    for missing in ["user.name", "user.email"] {
        let made = finished();
        made.moved_on();
        made.git(&["config", "--unset", missing]);
        unfit(&made, nobody);
        // A name that is set to nothing is no name, and neither is one git would cut down to nothing: spaces,
        // and the marks git takes off both ends of a name.
        for nothing in ["", " ", " \t ", "<>", "< >", ",;:\"'"] {
            made.git(&["config", missing, nothing]);
            unfit(&made, nobody);
        }
    }
    // No commit is made to move a branch forward, so nobody is asked who they are.
    let made = finished();
    kept(&made, &made.keep(&[]), &made.end(), "main", FORWARD);
}

#[test]
fn a_locked_index_and_another_keep_at_work_are_refused() {
    let made = finished();
    fs::write(made.repo.join(".git").join("index.lock"), "").unwrap();
    let locked = "the checkout's index is locked (index.lock): another git is at work in it, or one was cut off and \
                  left the file: the work of run {run} was not kept";
    let before = made.held();
    made.refuses(&made.keep(&[]), &before, locked);
    assert_eq!(printed(&made.keep(&["--ask"]))["why"], json!(locked.replace("{run}", &made.run)));
    fs::remove_file(made.repo.join(".git").join("index.lock")).unwrap();
    // The keeps' lock, held by this test as another keep would hold it.
    let before = made.held();
    let lock = fs::File::create(made.w.jm("runs").join("keep.lock")).unwrap();
    lock.lock().unwrap();
    let wait = [("JELLYMOON_TEST_KEEP_WAIT_MS", "200".into())];
    let busy = format!("another keep is at work in this home: {UNCHANGED}");
    made.refuses(&made.keep_in(&made.repo, &[], &wait), &before, &busy);
    assert_eq!(printed(&made.keep_in(&made.repo, &["--ask"], &wait))["fits"], json!(false));
    drop(lock);
    // A lock that cannot be made is said as that, and not as another keep: here a folder has its name.
    fs::remove_file(made.w.jm("runs").join("keep.lock")).unwrap();
    fs::create_dir(made.w.jm("runs").join("keep.lock")).unwrap();
    unfit(&made, "the keeps' lock (runs/keep.lock) could not be made or taken in this home");
    fs::remove_dir(made.w.jm("runs").join("keep.lock")).unwrap();
    kept(&made, &made.keep(&[]), &made.end(), "main", FORWARD);
}

#[test]
fn a_config_that_names_what_cannot_be_switched_off_on_a_command_line_is_refused() {
    let config = "the repo's git config names a filter, a merge driver or a transport that cannot be switched off \
                  on git's command line: there are too many of them, or one has `=` in its name";
    // More drivers than a command line carries, and one whose name holds the sign a setting is split at.
    let many: String = (0..40_000).map(|n| format!("[merge \"d{n}\"]\n\tdriver = true\n")).collect();
    for named in [many.as_str(), "[filter \"a=b\"]\n\tclean = cat\n", "[merge \"a=b\"]\n\tdriver = true\n"] {
        let made = finished();
        let file = made.repo.join(".git").join("config");
        let was = fs::read_to_string(&file).unwrap();
        fs::write(&file, format!("{was}{named}")).unwrap();
        unfit(&made, config);
        // With the config as it was, the work is kept.
        fs::write(&file, was).unwrap();
        kept(&made, &made.keep(&[]), &made.end(), "main", FORWARD);
    }
}

#[test]
fn a_wish_with_no_work_to_keep_is_refused_in_the_words_of_a_look() {
    let w = world();
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let keep = |wish: &str| w.run(&["run", "keep", "--wish", wish, "--date", DAY, "--repo", repo.to_str().unwrap()]);
    assert_eq!(super::refused(&keep(&wish)), said(&format!("wish {wish} has had no run, so there is no work to keep")));
    assert_eq!(keep("m7").code, 1);
    // A run that failed, and one that changed nothing.
    for (script, why) in [
        ("exit-3", "run {run} did not finish: there is no work of it to keep"),
        ("nothing", "run {run} changed nothing: there is no work of it to keep"),
    ] {
        let w = world();
        w.play(script, 0);
        let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
        let (run, _) = ran(&w.start(&wish, &repo));
        let made = Made { w, repo, wish, run };
        let before = made.held();
        made.refuses(&made.keep(&[]), &before, why);
        made.refuses(&made.keep(&["--ask"]), &before, why);
    }
}

#[test]
fn a_command_line_that_is_not_a_keeps_is_refused_as_usage() {
    let made = finished();
    let before = made.held();
    for more in [&["--file", "1"][..], &["--force"], &["--ask=yes"], &["extra"]] {
        let ran = made.keep(more);
        assert_eq!((ran.code, ran.stdout.as_str()), (2, ""), "{more:?}: {}", ran.stderr);
    }
    // The words of a keep are a keep's alone.
    for other in ["show", "stop"] {
        let ran = made.w.run(&["run", other, "--wish", &made.wish, "--date", DAY, "--ask"]);
        assert_eq!(ran.code, 2, "{other}: {}", ran.stderr);
    }
    assert!(before == made.held());
}
