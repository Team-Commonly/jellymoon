//! An undo cut off after each of its steps, as a power cut would, and one whose checkout was moved to another
//! branch just before its step (`docs/development/keep.md`, "Undone"). The next `run undo` finishes what was
//! cut; and the step writes one ref, the branch the change was kept on by its whole name, wherever `HEAD` is.

use std::fs;

use serde_json::json;

use super::undone::{UNDONE, waits};
use super::{landed, printed, refused, said};

const CUT_OFF: &str = "an undo of the work of run {run} was cut off: `run undo` goes on with it";

#[test]
fn an_undo_cut_off_after_any_of_its_steps_is_finished_by_the_next_one() {
    // 1: the record says an undo is about to step. 2: the files and the index are back. 3: the branch is too.
    for step in ["1", "2", "3"] {
        let made = landed();
        let (start, end) = (made.record()["start"].clone(), made.end());
        let cut = made.undo_with(&[], &[("JELLYMOON_TEST_UNDO_CUT", step.into())]);
        assert_eq!((cut.code, cut.stdout.as_str(), cut.stderr.as_str()), (70, "", ""), "step {step}");
        let record = made.record();
        assert_eq!((&record["work"], &record["back"], &record["before"]), (&json!("kept"), &start, &start), "{record}");
        let (tip, files) = (made.w.commit(&made.repo, "main"), made.repo.join("b.txt").exists());
        assert_eq!((json!(tip), files), if step == "3" { (start.clone(), true) } else { (json!(end), step == "2") });

        // A keep says that an undo is to go on first, and changes nothing.
        let before = made.held();
        assert_eq!(refused(&made.keep(&[])), said(&CUT_OFF.replace("{run}", &made.run)));
        assert!(before == made.held(), "a refused keep changed something");
        // The next undo ends as one that was never cut.
        assert_eq!(printed(&made.undo())["why"], json!(UNDONE), "step {step}");
        assert_eq!((json!(made.w.commit(&made.repo, "main")), made.status()), (start, String::new()));
        assert_eq!(fs::read_to_string(made.repo.join("a.txt")).unwrap(), "a.txt as it was\n");
        assert!(made.repo.join("b.txt").exists() && !made.repo.join("n.txt").exists());
        waits(&made.record());
        assert_eq!(made.w.branches(&made.repo).len(), 1);
    }
}

#[test]
fn an_undo_that_goes_on_and_finds_the_branch_moved_is_refused_and_begins_no_third() {
    let made = landed();
    let cut = made.undo_with(&[], &[("JELLYMOON_TEST_UNDO_CUT", "1".into())]);
    assert_eq!(cut.code, 70, "{}", cut.stderr);
    fs::write(made.repo.join("mine.txt"), "the person's own\n").unwrap();
    made.git(&["add", "mine.txt"]);
    made.git(&["commit", "-q", "-m", "the person's own commit"]);
    let tip = made.w.commit(&made.repo, "main");
    let stays = made.undo();
    assert!(refused(&stays).contains("the branch has a commit since this change was kept"), "{}", stays.stderr);
    let record = made.record();
    assert_eq!((&record["work"], record.get("back"), made.w.commit(&made.repo, "main")), (&json!("kept"), None, tip));
    assert_eq!(made.status(), "");
}

#[test]
fn the_step_moves_no_ref_but_the_branch_it_names_when_head_is_switched_away_just_before_it() {
    let made = landed();
    let (start, end) = (made.record()["start"].as_str().unwrap().to_string(), made.end());
    // The made-up git stands first on the undo's `PATH`, and just before the command that moves the files it
    // makes a branch at the kept commit and stands on it: every check has passed by then.
    let (vars, fake) = (made.w.fake_git(), made.w.at("fake"));
    fs::write(fake.join("on"), "gc.auto=0").unwrap();
    fs::write(fake.join("returns"), "").unwrap();
    let first = ["-C", made.repo.to_str().unwrap(), "checkout", "-q", "-b", "other"];
    fs::write(fake.join("first"), first.join("\n")).unwrap();
    let undone = made.undo_with(&[], &vars);
    assert!(fake.join("held").exists(), "the made-up git was never at the step: {}", undone.stderr);
    assert_eq!(printed(&undone)["why"], json!(UNDONE));
    // The branch the folder stands on by then is not moved, and no other ref is: only `main` is.
    let refs = made.git(&["for-each-ref", "--format=%(refname) %(objectname)"]);
    let want = [
        format!("refs/heads/jellymoon/run-{} {end}", made.run),
        format!("refs/heads/main {start}"),
        format!("refs/heads/other {end}"),
    ];
    assert_eq!(refs.lines().collect::<Vec<_>>(), want);
    assert_eq!(made.git(&["symbolic-ref", "HEAD"]), "refs/heads/other");
    waits(&made.record());
}
