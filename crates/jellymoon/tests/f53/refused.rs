//! Every follow-up that is not made, and every one that is not started: nothing is kept, no copy is made, no
//! harness is started, and nothing in the home, the repo or the harness's store is changed. A followed wish
//! that is not there, has had no run, or whose run goes, asks, did not finish, changed nothing or was tossed;
//! a project named beside the wish to follow; a day that has its eight; made wishes that name a wish they
//! cannot follow; and at the run, work that was tossed by then, a branch that was moved, another repo, and a
//! keep or a toss at work.

use std::fs;

use serde_json::{Value, json};

use super::{
    AS_THE_RUN_LEFT_IT, DAY, MORE, Made, ended, finished, follow, followed, given, printed, refused, said, wait_for,
    world,
};
use crate::support::map_key;

const TOSSED: &str = "the work of run {run} was tossed, so it cannot be followed";

/// One run of the made-up agent's script for the wish `m1`, whatever its end.
fn played(script: &str, number: usize) -> Made {
    let w = world();
    w.play(script, number);
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let run = printed(&w.start(&wish, &repo))["run"].as_str().unwrap_or_default().to_string();
    Made { w, repo, wish, run }
}

impl Made {
    /// Fails unless a follow-up of the first wish is refused in these words, with nothing kept or changed.
    fn unfollowed(&self, why: &str) {
        let before = self.all();
        self.unchanged(&follow(&self.w, &self.wish), &before, why);
    }
}

#[test]
fn a_follow_up_of_a_wish_that_is_not_there_or_has_no_finished_work_is_not_made() {
    let none = format!("the page of {DAY} has no made wish of that id; a made wish's id is m1 to m8");
    let made = finished();
    for id in ["m2", "m9", "w1", "../m1", ""] {
        let before = made.all();
        made.unchanged(&follow(&made.w, id), &before, &none);
    }
    // A wish of another day is followed on that day: the day named has no such wish.
    let before = made.all();
    let ran = made.w.run(&["wish", "--date", "2026-03-11", "--say", MORE, "--after", "m1"]);
    made.unchanged(&ran, &before, &none.replace(DAY, "2026-03-11"));
    let w = world();
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let unrun = Made { w, repo, wish, run: String::new() };
    unrun.unfollowed("wish m1 has had no run, so there is no work to follow");
    let ends = [
        ("asks", 0, "run {run} asks a question: there is no work of it to follow"),
        ("exit-3", 0, "run {run} did not finish: there is no work of it to follow"),
        ("nothing", 0, "run {run} changed nothing: there is no work of it to follow"),
    ];
    for (script, number, why) in ends {
        played(script, number).unfollowed(why);
    }
    // Work that was tossed cannot be followed; work that was kept can.
    let tossed = finished();
    assert_eq!(printed(&tossed.toss())["tossed"], true);
    tossed.unfollowed(TOSSED);
    let kept = finished();
    assert_eq!(printed(&kept.keep(&[]))["kept"], json!(kept.end()));
    assert_eq!(followed(&kept.w, "m1"), "m2");
}

#[test]
fn a_follow_up_of_a_run_that_is_going_or_was_stopped_is_not_made() {
    let w = world();
    w.play("forever", 0);
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let going = w.start_going(&wish, &repo);
    wait_for(&w.at("rec").join("started"));
    let run = w.kept(&wish)["run"].as_str().unwrap().to_string();
    let made = Made { w, repo, wish, run };
    assert_eq!(
        refused(&follow(&made.w, "m1")),
        said(&format!("run {} is going: its work can be followed once it has finished", made.run))
    );
    assert_eq!(printed(&made.w.run(&["run", "stop", "--wish", "m1", "--date", DAY]))["outcome"], "cancelled");
    drop(ended(going));
    made.unfollowed("run {run} was cancelled by a stop: there is no work of it to follow");
    assert_eq!(made.w.kept("m1")["id"], "m1");
}

#[test]
fn a_follow_up_takes_no_project_and_its_words_pass_the_door_a_wish_s_pass() {
    let made = finished();
    let (w, before) = (&made.w, made.all());
    let both = w.run(&["wish", "--date", DAY, "--say", MORE, "--after", "m1", "--project", "alpha"]);
    let usage =
        "jellymoon: wish takes --project or --after, not both: a follow-up is for the project of the wish it follows\n";
    assert!(both.code == 2 && both.stdout.is_empty() && both.stderr.starts_with(usage), "{}", both.stderr);
    let neither = w.run(&["wish", "--date", DAY, "--say", MORE]);
    assert!(neither.code == 2 && neither.stderr.starts_with("jellymoon: wish takes --say and --project or --after\n"));
    let say = |words: &str| w.run(&["wish", "--date", DAY, "--say", words, "--after", "m1"]);
    let key = map_key("kLmNo");
    let unkept = [
        (" \n\t".to_string(), "a wish needs words, and nothing is left of these once cleaned"),
        ("w".repeat(201), "a wish is at most 200 characters, and this one is longer"),
        (
            format!("'{key}'."),
            "nothing is left of these words but the mark of what cleaning replaced, so nothing was made of them",
        ),
    ];
    for (words, why) in &unkept {
        made.unchanged(&say(words), &before, why);
    }
    // What is shaped like a credential is replaced before the follow-up is kept, and each `"` is made `'`.
    let kept = printed(&say(&format!("use \"{key}\"\nfor the retry")));
    assert_eq!(kept["wish"]["said"], "use '[redacted]' for the retry");
    assert_eq!((&kept["wish"]["after"], &kept["wish"]["project"]), (&json!("m1"), &json!("alpha")));
}

