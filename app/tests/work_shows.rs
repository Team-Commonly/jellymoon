//! Work that waits shows itself: a look that finds a wish's run ended puts the wish's work on the page, as a tap on
//! its character does, under that wish's own words, and takes no note of another wish away (ADR 0030; the rules
//! of several wishes at once are in `several_wishes.rs`). The wishes are made and run
//! by the `jellymoon` command with the made-up agent of F39's tests, and the looks and taps are carried by the
//! made-up window of F47 (`support/tapper.rs`), a process of its own. Nothing here starts the real Claude Code or
//! opens a window.
#![allow(clippy::unwrap_used, clippy::expect_used)]

#[path = "work_shows/runs.rs"]
mod runs;
mod support;

use std::fs;

use runs::{day, ended, going, wish};
use serde_json::{Value, json};
use sticker_kit::{Note, Scene, State};
use support::wished::{SAID, World};

const CHOICES: [&str; 3] = ["Show me", "Keep it", "Toss it"];
const TOSSED: &str = "The change was thrown away.";
const KEPT: &str = "The change was kept.";
/// The looks that leave a wish's notes as they are after a tap on an option of one or core's answer to it
/// (`showing/turns.rs`).
const HOLD: usize = 3;

fn id_of(note: &Note) -> Option<&str> {
    match note {
        Note::Text { id, .. } | Note::Choice { id, .. } | Note::Told { id, .. } => Some(id),
        Note::Drawn { id, .. } if id.starts_with("work-") => Some(id),
        _ => None,
    }
}

/// The ids of the notes of a scene that have one, in their order.
fn ids(scene: &Scene) -> Vec<&str> {
    scene.notes.iter().filter_map(id_of).collect()
}

/// [`ids`] in the order of the letters: which notes a page holds, whatever their order on the scene, which the
/// column gives them anyway.
fn held(scene: &Scene) -> Vec<&str> {
    let mut ids = ids(scene);
    ids.sort_unstable();
    ids
}

fn note<'a>(scene: &'a Scene, id: &str) -> Option<&'a Note> {
    scene.notes.iter().find(|note| id_of(note) == Some(id))
}

fn options(scene: &Scene, id: &str) -> Vec<String> {
    match note(scene, id) {
        Some(Note::Text { options, .. } | Note::Drawn { options, .. }) => options.clone(),
        _ => Vec::new(),
    }
}

fn title<'a>(scene: &'a Scene, id: &str) -> &'a str {
    match note(scene, id) {
        Some(Note::Text { title, .. }) => title,
        Some(Note::Drawn { words, .. }) => words.first().map_or("", String::as_str),
        _ => "",
    }
}

fn drawn(scene: &Scene, id: &str) -> Option<(State, u8)> {
    scene.characters.iter().find(|one| one.id == id).map(|one| (one.state, one.stars))
}

/// A line the made-up window printed, as the scene it showed, or none when it showed nothing.
fn scene(line: &str) -> Option<Scene> {
    serde_json::from_str(line).expect(line)
}

/// Whether a line of a look shows no work of `id`: nothing shown, or a scene without that note.
fn no_work(line: &str, id: &str) -> bool {
    scene(line).is_none_or(|scene| note(&scene, &format!("work-{id}")).is_none())
}

/// What a person can tap or type in, of the pieces a `placed` line holds: every option and field as it is placed,
/// whole, with its place down the column measured from the top of its own card, to a tenth; and each character's
/// id, its place across and its size. The page is one column (ADR 0026): a card stands under the cards before it,
/// so a note that comes or goes higher up moves it down or up whole, and nothing inside it or beside it.
fn in_reach(line: &str) -> (Vec<Value>, Vec<(String, [f64; 3])>) {
    let placed: Vec<Value> = serde_json::from_str(line).expect(line);
    let kind = |piece: &Value, kinds: &[&str]| kinds.contains(&piece["kind"].as_str().unwrap());
    let card = |id: &Value| placed.iter().find(|piece| piece["kind"] == "note" && piece["id"] == *id);
    let in_card = |piece: &Value| {
        let top = card(&piece["id"]).expect("the card it is on")["y"].as_f64().unwrap();
        let from_top =
            |value: &mut Value| value["y"] = json!(((value["y"].as_f64().unwrap() - top) * 10.0).round() / 10.0);
        let mut piece = piece.clone();
        from_top(&mut piece);
        piece["text"].as_array_mut().unwrap().iter_mut().for_each(from_top);
        piece
    };
    let taps = placed.iter().filter(|piece| kind(piece, &["option", "field"])).map(in_card).collect();
    let across = |piece: &Value| {
        let [x, _, w, h] = support::rect(piece);
        (piece["id"].as_str().unwrap().to_string(), [x, w, h])
    };
    (taps, placed.iter().filter(|piece| kind(piece, &["character"])).map(across).collect())
}

