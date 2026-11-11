//! F37 in the app: a made wish is on today's page with the reading's wishes, before a reading and after one. The
//! wish is made by core, as `jellymoon wish` makes it; the built `jellymoon-app` is run as in `f34.rs`.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::fs;

use jellymoon_app::today::Today;
use sticker_kit::{Mark, Note, State};
use support::day::*;

const MADE: &str = "ship the changelog";

/// The labels drawn, in name order: where each stands on the page is the kit's layout.
fn drawn(world: &World, args: &[&str]) -> Vec<String> {
    let mut labels = labels(&shown(world, args));
    labels.sort();
    labels
}

fn make(world: &World) -> Vec<u8> {
    jellymoon_core::wish(&world.home, Some(DAY), MADE, "alpha").unwrap();
    fs::read(world.home.join("pages").join(format!("{DAY}.wishes.json"))).unwrap()
}

#[test]
fn a_made_wish_is_drawn_with_the_reading_s_wishes_and_a_new_reading_does_not_lose_it() {
    let world = world();
    let made = make(&world);
    for args in [&[][..], &["--fresh"]] {
        assert_eq!(drawn(&world, args), ["Fail fast", "Retry uploads", MADE], "{args:?}");
        assert!(called(&world), "{args:?}: the day was read");
        let page = fs::read_to_string(kept(&world)).unwrap();
        assert!(!page.contains(MADE), "the kept page is the reading's own: {page}");
        assert_eq!(fs::read(world.home.join("pages").join(format!("{DAY}.wishes.json"))).unwrap(), made);
    }
    assert_eq!(drawn(&world, &[]), ["Fail fast", "Retry uploads", MADE], "the kept page, with no reading");
    assert!(!called(&world));
}

// The window's first scene, before a log is opened, and the scene of a day that could not be read.
#[test]
fn a_made_wish_is_drawn_working_before_the_day_is_read_and_when_it_cannot_be() {
    let world = world();
    make(&world);
    let today = Today {
        date: DAY.to_string(),
        logs: world.dir.path().join("nowhere"),
        skip: Vec::new(),
        model: "sonnet".to_string(),
        home: world.home.clone(),
        harness: None,
        fresh: false,
        settings: None,
        bin: None,
    };
    let (first, kept) = today.first();
    assert_eq!(first.footer, "1 made wish is in your own words.", "nothing was read, and nothing says so");
    let rows = vec![(Mark::Busy, MADE.to_string())];
    assert_eq!(first.notes[1], Note::Whole { title: "wished here today".to_string(), rows });
    let drawn: Vec<_> = first.characters.iter().map(|one| (one.id.as_str(), one.label.as_str(), one.state)).collect();
    assert_eq!(drawn, [("m1", MADE, State::Busy)]);
    assert!(kept.is_none() && matches!(&first.notes[0], Note::Plain { title, .. } if title == "Reading your day…"));

    let nowhere = world.dir.path().join("nowhere");
    let placed = shown(&world, &["--logs", nowhere.to_str().unwrap()]);
    assert_eq!(labels(&placed), [MADE]);
    assert!(notes(&placed)[0].starts_with("Today could not be read."), "{:?}", notes(&placed));
    assert!(!called(&world));
}

/// The footer `layout` placed.
fn footer(placed: &[support::Piece]) -> String {
    placed.iter().find(|piece| piece.kind() == "footer").map(support::Piece::said).unwrap_or_default()
}

// What the page says of itself is true of every wish on it: the reading's part is the model's, a made wish is the
// person's, and a day not read yet says nothing of a reading.
#[test]
fn the_footer_says_which_wishes_are_the_model_s_reading_and_which_the_person_s_own() {
    let world = world();
    let read =
        "Read from 3 things you typed in 2 projects (alpha, private-notes). This is the model's reading, unedited.";
    assert_eq!(footer(&shown(&world, &[])), read);
    make(&world);
    assert_eq!(footer(&shown(&world, &[])), format!("{read} 1 made wish is in your own words."));
    jellymoon_core::wish(&world.home, Some(DAY), "and a second one", "alpha").unwrap();
    assert_eq!(footer(&shown(&world, &[])), format!("{read} 2 made wishes are in your own words."));
    let nowhere = world.dir.path().join("nowhere");
    fs::remove_file(kept(&world)).unwrap();
    let unread = footer(&shown(&world, &["--logs", nowhere.to_str().unwrap()]));
    assert_eq!(unread, "2 made wishes are in your own words.");
}

