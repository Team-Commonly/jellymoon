//! A keep cut off after each of its steps, as a power cut would cut it (`docs/development/keep.md`, "A keep that
//! is cut off"): the next `run keep` finishes it or begins it again, and the work never lands twice. A test's
//! build of `jellymoon` ends itself after the step `JELLYMOON_TEST_KEEP_CUT` numbers; a release never does.

use std::fs;

use serde_json::{Value, json};

use super::{BEFORE, DAY, FORWARD, JOINED, Made, UNCHANGED, ended, finished, kept, printed, worked};

const FINISHED: &str = "run {run} was kept: a keep that was cut off had landed its work, and this one finished it";

/// A keep that ends itself after step `step`.
fn cut(made: &Made, step: u8) {
    let ran = made.keep_in(&made.repo, &[], &[("JELLYMOON_TEST_KEEP_CUT", step.to_string().into())]);
    assert_eq!((ran.code, ran.stdout.as_str(), ran.stderr.as_str()), (70, "", ""), "step {step}");
}

/// The record's three words of a keep.
fn words(made: &Made) -> (Value, Value, Value) {
    let record = made.record();
    (record["work"].clone(), record["kept"].clone(), record["onto"].clone())
}

/// The commits of `main`, newest first.
fn history(made: &Made) -> Vec<String> {
    made.git(&["rev-list", "main"]).lines().map(str::to_string).collect()
}

#[test]
fn a_keep_cut_off_after_any_step_is_finished_or_begun_again_by_the_next_and_never_lands_twice() {
    for (step, joined) in [(1, false), (2, false), (3, false), (4, false), (1, true), (2, true), (3, true), (4, true)] {
        let made = finished();
        let (end, start) = (made.end(), made.w.commit(&made.repo, "main"));
        let theirs = if joined { made.moved_on() } else { start.clone() };
        let (person, branch) = (made.w.persons(&made.repo), vec![format!("jellymoon/run-{}", made.run)]);
        cut(&made, step);
        // What the cut left: the record's words say how far the keep came, and the checkout agrees with them.
        let (work, landing, onto) = words(&made);
        let tip = made.w.commit(&made.repo, "main");
        let main = json!("refs/heads/main");
        match step {
            1 => assert_eq!((work, landing, onto, &tip), (json!(null), json!(null), json!(null), &theirs)),
            2 => assert_eq!((work, onto, &tip), (json!(null), main, &theirs)),
            3 => assert_eq!((work, landing, onto), (json!(null), json!(tip), main)),
            _ => assert_eq!((work, landing, onto), (json!("kept"), json!(tip), main)),
        }
        assert_eq!(made.w.branches(&made.repo), branch, "the run's branch goes last");
        assert_eq!(step < 3, person == made.w.persons(&made.repo), "step {step}");
        // Asked, the next keep says what it would do, and does none of it.
        let before = made.held();
        let asked = printed(&made.keep(&["--ask"]));
        assert_eq!(asked["fits"], json!(step < 4), "{asked}");
        assert!(before == made.held());
        // The next keep ends it.
        let ran = made.keep(&[]);
        let tip = made.w.commit(&made.repo, "main");
        let why = match (step, joined) {
            (4, _) => BEFORE,
            (3, _) => FINISHED,
            (_, true) => JOINED,
            (_, false) => FORWARD,
        };
        kept(&made, &ran, &tip, "main", why);
        // Once: one commit on top of the person's last, the run's own when nothing was theirs.
        let want = if joined { vec![tip.clone(), theirs, start.clone()] } else { vec![end.clone(), start.clone()] };
        assert_eq!(history(&made), want, "step {step} joined {joined}");
        assert_eq!(words(&made), (json!("kept"), json!(tip), json!("refs/heads/main")));
        assert!(made.w.branches(&made.repo).is_empty(), "step {step}");
        assert_eq!(made.status(), "");
        assert_eq!(
            made.git(&["diff", "--name-status", &start, &end]),
            made.git(&["diff", "--name-status", &format!("{tip}~1"), &tip])
        );
        // And again changes nothing.
        let before = made.held();
        kept(&made, &made.keep(&[]), &tip, "main", BEFORE);
        assert!(before == made.held());
    }
}

