//! `jellymoon run undo` (`docs/development/keep.md`, "Undone"): a change that was kept by itself is taken out of
//! the person's folder again, and its work waits. Each reason a change stays is one fixed sentence, with nothing
//! changed.

use std::fs;
use std::time::{Duration, SystemTime};

use serde_json::{Value, json};

use super::{DAY, Made, finished, itself, landed, played, printed, wished};

pub const UNDONE: &str = "the change was undone: the branch names the commit it named before, and the work waits";
const WOULD: &str = "an undo would put the branch back at the commit it named before";
const NOTHING: &str = "nothing of this wish is kept, so there is no change to undo";
const TAPPED: &str = "this change was kept by a tap, or was kept by itself before a later day's change was, and such \
                      a change is not undone here: nothing was changed";
const COMMITTED: &str = "the branch has a commit since this change was kept, so it was not undone, and nothing was \
                         changed";
const LATER: &str = "a later change was kept on top of this one, and the newest kept change is undone first: nothing \
                     was changed";
const EDITS: &str = "a file of this change has unsaved or staged edits, so it was not undone, and nothing was changed";
const WAY: &str = "a file is in the way where one this change removed would come back, so it was not undone, and \
                   nothing was changed";
const AWAY: &str = "the folder stands on another branch than the change was kept on, or on no branch, so it was not \
                    undone, and nothing was changed";
const MIDWAY: &str = "a merge, a cherry-pick, a revert, a rebase or a bisect is going on in the folder, so the \
                      change was not undone, and nothing was changed";
const HELD: &str = "another name in the repo holds this change (a branch, a tag or a remote's branch), and a change \
                    that was already sent elsewhere is not undone here: nothing was changed";
const FORGED: &str = "the record of this change is not one Jellymoon wrote, so it was not undone, and nothing was \
                      changed";
const BUSY: &str = "a keep, a toss or the start of a follow-up's run is at work in this home: the change was not \
                    undone, and nothing was changed";

/// Fails unless an undo is refused in these words with nothing changed, and `--ask` says the same and no.
pub fn stays(made: &Made, why: &str) {
    let before = made.held();
    let asked = printed(&made.undo_with(&["--ask"], &[]));
    assert_eq!(asked, json!({"run": made.run, "wish": made.wish, "why": why, "fits": false}));
    made.refuses(&made.undo(), &before, why);
}

/// Fails unless the record says neither kept nor tossed, and holds nothing of a keep or an undo.
pub fn waits(record: &Value) {
    for word in ["work", "kept", "onto", "before", "back"] {
        assert_eq!(record.get(word), None, "{record}");
    }
}

#[test]
fn an_undo_puts_the_folder_and_the_index_back_and_the_work_waits_to_be_kept_or_tossed() {
    for then in ["keep", "toss"] {
        let w = itself();
        let repo = w.repo("alpha");
        let start = w.commit(&repo, "main");
        let was = (w.git(&repo, &["ls-files", "-s"]), w.persons(&repo).0);
        let (wish, said) = wished(&w, &repo, DAY, "edits");
        let made = Made { w, repo, wish, run: said["run"].as_str().unwrap().to_string() };
        assert_ne!(made.folder(), was);
        // The person goes on meanwhile, in files that are not the change's.
        fs::write(made.repo.join("c.txt"), "half an edit\n").unwrap();
        fs::write(made.repo.join("note.txt"), "git has never seen this\n").unwrap();
        fs::write(made.repo.join("staged.txt"), "staged\n").unwrap();
        made.git(&["add", "staged.txt"]);
        let before = made.held();
        let asked = printed(&made.undo_with(&["--ask"], &[]));
        assert_eq!(asked, json!({"run": made.run, "wish": made.wish, "why": WOULD, "fits": true}));
        assert!(before == made.held(), "a question changed something");

        let undone = made.undo();
        assert_eq!(printed(&undone), json!({"run": made.run, "wish": made.wish, "why": UNDONE}));
        made.w.never_says_the_agent(&undone);
        // The branch names the commit it named before, with no commit made, and the run's branch is there.
        assert_eq!(
            (made.w.commit(&made.repo, "main"), made.git(&["symbolic-ref", "HEAD"])),
            (start, "refs/heads/main".into())
        );
        assert_eq!(made.w.branches(&made.repo), [format!("jellymoon/run-{}", made.run)]);
        assert_eq!(made.status(), " M c.txt\nA  staged.txt\n?? note.txt");
        // But for what the person did meanwhile, the files and the index are byte for byte what they were.
        assert_eq!(fs::read_to_string(made.repo.join("c.txt")).unwrap(), "half an edit\n");
        fs::write(made.repo.join("c.txt"), "c.txt as it was\n").unwrap();
        fs::remove_file(made.repo.join("note.txt")).unwrap();
        made.git(&["rm", "-q", "--cached", "staged.txt"]);
        fs::remove_file(made.repo.join("staged.txt")).unwrap();
        assert_eq!(made.folder(), was);
        waits(&made.record());

        // The wish is done still, and its work waits as it did before F92: kept by a tap, or tossed.
        let show = ["run", "show", "--wish", &made.wish, "--date", DAY, "--repo", made.repo.to_str().unwrap()];
        assert_eq!(printed(&made.w.run(&show))["work"], json!("waits"));
        if then == "keep" {
            let end = made.end();
            assert_eq!(printed(&made.keep(&[]))["kept"], json!(end));
            assert_eq!(made.w.commit(&made.repo, "main"), end);
            // A change kept by a tap has no branch of its run left, and is not undone here.
            assert!(made.w.branches(&made.repo).is_empty());
            assert_eq!(made.record().get("before"), None);
            stays(&made, TAPPED);
        } else {
            assert_eq!(printed(&made.toss())["tossed"], json!(true));
            assert!(made.w.branches(&made.repo).is_empty());
            stays(&made, NOTHING);
        }
    }
}