/// The made wishes the note draws, in order, and how many it counts in `+N more`. Every line of the note belongs to
/// a wish, character for character, and lies inside the note with its mark.
fn in_note(placed: &[support::Piece], made: &[String], at: &str) -> (Vec<String>, usize) {
    let text = |run: &serde_json::Value| run["text"].as_str().unwrap().to_string();
    let note = placed.iter().find(|piece| piece.kind() == "note" && piece.said().starts_with("wished here today"));
    let note = note.unwrap_or_else(|| panic!("{at}: no note of made wishes in {:?}", notes(placed)));
    let [left, top, width, height] = note.rect();
    for run in note.runs() {
        let [x, y, w, h] = support::rect(run);
        let inside = x >= left && x + w <= left + width && y >= top && y + h <= top + height;
        assert!(inside, "{at}: {:?} is outside the note", text(run));
    }
    let lines: Vec<String> = note.runs().iter().filter(|run| run["role"] == "row_main").map(text).collect();
    let (mut next, mut drawn) = (0, Vec::new());
    for said in made {
        let (mut rest, mut used) = (said.as_str(), next);
        while let Some(after) = lines.get(used).and_then(|line| rest.trim_start().strip_prefix(line.as_str())) {
            (rest, used) = (after, used + 1);
            if rest.is_empty() {
                break;
            }
        }
        if rest.is_empty() {
            next = used;
            drawn.push(said.clone());
        }
    }
    assert_eq!(next, lines.len(), "{at}: every line is a made wish's own words: {lines:?}");
    let marks = placed.iter().filter(|piece| piece.kind() == "mark" && piece.id() == note.id()).count();
    assert_eq!(marks, drawn.len(), "{at}: one mark a wish drawn");
    let more = note.runs().iter().find(|run| run["role"] == "row_sub").map(text);
    let counted = more.map_or(0, |more| more.trim_start_matches('+').trim_end_matches(" more").parse().unwrap());
    (drawn, counted)
}

// A made wish has no title but the person's words, so the page shows them: whole, or counted, never cut.
#[test]
fn every_made_wish_is_drawn_whole_in_its_note_or_counted_there_at_both_sizes_in_both_themes() {
    let world = world();
    let words = "write the changelog for the release and name every change a person would see ";
    let long: String = words.repeat(3).chars().take(199).chain(['x']).collect();
    let typed = [long.as_str(), "ship it", "a third", "a fourth", "a fifth", "a sixth", "a seventh", "the eighth"];
    let cases = [("sticker", "960x640"), ("sticker", "1440x900"), ("plain", "960x640"), ("plain", "1440x900")];
    // Every piece of the clash's note and of the delivery note: its place, its size and its words.
    let ruled_on = |placed: &[support::Piece]| -> Vec<String> {
        let told = placed.iter().find(|piece| piece.said().starts_with("Your sessions are not told yet."));
        let ids = ["clash-1", told.expect("the delivery note").id()];
        placed.iter().filter(|piece| ids.contains(&piece.id())).map(|piece| piece.0.to_string()).collect()
    };
    let draw = |theme: &str, size: &str| shown(&world, &["--theme", theme, "--size", size]);
    let before = cases.map(|(theme, size)| ruled_on(&draw(theme, size)));
    assert!(before.iter().all(|pieces| pieces.len() == 6), "two notes, two tapes, two options: {before:?}");
    let mut made = Vec::new();
    for (count, typed) in typed.iter().enumerate() {
        let said = jellymoon_core::wish(&world.home, Some(DAY), typed, "alpha").unwrap().wish.said;
        assert_eq!(said, *typed);
        made.push(said);
        if ![1, 2, 8].contains(&(count + 1)) {
            continue;
        }
        for ((theme, size), before) in cases.iter().zip(&before) {
            let at = format!("{} made, {theme} at {size}", made.len());
            let placed = draw(theme, size);
            assert_eq!(ruled_on(&placed), *before, "{at}: a made wish moves and shrinks nothing a person rules on");
            let (drawn, counted) = in_note(&placed, &made, &at);
            assert_eq!(drawn.len() + counted, made.len(), "{at}: drawn {drawn:?}, and {counted} counted");
            // Beside the list, the delivery note and the clash, the sticker theme has room for the count alone. The
            // plain theme draws one or two whole, and of eight passes over the long one for the short ones after it.
            let few = made.len() <= 2;
            let fits = if few { drawn == made } else { !drawn.is_empty() && !drawn.contains(&made[0]) };
            assert!(if *theme == "plain" { fits } else { drawn.is_empty() }, "{at}: {drawn:?} and {counted}");
            assert!(notes(&placed).iter().any(|note| note.starts_with("you changed your mind")), "{at}: the clash");
        }
    }
    assert_eq!(made[0].chars().count(), 200);
}