/// Leaves the file a `hold` waits for.
fn release(w: &World, name: &str) {
    fs::write(w.at("tmp").join(name), "").unwrap();
}

fn hold(w: &World, name: &str) -> String {
    format!("hold:{}", w.at("tmp").join(name).display())
}

#[test]
fn a_look_that_finds_a_run_ended_shows_its_work_as_a_tap_does_and_once() {
    let w = day();
    let id = wish(&w);
    assert_eq!(id, "m1");
    let mut lines = Vec::new();
    going(&w, &["m1"], |go| {
        let mut window = w.taps("logs", &["look", "placed", &hold(&w, "ended"), "look", "placed", "look", "look"]);
        window.printed(2);
        go();
        release(&w, "ended");
        lines = window.ended();
    });
    let [going, before, ended, after, again, third] = &lines[..] else { panic!("six lines: {lines:#?}") };
    assert!(no_work(going, "m1"), "the run goes: {going}");
    let shown = scene(ended).expect("the look after the run's end shows a scene");
    assert_eq!(drawn(&shown, "m1"), Some((State::Idle, 1)), "the star, as a look drew it before");
    assert_eq!(ids(&shown), ["work-m1"], "the work's note, and no other of a wish");
    assert_eq!(options(&shown, "work-m1"), CHOICES);
    assert_eq!(title(&shown, "work-m1"), "Your folder is not changed yet.");
    // Shown once: the next looks have nothing new, and ask nothing.
    assert_eq!((again.as_str(), third.as_str()), ("null", "null"));
    // No character moved because the run ended and its work was shown.
    assert_eq!(in_reach(before).1, in_reach(after).1);
    assert!(in_reach(before).0.is_empty() && in_reach(after).0.len() >= 3, "{after}");
    // The notes are the ones a tap on the character gives, in a window that never looked.
    let tapped = w.taps("logs", &["poke:m1"]).showed().remove(0).expect("a tap opens the wish");
    assert_eq!(shown.notes, tapped.notes);
    // And a tap on the character of work on show changes no note: it opens the words of the drawn one.
    let both = w.taps("logs", &["look", "poke:m1", "look"]).showed();
    assert!(both[0].is_some() && both[2].is_none(), "{both:?}");
    assert_eq!(both[0].as_ref().map(|scene| &scene.notes), both[1].as_ref().map(|scene| &scene.notes));
    // The looks wrote nothing: the branch, the run's record and the run's copy are as the run left them.
    let repo = w.at("work").join("alpha");
    let was = w.everything(&repo);
    let looked = w.taps("logs", &["look", "look", "look"]).showed();
    assert_eq!(looked[0].as_ref().map(ids), Some(vec!["work-m1"]), "a look that reads the work");
    assert_eq!(w.everything(&repo), was, "a look wrote something");
}

/// The words of the notes of a `placed` line, or of pieces `layout` placed, each its title and body.
fn said(placed: Vec<Value>) -> Vec<String> {
    let notes = placed.into_iter().filter(|piece| piece["kind"] == "note");
    notes.map(|piece| support::Piece(piece).said()).collect()
}

