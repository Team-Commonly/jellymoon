//! F37 in the app: a made wish is on today's page with the reading's wishes, before a reading and after one. The
//! wish is made by core, as `jellymoon wish` makes it; the built `jellymoon-app` is run as in `f34.rs`.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::fs;

use jellymoon_app::scene::ruled;
use jellymoon_app::today::Today;
use serde_json::{Value, json};
use sticker_kit::{FixedAdvance, Mark, Note, PieceKind, Placed, Scene, State, Theme, layout};
use support::day::*;

const MADE: &str = "ship the changelog";

/// What `layout` placed of what the person typed for made wishes, in the column's order: each a bubble of the
/// words, whole, with the id `typed-<wish>`.
fn typed(placed: &[support::Piece]) -> Vec<String> {
    let typed = placed.iter().filter(|piece| piece.kind() == "quote" && piece.id().starts_with("typed-"));
    typed.map(|piece| piece.said().replace('\u{a0}', " ")).collect()
}

/// The labels drawn and the made wishes' words, in name order: where each stands on the page is the kit's layout.
fn drawn(world: &World, args: &[&str]) -> Vec<String> {
    let placed = shown(world, args);
    let mut labels = [labels(&placed), typed(&placed)].concat();
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

// The window's first scene, before a log is opened, and the scene of a day with no logs to read.
#[test]
fn a_made_wish_is_drawn_working_before_the_day_is_read_and_on_a_day_with_no_logs() {
    let world = world();
    make(&world);
    let today = Today {
        date: DAY.to_string(),
        logs: world.dir.path().join("nowhere"),
        skip: Vec::new(),
        model: "sonnet".to_string(),
        home: world.home.clone(),
        harness: Some(world.claude()),
        fresh: false,
        settings: None,
        bin: None,
    };
    let (first, kept) = today.first();
    assert_eq!(first.footer, "1 made wish is in your own words.", "nothing was read, and nothing says so");
    let rows = vec![(Mark::Busy, MADE.to_string())];
    // Under the note of the reading, the sentence that the wish is being worked on, then the wish itself.
    let going = |words: &[String]| words.first().is_some_and(|word| word == "Your wish is being worked on.");
    assert!(matches!(&first.notes[1], Note::Drawn { id, words, .. } if id == "steps-m1" && going(words)));
    assert_eq!(first.notes[2], Note::Whole { title: "wished here today".to_string(), rows });
    let drawn: Vec<_> = first.characters.iter().map(|one| (one.id.as_str(), one.label.as_str(), one.state)).collect();
    assert_eq!(drawn, [("m1", MADE, State::Busy)]);
    assert!(kept.is_none() && matches!(&first.notes[0], Note::Plain { title, .. } if title == "Reading your day…"));

    let nowhere = world.dir.path().join("nowhere");
    let placed = shown(&world, &["--logs", nowhere.to_str().unwrap()]);
    assert_eq!((labels(&placed), typed(&placed)), (Vec::new(), vec![MADE.to_string()]), "its words, in its bubble");
    // No logs and a made wish: the day has its page all the same (F63), and is not said to be unreadable.
    assert!(notes(&placed).iter().all(|note| !note.contains("could not be read")), "{:?}", notes(&placed));
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
    assert_eq!(
        unread,
        "No Claude Code session logs were found, so nothing was read. 2 made wishes are in your own words."
    );
}

// The one page a made wish moves a clash on: a page whose reading holds a clash and no wish. It says "Nothing wished
// for yet today." above the clash, and a made wish is a wish, so the note goes and the clash stands where it was.
#[test]
fn on_a_page_with_a_clash_and_no_wish_read_a_made_wish_takes_the_empty_note_away_and_the_clash_moves_up() {
    let wish = json!({"id": "m1", "label": MADE, "said": MADE, "project": "alpha", "state": "working", "stars": []});
    let clash = json!({"a": "w1", "b": "w2", "quote_a": "retry", "quote_b": "never retry", "settled": false});
    let page = |wishes: Vec<Value>| {
        let read = json!({"turns": 2, "projects": ["alpha"]});
        let data = json!({"day": DAY, "wishes": wishes, "clashes": [clash], "read": read});
        jellymoon_app::read::page(&data.to_string()).unwrap()
    };
    let (none, one) = (ruled(&page(vec![]), &[], 0), ruled(&page(vec![wish.clone()]), &[], 1));
    let titles = |scene: &Scene| -> Vec<String> {
        let title = |note: &Note| match note {
            Note::Plain { title, .. } | Note::Choice { title, .. } | Note::Whole { title, .. } => title.clone(),
            Note::List { title, .. } | Note::Text { title, .. } | Note::Told { title, .. } => title.clone(),
            Note::Said { text: title, .. } | Note::Drawn { id: title, .. } => title.clone(),
        };
        scene.notes.iter().map(title).collect()
    };
    assert_eq!(titles(&none), ["Nothing wished for yet today.", "these two clash"]);
    assert_eq!(titles(&one), ["these two clash", "wished here today"]);
    for theme in [Theme::sticker(), Theme::plain()] {
        let clash = |scene: &Scene| {
            let placed = layout(scene, &theme, jellymoon_app::SMALLEST, &FixedAdvance(0.56));
            placed.into_iter().find(|piece| piece.piece == PieceKind::Note && piece.id == "clash-1").unwrap()
        };
        let (before, after) = (clash(&none), clash(&one));
        assert!(after.rect.y < before.rect.y, "the clash moves up to where the empty note was");
        let size = |note: &Placed| (note.rect.w, note.text.iter().map(|run| run.size).collect::<Vec<_>>());
        let same = (after.rect.h - before.rect.h).abs() < 0.01 && size(&after) == size(&before);
        assert!(same, "and is drawn at the same size, every word of it: {before:?} {after:?}");
    }
}

// A made wish has no title but the person's words, so the page shows them: each whole, in a bubble of its own, in
// the order they were made. However many there are, none is counted or cut, and none moves aside, shrinks or
// rewrites a note a person rules on: those stand over them in the column.
#[test]
fn every_made_wish_is_drawn_whole_in_a_bubble_of_its_own_at_both_sizes_in_both_themes() {
    let world = world();
    let words = "write the changelog for the release and name every change a person would see ";
    let long: String = words.repeat(3).chars().take(199).chain(['x']).collect();
    let said = [long.as_str(), "ship it", "a third", "a fourth", "a fifth", "a sixth", "a seventh", "the eighth"];
    let cases = [("sticker", "960x640"), ("sticker", "1440x900"), ("plain", "960x640"), ("plain", "1440x900")];
    // Every piece of the clash's note and of the delivery note: its place across, its size and its words. Its
    // place down the column is under the footer, which says one line more once a wish is made.
    let ruled_on = |placed: &[support::Piece]| -> Vec<String> {
        let told = placed.iter().find(|piece| piece.said().starts_with("Your sessions are not told yet."));
        let ids = ["clash-1", told.expect("the delivery note").id()];
        let across = |piece: &support::Piece| {
            let [x, _, w, h] = piece.rect();
            let sizes: Vec<_> = piece.runs().iter().map(|run| run["size"].clone()).collect();
            format!("{} {} {x} {w} {h} {} {sizes:?}", piece.kind(), piece.id(), piece.said())
        };
        placed.iter().filter(|piece| ids.contains(&piece.id())).map(across).collect()
    };
    let draw = |theme: &str, size: &str| shown(&world, &["--theme", theme, "--size", size]);
    let before = cases.map(|(theme, size)| ruled_on(&draw(theme, size)));
    assert!(before.iter().all(|pieces| pieces.len() == 6), "two notes, two tapes, two options: {before:?}");
    let mut made = Vec::new();
    for (count, said) in said.iter().enumerate() {
        let kept = jellymoon_core::wish(&world.home, Some(DAY), said, "alpha").unwrap().wish.said;
        assert_eq!(kept, *said);
        made.push(kept);
        if ![1, 2, 8].contains(&(count + 1)) {
            continue;
        }
        for ((theme, size), before) in cases.iter().zip(&before) {
            let at = format!("{} made, {theme} at {size}", made.len());
            let placed = draw(theme, size);
            assert_eq!(ruled_on(&placed), *before, "{at}: a made wish moves aside and shrinks nothing ruled on");
            assert_eq!(typed(&placed), made, "{at}: every made wish, whole and in order");
            assert!(placed.iter().all(|piece| !piece.said().contains(" more of ")), "{at}: something is counted");
            let tops: Vec<f64> =
                placed.iter().filter(|piece| piece.id().starts_with("typed-")).map(|p| p.rect()[1]).collect();
            assert!(tops.windows(2).all(|pair| pair[0] < pair[1]), "{at}: the older over the newer: {tops:?}");
            assert!(notes(&placed).iter().any(|note| note.starts_with("you changed your mind")), "{at}: the clash");
        }
    }
    assert_eq!(made[0].chars().count(), 200);
}
