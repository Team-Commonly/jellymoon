//! Work that waits shows itself: a look that finds a wish's run ended puts the wish's work on the page, as a tap on
//! its character does, and never over a note a person may be about to tap or type in. The wishes are made and run
//! by the `jellymoon` command with the made-up agent of F39's tests, and the looks and taps are carried by the
//! made-up window of F47 (`support/tapper.rs`), a process of its own. Nothing here starts the real Claude Code or
//! opens a window.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::{fs, thread};

use serde_json::Value;
use sticker_kit::{Note, Scene, State};
use support::day::DAY;
use support::wished::{SAID, World, quiet, wait_until};

const CHOICES: [&str; 3] = ["Show me", "Keep it", "Toss it"];
const TOSSED: &str = "The change was thrown away.";
const KEPT: &str = "The change was kept.";
/// The looks that show no work after a tap on an option or core's answer to it (`showing.rs`).
const HOLD: usize = 3;

/// A day nothing was typed on, so no clash is on its page, with `alpha` listed as a person lists it.
fn day() -> World {
    let w = quiet();
    let repo = w.at("work").join("alpha");
    let added = w.jellymoon(&["project", "add", "--repo", repo.to_str().unwrap(), "--date", DAY]);
    assert_eq!(added["name"], "alpha", "{added}");
    fs::create_dir(w.at("harness")).unwrap();
    w
}

/// A wish made for `alpha`, as a person types it: its id.
fn wish(w: &World) -> String {
    let made = w.jellymoon(&["wish", "--date", DAY, "--say", SAID, "--project", "alpha"]);
    made["wish"]["id"].as_str().unwrap().to_string()
}

/// The wish's run, from its start to its end: what it ended as.
fn run(w: &World, id: &str) -> String {
    let logs = w.at("logs");
    let ran = w.jellymoon(&["run", "--wish", id, "--date", DAY, "--logs", logs.to_str().unwrap()]);
    ran["outcome"].as_str().unwrap().to_string()
}

/// A wish whose run ended with the made-up agent playing `script`.
fn ended(w: &World, script: &str, outcome: &str) -> String {
    w.play(script);
    let id = wish(w);
    assert_eq!(run(w, &id), outcome);
    id
}

/// How core keeps the made wish `id`.
fn state(w: &World, id: &str) -> String {
    let kept = fs::read_to_string(w.at("jm").join("pages").join(format!("{DAY}.wishes.json"))).unwrap_or_default();
    let kept: Value = serde_json::from_str(&kept).unwrap_or_default();
    let wishes = kept["wishes"].as_array().cloned().unwrap_or_default();
    wishes.iter().find(|wish| wish["id"] == id).map_or(String::new(), |wish| wish["state"].as_str().unwrap().into())
}

/// Runs `then` while the runs of `ids` go, each waiting for the test's word, which is given by the `go` handed to
/// `then`: it comes back when every one of those runs has ended, each with its one commit.
fn going(w: &World, ids: &[&str], then: impl FnOnce(&dyn Fn())) {
    let before = w.starts();
    w.play("slow");
    thread::scope(|scope| {
        let runs: Vec<_> = ids.iter().map(|id| scope.spawn(move || run(w, id))).collect();
        wait_until("the agents", || w.starts() == before + ids.len());
        wait_until("the runs", || ids.iter().all(|id| state(w, id) == "working"));
        let runs = std::cell::RefCell::new(runs);
        then(&|| {
            fs::write(w.at("rec").join("go"), "").unwrap();
            for run in runs.borrow_mut().drain(..) {
                assert_eq!(run.join().unwrap(), "completed");
            }
        });
        assert!(runs.borrow().is_empty(), "the test let the runs end");
    });
}

fn id_of(note: &Note) -> Option<&str> {
    match note {
        Note::Text { id, .. } | Note::Choice { id, .. } | Note::Told { id, .. } => Some(id),
        _ => None,
    }
}

/// The ids of the notes of a scene that have one, in their order.
fn ids(scene: &Scene) -> Vec<&str> {
    scene.notes.iter().filter_map(id_of).collect()
}

fn note<'a>(scene: &'a Scene, id: &str) -> Option<&'a Note> {
    scene.notes.iter().find(|note| id_of(note) == Some(id))
}

fn options(scene: &Scene, id: &str) -> Vec<String> {
    match note(scene, id) {
        Some(Note::Text { options, .. }) => options.clone(),
        _ => Vec::new(),
    }
}