#[test]
fn an_undo_is_refused_with_one_sentence_and_nothing_changed() {
    // Nothing of the wish is kept; and a change kept by a tap.
    let made = finished();
    stays(&made, NOTHING);
    made.keep(&[]);
    stays(&made, TAPPED);

    // The person committed on top of the change.
    let made = landed();
    fs::write(made.repo.join("mine.txt"), "the person's own\n").unwrap();
    made.git(&["add", "mine.txt"]);
    made.git(&["commit", "-q", "-m", "the person's own commit"]);
    stays(&made, COMMITTED);

    // A file of the change is edited and not saved to git, or staged.
    let made = landed();
    fs::write(made.repo.join("a.txt"), "half an edit\n").unwrap();
    stays(&made, EDITS);
    made.git(&["add", "a.txt"]);
    stays(&made, EDITS);

    // Something is where a file the change removed would come back: a file git has never seen, or one it ignores.
    let made = landed();
    fs::write(made.repo.join("b.txt"), "the person's own\n").unwrap();
    stays(&made, WAY);
    fs::create_dir_all(made.repo.join(".git").join("info")).unwrap();
    fs::write(made.repo.join(".git").join("info").join("exclude"), "b.txt\n").unwrap();
    assert_eq!(made.status(), "!! b.txt");
    stays(&made, WAY);
    assert_eq!(fs::read_to_string(made.repo.join("b.txt")).unwrap(), "the person's own\n");

    // The folder stands on another branch, which is not moved, or on none.
    let made = landed();
    made.git(&["checkout", "-q", "-b", "other"]);
    stays(&made, AWAY);
    assert_eq!(made.w.commit(&made.repo, "other"), made.end());
    made.git(&["checkout", "-q", "--detach"]);
    stays(&made, AWAY);

    // Something of git's is going on in the folder.
    let made = landed();
    fs::write(made.repo.join(".git").join("MERGE_HEAD"), format!("{}\n", made.end())).unwrap();
    stays(&made, MIDWAY);
    fs::remove_file(made.repo.join(".git").join("MERGE_HEAD")).unwrap();
    fs::create_dir(made.repo.join(".git").join("rebase-merge")).unwrap();
    stays(&made, MIDWAY);
}

#[test]
fn a_change_that_another_name_holds_is_not_undone_and_neither_is_one_while_the_lock_is_busy() {
    let made = landed();
    let end = made.end();
    for name in ["refs/heads/sent", "refs/tags/v1", "refs/remotes/origin/main"] {
        made.git(&["update-ref", name, &end]);
        stays(&made, HELD);
        made.git(&["update-ref", "-d", name]);
    }
    let lock = fs::File::create(made.w.jm("runs").join("keep.lock")).unwrap();
    lock.lock().unwrap();
    stays(&made, BUSY);
    drop(lock);
    assert_eq!(printed(&made.undo())["why"], json!(UNDONE));
}

#[test]
fn two_kept_changes_are_undone_newest_first() {
    let first = landed();
    let (start, one) = (first.record()["start"].as_str().unwrap().to_string(), first.end());
    // A second wish of the day: its run starts at the first change, edits its file again and makes one more.
    let (wish, said) = wished(&first.w, &first.repo, DAY, "follow");
    assert_eq!(said["kept_itself"], json!(true), "{said}");
    let run = said["run"].as_str().unwrap().to_string();
    let record = first.w.record(&run);
    assert_eq!(
        (&record["before"], first.w.commit(&first.repo, "main")),
        (&json!(one), record["end"].as_str().unwrap().into())
    );
    stays(&first, LATER);

    let asked = ["run", "undo", "--wish", &wish, "--date", DAY, "--repo", first.repo.to_str().unwrap()];
    assert_eq!(printed(&first.w.run(&asked)), json!({"run": run, "wish": wish, "why": UNDONE}));
    assert_eq!(first.w.commit(&first.repo, "main"), one);
    assert!(!first.repo.join("m.txt").exists());
    assert_eq!(fs::read_to_string(first.repo.join("a.txt")).unwrap(), "edited by the run\n");
    assert_eq!(printed(&first.undo())["why"], json!(UNDONE));
    assert_eq!((first.w.commit(&first.repo, "main"), first.status()), (start, String::new()));
    assert_eq!(first.w.branches(&first.repo).len(), 2);
}

