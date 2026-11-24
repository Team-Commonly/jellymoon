//! Which made wishes' notes a look makes again, with no core and no window: the scenes are typed here.

use sticker_kit::{Character, Note, State};

use super::turns::HOLD;
use super::*;

/// A page of made wishes `m1`, `m2`, in these states, under `notes`.
fn page(states: &[State], mut notes: Vec<Note>) -> Scene {
    let wish =
        |(at, state): (usize, &State)| Character { id: format!("m{}", at + 1), state: *state, ..Character::default() };
    let rows = states.iter().map(|_| (sticker_kit::Mark::Star, String::new())).collect();
    notes.push(Note::Whole { title: crate::scene::MADE.to_string(), rows });
    Scene { characters: states.iter().enumerate().map(wish).collect(), notes, ..Scene::default() }
}

fn text(id: &str, options: &[&str]) -> Note {
    Note::Text {
        id: id.to_string(),
        title: String::new(),
        body: Vec::new(),
        options: options.iter().map(|option| option.to_string()).collect(),
        quoted: String::new(),
        field: None,
    }
}

/// The wishes whose notes the look of `scene` makes again.
fn asked(turns: &mut Turns, scene: &Scene) -> Vec<String> {
    turns.asks(scene).0
}

#[test]
fn core_is_asked_of_the_ended_wishes_when_the_window_opens_and_of_none_by_the_looks_after() {
    let done = page(&[State::Idle, State::Busy, State::Dimmed], Vec::new());
    let mut turns = Turns::default();
    // The first look: every wish that has ended, and never one whose run goes.
    assert_eq!(asked(&mut turns, &done), ["m1", "m3"]);
    turns.saw(Some(&done));
    // Look after look with nothing changed: core is asked nothing of a wish that has ended.
    for _ in 0..5 {
        assert_eq!(asked(&mut turns, &done), [""; 0]);
        turns.saw(Some(&done));
    }
}

#[test]
fn a_run_that_ends_owes_every_ended_wish_its_notes_once() {
    let (going, ended) = (page(&[State::Idle, State::Busy], Vec::new()), page(&[State::Idle, State::Idle], Vec::new()));
    let mut turns = Turns::default();
    assert_eq!(asked(&mut turns, &going), ["m1"]);
    // The look that finds the run ended makes the notes itself (`Today::looked`): nothing is owed after it.
    assert_eq!(asked(&mut turns, &going), [""; 0]);
    turns.saw(Some(&ended));
    assert_eq!(asked(&mut turns, &ended), [""; 0]);
    // A run a tap stopped is drawn ended with no look: the next look makes the notes of both wishes again.
    let mut turns = Turns::default();
    assert_eq!(asked(&mut turns, &going), ["m1"]);
    assert_eq!(asked(&mut turns, &ended), ["m1", "m2"]);
    assert_eq!(asked(&mut turns, &ended), [""; 0]);
}

#[test]
fn a_look_that_was_dropped_is_made_again_and_core_s_answer_to_a_choice_owes_every_ended_wish() {
    let done = page(&[State::Idle, State::Idle], Vec::new());
    let mut turns = Turns::default();
    assert_eq!(asked(&mut turns, &done), ["m1", "m2"]);
    // The look's scene was dropped, the page having changed under it: both are asked of again.
    turns.saw(None);
    assert_eq!(asked(&mut turns, &done), ["m1", "m2"]);
    turns.saw(Some(&done));
    assert_eq!(asked(&mut turns, &done), [""; 0]);
    // Core answered a keep on `m1`: `m2` is made again at once, and `m1` when its answer has been read.
    turns.owe(&done);
    turns.acted(Some("m1".to_string()));
    assert_eq!(turns.asks(&done), (vec!["m2".to_string()], vec!["m1".to_string()]));
    for _ in 1..HOLD {
        assert_eq!(asked(&mut turns, &done), [""; 0]);
    }
    assert_eq!(asked(&mut turns, &done), ["m1"]);
}

#[test]
fn a_wish_in_its_hold_or_with_a_note_in_hand_is_not_made_again_and_stays_owed() {
    let (going, ended) = (page(&[State::Busy, State::Idle], Vec::new()), page(&[State::Idle, State::Idle], Vec::new()));
    let mut turns = Turns::default();
    assert_eq!(asked(&mut turns, &going), ["m2"]);
    // The person tapped a choice of `m2`, and the look in its hold finds the run of `m1` ended: that look
    // leaves the notes of `m2`, which are owed when the hold is over.
    turns.acted(Some("m2".to_string()));
    assert_eq!(turns.asks(&going), (Vec::new(), vec!["m2".to_string()]));
    turns.saw(Some(&ended));
    for _ in 1..HOLD {
        assert_eq!(asked(&mut turns, &ended), [""; 0]);
    }
    assert_eq!(asked(&mut turns, &ended), ["m2"]);
    // A file of a wish is on show: its notes are not made again under the person's eyes, and are owed still.
    let held = page(&[State::Idle, State::Idle], vec![text("file-1-m1", &[])]);
    let mut turns = Turns::default();
    assert_eq!(asked(&mut turns, &held), ["m2"]);
    assert_eq!(asked(&mut turns, &held), [""; 0]);
    assert_eq!(asked(&mut turns, &ended), ["m1"]);
}