#[test]
fn the_page_says_a_wish_is_being_worked_on_and_its_work_still_shows_itself_when_the_run_ends() {
    let w = day();
    assert_eq!(wish(&w), "m1");
    let (mut lines, mut placed) = (Vec::new(), Vec::new());
    going(&w, &["m1"], |go| {
        placed = said(w.placed().into_iter().map(|piece| piece.0).collect());
        let mut window = w.taps("logs", &["placed", &hold(&w, "ended"), "look", "placed", "look"]);
        window.printed(1);
        go();
        release(&w, "ended");
        lines = window.ended();
    });
    let going = format!("Your wish is being worked on. “{SAID}”. What it did will show here when it is done.");
    // The page's own notes start with this; the choice to stop the run stands under the wish's own words.
    assert_eq!(placed.first(), Some(&going), "the first of the page's own notes at layout: {placed:?}");
    let [before, ended, after, again] = &lines[..] else { panic!("four lines: {lines:#?}") };
    assert!(said(serde_json::from_str(before).unwrap()).contains(&going), "{before}");
    // The sentence is no note in the person's hand: the look after the run's end shows the work, and it is gone.
    let shown = scene(ended).expect("the look after the run's end shows a scene");
    assert_eq!((ids(&shown), options(&shown, "work-m1")), (vec!["work-m1"], CHOICES.map(String::from).to_vec()));
    let words = said(serde_json::from_str(after).unwrap());
    assert!(words.iter().all(|note| !note.contains("being worked on")), "{words:?}");
    assert_eq!(again, "null");
}

/// Each answer says only "The change was kept." or "thrown away". It stands under the typed words of its own
/// wish (`column.rs`), so two on a page are told apart, and the answer to a tap takes nothing of another wish away.
#[test]
fn each_answer_stays_under_its_own_wish_when_another_wish_s_work_is_kept_or_tossed() {
    let w = day();
    assert_eq!((ended(&w, "edits", "completed"), ended(&w, "edits", "completed")), ("m1".into(), "m2".into()));
    let acts = ["look", "choose:work-m2:2", "run", "look", "look", "look", "look", "choose:work-m1:1", "run"];
    let lines = w.taps("logs", &acts).ended();
    let [_, _, _, _, _, _, after, at_once, kept] = &lines[..] else { panic!("nine lines: {lines:#?}") };
    // Past the looks held after the toss its wish is drawn as it stands, over the answer: a look takes none away.
    assert_eq!(held(&scene(after).expect("the tossed wish as it stands")), ["said-m2", "wish-m2", "work-m1"]);
    assert_eq!(held(&scene(at_once).expect("a tap on Keep it")), ["busy-m1", "said-m2", "wish-m2"]);
    let kept = scene(kept).expect("core's answer");
    assert_eq!(held(&kept), ["said-m1", "said-m2", "wish-m2"]);
    assert_eq!((title(&kept, "said-m1"), title(&kept, "said-m2")), (KEPT, TOSSED));
}

#[test]
fn work_that_waited_when_the_window_opened_is_all_shown_and_a_toss_of_one_leaves_the_other_s_choices() {
    let w = day();
    let (first, second) = (ended(&w, "edits", "completed"), ended(&w, "edits", "completed"));
    assert_eq!((first.as_str(), second.as_str()), ("m1", "m2"));
    let acts = ["look", "look", "choose:work-m2:2", "run", "look", "look", "look", "look", "look"];
    let lines = w.taps("logs", &acts).ended();
    let [opened, again, at_once, tossed, looks @ ..] = &lines[..] else { panic!("nine lines: {lines:#?}") };
    let opened = scene(opened).expect("the first look shows the work that waits");
    assert_eq!(held(&opened), ["work-m1", "work-m2"], "the work of every wish, each with its choices");
    assert_eq!(
        (options(&opened, "work-m1"), options(&opened, "work-m2")),
        (CHOICES.map(String::from).to_vec(), CHOICES.map(String::from).to_vec())
    );
    assert_eq!(again, "null");
    assert_eq!(held(&scene(at_once).expect("a tap on Toss it")), ["busy-m2", "work-m1"]);
    let tossed = scene(tossed).expect("core's answer");
    assert_eq!((held(&tossed), title(&tossed, "said-m2")), (vec!["said-m2", "work-m1"], TOSSED));
    // The older wish's work is on the page all the while, with its choices, and what was said of the toss stays.
    for shown in looks.iter().filter_map(|line| scene(line)).chain([tossed]) {
        assert_eq!(options(&shown, "work-m1"), CHOICES, "{shown:?}");
        assert_eq!(title(&shown, "said-m2"), TOSSED, "{shown:?}");
    }
    // A tap on its character opens it, as before.
    assert_eq!(ids(&scene(&w.taps("logs", &["poke:m1"]).ended()[0]).expect("a tap opens the wish")), ["work-m1"]);
}

/// What `in_reach` gives of the note `id`: its options and its field.
fn of_note(line: &str, id: &str) -> Vec<Value> {
    in_reach(line).0.into_iter().filter(|piece| piece["id"] == id).collect()
}

