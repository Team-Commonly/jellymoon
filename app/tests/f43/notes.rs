//! What a stop's note outlives and what it replaces, a tap on a choice that is gone, and a run that died: the
//! findings of the gate on F43. No run is asked for here.

use std::fs;

use jellymoon_app::stopping::{Chosen, UNSTOPPED};
use jellymoon_app::today::Next;
use jellymoon_app::wishing::UNKEPT;
use serde_json::Value;
use sticker_kit::{Note, Scene, State};

use super::{drawn, first, planted, poke, stops, wished_alone};
use crate::support::day::DAY;
use crate::support::wished::SAID;

fn plain(scene: &Scene) -> Vec<&str> {
    fn title(note: &Note) -> Option<&str> {
        match note {
            Note::Plain { title, .. } => Some(title.as_str()),
            _ => None,
        }
    }
    scene.notes.iter().filter_map(title).filter(|title| [UNSTOPPED, UNKEPT].contains(title)).collect()
}

// The scene on show changes while core stops the run: the answer puts its note on the new scene and takes
// nothing else of it away.
#[test]
fn a_stop_s_answer_that_comes_to_another_scene_only_puts_its_note_on_it() {
    let (_world, mut on) = wished_alone();
    on.scene.notes.insert(0, planted("m1"));
    let (_, work) = on.stopping("stop-m1", 0).unwrap();
    let stopped = work();
    // Words core refuses, entered meanwhile: their note is on show when the stop's answer comes.
    let (meanwhile, _) = on.wished(&"w".repeat(201), Some("alpha"));
    assert_eq!(first(&meanwhile).map(|(title, _)| title), Some(UNKEPT));
    let scene = on.stopped(stopped);
    assert_eq!(plain(&scene), [UNSTOPPED, UNKEPT], "the answer replaced the scene on show");
    assert_eq!((scene.notes.len(), &scene), (meanwhile.notes.len(), &on.scene), "the note of the stop going stays");
}

// A reading that ends says nothing of a stop: it keeps the choice while its wish is working, and what a stop said.
#[test]
fn a_reading_that_ends_keeps_a_stop_s_note_and_the_choice_of_a_working_wish_alone() {
    let (_world, mut on) = wished_alone();
    on.scene.notes.insert(0, planted("m2"));
    let (_, work) = on.stopping("stop-m2", 0).unwrap();
    on.stopped(work());
    assert_eq!(plain(&on.scene), [UNSTOPPED]);
    // `m1` is working. `w2` is a wish of the reading, and is not.
    on.scene.notes.insert(0, planted("w2"));
    on.scene.notes.insert(0, planted("m1"));
    assert_eq!(drawn(&on.scene, "m1").map(|drawn| drawn.0), Some(State::Busy));

    let page = on.page.clone().unwrap();
    let read = Next { scene: on.today.draw(&page), page: Some(page), later: None };
    let after = on.show(&read);
    assert_eq!(stops(&after).iter().map(|stop| stop.0).collect::<Vec<_>>(), ["stop-m1"]);
    assert_eq!(plain(&after), [UNSTOPPED], "the reading took the stop's note away");
    assert_eq!(after.notes.len(), read.scene.notes.len() + 2);
}

#[test]
fn a_tap_on_a_choice_to_stop_that_is_not_on_show_does_nothing_and_adds_no_note() {
    let (_world, mut on) = wished_alone();
    let before = on.scene.clone();
    for (note, index) in [("stop-m1", 0), ("stop-m1", 1), ("stop-", 0), ("stop-w1", 0)] {
        assert!(matches!(on.chose(note, index), Chosen::Nothing), "a tap on {note:?} did something");
        assert_eq!(on.scene, before, "a tap on {note:?} changed the scene");
    }
    // The second tap on a choice: the first took it off the page.
    on.scene.notes.insert(0, planted("m1"));
    assert!(matches!(on.chose("stop-m1", 0), Chosen::Stop(..)));
    let going = on.scene.clone();
    assert!(matches!(on.chose("stop-m1", 0), Chosen::Nothing));
    assert_eq!(on.scene, going);
    // Any other note's tap is still carried on.
    assert!(matches!(on.chose("clash-1", 0), Chosen::Tapped(_)));
}

#[test]
fn a_new_stop_s_note_replaces_the_notes_of_the_stops_before() {
    let (_world, mut on) = wished_alone();
    for wish in ["m1", "m2", "m1"] {
        on.scene.notes.insert(0, planted(wish));
        let (_, work) = on.stopping(&format!("stop-{wish}"), 0).unwrap();
        let scene = on.stopped(work());
        assert_eq!(plain(&scene), [UNSTOPPED], "the stop of {wish} left an older stop's note");
        // The note is the newest stop's: core names the wish `m1`, which is kept, and no wish `m2`.
        assert_eq!(first(&scene).unwrap().1.contains("wish m1 has no run going"), wish == "m1");
    }
    // A stop that is still going keeps its own note when another ends.
    on.scene.notes.insert(0, planted("m1"));
    on.scene.notes.insert(0, planted("m2"));
    let (_, slow) = on.stopping("stop-m1", 0).unwrap();
    let (_, quick) = on.stopping("stop-m2", 0).unwrap();
    let scene = on.stopped(quick());
    let said = |scene: &Scene| scene.notes.iter().filter(|note| matches!(note, Note::Plain { .. })).count();
    let before = said(&scene);
    assert_eq!(plain(&scene), [UNSTOPPED]);
    let scene = on.stopped(slow());
    assert_eq!((plain(&scene), said(&scene)), (vec![UNSTOPPED], before - 1));
}

// A made wish that names a run nothing holds and nothing is kept of: core gives the run its end when it is asked
// which runs are going, so the wish is not offered a stop, and is not working for ever.
#[test]
fn a_wish_whose_run_died_is_given_its_end_before_it_is_said_to_be_going() {
    let (world, mut on) = wished_alone();
    let wishes = world.home.join("pages").join(format!("{DAY}.wishes.json"));
    let mut kept: Value = serde_json::from_str(&fs::read_to_string(&wishes).unwrap()).unwrap();
    assert_eq!(
        (&kept["wishes"][0]["state"], &kept["wishes"][0]["said"]),
        (&Value::from("working"), &Value::from(SAID))
    );
    kept["wishes"][0]["run"] = Value::from(format!("r{}", "0123456789abcdef"));
    fs::write(&wishes, kept.to_string()).unwrap();

    assert!(poke(&mut on, "m1").is_none(), "a run that died was said to be going");
    assert_eq!(jellymoon_core::going(&world.home, DAY), [""; 0]);
    let kept: Value = serde_json::from_str(&fs::read_to_string(&wishes).unwrap()).unwrap();
    assert_ne!(kept["wishes"][0]["state"], "working", "the wish still waits for a run that is gone");
}
