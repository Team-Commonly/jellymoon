//! A toss cut off after each of its steps, as a power cut would cut it (`docs/development/toss.md`, "A toss that
//! is cut off"), a toss whose command was killed while it held the home, and a toss and a keep of one run in
//! every order. A test's build of `jellymoon` ends itself after the step `JELLYMOON_TEST_TOSS_CUT` numbers, or
//! `JELLYMOON_TEST_KEEP_CUT` for a keep; a release never does.

use std::ffi::OsString;
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use super::unfinished::{CUT_OFF, NO_COPY, REMOVED, UNFINISHED, unfinished};
use super::{BEFORE, DAY, DELETED, Made, WORK, ended, finished, printed, tossed};

const TOSS: &str = "JELLYMOON_TEST_TOSS_CUT";
const KEEP: &str = "JELLYMOON_TEST_KEEP_CUT";
const GONE: &str = "its branch was not in the repo, and the wish is let go";
const WAS_TOSSED: &str = "the work of run {run} was tossed, so nothing of it was kept";
const WAS_KEPT: &str = "the work of run {run} was kept, so it is not tossed: nothing was changed";

/// A toss, or a keep, that ends itself after step `step`.
fn cut(made: &Made, of: &'static str, step: u8) {
    let vars = [(of, OsString::from(step.to_string()))];
    let ran = if of == KEEP { made.keep_in(&made.repo, &[], &vars) } else { made.toss_in(&made.repo, &vars) };
    assert_eq!((ran.code, ran.stdout.as_str(), ran.stderr.as_str()), (70, "", ""), "{of} {step}");
}

#[test]
fn a_toss_of_work_cut_off_after_any_step_is_finished_by_the_next() {
    // A run that finished has no kept copy, so no third step.
    for step in [1, 2, 4] {
        let made = finished();
        let (end, person, others) = (json!(made.end()), made.w.persons(&made.repo), made.others());
        cut(&made, TOSS, step);
        // What the cut left: nothing, then the record's word, then the branch gone with the wish not yet told.
        let left = (made.record()["work"].clone(), made.w.branches(&made.repo).len());
        let work = if step == 1 { Value::Null } else { json!("tossed") };
        assert_eq!(left, (work, usize::from(step < 4)), "step {step}");
        assert_eq!(made.w.kept(&made.wish)["state"], "done", "step {step}");
        let why = match step {
            1 => format!("{WORK}: {DELETED}"),
            2 => format!("{CUT_OFF}: {DELETED}"),
            _ => format!("{CUT_OFF}: {GONE}"),
        };
        tossed(&made, &made.toss(), true, end.clone(), &why);
        made.is_tossed(&end, 1);
        assert!(made.w.branches(&made.repo).is_empty(), "step {step}");
        assert!(person == made.w.persons(&made.repo) && others == made.others(), "step {step}");
        tossed(&made, &made.toss(), false, end, BEFORE);
    }
}

#[test]
fn a_toss_of_a_run_that_did_not_finish_cut_off_after_any_step_is_finished_by_the_next() {
    for step in 1..=4 {
        let made = unfinished(false);
        let (start, wish) = (json!(made.w.commit(&made.repo, "main")), made.w.kept(&made.wish));
        cut(&made, TOSS, step);
        // The record's word comes first, then the copy goes, then the branch, and the wish is told last.
        let left = (made.record()["work"].is_string(), made.w.copies().len(), made.w.branches(&made.repo).len());
        let want = match step {
            1 => (false, 1, 1),
            2 => (true, 1, 1),
            3 => (true, 0, 1),
            _ => (true, 0, 0),
        };
        assert_eq!((left, made.w.kept(&made.wish)), (want, wish), "step {step}");
        let why = match step {
            1 => format!("{UNFINISHED}: {REMOVED}{DELETED}"),
            2 => format!("{CUT_OFF}: {REMOVED}{DELETED}"),
            3 => format!("{CUT_OFF}: {NO_COPY}{DELETED}"),
            _ => format!("{CUT_OFF}: {NO_COPY}{GONE}"),
        };
        tossed(&made, &made.toss(), true, start.clone(), &why);
        made.is_tossed(&start, 0);
        assert_eq!((made.w.copies().len(), made.w.branches(&made.repo).len()), (0, 0), "step {step}");
        tossed(&made, &made.toss(), false, start, BEFORE);
    }
}