#[test]
fn work_that_ends_comes_under_its_own_words_and_the_options_of_work_on_show_stay_where_they_are() {
    let w = day();
    assert_eq!(ended(&w, "edits", "completed"), "m1");
    assert_eq!(wish(&w), "m2");
    let mut lines = Vec::new();
    going(&w, &["m2"], |go| {
        let held = hold(&w, "ended");
        let acts = ["poke:m1", "placed", &held, "look", "placed", "look", "choose:work-m1:2", "run"];
        let mut window = w.taps("logs", &[&acts[..], &["look"; HOLD + 2][..]].concat());
        window.printed(2);
        go();
        release(&w, "ended");
        lines = window.ended();
    });
    let [poked, before, ended, after, again, at_once, tossed, quiet @ .., stands, last] = &lines[..] else {
        panic!("nine lines and the looks held: {lines:#?}")
    };
    assert_eq!(ids(&scene(poked).expect("a tap opens the wish")), ["work-m1"]);
    // The other wish's run ended: its star is drawn, and its work is on the page with the work on show.
    let ended = scene(ended).expect("the look draws the run's end");
    assert_eq!(drawn(&ended, "m2"), Some((State::Idle, 1)));
    assert_eq!(held(&ended), ["work-m1", "work-m2"]);
    let choices = |line: &str| -> Vec<Value> {
        of_note(line, "work-m1").into_iter().filter(|piece| piece["kind"] == "option").collect()
    };
    assert!(choices(before).len() == 3, "{before}");
    assert!(of_note(before, "work-m1").iter().any(|piece| piece["kind"] == "field"), "{before}");
    // The newer wish stands under the older in the column, so no option of the work on show moved. The field
    // of a follow-up is drawn under one wish: `placed` lays the scene out as `layout` does, which knows no tap,
    // so there it is under the newest wish that may be followed (the window's rule is in `several_wishes.rs`).
    assert_eq!(choices(before), choices(after), "no option of the work on show moved");
    assert!(of_note(after, "work-m2").iter().any(|piece| piece["kind"] == "field"), "{after}");
    assert_eq!(in_reach(before).1, in_reach(after).1, "no character moved");
    assert_eq!(again, "null");
    // The person decides on the older work: the newer keeps its choices under its own words meanwhile.
    assert_eq!(held(&scene(at_once).expect("a tap on Toss it")), ["busy-m1", "work-m2"]);
    assert_eq!(held(&scene(tossed).expect("core's answer")), ["said-m1", "work-m2"]);
    assert_eq!(quiet, ["null"; HOLD], "what was said of the toss is read before its wish is drawn again");
    // Then the tossed wish is as core says it stands, over the answer, which stays.
    let stands = scene(stands).expect("the look after the hold");
    assert_eq!(held(&stands), ["said-m1", "wish-m1", "work-m2"]);
    assert_eq!((options(&stands, "work-m2"), title(&stands, "said-m1")), (CHOICES.map(String::from).to_vec(), TOSSED));
    assert_eq!(last, "null");
}

#[test]
fn a_field_to_type_in_keeps_its_note_and_its_place_when_a_run_ends() {
    let w = day();
    assert_eq!(ended(&w, "asks", "asks"), "m1");
    assert_eq!(wish(&w), "m2");
    let mut lines = Vec::new();
    going(&w, &["m2"], |go| {
        let mut window = w.taps("logs", &["poke:m1", "placed", &hold(&w, "ended"), "look", "placed", "look"]);
        window.printed(2);
        go();
        release(&w, "ended");
        lines = window.ended();
    });
    let [poked, before, ended, after, again] = &lines[..] else { panic!("five lines: {lines:#?}") };
    let poked = scene(poked).expect("a tap opens the wish that asks");
    assert!(matches!(note(&poked, "wish-m1"), Some(Note::Text { field: Some(_), options, .. }) if options.is_empty()));
    let ended = scene(ended).expect("the look draws the run's end");
    assert_eq!(drawn(&ended, "m2"), Some((State::Idle, 1)));
    assert_eq!(held(&ended), ["wish-m1", "work-m2"], "the other wish's work, and the note with the field");
    assert_eq!(note(&ended, "wish-m1"), note(&poked, "wish-m1"), "and that note is as it was");
    let taps = of_note(before, "wish-m1");
    assert!(taps.iter().any(|piece| piece["kind"] == "field"), "{before}");
    assert_eq!(taps, of_note(after, "wish-m1"), "the field is where it was");
    assert_eq!(in_reach(before).1, in_reach(after).1, "no character moved");
    assert_eq!(again, "null");
}

