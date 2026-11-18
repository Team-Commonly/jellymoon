//! Whose turn it is to be shown by a look, with no core and no window: the scenes are typed here.

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

/// Whom the look of `from` asks core of, when it draws `now`.
fn asked(turns: &mut Turns, from: &Scene, now: &Scene) -> Option<(Asks, Vec<String>)> {
    turns.asks(from).map(|asks| {
        let whom = asks.whom(now);
        (asks, whom)
    })
}

#[test]
fn core_is_asked_of_a_wish_once_for_an_end_of_its_run_and_never_while_it_goes() {
    let (going, ended) = (page(&[State::Busy], Vec::new()), page(&[State::Idle], Vec::new()));
    let mut turns = Turns::default();
    // The window opens on a wish at work: nothing is asked while its run goes, look after look.
    for _ in 0..3 {
        assert_eq!(asked(&mut turns, &going, &going).map(|(_, whom)| whom), Some(Vec::new()));
    }
    // The look that draws the run's end asks, and is told there is no work.
    let (asks, whom) = asked(&mut turns, &going, &ended).expect("nothing is in hand");
    assert_eq!(whom, ["m1"]);
    asks.none("m1".to_string());
    // No look after it asks again, and the wish asked of at the opening is not asked of again either.
    for _ in 0..3 {
        assert_eq!(asked(&mut turns, &ended, &ended).map(|(_, whom)| whom), Some(Vec::new()));
    }
    // Its next run is another end.
    assert_eq!(asked(&mut turns, &going, &ended).map(|(_, whom)| whom), Some(vec!["m1".to_string()]));
}

#[test]
fn work_that_was_shown_has_had_its_turn_and_a_look_that_was_dropped_is_made_again() {
    let (going, ended) = (page(&[State::Busy], Vec::new()), page(&[State::Idle], Vec::new()));
    let shown = page(&[State::Idle], vec![text("work-m1", &["Show me"])]);
    let mut turns = Turns::default();
    assert!(asked(&mut turns, &going, &going).is_some());
    assert_eq!(asked(&mut turns, &going, &ended).map(|(_, whom)| whom), Some(vec!["m1".to_string()]));
    // The look's scene was dropped, the page having changed under it: the wish is asked of again.
    turns.saw(None, true);
    assert_eq!(asked(&mut turns, &going, &ended).map(|(_, whom)| whom), Some(vec!["m1".to_string()]));
    // Shown, and then taken off the page by a tap elsewhere: it is not shown by itself again.
    turns.saw(Some(&shown), true);
    assert_eq!(asked(&mut turns, &ended, &ended).map(|(_, whom)| whom), Some(Vec::new()));
}

#[test]
fn the_first_look_asks_the_done_wishes_newest_first_and_a_note_in_hand_keeps_every_turn() {
    let done = page(&[State::Idle, State::Dimmed, State::Idle], Vec::new());
    let mut turns = Turns::default();
    // A choice on show: nothing is asked, and the first look is still owed.
    let held = page(&[State::Idle, State::Dimmed, State::Idle], vec![text("work-m1", &["Show me"])]);
    assert!(turns.asks(&held).is_none() && turns.first == First::Due);
    assert_eq!(asked(&mut turns, &done, &done).map(|(_, whom)| whom), Some(vec!["m3".to_string(), "m1".to_string()]));
    // A first look that was dropped with work in it is owed again; one that was shown is not.
    turns.saw(None, true);
    assert_eq!(turns.first, First::Due);
    assert_eq!(asked(&mut turns, &done, &done).map(|(_, whom)| whom.len()), Some(2));
    assert_eq!(asked(&mut turns, &done, &done).map(|(_, whom)| whom), Some(Vec::new()));
    // A wish made since the window opened has its turn when its run is not going, in core's order.
    let more = page(&[State::Idle, State::Dimmed, State::Idle, State::NeedsYou, State::Busy], Vec::new());
    assert!(asked(&mut turns, &more, &more).is_some());
    assert_eq!(asked(&mut turns, &more, &more).map(|(_, whom)| whom), Some(vec!["m4".to_string()]));
}

#[test]
fn no_look_shows_work_in_the_looks_after_an_act_of_the_person_and_the_turn_is_kept() {
    let (going, ended) = (page(&[State::Busy], Vec::new()), page(&[State::Idle], Vec::new()));
    let mut turns = Turns::default();
    assert!(asked(&mut turns, &going, &going).is_some());
    turns.acted();
    for _ in 0..HOLD {
        assert!(turns.asks(&ended).is_none(), "a look within the hold asks core of nothing");
    }
    assert_eq!(asked(&mut turns, &ended, &ended).map(|(_, whom)| whom), Some(vec!["m1".to_string()]));
    // Core's answer counts the looks from the start again.
    turns.acted();
    assert!(turns.asks(&ended).is_none());
    turns.acted();
    for _ in 0..HOLD {
        assert!(turns.asks(&ended).is_none());
    }
    assert!(turns.asks(&ended).is_some());
}

#[test]
fn a_wish_that_was_not_on_the_scene_the_look_was_made_of_is_not_asked_of_by_that_look() {
    let (one, two) = (page(&[State::Idle], Vec::new()), page(&[State::Idle, State::Idle], Vec::new()));
    let mut turns = Turns::default();
    // The first look, which asks the done wishes: a wish made and done elsewhere since is left to the next look,
    // whose page has its character.
    assert_eq!(asked(&mut turns, &one, &two).map(|(_, whom)| whom), Some(vec!["m1".to_string()]));
    assert_eq!(asked(&mut turns, &two, &two).map(|(_, whom)| whom), Some(vec!["m2".to_string()]));
}

#[test]
fn a_look_in_flight_when_the_person_taps_an_option_is_dropped_whole_and_its_turn_is_kept() {
    let going = page(&[State::Busy, State::Busy], Vec::new());
    let today = crate::today::Today {
        date: "2026-03-10".to_string(),
        logs: "none".into(),
        skip: Vec::new(),
        model: String::new(),
        home: "none".into(),
        harness: Some("none".into()),
        fresh: false,
        settings: None,
        bin: None,
    };
    let shown = Showing::of(Some(OnShow { today, page: None, scene: going.clone() }));
    // A look is made of a page with nothing in hand, and is in flight with another wish's work on its scene.
    assert!(shown.look().is_some());
    let mut late = going.clone();
    late.notes.insert(0, text("work-m2", &["Show me", "Keep it", "Toss it"]));
    // Meanwhile a tap on a character shows the choice to stop its run, and the person taps that choice.
    let mut stop = going.clone();
    let options = vec![crate::stopping::STOP.to_string()];
    let (id, title, footer) = ("stop-m1".to_string(), String::new(), String::new());
    stop.notes.insert(0, Note::Choice { id, title, options, footer, struck_first: false, whole: false });
    assert!(shown.seen(Seen { from: going.clone(), now: stop }).is_some());
    let (at_once, _) = shown.chose("stop-m1", 0).expect("the choice is on show");
    assert!(shown.seen(Seen { from: going, now: late }).is_none(), "the stale look is dropped");
    assert_eq!(shown.scene(), Some(at_once), "nothing of the look is on the page");
    let turns = shown.2.borrow();
    assert!(turns.turn.contains("m2") && turns.first == First::Due && turns.hold == HOLD);
}
