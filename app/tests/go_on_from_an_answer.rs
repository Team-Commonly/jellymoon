//! Go on from an answer: a made wish whose run finished and changed nothing (a person asked a question) has the
//! field of a follow-up under its done note, under no offer, and Enter there makes a follow-up of that run by
//! core's `wish_after`, whose run forks the run's session. No `Keep it`, `Toss it` or `Show me`: there is no work.
//! The wishes are made and run by the `jellymoon` command with the made-up agent of F39's tests; the taps and the
//! Enter are carried by the made-up window of `support/tapper.rs`. Nothing starts the real Claude Code or a window.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::fs;

use serde_json::{Value, json};
use sticker_kit::{Field, Note, Scene};
use support::day::DAY;
use support::wished::{SAID, World, wait_until, world};

const DONE: &str = "This wish is done.";
const AT_WORK: &str = "This wish is being worked on.";
const MORE: &str = "More like this";
const HINT: &str = "Type what to do next, then Enter";
const NEXT: &str = "then tell me what it costs";
const CHOICES: [&str; 3] = ["Show me", "Keep it", "Toss it"];

fn ready() -> World {
    let w = world();
    fs::create_dir(w.at("harness")).unwrap();
    w
}

/// A wish for `alpha` and its run, the made-up agent playing `script` with `number`: the wish's id.
fn ran(w: &World, script: &str, number: usize) -> String {
    fs::write(w.at("agent").join("play"), format!("{script}\n{}\n{number}\n", w.at("rec").display())).unwrap();
    let made = w.jellymoon(&["wish", "--date", DAY, "--say", SAID, "--project", "alpha"]);
    let wish = made["wish"]["id"].as_str().unwrap().to_string();
    let (repo, logs) = (w.at("work").join("alpha"), w.at("logs"));
    let named = ["--wish", &wish, "--date", DAY, "--repo", repo.to_str().unwrap(), "--logs", logs.to_str().unwrap()];
    if !script.is_empty() {
        assert_eq!(w.jellymoon(&[&["run"], &named[..]].concat())["outcome"], "completed");
    }
    wish
}

fn kept(w: &World, id: &str) -> Value {
    let kept = fs::read_to_string(w.at("jm").join("pages").join(format!("{DAY}.wishes.json"))).unwrap();
    let kept: Value = serde_json::from_str(&kept).unwrap();
    kept["wishes"].as_array().unwrap().iter().find(|kept| kept["id"] == id).cloned().unwrap_or_default()
}

fn acted(w: &World, acts: &[String]) -> Vec<Value> {
    let acts: Vec<&str> = acts.iter().map(String::as_str).collect();
    w.taps("logs", &acts).ended().iter().map(|line| serde_json::from_str(line).expect(line)).collect()
}

fn scene(printed: &Value) -> Scene {
    serde_json::from_value(printed.clone()).expect("a scene")
}

/// The note `id` on a scene: its title, options and field.
fn note<'a>(scene: &'a Scene, id: &str) -> Option<(&'a str, &'a [String], Option<&'a Field>)> {
    scene.notes.iter().find_map(|note| match note {
        Note::Text { id: own, title, options, field, .. } if own == id => {
            Some((title.as_str(), &options[..], field.as_ref()))
        }
        _ => None,
    })
}

fn field(offer: &str, wish: &str) -> Field {
    Field { offer: offer.to_string(), hint: HINT.to_string(), most: 200, key: format!("more-{wish}") }
}

#[test]
fn a_done_wish_that_changed_nothing_has_the_field_and_enter_there_follows_its_run_in_its_session() {
    let w = ready();
    let asked = ran(&w, "nothing", 0);
    let run = kept(&w, &asked)["run"].as_str().unwrap().to_string();
    let record = fs::read_to_string(w.at("jm").join("runs").join(&run).join("run.json")).unwrap();
    let record: Value = serde_json::from_str(&record).unwrap();
    let session = record["session"].as_str().unwrap().to_string();
    fs::write(w.at("agent").join("play"), format!("follow\n{}\n0\n", w.at("rec").display())).unwrap();
    let bin = w.install().parent().unwrap().display().to_string();
    let wish_note = format!("wish-{asked}");
    let acts = [format!("bin:{bin}"), format!("poke:{asked}"), format!("type:{wish_note}:{NEXT}"), "start".into()];
    let printed = acted(&w, &acts);
    // The done note keeps its words, has the follow-up's field under no offer, and no choice of work.
    let opened = scene(&printed[0]);
    assert_eq!(note(&opened, &wish_note), Some((DONE, &[][..], Some(&field("", &asked)))), "{opened:?}");
    assert!(opened.notes.iter().all(|one| !matches!(one, Note::Text { id, .. } if id.starts_with("work-"))));
    let text = serde_json::to_string(&opened).unwrap();
    assert!([MORE, CHOICES[0], CHOICES[1], CHOICES[2]].iter().all(|word| !text.contains(word)), "{text}");
    // Enter: core kept the follow-up after the wish, and the field was emptied under the same note.
    assert_eq!(printed[1]["kept"], json!(true), "{}", printed[1]);
    assert_eq!(note(&scene(&printed[1]["scene"]), &wish_note).and_then(|one| one.2), Some(&field("", &asked)));
    let follow = kept(&w, "m2");
    assert_eq!((&follow["said"], &follow["after"], &follow["state"]), (&json!(NEXT), &json!(asked), &json!("working")));
    // Its run forks the session of the run that answered, so the answer is known to it.
    wait_until("the follow-up's run", || kept(&w, "m2")["run"].is_string());
    let started = w.at("rec").join("line2");
    wait_until("the agent's second start", || started.is_file());
    let line = fs::read_to_string(&started).unwrap();
    assert!(line.contains(&format!("--resume\n{session}\n--fork-session")), "{line}");
    wait_until("the follow-up's end", || kept(&w, "m2")["state"] != "working");
    let poked = acted(&w, &["poke:m2".into()]);
    assert_eq!(note(&scene(&poked[0]), "work-m2").map(|one| one.1.len()), Some(3), "its work waits with its choices");
}

#[test]
fn a_wish_at_work_a_wish_with_work_and_a_wish_of_the_reading_are_as_before() {
    let w = ready();
    let work = ran(&w, "edits", 0);
    let going = ran(&w, "", 0);
    let file = w.at("jm").join("pages").join(format!("{DAY}.json"));
    let mut page: Value = serde_json::from_str(&fs::read_to_string(&file).unwrap()).unwrap();
    let wish = json!({"id": "a", "label": "The uploader", "said": "look", "project": "alpha", "state": "done",
        "stars": []});
    page["wishes"].as_array_mut().unwrap().push(wish);
    fs::write(&file, page.to_string()).unwrap();
    let printed = acted(&w, &[format!("poke:{work}"), format!("poke:{going}"), "poke:a".into()]);
    let shown: Vec<Scene> = printed.iter().map(scene).collect();
    let work_note = note(&shown[0], &format!("work-{work}")).unwrap();
    assert_eq!((work_note.1, work_note.2), (&CHOICES.map(String::from)[..], Some(&field(MORE, &work))));
    assert_eq!(note(&shown[1], &format!("wish-{going}")).map(|one| (one.0, one.2)), Some((AT_WORK, None)));
    assert_eq!(note(&shown[2], "wish-a").and_then(|one| one.2), None, "a wish of the reading has no field");
}