#[test]
fn a_change_kept_by_itself_on_a_later_day_takes_the_branch_and_the_undo_of_an_earlier_days() {
    const EARLIER: &str = "2026-03-09";
    let w = itself();
    let repo = w.repo("alpha");
    let (old, said) = wished(&w, &repo, EARLIER, "edits");
    let old_run = said["run"].as_str().unwrap().to_string();
    assert_eq!((&said["kept_itself"], w.branches(&repo).len()), (&json!(true), 1), "{said}");
    assert!(w.record(&old_run).get("before").is_some());

    let (wish, said) = wished(&w, &repo, DAY, "follow");
    let run = said["run"].as_str().unwrap().to_string();
    assert_eq!(said["kept_itself"], json!(true), "{said}");
    // The earlier day's change is kept still, with no branch of its run and no commit to go back to.
    assert_eq!(w.branches(&repo), [format!("jellymoon/run-{run}")]);
    let record = w.record(&old_run);
    assert_eq!((&record["work"], record.get("before")), (&json!("kept"), None), "{record}");
    let made = Made { w, repo, wish, run };
    let earlier = ["run", "undo", "--wish", &old, "--date", EARLIER, "--repo", made.repo.to_str().unwrap()];
    let before = made.held();
    let refused = made.w.run(&earlier);
    assert_eq!(super::refused(&refused), super::said(TAPPED));
    assert!(before == made.held(), "a refused undo changed something");
    // The newest is undone as ever, and the earlier day's is not undone after it either.
    assert_eq!(printed(&made.undo())["why"], json!(UNDONE));
    assert_eq!(super::refused(&made.w.run(&earlier)), super::said(TAPPED));
}

#[test]
fn an_undo_spares_what_git_ignores_below_a_folder_that_becomes_a_file_again() {
    // The change made a folder of the file `b.txt`, with a file in it.
    let (made, said) = played("folder");
    assert_eq!(said["kept_itself"], json!(true), "{said}");
    assert!(made.repo.join("b.txt").join("x").is_file());
    fs::create_dir_all(made.repo.join(".git").join("info")).unwrap();
    fs::write(made.repo.join(".git").join("info").join("exclude"), "*.log\nlogs/\n").unwrap();
    // The person keeps a file git ignores in that folder, or a folder of them: git would remove both unasked.
    for mine in ["b.txt/secret.log", "b.txt/logs/a"] {
        let file = made.repo.join(mine);
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(&file, "the person's own\n").unwrap();
        assert!(made.status().starts_with("!! b.txt/"), "{}", made.status());
        stays(&made, WAY);
        assert_eq!(fs::read_to_string(&file).unwrap(), "the person's own\n");
        fs::remove_file(&file).unwrap();
    }
    // An empty folder of the person's is theirs too. With nothing below but what the change made, it is undone.
    stays(&made, WAY);
    fs::remove_dir(made.repo.join("b.txt").join("logs")).unwrap();
    assert_eq!(printed(&made.undo())["why"], json!(UNDONE));
    assert_eq!(fs::read_to_string(made.repo.join("b.txt")).unwrap(), "b.txt as it was\n");
    assert_eq!(made.status(), "");
}

#[test]
fn an_undo_of_a_record_whose_commits_are_not_a_keep_s_is_refused() {
    let first = landed();
    let real = first.record()["before"].clone();
    // A commit with no parent, and a tree: neither is the commit before the kept one.
    let tree = first.git(&["rev-parse", "main^{tree}"]);
    let root = first.git(&["commit-tree", "-m", "a root of its own", &tree]);
    for forged in [root, tree] {
        first.forge(&json!(forged));
        stays(&first, FORGED);
        assert_eq!(first.stood()[0]["stands"], json!("elsewhere"));
    }
    first.forge(&real);
    assert_eq!(first.stood()[0]["stands"], json!("here"));
    // The commit before the commit before: a second change is kept on the first, and its record is told so.
    let (wish, said) = wished(&first.w, &first.repo, DAY, "follow");
    let second = Made { w: first.w, repo: first.repo, wish, run: said["run"].as_str().unwrap().to_string() };
    second.forge(&real);
    stays(&second, FORGED);
}

#[test]
fn an_undo_is_not_stopped_by_a_file_that_was_only_touched_or_by_a_stash() {
    // A file of the change holds what it held, at another time.
    let made = landed();
    let touched = fs::File::options().write(true).open(made.repo.join("a.txt")).unwrap();
    touched.set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000_000)).unwrap();
    drop(touched);
    assert_eq!(printed(&made.undo())["why"], json!(UNDONE));
    assert_eq!(made.status(), "");

    // The person stashed an edit of their own after the keep: the stash stands on the kept commit, and stays.
    let made = landed();
    fs::write(made.repo.join("c.txt"), "half an edit\n").unwrap();
    made.git(&["stash", "-q"]);
    let stash = made.git(&["rev-parse", "refs/stash"]);
    assert_eq!(printed(&made.undo())["why"], json!(UNDONE));
    assert_eq!((made.git(&["rev-parse", "refs/stash"]), made.status()), (stash, String::new()));
}
