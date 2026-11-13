//! F47, core's part: which made wishes have work that waits, and what a page says of it (ADR 0017, decision 9 and
//! "Found while building F47"; `docs/development/show.md`, "Work that waits"). `jellymoon run waiting` is run as
//! a black box on runs the made-up `fake-agent` ended in F39's test folder (`f39/world.rs`). The app's part, the
//! taps, is `app/tests/f47.rs`.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;
#[path = "f39/world.rs"]
mod world;

use std::path::Path;

use serde_json::{Value, json};
pub use world::*;

/// `jellymoon run waiting` in the repo, with `more` arguments: the wishes it lists.
fn waiting(w: &World, repo: Option<&Path>, more: &[&str]) -> Vec<Value> {
    let repo = repo.map(|repo| ["--repo", repo.to_str().unwrap()]);
    let ran = w
        .run(&[&["run", "waiting", "--date", DAY][..], repo.as_ref().map_or(&[][..], |repo| &repo[..]), more].concat());
    let said = printed(&ran);
    assert_eq!((said.as_object().unwrap().len(), &said["day"]), (2, &json!(DAY)), "{said}");
    w.never_says_the_agent(&ran);
    said["waits"].as_array().unwrap().clone()
}

/// A wish for `alpha` and its run, with the agent playing `script`: the wish's id and the run's.
fn ran_as(w: &World, repo: &Path, script: &str) -> (String, String) {
    w.play(script, 0);
    let wish = w.wish("alpha");
    let (run, _) = ran(&w.start(&wish, repo));
    (wish, run)
}

#[test]
fn run_waiting_lists_the_made_wishes_whose_finished_work_waits_with_counts_and_names_and_writes_nothing() {
    let w = world();
    let repo = w.repo("alpha");
    assert_eq!(waiting(&w, Some(&repo), &[]), [] as [Value; 0], "a day with no made wish");
    let (wish, run) = ran_as(&w, &repo, "edits");
    // A wish with no run, one whose run changed nothing and one whose run failed: no work of theirs waits.
    let bare = w.wish("alpha");
    let (idle, _) = ran_as(&w, &repo, "nothing");
    let (failed, _) = ran_as(&w, &repo, "is-error");
    assert_eq!([bare.as_str(), idle.as_str(), failed.as_str()], ["m2", "m3", "m4"]);
    let before = (w.persons(&repo), w.branches(&repo), support::tree(&w.jm("pages")), w.record(&run));

    let why = format!("the work of run {run} waits: files made 1, changed 1, removed 1");
    let want = json!({
        "wish": wish, "run": run, "said": SAID, "read": true, "made": 1, "changed": 1, "removed": 1,
        "names": ["a.txt", "b.txt", "n.txt"], "more": 0, "obeyed": [], "why": why,
    });
    assert_eq!(waiting(&w, Some(&repo), &[]), [want]);
    assert_eq!((w.persons(&repo), w.branches(&repo), support::tree(&w.jm("pages")), w.record(&run)), before);

    // Kept, the work waits no more; and a wish is no word of this command.
    let kept = w.run(&["run", "keep", "--wish", &wish, "--date", DAY, "--repo", repo.to_str().unwrap()]);
    assert!(printed(&kept)["kept"].is_string(), "{}", kept.stderr);
    assert_eq!(waiting(&w, Some(&repo), &[]), [] as [Value; 0]);
    let odd = w.run(&["run", "waiting", "--wish", "m1"]);
    assert_eq!((odd.code, odd.stdout.as_str()), (2, ""), "{}", odd.stderr);
}

#[test]
fn work_that_touches_a_file_the_person_s_tools_obey_names_it_and_tossed_work_waits_no_more() {
    let w = world();
    let repo = w.repo("alpha");
    let (wish, _) = ran_as(&w, &repo, "rules");
    let listed = waiting(&w, Some(&repo), &[]);
    let [one] = &listed[..] else { panic!("{listed:?}") };
    let names = json!(["0.md", "CLAUDE.md", "a.txt", "b.txt", "n.txt"]);
    assert_eq!((&one["obeyed"], &one["names"]), (&json!(["CLAUDE.md"]), &names));
    assert_eq!((&one["made"], &one["changed"], &one["removed"]), (&json!(3), &json!(1), &json!(1)));
    let tossed = w.run(&["run", "toss", "--wish", &wish, "--date", DAY, "--repo", repo.to_str().unwrap()]);
    assert_eq!(printed(&tossed)["tossed"], true, "{}", tossed.stderr);
    assert_eq!(waiting(&w, Some(&repo), &[]), [] as [Value; 0]);
}

#[test]
fn work_whose_repo_is_not_found_is_listed_with_core_s_reason_and_no_file() {
    let w = world();
    let repo = w.repo("alpha");
    let (wish, run) = ran_as(&w, &repo, "edits");
    let before = (w.persons(&repo), w.branches(&repo), w.record(&run), w.copies());
    // No folder is named, and the logs hold no session: the repo is not found.
    let logs = w.at("logs");
    let listed = waiting(&w, None, &["--logs", logs.to_str().unwrap()]);
    let why = format!("no folder of the name the wish gives was typed in on {DAY}; name the folder with --repo");
    let want = json!({
        "wish": wish, "run": run, "said": SAID, "read": false, "made": 0, "changed": 0, "removed": 0,
        "names": [], "more": 0, "obeyed": [], "why": why,
    });
    assert_eq!(listed, [want]);
    // A folder of another name is not the wish's repo either.
    let other = w.repo("beta");
    let listed = waiting(&w, Some(&other), &[]);
    assert_eq!(
        (&listed[0]["read"], listed[0]["why"].as_str().map(|why| why.starts_with("--repo is not"))),
        (&json!(false), Some(true))
    );
    assert_eq!((w.persons(&repo), w.branches(&repo), w.record(&run), w.copies()), before);
}
