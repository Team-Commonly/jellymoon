//! The made wishes of a day nothing was typed on: they are placed in their states as a typed day's page places
//! them, and a wish entered on such a day is on the page at once, with a run that finds its project on the list.

use std::collections::BTreeMap;
use std::fs;

use serde_json::Value;
use sticker_kit::{Scene, State};

use super::{EVERY, HINT, Logs, NOTHING, list, nothing_typed, of, values, words};
use crate::support::day::{DAY, called, notes};
use crate::support::wished::{SAID, World, showed, wait_until, world};
use crate::support::{Piece, open_above};

/// Three made wishes for `alpha`, as a person types them: `m1` with a run that finished, so its work waits;
/// `m2` with no run; `m3` with a run that asks. The repo is found as the day finds it: no folder is named.
fn three(w: &World) {
    fs::create_dir(w.at("harness")).unwrap();
    let logs = w.at("logs");
    let made = |script: &str, run: Option<&str>| {
        w.play(script);
        let wish = w.jellymoon(&["wish", "--date", DAY, "--say", SAID, "--project", "alpha"]);
        let id = wish["wish"]["id"].as_str().unwrap().to_string();
        if let Some(outcome) = run {
            let ran = w.jellymoon(&["run", "--wish", &id, "--date", DAY, "--logs", logs.to_str().unwrap()]);
            assert_eq!(ran["outcome"], outcome, "{ran}");
        }
    };
    made("edits", Some("completed"));
    made("edits", None);
    made("asks", Some("asks"));
}

/// What is placed of a made wish: its character's state, and how many pieces of each kind carry its id.
type Drawn = (Value, BTreeMap<String, usize>);

/// The made wish a piece is of, by the piece's id: `m1`, or `work-m1` for the note of its work.
fn wish_of(piece: &Piece) -> Option<&str> {
    let id = piece.id().trim_start_matches("work-");
    let digits = id.strip_prefix('m')?;
    (!digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit())).then_some(id)
}

fn drawn(placed: &[Piece]) -> BTreeMap<String, Drawn> {
    let mut wishes: BTreeMap<String, Drawn> = BTreeMap::new();
    for piece in placed {
        let Some(id) = wish_of(piece) else { continue };
        let wish = wishes.entry(id.to_string()).or_default();
        *wish.1.entry(piece.kind().to_string()).or_default() += 1;
        if piece.kind() == "character" {
            wish.0 = piece.0["state"].clone();
        }
    }
    wishes
}

#[test]
fn the_made_wishes_of_such_a_day_are_placed_in_their_states_as_a_typed_day_places_them() {
    // A typed day, read once: its page has the reading's two wishes, and then the three made ones.
    let typed = world();
    three(&typed);
    let on_typed = drawn(&typed.placed());
    assert_eq!(on_typed.keys().collect::<Vec<_>>(), ["m1", "m2", "m3"], "{on_typed:?}");
    let states: Vec<&Value> = on_typed.values().map(|wish| &wish.0).collect();
    assert_eq!(states, ["idle", "busy", "needs_you"], "done, working, and one that needs the person");
    assert_eq!(on_typed["m1"].1["option"], 3, "the work of m1 waits, with its three choices: {on_typed:?}");

    for logs in EVERY {
        let w = nothing_typed(logs);
        list(&w, "alpha");
        three(&w);
        let placed = w.placed();
        assert_eq!(drawn(&placed), on_typed, "{logs:?}");
        // The made wishes and nothing else: no wish, clash or reading is made up for the day.
        assert_eq!(of(&placed, "character").len(), 3, "{logs:?}");
        let said = notes(&placed);
        assert!(said.iter().all(|note| !note.contains("clash") && !note.contains(NOTHING)), "{logs:?}: {said:?}");
        assert_eq!(words(&placed, "entry"), [HINT], "{logs:?}");
        assert_eq!(words(&placed, "chip"), ["alpha"], "{logs:?}");
        assert_eq!(values(&w.placed()), values(&placed), "{logs:?}: the same day again");
        assert_eq!((w.starts(), called(&w.day)), (2, false), "{logs:?}: two runs, and no reading call");
    }
}

/// The state and the stars of a wish's character on a scene.
fn character(scene: &Scene, id: &str) -> Option<(State, u8)> {
    scene.characters.iter().find(|one| one.id == id).map(|one| (one.state, one.stars))
}

#[test]
fn a_wish_entered_on_such_a_day_is_on_the_page_at_once_and_its_run_finds_the_project_on_the_list() {
    let w = nothing_typed(Logs::Absent);
    list(&w, "alpha");
    w.install();
    if open_above(w.day.dir.path()).is_some() {
        return;
    }
    // The page a window opened on that day shows first: the one `layout` kept, with the entry and its chip.
    assert_eq!(words(&w.placed(), "chip"), ["alpha"]);
    w.play("slow");
    let window = w.enters("alpha", "watch");
    wait_until("the agent", || w.at("rec").join("started").is_file());
    let going = w.kept();
    assert_eq!((&going["state"], going["run"].is_string()), (&Value::from("working"), true), "{going}");
    fs::write(w.at("rec").join("go"), "").unwrap();

    let scenes = showed(window);
    assert_eq!(scenes.len(), 2, "the wish at once, and the star: the asking had nothing to say");
    assert_eq!(character(&scenes[0], "m1"), Some((State::Busy, 0)), "at once, working, before any run");
    assert_eq!(character(&scenes[1], "m1"), Some((State::Idle, 1)));
    w.ended();
    // The run worked in a copy of the listed repo: its one commit is on its branch there.
    let (repo, run) = (w.at("work").join("alpha"), w.kept()["run"].as_str().unwrap().to_string());
    let commits = w.told(&repo, &["rev-list", "--count", &format!("main..jellymoon/run-{run}")]);
    assert_eq!(commits.trim(), "1");
    // And the day's page then: the wish, the entry and the chip, with no reading call and one start of the agent.
    let placed = w.placed();
    assert_eq!((of(&placed, "character").len(), words(&placed, "chip")), (1, vec!["alpha".to_string()]));
    assert_eq!((w.starts(), called(&w.day)), (1, false));
}
