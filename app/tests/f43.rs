//! F43: a tap on a working made wish stops its run. The rules are core's and are proved at `jellymoon run stop`
//! (`crates/jellymoon/tests/f40.rs`). Here is the app's part, followed with no window through `OnShow`: a tap on
//! the wish's character shows the choice, a tap on the choice is carried to core's `stop_run`, and the page says
//! what core says. The run is asked for by a made-up window in a process of its own (`support/wished.rs`), and
//! the made-up agent of F39's tests is what runs: nothing here starts the real Claude Code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

#[path = "f43/notes.rs"]
mod notes;
mod support;

use std::fs;

use jellymoon_app::stopping::{ENDED, STOP, STOPPED, STOPPING, UNSTOPPED};
use jellymoon_app::taps::OnShow;
use serde_json::json;
use sticker_kit::{Note, Scene, State};
use support::Piece;
use support::day::{self, DAY, names, shown};
use support::open_above;
use support::wished::{SAID, World, showed, wait_until, world};

/// The choices to stop on a scene: each one's id and options.
fn stops(scene: &Scene) -> Vec<(&str, &[String])> {
    fn stop(note: &Note) -> Option<(&str, &[String])> {
        match note {
            Note::Choice { id, options, .. } if id.starts_with("stop-") => Some((id.as_str(), options.as_slice())),
            _ => None,
        }
    }
    scene.notes.iter().filter_map(stop).collect()
}

fn first(scene: &Scene) -> Option<(&str, &str)> {
    match scene.notes.first()? {
        Note::Plain { title, body } => Some((title, body)),
        _ => None,
    }
}

/// The state and the stars of a wish's character.
fn drawn(scene: &Scene, id: &str) -> Option<(State, u8)> {
    scene.characters.iter().find(|one| one.id == id).map(|one| (one.state, one.stars))
}

/// The options `layout` printed: each one's note and words.
fn options(placed: &[Piece]) -> Vec<(String, String)> {
    let option = |piece: &&Piece| piece.kind() == "option";
    placed.iter().filter(option).map(|piece| (piece.id().to_string(), piece.said())).collect()
}

/// A wish entered and its run going, with the agent playing `script`. None where the installed command is not
/// taken (see `open_above`).
fn going(script: &str) -> Option<World> {
    let w = world();
    w.install();
    if open_above(w.day.dir.path()).is_some() {
        return None;
    }
    w.play(script);
    assert_eq!(showed(w.enters("alpha", "leave")).len(), 1, "the wish at once, and nothing to say of its run");
    wait_until("the agent", || w.at("rec").join("started").is_file());
    Some(w)
}

/// A tap on the character `id`, as the window carries it: the scene it shows, when it shows one.
fn poke(on: &mut OnShow, id: &str) -> Option<Scene> {
    let seen = (on.poked(id))()?;
    Some(on.seen(seen).expect("the scene on show is the one the tap was made on"))
}

fn sends<T: Send>(_: &T) {}

#[test]
fn a_tap_on_a_working_wish_s_character_shows_stop_it_and_its_tap_stops_the_run() {
    let Some(w) = going("lingers") else { return };
    let mut on = w.on_show();
    assert_eq!((drawn(&on.scene, "m1"), stops(&on.scene).len()), (Some((State::Busy, 0)), 0));
    // `layout` has no tap: it prints the choice of a made wish that is working and names a run.
    assert_eq!(options(&shown(&w.day, &[])), [("stop-m1".to_string(), STOP.to_string())]);

    sends(&on.poked("m1"));
    let scene = poke(&mut on, "m1").expect("the wish's run is going");
    let stop = [STOP.to_string()];
    assert_eq!(stops(&scene), [("stop-m1", &stop[..])]);
    assert!(matches!(scene.notes.first(), Some(Note::Choice { id, .. }) if id == "stop-m1"), "it is the first note");
    assert!(poke(&mut on, "m1").is_none(), "the choice is shown once");
    assert!((on.look())().is_none(), "a look took the choice away");
    for other in [("stop-m1", 1), ("stop-m2", 0), ("clash-1", 0), ("m1", 0)] {
        assert!(on.stopping(other.0, other.1).is_none(), "{other:?} is no choice to stop on show");
    }
    assert_eq!((w.kept()["state"].as_str(), w.at("rec").join("go").exists()), (Some("working"), false));

    let (at_once, work) = on.stopping("stop-m1", 0).expect("the choice is on show");
    sends(&work);
    assert_eq!(first(&at_once).map(|(title, _)| title), Some(STOPPING));
    assert_eq!((stops(&at_once).len(), &at_once), (0, &on.scene), "the choice gives way to what the page says");
    assert_eq!(w.kept()["state"], "working", "core was asked before the work was run");
    let run = w.kept()["run"].as_str().unwrap().to_string();
    let stopped = work();
    w.ended();
    let scene = on.stopped(stopped);
    let why = format!("run {run} was cancelled by a stop: its copy is kept and no star was added");
    assert_eq!(first(&scene), Some((STOPPED, format!("{why}.").as_str())));
    assert_eq!((drawn(&scene, "m1"), &scene), (Some((State::NeedsYou, 0)), &on.scene), "needing the person, no star");
    assert!(!scene.notes.iter().any(|note| matches!(note, Note::Plain { title, .. } if title == STOPPING)));
    let kept = w.kept();
    assert_eq!((&kept["state"], &kept["stars"], &kept["why"]), (&json!("needs_you"), &json!([]), &json!(why)));

    // A stopped run is not going: no choice for it, in the window or in `layout`.
    assert!(poke(&mut on, "m1").is_none());
    assert_eq!(options(&shown(&w.day, &[])), []);
    assert!((on.look())().is_none(), "the note stays");
}