#[test]
fn a_follow_up_is_one_of_the_day_s_eight_made_wishes() {
    let made = finished();
    for n in 2..=8 {
        assert_eq!(followed(&made.w, "m1"), format!("m{n}"));
    }
    let full = format!("the page of {DAY} has its 8 made wishes already, so this one was not kept");
    made.unfollowed(&full);
}

#[test]
fn made_wishes_that_name_a_wish_they_cannot_follow_are_not_used() {
    let made = finished();
    assert_eq!(followed(&made.w, "m1"), "m2");
    let file = made.w.jm("pages").join(format!("{DAY}.wishes.json"));
    let kept: Value = serde_json::from_str(&fs::read_to_string(&file).unwrap()).unwrap();
    let unused = format!("the made wishes of {DAY} are not used: they are not the made wishes of that day");
    // A wish follows one made before it that day: not itself, not a later one, and nothing that is no id.
    for (at, after) in [(1, "m2"), (1, "m3"), (0, "m1"), (0, "m2"), (1, "w1"), (1, "m01"), (1, ""), (1, "m1 ")] {
        let mut odd = kept.clone();
        odd["wishes"][at]["after"] = json!(after);
        fs::write(&file, format!("{odd}\n")).unwrap();
        assert_eq!(refused(&made.asked(&[], "m2", &[])), said(&unused), "{after:?} on wish {}", at + 1);
        assert_eq!(refused(&follow(&made.w, "m1")), said(&unused));
    }
    fs::write(&file, format!("{kept}\n")).unwrap();
    assert_eq!(followed(&made.w, "m1"), "m3");
}

#[test]
fn a_follow_up_s_run_is_refused_when_the_work_it_follows_was_tossed_or_moved_or_is_in_another_repo() {
    let made = finished();
    let (w, wish) = (&made.w, followed(&made.w, "m1"));
    w.play("follow", 0);
    // A keep or a toss is at work: the follow-up's run waits for it as a keep does, and is then refused.
    let lock = fs::File::create(w.jm("runs").join("keep.lock")).unwrap();
    lock.lock().unwrap();
    let before = made.all();
    let wait = [("JELLYMOON_TEST_KEEP_WAIT_MS", "200".into())];
    let named = ["run", "--wish", &wish, "--date", DAY, "--repo", made.repo.to_str().unwrap()];
    made.unchanged(
        &w.run_with(&wait, &named),
        &before,
        "another keep or toss is at work in this home: nothing was started",
    );
    drop(lock);
    // Another repo of the project's name is not the repo the followed work is in.
    let other = w.repo("elsewhere/alpha");
    let before = made.all();
    let ran = w.run(&["run", "--wish", &wish, "--date", DAY, "--repo", other.to_str().unwrap()]);
    made.unchanged(&ran, &before, "--repo is not a checkout of the repo the followed work is in");
    assert!(w.branches(&other).is_empty());
    // The followed run's branch is not at the commit its record names: moved by hand, or gone.
    let (name, end) = (made.branch(), made.end());
    let moved =
        "the branch of the work this wish follows is not at the commit its record names, so no copy was made of it";
    made.git(&["update-ref", &name, "main"]);
    let before = made.all();
    made.unchanged(&made.asked(&[], &wish, &[]), &before, moved);
    made.git(&["update-ref", "-d", &name]);
    let before = made.all();
    made.unchanged(&made.asked(&[], &wish, &[]), &before, moved);
    made.git(&["update-ref", &name, &end]);
    // A follow-up that has had no run does not hold the work it follows: it is tossed, and cannot be followed.
    assert_eq!(printed(&made.toss())["tossed"], true);
    let before = made.all();
    made.unchanged(&made.asked(&[], &wish, &[]), &before, TOSSED);
    assert_eq!((w.kept(&wish).get("run"), w.starts(), w.copies()), (None, 1, vec![]));
}

#[test]
fn a_refused_follow_up_s_run_is_run_once_what_refused_it_is_mended() {
    let made = finished();
    let (w, wish, name) = (&made.w, followed(&made.w, "m1"), made.branch());
    w.play("follow", 0);
    made.git(&["update-ref", &name, "main"]);
    assert_eq!(made.asked(&[], &wish, &[]).code, 1);
    made.git(&["update-ref", &name, &made.end()]);
    assert_eq!(printed(&made.asked(&[], &wish, &[]))["outcome"], "completed");
    assert_eq!(given(w, 2).1, AS_THE_RUN_LEFT_IT);
    // The wish has had its run: a second is refused, as any made wish's.
    let run = w.kept(&wish)["run"].as_str().unwrap().to_string();
    let had = format!("wish {wish} has had its run, {run}: a made wish gets one run");
    assert_eq!(refused(&made.asked(&[], &wish, &[])), said(&had));
}
