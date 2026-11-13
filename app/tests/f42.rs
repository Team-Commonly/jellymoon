//! F42: a wish typed in the window is kept and run. The rules are core's and are proved at `jellymoon wish` and
//! `jellymoon wish --run` (`crates/jellymoon/tests/f37.rs`, `f41.rs`). Here is the app's part: `layout` prints the
//! entry, with a chip for each project read that a wish can be made for; Enter, followed with no window through `OnShow`, is carried to core
//! and drawn at once; and a look draws what core keeps since. `f42/run.rs` holds the run, which a made-up window
//! (`support/wisher.rs`) asks for in a process of its own: nothing here starts the real Claude Code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

#[path = "f42/notes.rs"]
mod notes;
#[path = "f42/run.rs"]
mod run;
mod support;

use std::fs;

use jellymoon_app::taps::OnShow;
use jellymoon_app::today::Today;
use jellymoon_app::wishing::UNKEPT;
use serde_json::{Value, json};
use sticker_kit::entry::EntryState;
use sticker_kit::{Mark, Note, Scene, State};
use support::Piece;
use support::day::*;

pub use support::wished::SAID;

/// The day, with no harness and no command that could ever be reached.
fn today(world: &World) -> Today {
    Today {
        date: DAY.to_string(),
        logs: world.dir.path().join("logs"),
        skip: Vec::new(),
        model: "sonnet".to_string(),
        home: world.home.clone(),
        harness: Some(world.rec.join("no-harness")),
        fresh: false,
        settings: None,
        bin: None,
    }
}

/// The day as the window holds it, with its kept page on show.
pub fn on_show(world: &World) -> OnShow {
    let today = today(world);
    let page = today.kept().expect("the day's page is kept");
    let scene = today.draw(&page);
    OnShow { today, page: Some(page), scene }
}

/// The day read and kept, with nothing left of the reading call's record.
pub fn read() -> World {
    let world = world();
    shown(&world, &[]);
    assert!(called(&world));
    world
}

/// A scene's first note, when it is a plain one.
pub fn first(scene: &Scene) -> Option<(&str, &str)> {
    match scene.notes.first()? {
        Note::Plain { title, body } => Some((title, body)),
        _ => None,
    }
}

/// A made wish as a scene draws it: its little one's state and stars, and its row in the note of made wishes.
pub fn drawn(scene: &Scene, id: &str) -> Option<(State, u8, Mark, String)> {
    let one = scene.characters.iter().find(|one| one.id == id)?;
    let rows = scene.notes.iter().find_map(|note| match note {
        Note::Whole { title, rows } if title == "wished here today" => Some(rows),
        _ => None,
    })?;
    let place = scene.characters.iter().filter(|one| one.id.starts_with('m')).position(|one| one.id == id)?;
    let (mark, said) = rows.get(place)?.clone();
    Some((one.state, one.stars, mark, said))
}

/// The day's made wishes as they are kept. None when none was ever kept.
pub fn kept_wishes(world: &World) -> Option<Value> {
    let kept = fs::read_to_string(world.home.join("pages").join(format!("{DAY}.wishes.json"))).ok()?;
    Some(serde_json::from_str::<Value>(&kept).unwrap()["wishes"].clone())
}

fn chips(placed: &[Piece]) -> Vec<String> {
    placed.iter().filter(|piece| piece.kind() == "chip").map(Piece::said).collect()
}

fn entries(placed: &[Piece]) -> Vec<&Piece> {
    placed.iter().filter(|piece| piece.kind() == "entry").collect()
}

#[test]
fn layout_prints_one_entry_on_a_day_s_page_with_a_chip_for_each_project_read() {
    let world = world();
    for again in [false, true] {
        let placed = shown(&world, &[]);
        assert_eq!(called(&world), !again, "the second page is the kept one");
        let entry = entries(&placed);
        assert_eq!((entry.len(), entry[0].id()), (1, "wish"));
        assert_eq!(entry[0].said(), "Type a wish: Tab for its project, Enter", "the hint, whole");
        assert_eq!(chips(&placed), ["alpha", "private-notes"]);
        let last = placed.len() - 3;
        assert!(placed[last..].iter().all(|piece| ["entry", "chip"].contains(&piece.kind())), "they are printed last");
    }
    // The hint is whole at the smallest page, the design's size and one between, in both themes, as `layout`
    // measures it: by the advances of the bundled Nunito, which also stands in for the plain theme's system font.
    for (theme, size) in
        ["sticker", "plain"].iter().flat_map(|theme| ["960x640", "1200x760", "1440x900"].map(|s| (theme, s)))
    {
        let placed = shown(&world, &["--theme", theme, "--size", size]);
        let said = entries(&placed)[0].said();
        assert_eq!(said, "Type a wish: Tab for its project, Enter", "{theme} at {size}: the hint, whole");
    }
    // A folder switched off was not read, so no wish can be made for it.
    assert_eq!(chips(&shown(&world, &["--fresh", "--skip", "private-notes"])), ["alpha"]);
}