fn title<'a>(scene: &'a Scene, id: &str) -> &'a str {
    match note(scene, id) {
        Some(Note::Text { title, .. }) => title,
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
/// whole, and each character's id and place.
fn in_reach(line: &str) -> (Vec<Value>, Vec<(String, [f64; 4])>) {
    let placed: Vec<Value> = serde_json::from_str(line).expect(line);
    let kind = |piece: &Value, kinds: &[&str]| kinds.contains(&piece["kind"].as_str().unwrap());
    let taps = placed.iter().filter(|piece| kind(piece, &["option", "field"])).cloned().collect();
    let little = placed.iter().filter(|piece| kind(piece, &["character"]));
    (taps, little.map(|piece| (piece["id"].as_str().unwrap().to_string(), support::rect(piece))).collect())
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
    // And a tap on the character of work on show does nothing, as before.
    let both = w.taps("logs", &["look", "poke:m1", "look"]).showed();
    assert!(both[0].is_some() && both[1].is_none() && both[2].is_none(), "{both:?}");
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
    // `layout` has no tap, so it prints the choice to stop the run first; the page's own notes start with this.
    assert_eq!(placed.get(1), Some(&going), "the first of the page's own notes at layout: {placed:?}");
    let [before, ended, after, again] = &lines[..] else { panic!("four lines: {lines:#?}") };
    assert!(said(serde_json::from_str(before).unwrap()).contains(&going), "{before}");
    // The sentence is no note in the person's hand: the look after the run's end shows the work, and it is gone.
    let shown = scene(ended).expect("the look after the run's end shows a scene");
    assert_eq!((ids(&shown), options(&shown, "work-m1")), (vec!["work-m1"], CHOICES.map(String::from).to_vec()));
    let words = said(serde_json::from_str(after).unwrap());
    assert!(words.iter().all(|note| !note.contains("being worked on")), "{words:?}");
    assert_eq!(again, "null");
}

/// Each answer said only "The change was kept." or "thrown away": three of them on a page told nobody which was
/// of which wish. The answer to a tap takes the place of what was said of another wish's work.
#[test]
fn one_answer_stands_on_the_page_and_it_is_of_the_work_just_kept_or_tossed() {
    let w = day();
    assert_eq!((ended(&w, "edits", "completed"), ended(&w, "edits", "completed")), ("m1".into(), "m2".into()));
    let acts = ["look", "choose:work-m2:2", "run", "look", "look", "look", "look", "choose:work-m1:1", "run"];
    let lines = w.taps("logs", &acts).ended();
    let [_, _, _, _, _, _, turn, at_once, kept] = &lines[..] else { panic!("nine lines: {lines:#?}") };
    // The older work comes under the eyes with the toss's answer still there: a look takes no answer away.
    assert_eq!(ids(&scene(turn).expect("the older wish's work has its turn")), ["work-m1", "said-m2"]);
    assert_eq!(ids(&scene(at_once).expect("a tap on Keep it")), ["busy-m1", "said-m2"]);
    let kept = scene(kept).expect("core's answer");
    assert_eq!((ids(&kept), title(&kept, "said-m1")), (vec!["said-m1"], KEPT));
}

#[test]
fn of_work_that_waited_when_the_window_opened_the_newest_is_shown_and_the_older_has_its_turn() {
    let w = day();
    let (first, second) = (ended(&w, "edits", "completed"), ended(&w, "edits", "completed"));
    assert_eq!((first.as_str(), second.as_str()), ("m1", "m2"));
    let acts = ["look", "look", "choose:work-m2:2", "run", "look", "look", "look", "look", "look"];
    let lines = w.taps("logs", &acts).ended();
    let [opened, again, at_once, tossed, later, last, held, turn, after] = &lines[..] else {
        panic!("nine lines: {lines:#?}")
    };
    let opened = scene(opened).expect("the first look shows the work that waits");
    assert_eq!(ids(&opened), ["work-m2"], "the newest wish's work, and that alone");
    assert_eq!(options(&opened, "work-m2"), CHOICES);
    assert_eq!(again, "null");
    assert_eq!(ids(&scene(at_once).expect("a tap on Toss it")), ["busy-m2"]);
    let tossed = scene(tossed).expect("core's answer");
    assert_eq!((ids(&tossed), title(&tossed, "said-m2")), (vec!["said-m2"], TOSSED));
    // The older wish's work waited at the window's opening too: it is not put under a second tap.
    assert!(no_work(later, "m1") && no_work(last, "m1") && no_work(held, "m1"), "{later} {last} {held}");
    let last = [later, last, held].into_iter().filter_map(|line| scene(line)).next_back().unwrap_or(tossed);
    assert_eq!(ids(&last), ["said-m2"], "what was said of the toss stays");
    // Past the looks held back it has its turn: nothing else on the page says a change still waits.
    let turn = scene(turn).expect("the older wish's work has its turn");
    assert_eq!(
        (ids(&turn), options(&turn, "work-m1")),
        (vec!["work-m1", "said-m2"], CHOICES.map(String::from).to_vec())
    );
    assert_eq!(after, "null", "and it is in the person's hand");
    // A tap on its character opens it, as before.
    assert_eq!(ids(&scene(&w.taps("logs", &["poke:m1"]).ended()[0]).expect("a tap opens the wish")), ["work-m1"]);
}

#[test]
fn work_is_not_opened_over_work_on_show_whose_options_and_field_stay_where_they_are() {
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
    let [poked, before, ended, after, again, at_once, tossed, held @ .., turn, last] = &lines[..] else {
        panic!("nine lines and the looks held: {lines:#?}")
    };
    assert_eq!(ids(&scene(poked).expect("a tap opens the wish")), ["work-m1"]);
    // The other wish's run ended: its star is drawn, and its work is not put over the work on show.
    let ended = scene(ended).expect("the look draws the run's end");
    assert_eq!(drawn(&ended, "m2"), Some((State::Idle, 1)));
    assert_eq!(ids(&ended), ["work-m1"]);
    let (taps, little) = in_reach(before);
    assert!(taps.iter().filter(|piece| piece["kind"] == "option").count() == 3, "{before}");
    assert!(taps.iter().any(|piece| piece["kind"] == "field" && piece["id"] == "work-m1"), "{before}");
    assert_eq!((taps, little), in_reach(after), "no option, field or character moved");
    assert!(no_work(again, "m2"), "{again}");
    // The person decides on the work on show. What was said of the toss stands alone for the looks held back:
    // no other work's `Keep it` and `Toss it` come where a second tap would land.
    assert_eq!(ids(&scene(at_once).expect("a tap on Toss it")), ["busy-m1"]);
    assert_eq!(ids(&scene(tossed).expect("core's answer")), ["said-m1"]);
    assert_eq!(held, ["null"; HOLD], "nothing is put over what was said of the toss");
    // Then the other's turn comes, which was kept: its work, over what was said.
    let turn = scene(turn).expect("the look after the hold shows the work that waited its turn");
    assert_eq!(ids(&turn), ["work-m2", "said-m1"]);
    assert_eq!(options(&turn, "work-m2"), CHOICES);
    assert_eq!(title(&turn, "said-m1"), TOSSED);
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
    assert_eq!(ids(&ended), ["wish-m1"], "no note is over the one with the field");
    assert_eq!(note(&ended, "wish-m1"), note(&poked, "wish-m1"), "and that note is as it was");
    let (taps, little) = in_reach(before);
    assert!(taps.iter().any(|piece| piece["kind"] == "field" && piece["id"] == "wish-m1"), "{before}");
    assert_eq!((taps, little), in_reach(after), "the field is where it was");
    assert!(no_work(again, "m2"), "{again}");
}

#[test]
fn of_two_runs_that_ended_the_first_wishs_work_is_shown_and_the_other_waits_its_turn() {
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
    assert_eq!(ids(&ended), ["work-m1"], "one work is shown: the first wish's, in core's order");
    assert_eq!(again, "null");
    assert_eq!(ids(&scene(at_once).expect("a tap on Keep it")), ["busy-m1"]);
    assert_eq!(slow, ["null"; HOLD], "nothing is put over a choice core has not answered");
    let kept = scene(kept).expect("core's answer");
    assert_eq!((ids(&kept), title(&kept, "said-m1")), (vec!["said-m1"], KEPT));
    // The looks held back are counted from the answer: the other's work is not put under a second tap.
    assert_eq!((first.as_str(), second.as_str()), ("null", "null"), "what was said of the keep stays, alone");
    // A tap on the other's character is the person's own act: it opens that work at once, within the hold.
    assert_eq!(ids(&scene(poked).expect("a tap opens the wish")), ["work-m2"]);
    assert_eq!(last, "null");
}

#[test]
fn work_is_not_opened_over_a_choice_to_stop_a_run_and_has_its_turn_when_that_choice_is_gone() {
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
    let [poked, first, second, ended, held @ .., turn, last] = &lines[..] else {
        panic!("four lines and the looks held: {lines:#?}")
    };
    // A tap on the wish whose run goes: the choice to stop it. The older wish's work waited at the opening, and
    // no look puts it over that choice.
    assert_eq!(ids(&scene(poked).expect("a tap on a wish at work")), ["stop-m2"]);
    assert_eq!((first.as_str(), second.as_str()), ("null", "null"));
    // The run ended: the look draws its end and takes the choice, as before, and puts nothing in its place.
    let ended = scene(ended).expect("the look draws the run's end");
    assert_eq!((drawn(&ended, "m2"), ids(&ended)), (Some((State::Idle, 1)), Vec::new()));
    // The page left the person's hand with no tap: the looks held after a tap are held after it too, so no
    // `Toss it` comes where `Stop it` stood under a tap meant for it.
    assert_eq!(held, ["null"; HOLD], "no work in the looks after the choice went");
    // Then the work waited its turn: the newest wish's.
    assert_eq!(ids(&scene(turn).expect("the look after the hold")), ["work-m2"]);
    assert_eq!(last, "null");
}
