//! What a made wish's run is doing shows while it works (live work): over its little one the step it is on, and a
//! note of its steps so far before the note that says it is being worked on, in Jellymoon's words with core's
//! cleaned file names. The wish is made and run by the `jellymoon` command with the made-up agent of F39's tests,
//! which reads `b.txt`, waits for the test's word and then changes `a.txt`; the page is placed by
//! `jellymoon-app layout` and looked at by the made-up window of F47 (`support/tapper.rs`). Nothing here starts
//! the real Claude Code or opens a window.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::{fs, thread};

use sticker_kit::{Note, Scene};
use support::Piece;
use support::day::DAY;
use support::wished::{SAID, World, quiet, wait_until};

/// The two sizes the page is looked at: the design's, and the smallest the window opens at.
const SIZES: [&str; 2] = ["1440x900", "960x640"];
const GOING: &str = "Your wish is being worked on.";

/// A day nothing was typed on, with `alpha` listed, and a made wish for it: `m1`.
fn day() -> World {
    let w = quiet();
    let repo = w.at("work").join("alpha");
    w.jellymoon(&["project", "add", "--repo", repo.to_str().unwrap(), "--date", DAY]);
    fs::create_dir(w.at("harness")).unwrap();
    let made = w.jellymoon(&["wish", "--date", DAY, "--say", SAID, "--project", "alpha"]);
    assert_eq!(made["wish"]["id"], "m1", "{made}");
    w
}

fn of<'a>(placed: &'a [Piece], kind: &str, id: &str) -> Vec<&'a Piece> {
    placed.iter().filter(|piece| piece.kind() == kind && piece.id() == id).collect()
}

fn overlap(a: [f64; 4], b: [f64; 4]) -> bool {
    a[0] < b[0] + b[2] && b[0] < a[0] + a[2] && a[1] < b[1] + b[3] && b[1] < a[1] + a[3]
}

/// The page at `layout` mid-way, at `size`: the little one says it reads `b.txt`, the note of its steps says so
/// under its title with the count, and neither is over the entry or another note.
fn mid_way(w: &World, size: &str) {
    let placed = w.placed_with(&["--size", size]);
    let says = of(&placed, "says", "m1");
    let [says] = &says[..] else { panic!("{size}: one bubble: {placed:?}") };
    assert_eq!(says.said(), "reading b.txt", "{size}");
    let steps = of(&placed, "note", "steps-m1");
    let [steps] = &steps[..] else { panic!("{size}: one note of the steps: {placed:?}") };
    let quoted: Vec<String> = of(&placed, "quote", "steps-m1").iter().map(|piece| piece.said()).collect();
    let said = [steps.said(), quoted.join(" ")].join(" ").split_whitespace().collect::<Vec<_>>().join(" ");
    let words = ["What it is doing", "Reading", "1 step so far.", "b.txt"];
    assert!(words.iter().all(|word| said.contains(word)), "{size}: {said}");
    assert!(placed.iter().any(|piece| piece.kind() == "note" && piece.said().starts_with(GOING)), "{size}");
    let entry = ["entry", "chip", "go", "action"];
    for piece in placed.iter().filter(|piece| entry.contains(&piece.kind()) || piece.kind() == "note") {
        if piece.id() != "steps-m1" {
            assert!(!overlap(says.rect(), piece.rect()), "{size}: the bubble is over {piece:?}");
            assert!(!overlap(steps.rect(), piece.rect()), "{size}: the note of the steps is over {piece:?}");
        }
    }
}

/// A kept name with a blank of its own, as a file can be called: the pill says the step without it, so no name
/// reads as Jellymoon's words. The kept steps are put back as they were.
fn blank(w: &World) {
    let runs = w.at("jm").join("runs");
    let mut kept = fs::read_dir(&runs).unwrap().map(|entry| entry.unwrap().path().join("steps.json"));
    let run = kept.find(|file| file.is_file()).unwrap();
    let was = fs::read_to_string(&run).unwrap();
    fs::write(&run, r#"{"count":1,"last":[{"tool":"read","file":"is done. Kept"}]}"#).unwrap();
    let placed = w.placed_with(&["--size", SIZES[0]]);
    let says: Vec<String> = of(&placed, "says", "m1").iter().map(|piece| piece.said()).collect();
    fs::write(&run, was).unwrap();
    assert_eq!(says, ["reading a file"]);
}

fn scene(line: &str) -> Option<Scene> {
    serde_json::from_str(line).expect(line)
}

fn id(note: &Note) -> Option<&str> {
    match note {
        Note::Text { id, .. } | Note::Choice { id, .. } | Note::Told { id, .. } => Some(id.as_str()),
        _ => None,
    }
}

fn ids(scene: &Scene) -> Vec<&str> {
    scene.notes.iter().filter_map(id).collect()
}

#[test]
fn the_page_says_what_a_run_is_doing_while_it_works_and_only_today_s_notes_once_it_ends() {
    let w = day();
    w.play("steps");
    let (home, logs) = (w.at("jm"), w.at("logs"));
    let mut lines = Vec::new();
    let args = ["run", "--wish", "m1", "--date", DAY, "--logs", logs.to_str().unwrap()];
    thread::scope(|scope| {
        let run = scope.spawn(|| w.jellymoon(&args));
        let first = |(_, steps): &(String, jellymoon_core::Steps)| steps.count == 1;
        wait_until("the first step", || jellymoon_core::doing(&home, DAY).iter().any(first));
        SIZES.iter().for_each(|size| mid_way(&w, size));
        blank(&w);
        let mut window = w.taps("logs", &["placed", &format!("hold:{}", w.at("tmp").join("ended").display()), "look"]);
        window.printed(1);
        fs::write(w.at("rec").join("go"), "").unwrap();
        assert_eq!(run.join().unwrap()["outcome"], "completed");
        fs::write(w.at("tmp").join("ended"), "").unwrap();
        lines = window.ended();
    });
    let [opened, ended] = &lines[..] else { panic!("two lines: {lines:#?}") };
    // The window opened mid-way: the same bubble at its own look.
    let opened: Vec<serde_json::Value> = serde_json::from_str(opened).unwrap();
    let bubble = opened.iter().find(|piece| piece["kind"] == "says").expect("a bubble");
    assert_eq!(Piece(bubble.clone()).said(), "reading b.txt");
    // Ended: the work's note, as today, and nothing of the steps; the little one says nothing.
    let ended = scene(ended).expect("the look after the run's end shows a scene");
    assert_eq!(ids(&ended), ["work-m1"]);
    assert!(ended.characters.iter().all(|one| one.says.is_empty()), "{ended:?}");
    let placed = w.placed_with(&["--size", SIZES[1]]);
    assert!(placed.iter().all(|piece| piece.kind() != "says" && piece.id() != "steps-m1"), "{placed:?}");
    // Core keeps both steps of the ended run, for a later page to say.
    let done = jellymoon_core::doing(&home, DAY);
    assert_eq!(done.iter().map(|(wish, steps)| (wish.as_str(), steps.count)).collect::<Vec<_>>(), [("m1", 2)]);
}