#[test]
fn a_keep_cut_off_before_it_landed_is_tossed_and_one_that_landed_is_kept() {
    for (step, joined) in [(1, false), (2, false), (2, true)] {
        let made = finished();
        let end = json!(made.end());
        let theirs = if joined { made.moved_on() } else { made.w.commit(&made.repo, "main") };
        let person = made.w.persons(&made.repo);
        cut(&made, KEEP, step);
        tossed(&made, &made.toss(), true, end.clone(), &format!("{WORK}: {DELETED}"));
        // Nothing a keep meant is left in the record, and nothing of the work is in the checkout.
        made.is_tossed(&end, 1);
        assert_eq!((made.w.commit(&made.repo, "main"), made.w.branches(&made.repo).len()), (theirs, 0));
        assert!(person == made.w.persons(&made.repo), "step {step}");
        let before = made.held();
        made.refuses(&made.keep(&[]), &before, WAS_TOSSED);
    }
    // The work is on the person's branch and the record does not say kept yet: it is kept, and a keep says so.
    let made = finished();
    cut(&made, KEEP, 3);
    let before = made.held();
    let landed = "a keep of run {run} that was cut off had landed its work, so it is kept and not tossed: `run keep` \
                  finishes that keep, and nothing was changed";
    made.refuses(&made.toss(), &before, landed);
    assert_eq!(made.w.kept(&made.wish)["state"], "done");
    for finish in [false, true] {
        let made = finished();
        cut(&made, KEEP, 4);
        if finish {
            printed(&made.keep(&[]));
        }
        let before = made.held();
        made.refuses(&made.toss(), &before, WAS_KEPT);
        assert_eq!(
            (made.w.commit(&made.repo, "main"), &made.w.kept(&made.wish)["state"]),
            (made.end(), &json!("done"))
        );
    }
}

#[test]
fn a_toss_that_wrote_its_word_is_not_undone_by_a_keep() {
    for step in [2, 4] {
        let made = finished();
        let (end, main) = (json!(made.end()), made.w.commit(&made.repo, "main"));
        cut(&made, TOSS, step);
        let before = made.held();
        made.refuses(&made.keep(&[]), &before, WAS_TOSSED);
        made.refuses(&made.keep(&["--ask"]), &before, WAS_TOSSED);
        tossed(&made, &made.toss(), true, end, &format!("{CUT_OFF}: {}", if step == 2 { DELETED } else { GONE }));
        assert_eq!(made.w.commit(&made.repo, "main"), main);
    }
    // A toss cut off before its word wrote nothing: the work waits, and is kept.
    let made = finished();
    cut(&made, TOSS, 1);
    printed(&made.keep(&[]));
    assert_eq!((made.w.commit(&made.repo, "main"), &made.record()["work"]), (made.end(), &json!("kept")));
}

#[test]
fn a_toss_whose_command_was_killed_while_it_held_the_home_is_finished_by_the_next() {
    let made = finished();
    let end = json!(made.end());
    let asked = ["run", "toss", "--wish", &made.wish, "--date", DAY, "--repo", made.repo.to_str().unwrap()];
    // A test's build waits after each writing of a record, with the keeps' lock held.
    let mut going = made.w.going(&[("JELLYMOON_TEST_RECORD_WAIT_MS", "60000".into())], &asked);
    let from = Instant::now();
    while !std::fs::read_to_string(made.w.jm("runs").join(&made.run).join("run.json")).unwrap().contains("tossed") {
        assert!(from.elapsed() < Duration::from_secs(30), "the toss never wrote its word");
        thread::sleep(Duration::from_millis(5));
    }
    going.kill().unwrap();
    going.wait().unwrap();
    assert_eq!(made.w.branches(&made.repo).len(), 1, "it was killed after the record and before the branch");
    // The lock's file is still there, and is nobody's: the system dropped the lock with its holder.
    assert!(made.w.jm("runs").join("keep.lock").exists());
    tossed(&made, &made.toss(), true, end.clone(), &format!("{CUT_OFF}: {DELETED}"));
    made.is_tossed(&end, 1);
}

#[test]
fn tosses_at_once_toss_once_and_a_keep_and_a_toss_at_once_end_as_one_of_them() {
    let made = finished();
    let toss = |made: &Made| {
        let asked = ["run", "toss", "--wish", &made.wish, "--date", DAY, "--repo", made.repo.to_str().unwrap()];
        made.w.going(&[], &asked)
    };
    let going: Vec<_> = (0..4).map(|_| toss(&made)).collect();
    let said: Vec<Value> = going.into_iter().map(|going| printed(&ended(going))).collect();
    assert_eq!(said.iter().filter(|said| said["tossed"] == true).count(), 1, "{said:?}");
    assert!(said.iter().all(|said| said["commit"] == json!(made.end())), "{said:?}");
    made.is_tossed(&json!(made.end()), 1);
    for _ in 0..3 {
        let made = finished();
        let (main, end) = (made.w.commit(&made.repo, "main"), made.end());
        let keep = ["run", "keep", "--wish", &made.wish, "--date", DAY, "--repo", made.repo.to_str().unwrap()];
        let (keeping, tossing) = (made.w.going(&[], &keep), toss(&made));
        let (kept, tossed) = (ended(keeping), ended(tossing));
        let was_kept = made.record()["work"] == "kept";
        assert_eq!(
            (kept.code, tossed.code),
            if was_kept { (0, 1) } else { (1, 0) },
            "{} {}",
            kept.stderr,
            tossed.stderr
        );
        assert_eq!(made.w.commit(&made.repo, "main"), if was_kept { end } else { main });
        assert!(made.w.branches(&made.repo).is_empty());
    }
}