// F50: the day the app reads leaves out a run's copy, so a folder in one is no project a wish can be made for.
#[test]
fn a_session_in_a_run_s_copy_gets_no_chip_and_nothing_typed_there_is_placed() {
    let world = world();
    let copy = world.home.join("copies").join("r0123456789abcdef").join("zeta");
    write_log(&world, "-jm-copies-run/s3.jsonl", &[said(&noon(3), copy.to_str().unwrap(), "quince-heron in a copy")]);
    let out = run(&world, &["--fresh"], &[]);
    assert!(out.status.success() && !String::from_utf8_lossy(&out.stdout).contains("quince-heron"));
    assert_eq!(chips(&shown(&world, &["--fresh"])), ["alpha", "private-notes"]);
}

#[test]
fn a_day_with_no_project_read_prints_no_entry() {
    let world = world();
    // The reading failed: there is no page, so no project was read.
    answer(&world, "not a reply", 1);
    let placed = shown(&world, &[]);
    assert_eq!((entries(&placed).len(), chips(&placed).len()), (0, 0));
    assert_eq!(notes(&placed).len(), 1, "the day could not be read");
    // The waiting page, and a day on which nothing was typed.
    let waiting = shown(&world, &["--waiting"]);
    assert_eq!((entries(&waiting).len(), chips(&waiting).len()), (0, 0));
    let quiet = shown(&world, &["--date", "2026-03-11"]);
    assert_eq!((entries(&quiet).len(), chips(&quiet).len()), (0, 0));
    assert!(today(&world).entry(None).is_none());
}

#[test]
fn words_entered_are_kept_by_core_and_drawn_at_once_as_working_with_no_reading() {
    let world = read();
    let mut shown = on_show(&world);
    let entry = (shown.offering())().unwrap();
    assert_eq!(
        (entry.id.as_str(), &entry.options[..], entry.most),
        ("wish", &["alpha".to_string(), "private-notes".to_string()][..], 200)
    );
    assert_eq!(drawn(&shown.scene, "m1"), None);

    let (scene, asking) = shown.wished(SAID, Some("private-notes"));
    assert!(asking.is_some(), "a kept wish asks for its run");
    assert_eq!(drawn(&scene, "m1"), Some((State::Busy, 0, Mark::Busy, SAID.to_string())));
    assert_eq!(scene, shown.scene, "what was given is what is on show");
    assert!(first(&scene).is_none_or(|(title, _)| title != UNKEPT), "{:?}", first(&scene));
    let kept = json!([{"id": "m1", "said": SAID, "project": "private-notes", "state": "working", "stars": []}]);
    assert_eq!(kept_wishes(&world), Some(kept));
    assert!(!called(&world), "a wish made a reading call");
    // A wish as long as core takes is kept whole: the entry's cap is core's.
    let longest = "w".repeat(200);
    assert_eq!(drawn(&shown.wished(&longest, Some("alpha")).0, "m2"), Some((State::Busy, 0, Mark::Busy, longest)));
}

#[test]
fn words_core_refuses_leave_one_note_in_core_s_words_and_write_nothing() {
    let world = read();
    let mut shown = on_show(&world);
    let before = (names(&world.home), names(&world.home.join("pages")));
    let long = "a wish is at most 200 characters, and this one is longer.";
    let unchosen = "No project was chosen, so nothing was sent.";
    let gone = "no folder of that project that is switched on was typed in on 2026-03-10, so this wish was not kept.";
    let cases = [
        ("w".repeat(201), Some("alpha"), long),
        (SAID.to_string(), Some("no-such"), gone),
        (SAID.to_string(), None, unchosen),
    ];
    for (typed, project, why) in cases {
        let (scene, asking) = shown.wished(&typed, project);
        assert!(asking.is_none(), "{why}: words that were not kept asked for a run");
        // The note names the words, then says why.
        let (title, body) = first(&scene).unwrap();
        assert_eq!((title, body.split_once("”: ").map(|(_, why)| why)), (UNKEPT, Some(why)));
        let said = scene.notes.iter().filter(|note| matches!(note, Note::Plain { title, .. } if title == UNKEPT));
        assert_eq!(said.count(), 1, "{why}: the note of the wish before is still there");
        assert_eq!((names(&world.home), names(&world.home.join("pages"))), before, "{why}: something was written");
    }
    assert_eq!(kept_wishes(&world), None);
    // The next wish that is kept takes the note away.
    assert_eq!(first(&shown.wished(SAID, Some("alpha")).0).map(|(title, _)| title), None);
}

