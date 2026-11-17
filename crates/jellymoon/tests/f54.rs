//! F54, core's part: which made wishes a follow-up may be made of, said where a page asks which work waits
//! (ADR 0018, "Found while building F54"; `docs/development/wish.md`, "A follow-up"). `jellymoon run waiting` is
//! run as a black box on runs the made-up `fake-agent` ended in F39's test folder (`f39/world.rs`), and what it
//! names is held against `jellymoon wish --after`, which makes the follow-up or refuses it. The app's part, the
//! field under a wish, is `app/tests/f54.rs`.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;
#[path = "f39/world.rs"]
mod world;

use std::fs;
use std::path::Path;

use serde_json::{Value, json};
pub use world::*;

/// The made wishes `jellymoon run waiting` says a follow-up may be made of.
fn follows(w: &World, repo: &Path) -> Vec<String> {
    let ran = w.run(&["run", "waiting", "--date", DAY, "--repo", repo.to_str().unwrap()]);
    let said = printed(&ran);
    assert_eq!((said.as_object().unwrap().len(), &said["day"]), (3, &json!(DAY)), "{said}");
    w.never_says_the_agent(&ran);
    said["follows"].as_array().unwrap().iter().map(|wish| wish.as_str().unwrap().to_string()).collect()
}

/// A wish for `alpha` and its run, with the agent playing `script`: the wish's id.
fn ran_as(w: &World, repo: &Path, script: &str) -> String {
    w.play(script, 0);
    let wish = w.wish("alpha");
    let ran = w.start(&wish, repo);
    assert!(printed(&ran)["run"].is_string(), "{}", ran.stderr);
    wish
}

fn asked(w: &World, repo: &Path, verb: &str, wish: &str) {
    let ran = w.run(&["run", verb, "--wish", wish, "--date", DAY, "--repo", repo.to_str().unwrap()]);
    assert_eq!(ran.code, 0, "run {verb}: {}", ran.stderr);
}

#[test]
fn run_waiting_names_the_wishes_whose_work_waits_or_was_kept_as_the_ones_a_follow_up_may_be_made_of() {
    let w = world();
    let repo = w.repo("alpha");
    assert_eq!(follows(&w, &repo), [] as [String; 0], "a day with no made wish");
    let waits = ran_as(&w, &repo, "edits");
    assert_eq!(follows(&w, &repo), [waits.as_str()]);
    // A wish with no run, one whose run asks, one that failed and one that changed nothing have no work to follow.
    let unrun = w.wish("alpha");
    let asks = ran_as(&w, &repo, "asks");
    let failed = ran_as(&w, &repo, "exit-3");
    let nothing = ran_as(&w, &repo, "nothing");
    assert_eq!(follows(&w, &repo), [waits.as_str()]);
    // Work that was kept is followed, and work that was tossed is not.
    let tossed = ran_as(&w, &repo, "edits");
    assert_eq!(follows(&w, &repo), [waits.clone(), tossed.clone()]);
    asked(&w, &repo, "toss", &tossed);
    assert_eq!(follows(&w, &repo), [waits.as_str()]);
    asked(&w, &repo, "keep", &waits);
    assert_eq!(follows(&w, &repo), [waits.as_str()], "kept work is followed as work that waits is");
    // A run that is going has no work yet, and this one changes nothing, so it has none when it has ended.
    w.play("slow", 0);
    let going = w.wish("alpha");
    let command = w.start_going(&going, &repo);
    wait_for(&w.at("rec").join("started"));
    assert_eq!(follows(&w, &repo), [waits.as_str()]);
    fs::write(w.at("rec").join("go"), "").unwrap();
    assert_eq!(ended(command).code, 0);
    assert_eq!(follows(&w, &repo), [waits.as_str()]);

    // What is named is what `wish --after` makes a follow-up of, and every other wish is refused by it, for
    // the work there is none of. The day has seven made wishes: the first follow-up is its eighth, and the
    // next is refused for the eight alone.
    let said = |after: &str| w.run(&["wish", "--date", DAY, "--say", "and once more", "--after", after]);
    for other in [&unrun, &asks, &failed, &nothing, &tossed, &going] {
        let refused = said(other);
        assert!(refused.code != 0 && !refused.stderr.contains("its 8 made wishes"), "{other}: {}", refused.stderr);
    }
    assert_eq!(printed(&said(&waits))["wish"]["after"], json!(waits));
    assert!(said(&waits).stderr.contains("its 8 made wishes already"), "{}", said(&waits).stderr);
    assert_eq!(follows(&w, &repo), [waits], "a follow-up with no run has no work to follow");
}

#[test]
fn what_run_waiting_names_to_follow_is_read_from_the_home_alone_and_writes_nothing() {
    let w = world();
    let repo = w.repo("alpha");
    let wish = ran_as(&w, &repo, "edits");
    let before = support::tree(&w.at("jm"));
    // No repo is named and none is found: the work is not read, and the wish is still named to follow.
    let ran = w.run(&["run", "waiting", "--date", DAY]);
    let said: Value = printed(&ran);
    assert_eq!(said["follows"], json!([wish]));
    assert!(support::tree(&w.at("jm")) == before, "a look wrote something");
    // Another day has none.
    let other = printed(&w.run(&["run", "waiting", "--date", "2026-03-11"]));
    assert_eq!((&other["follows"], &other["waits"]), (&json!([]), &json!([])));
}