#[test]
fn a_keep_begun_again_goes_by_the_checkout_as_it_is_then() {
    // The person committed between the cut and the next keep: what was meant to land no longer fits as it was.
    let made = finished();
    let start = made.w.commit(&made.repo, "main");
    cut(&made, 2);
    let theirs = made.moved_on();
    let ran = made.keep(&[]);
    let tip = made.w.commit(&made.repo, "main");
    kept(&made, &ran, &tip, "main", JOINED);
    assert_eq!(history(&made), vec![tip, theirs, start]);
    // The person moved to another branch: the work lands where they stand now, and the record says so.
    let made = finished();
    cut(&made, 2);
    made.git(&["checkout", "-q", "-b", "other"]);
    kept(&made, &made.keep(&[]), &made.end(), "other", FORWARD);
    assert_eq!(made.record()["onto"], json!("refs/heads/other"));
    assert_eq!(made.w.commit(&made.repo, "main"), made.git(&["rev-parse", "other~1"]));
    // The branch the cut keep meant is gone: it is begun again on the one stood on.
    let made = finished();
    made.git(&["checkout", "-q", "-b", "other"]);
    cut(&made, 2);
    made.git(&["checkout", "-q", "main"]);
    made.git(&["branch", "-q", "-D", "other"]);
    kept(&made, &made.keep(&[]), &made.end(), "main", FORWARD);
}

#[test]
fn a_keep_that_landed_is_finished_whatever_became_of_the_runs_branch_and_wherever_the_person_stands() {
    let made = finished();
    let end = made.end();
    cut(&made, 3);
    made.git(&["update-ref", "-d", &made.branch()]);
    made.who();
    fs::write(made.repo.join("c.txt"), "the person went on\n").unwrap();
    made.git(&["commit", "-q", "-am", "on top of the kept work"]);
    made.git(&["checkout", "-q", "-b", "elsewhere"]);
    let before = made.w.persons(&made.repo);
    kept(&made, &made.keep(&[]), &end, "main", FINISHED);
    assert!(before == made.w.persons(&made.repo));
    assert_eq!(made.record()["work"], json!("kept"));
}

#[test]
fn a_runs_branch_a_checkout_came_to_stand_on_is_left_where_it_is() {
    let made = finished();
    let end = made.end();
    cut(&made, 4);
    let looked = made.w.at("work").join("looked");
    made.git(&["worktree", "add", "-q", looked.to_str().unwrap(), &format!("jellymoon/run-{}", made.run)]);
    kept(&made, &made.keep(&[]), &end, "main", BEFORE);
    assert_eq!(made.w.commit(&made.repo, &made.branch()), end);
    assert_eq!(made.w.git(&looked, &["status", "--porcelain"]), "");
}

#[test]
fn a_checkout_git_had_written_whole_before_the_cut_is_finished_by_the_next_keep() {
    // git had written the index and the files, and had not yet moved the branch.
    let made = finished();
    let end = made.end();
    cut(&made, 2);
    made.git(&["read-tree", "-m", "-u", "HEAD", &end]);
    assert_ne!(made.status(), "");
    kept(&made, &made.keep(&[]), &end, "main", FORWARD);
    assert_eq!(made.status(), "");
    assert_eq!(made.files(), worked());
}

#[test]
fn files_a_cut_checkout_left_half_written_are_in_the_way_until_the_person_clears_them() {
    // git itself was cut while it wrote the checkout: a file of the work is there, and git's index does not know it.
    let made = finished();
    cut(&made, 2);
    fs::write(made.repo.join("n.txt"), "new from the run\n").unwrap();
    let before = made.held();
    let why =
        format!("an unsaved change, or a file that is not in the branch, is where the work would go: {UNCHANGED}");
    made.refuses(&made.keep(&[]), &before, &why);
    fs::remove_file(made.repo.join("n.txt")).unwrap();
    // A lock a cut keep left on Jellymoon's own index file is nobody's, and is no reason to stop.
    fs::write(made.w.jm("runs").join(&made.run).join("keep.index.lock"), "").unwrap();
    kept(&made, &made.keep(&[]), &made.end(), "main", FORWARD);
    assert!(!made.w.jm("runs").join(&made.run).join("keep.index.lock").exists());
    assert_eq!(made.files(), worked());
}

#[test]
fn two_keeps_at_once_land_the_work_once() {
    for joined in [false, true] {
        let made = finished();
        let start = made.w.commit(&made.repo, "main");
        let theirs = if joined { Some(made.moved_on()) } else { None };
        let asked = ["run", "keep", "--wish", &made.wish, "--date", DAY, "--repo", made.repo.to_str().unwrap()];
        let going: Vec<_> = (0..4).map(|_| made.w.going(&[], &asked)).collect();
        let said: Vec<Value> = going.into_iter().map(|going| printed(&ended(going))).collect();
        let tip = made.w.commit(&made.repo, "main");
        let did = said.iter().filter(|said| said["why"] != json!(BEFORE.replace("{run}", &made.run))).count();
        assert_eq!(did, 1, "{said:?}");
        assert!(said.iter().all(|said| said["kept"] == json!(tip)), "{said:?}");
        let want = match theirs {
            Some(theirs) => vec![tip.clone(), theirs, start],
            None => vec![tip.clone(), start],
        };
        assert_eq!(history(&made), want);
        assert!(made.w.branches(&made.repo).is_empty());
    }
}