// The poll's look: what core keeps now is drawn, a note over the page stays, and a look made from a scene that is
// no longer the one on show is dropped.
#[test]
fn a_look_draws_what_is_kept_now_and_never_takes_a_note_away() {
    let world = read();
    let mut shown = on_show(&world);
    assert!((shown.look())().is_none(), "nothing changed, so there is nothing to show");
    let tapped = shown.tap("clash-9", 0).unwrap();
    assert_eq!(first(&tapped).map(|(title, _)| title), Some("That tap did nothing."));
    assert!((shown.look())().is_none(), "a note over the page is no change");

    // A wish made in a terminal while the window is open.
    let stale = shown.look();
    jellymoon_core::wish(&world.home, Some(DAY), SAID, "alpha").unwrap();
    let seen = (shown.look())().expect("the made wish is new");
    let scene = shown.seen(seen).expect("the scene on show is the one the look was made from");
    assert_eq!(drawn(&scene, "m1"), Some((State::Busy, 0, Mark::Busy, SAID.to_string())));
    assert_eq!(first(&scene).map(|(title, _)| title), Some("That tap did nothing."), "the look took the note away");
    assert_eq!(scene.notes.len(), tapped.notes.len() + 1, "the note of made wishes is the one new note");
    assert!((shown.look())().is_none());
    // The look made before the wish was drawn finds another scene on show.
    assert!(stale().is_some_and(|seen| shown.seen(seen).is_none()), "a look replaced a newer scene");
}

// Two notes are the page's own and go when what is kept changes: a look takes those away, and carries neither over
// as a note of the app's.
#[test]
fn a_look_takes_away_a_note_of_the_page_s_own_that_no_longer_holds() {
    let titles = |scene: &Scene| {
        scene
            .notes
            .iter()
            .map(|note| {
                first(&Scene { notes: vec![note.clone()], ..Scene::default() }).map(|(title, _)| title.to_string())
            })
            .collect::<Vec<_>>()
    };
    let looked = |shown: &mut OnShow| {
        let seen = (shown.look())().expect("what is kept changed");
        shown.seen(seen).unwrap()
    };
    // A page with no wish on it, and then a wish made in a terminal.
    let quiet = world();
    answer(&quiet, &result(r#"{"wishes":[],"clashes":[]}"#), 0);
    shown(&quiet, &[]);
    let mut empty = on_show(&quiet);
    assert_eq!(titles(&empty.scene), [Some("Nothing wished for yet today.".to_string())]);
    jellymoon_core::wish(&quiet.home, Some(DAY), SAID, "alpha").unwrap();
    assert_eq!(titles(&looked(&mut empty)), [None], "only the note of made wishes is left");

    // A page whose first note says no session is told, and then the hook put in from a terminal.
    let world = world();
    let open = reading().replace("\"settled\":true", "\"settled\":false").replace("[\"no retry left\"]", "[]");
    answer(&world, &result(&open), 0);
    shown(&world, &[]);
    let mut untold = on_show(&world);
    let before = titles(&untold.scene);
    assert_eq!(before[0].as_deref(), Some("Your sessions are not told yet."), "{before:?}");
    let settings = world.dir.path().join("settings.json");
    jellymoon_core::install_hook(&settings, &world.home, &world.rec.join("claude")).unwrap();
    assert_eq!(titles(&looked(&mut untold)), before[1..]);

    // A plain note the page draws first whatever is kept is the page's, and is drawn once.
    let mut counted = on_show(&world);
    counted.page.as_mut().unwrap().left_out.stars = 3;
    counted.scene = counted.today.draw(counted.page.as_ref().unwrap());
    let before = titles(&counted.scene);
    assert_eq!(before[0].as_deref(), Some("not everything is drawn"));
    jellymoon_core::wish(&world.home, Some(DAY), SAID, "alpha").unwrap();
    let after = titles(&looked(&mut counted));
    assert_eq!(
        (&after[..before.len()], after.len()),
        (&before[..], before.len() + 1),
        "one note more: the made wishes"
    );
}

// The kit's part, with the app's own entry: Enter while an input method is composing confirms the composition,
// and the words that are entered stay in the entry until core has answered.
#[test]
fn enter_during_a_composition_enters_nothing() {
    let world = read();
    let mut typed = EntryState::of(&(on_show(&world).offering())().unwrap());
    typed.replace(None, "牛奶を");
    typed.mark(None, "kau", None);
    assert_eq!(typed.enter(), None, "text being composed was entered");
    assert_eq!((typed.text(), typed.marked().is_some()), ("牛奶をkau", true));
    typed.replace(None, "買う");
    typed.next_option();
    let made = typed.enter().expect("the composition is confirmed");
    assert_eq!((made.text.as_str(), made.option), ("牛奶を買う", Some(1)));
    assert_eq!((typed.text(), typed.waiting()), ("牛奶を買う", true), "the words left before core kept them");
    typed.answer(true);
    assert_eq!((typed.text(), typed.waiting()), ("", false));
}