#[test]
fn two_runs_that_ended_show_both_their_work_and_a_keep_of_one_leaves_the_other_s_choices() {
    let w = day();
    assert_eq!((wish(&w), wish(&w)), ("m1".to_string(), "m2".to_string()));
    let mut lines = Vec::new();
    going(&w, &["m1", "m2"], |go| {
        // `Keep it`, and core's answer is long in coming: as many looks pass before it as are held back after.
        let acts = ["look", &hold(&w, "ended"), "look", "look", "choose:work-m1:1"];
        let after = ["run", "look", "look", "poke:m2", "look"];
        let mut window = w.taps("logs", &[&acts[..], &["look"; HOLD][..], &after[..]].concat());
        window.printed(1);
        go();
        release(&w, "ended");
        lines = window.ended();
    });
    let [_, ended, again, at_once, slow @ .., kept, first, second, poked, last] = &lines[..] else {
        panic!("nine lines and the looks before the answer: {lines:#?}")
    };
    let ended = scene(ended).expect("the look after both ends");
    assert_eq!((drawn(&ended, "m1"), drawn(&ended, "m2")), (Some((State::Idle, 1)), Some((State::Idle, 1))));
    assert_eq!(held(&ended), ["work-m1", "work-m2"], "the work of both, each with its choices");
    assert_eq!(
        (options(&ended, "work-m1"), options(&ended, "work-m2")),
        (CHOICES.map(String::from).to_vec(), CHOICES.map(String::from).to_vec())
    );
    assert_eq!(again, "null");
    assert_eq!(held(&scene(at_once).expect("a tap on Keep it")), ["busy-m1", "work-m2"]);
    assert_eq!(slow, ["null"; HOLD], "nothing changes while core has not answered");
    let kept = scene(kept).expect("core's answer");
    assert_eq!((held(&kept), title(&kept, "said-m1")), (vec!["said-m1", "work-m2"], KEPT));
    // The looks held back are counted from the answer: what was said of the keep is as it was.
    for look in [first, second, last] {
        assert!(scene(look).is_none_or(|shown| held(&shown) == ["said-m1", "work-m2"]), "{look}");
    }
    // A tap on the other's character is the person's own act, and its work is there with its choices.
    let poked = scene(poked).expect("a tap on the wish");
    assert_eq!(
        (held(&poked), options(&poked, "work-m2")),
        (vec!["said-m1", "work-m2"], CHOICES.map(String::from).to_vec())
    );
}

#[test]
fn a_choice_to_stop_a_run_stays_while_another_wish_s_work_comes_and_goes_when_its_run_ends() {
    let w = day();
    assert_eq!(ended(&w, "edits", "completed"), "m1");
    assert_eq!(wish(&w), "m2");
    let mut lines = Vec::new();
    going(&w, &["m2"], |go| {
        let acts = ["poke:m2", "look", "look", &hold(&w, "ended"), "look"];
        let mut window = w.taps("logs", &[&acts[..], &["look"; HOLD + 2][..]].concat());
        window.printed(3);
        go();
        release(&w, "ended");
        lines = window.ended();
    });
    let [poked, first, second, ended, rest @ ..] = &lines[..] else { panic!("four lines and more: {lines:#?}") };
    // A tap on the wish whose run goes: the choice to stop it. The older wish's work waited at the opening: the
    // first look shows it, under its own wish's words, and the choice stays.
    assert_eq!(ids(&scene(poked).expect("a tap on a wish at work")), ["stop-m2"]);
    assert_eq!(held(&scene(first).expect("the window's first look")), ["stop-m2", "work-m1"]);
    assert_eq!(second, "null");
    // The run ended: the look draws its end, takes the choice, as before, and shows its work with the older.
    let ended = scene(ended).expect("the look draws the run's end");
    assert_eq!((drawn(&ended, "m2"), held(&ended)), (Some((State::Idle, 1)), vec!["work-m1", "work-m2"]));
    assert_eq!(rest, ["null"; HOLD + 2], "and the looks after it have nothing new");
}