#[test]
fn a_stop_of_a_run_that_ended_meanwhile_stops_nothing_writes_nothing_and_says_what_core_says() {
    let Some(w) = going("slow") else { return };
    let mut on = w.on_show();
    assert_eq!(poke(&mut on, "m1").map(|scene| stops(&scene).len()), Some(1));
    // The run ends by itself while the choice is on show.
    fs::write(w.at("rec").join("go"), "").unwrap();
    wait_until("the star", || w.kept()["stars"].as_array().is_some_and(|stars| stars.len() == 1));
    w.ended();
    let run = w.kept()["run"].as_str().unwrap().to_string();
    let record = w.at("jm").join("runs").join(&run);
    let before = (w.kept(), names(&record), names(&w.at("jm").join("pages")));

    let (_, work) = on.stopping("stop-m1", 0).expect("the choice is still on show");
    let scene = on.stopped(work());
    let why = format!("run {run} finished: its work is one commit on the branch jellymoon/run-{run}.");
    assert_eq!(first(&scene), Some((ENDED, why.as_str())));
    assert_eq!((drawn(&scene, "m1"), stops(&scene).len()), (Some((State::Idle, 1)), 0), "its star stays");
    assert_eq!((w.kept(), names(&record), names(&w.at("jm").join("pages"))), before, "the stop wrote something");
    assert_eq!(w.starts(), 1);
}

/// A day read and kept with one made wish, `m1`, that is working and has no run: it is kept by core, as
/// `jellymoon wish` keeps one, and no run is ever asked for.
fn wished_alone() -> (day::World, OnShow) {
    let world = day::world();
    shown(&world, &[]);
    jellymoon_core::wish(&world.home, Some(DAY), SAID, "alpha").unwrap();
    let today = jellymoon_app::today::Today {
        date: DAY.to_string(),
        logs: world.dir.path().join("logs"),
        skip: Vec::new(),
        model: "sonnet".to_string(),
        home: world.home.clone(),
        harness: None,
        fresh: false,
        settings: None,
        bin: None,
    };
    let (scene, page) = today.first();
    (world, OnShow { today, page, scene })
}

/// A choice to stop the run of `wish`, as one left on show.
fn planted(wish: &str) -> Note {
    Note::Choice {
        id: format!("stop-{wish}"),
        title: String::new(),
        options: vec![STOP.to_string()],
        footer: String::new(),
        struck_first: false,
        whole: false,
    }
}

#[test]
fn a_tap_on_any_other_character_or_on_a_wish_with_no_run_going_stops_nothing_and_writes_nothing() {
    let (world, mut on) = wished_alone();
    let wishes = world.home.join("pages").join(format!("{DAY}.wishes.json"));
    let kept = || (fs::read_to_string(&wishes).unwrap(), names(&world.home), names(&world.home.join("pages")));
    let before = kept();
    assert_eq!(drawn(&on.scene, "m1"), Some((State::Busy, 0)), "working, with no run");
    let characters: Vec<String> = on.scene.characters.iter().map(|one| one.id.clone()).collect();
    assert_eq!(characters, ["w1", "w2", "m1"]);
    for id in ["me", "w1", "w2", "m1", "m2", "stop-m1", ""] {
        assert!(poke(&mut on, id).is_none(), "a tap on {id:?} showed something");
    }
    assert_eq!(options(&shown(&world, &[])).iter().filter(|(note, _)| note.starts_with("stop-")).count(), 0);
    assert!(on.stopping("stop-m1", 0).is_none(), "no choice is on show");

    // A choice left on show for a wish with no run (a page that changed under the person): core refuses.
    on.scene.notes.insert(0, planted("m1"));
    let (_, work) = on.stopping("stop-m1", 0).unwrap();
    let scene = on.stopped(work());
    let (title, body) = first(&scene).unwrap();
    assert!(title == UNSTOPPED && body.ends_with("wish m1 has no run going, so nothing was stopped."), "{body}");
    assert_eq!(kept(), before, "something was written");

    // A choice whose wish is not drawn as working goes with the next look.
    on.scene.notes.insert(0, planted("w2"));
    on.scene.notes.insert(0, planted("m1"));
    let seen = (on.look())().expect("one choice no longer holds");
    let scene = on.seen(seen).unwrap();
    assert_eq!(stops(&scene).iter().map(|stop| stop.0).collect::<Vec<_>>(), ["stop-m1"]);
    let said = |note: &&Note| matches!(note, Note::Plain { title, .. } if title == UNSTOPPED);
    assert_eq!(scene.notes.iter().filter(said).count(), 1, "the note of the stop stays under it");
}