/// A day of 2026-03-10 kept in `home`, which no harness reads.
fn today(home: &std::path::Path) -> crate::today::Today {
    crate::today::Today {
        date: "2026-03-10".to_string(),
        logs: home.join("logs"),
        skip: Vec::new(),
        model: String::new(),
        home: home.to_path_buf(),
        harness: Some(home.join("harness")),
        fresh: false,
        settings: None,
        bin: None,
    }
}

#[test]
fn a_look_in_flight_when_the_person_taps_an_option_is_dropped_whole_and_what_it_was_given_is_owed() {
    let going = page(&[State::Busy, State::Idle], Vec::new());
    let shown = Showing::of(Some(OnShow { today: today("none".as_ref()), page: None, scene: going.clone() }));
    // A look is made, and is in flight with the other wish's work on its scene.
    assert!(shown.look().is_some());
    let mut late = going.clone();
    late.notes.insert(0, text("work-m2", &["Show me", "Keep it", "Toss it"]));
    // Meanwhile a tap on a character shows the choice to stop its run, and the person taps that choice.
    let mut stop = going.clone();
    let options = vec![crate::stopping::STOP.to_string()];
    let (id, title, footer) = ("stop-m1".to_string(), String::new(), String::new());
    stop.notes.insert(0, Note::Choice { id, title, options, footer, struck_first: false, whole: false });
    assert!(shown.seen(Seen { from: going.clone(), now: stop }).is_some());
    let (at_once, _) = shown.chose("stop-m1", 0, crate::stopping::STOP).expect("the choice is on show");
    assert!(shown.seen(Seen { from: going, now: late }).is_none(), "the stale look is dropped");
    assert_eq!(shown.scene(), Some(at_once.clone()), "nothing of the look is on the page");
    assert_eq!(asked(&mut shown.2.borrow_mut(), &at_once), ["m2"], "the next look makes its notes");
}

#[test]
fn an_act_on_a_wish_s_note_holds_that_wish_and_gives_it_the_offer_and_a_tap_on_a_clash_holds_none() {
    let home = tempfile::tempdir().unwrap();
    let field = sticker_kit::Field { offer: String::new(), hint: String::new(), most: 0, key: String::new() };
    let Note::Text { id, title, body, options, quoted, .. } = text("wish-m1", &[]) else { unreachable!() };
    let asks = Note::Text { id, title, body, options, quoted, field: Some(field) };
    let (id, options) = ("clash-1".to_string(), vec!["not a side".to_string()]);
    let clash =
        Note::Choice { id, title: String::new(), options, footer: String::new(), struck_first: false, whole: false };
    let file = r#"{"day": "2026-03-10", "read": {"turns": 2, "projects": ["beta"]}, "wishes": [
        {"id": "w1", "label": "a", "said": "a", "project": "beta", "state": "working", "stars": []},
        {"id": "w2", "label": "b", "said": "b", "project": "beta", "state": "working", "stars": []}],
        "clashes": [{"a": "w1", "b": "w2", "quote_a": "x", "quote_b": "y", "settled": false}]}"#;
    let scene = page(&[State::Idle, State::Idle], vec![asks, clash]);
    let on = || OnShow { today: today(home.path()), page: crate::read::page(file).ok(), scene: scene.clone() };
    let held = |shown: &Showing| shown.2.borrow_mut().asks(&scene).1;
    let shown = Showing::of(Some(on()));
    assert!(shown.typing("wish-m1", "yes").is_some_and(|(at_once, _)| at_once.is_some()));
    assert_eq!(held(&shown), ["m1"], "an Enter in a field under a note of the wish");
    // The tapped words are no side core has, so nothing is ruled or written; no wish is held for it.
    let shown = Showing::of(Some(on()));
    let refused = |note: &Note| matches!(note, Note::Plain { title, .. } if title == "That tap did nothing.");
    let carried = shown.chose("clash-1", 0, "not a side");
    assert!(carried.is_some_and(|(now, after)| after.is_none() && now.notes.first().is_some_and(refused)));
    assert_eq!(held(&shown), [""; 0], "a tap on a clash");
    // A tap that carries other words than the note has at that place now is carried nowhere: the scene on show
    // is given back as it is, and no wish is held for it.
    let shown = Showing::of(Some(on()));
    let unseen = shown.chose("clash-1", 0, "a side drawn before");
    assert!(matches!(&unseen, Some((now, None)) if *now == scene), "{:?}", unseen.map(|(now, _)| now));
    let gone = shown.chose("clash-9", 0, "not a side");
    assert!(matches!(&gone, Some((now, None)) if *now == scene), "a note that is on show no more");
    assert_eq!(shown.scene(), Some(scene.clone()), "nothing was carried: no note says a tap did nothing");
    assert_eq!(held(&shown), [""; 0]);
}
