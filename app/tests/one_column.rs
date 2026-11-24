//! The page is one column, read from its top: what the person typed for a wish, then what came of it, in one
//! order, the older wish over the newer, and the field to type the next wish in a band at the window's bottom
//! whatever the column holds. Nothing is put beside, over or in place of anything else, and nothing is left out
//! or cut short because the window is small: the column is as long as it has to be, and the view moves it.
//!
//! The wishes are made and run by the `jellymoon` command with the made-up agent of F39's tests, and the looks
//! and taps are carried by the made-up window of F47 (`support/tapper.rs`). Nothing here starts the real Claude
//! Code or opens a window: what is checked is where the layout puts each piece, in the column's own places.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::{fs, thread};

use jellymoon_app::column::column;
use jellymoon_app::scene::MADE;
use serde_json::{Value, json};
use sticker_kit::{
    BundledMeasure, Character, Entry, Mark, Note, PieceKind, Scene, Theme, Viewport, entry_top, layout, layout_entry,
    scroll_most, speaker,
};
use support::Piece;
use support::day::DAY;
use support::wished::{SAID, World, quiet, wait_until};

const SIZES: [(f32, f32); 2] = [(1440.0, 900.0), (960.0, 640.0)];

/// A day nothing was typed on, with `alpha` listed as a person lists it.
fn day() -> World {
    let w = quiet();
    let repo = w.at("work").join("alpha");
    w.jellymoon(&["project", "add", "--repo", repo.to_str().unwrap(), "--date", DAY]);
    fs::create_dir(w.at("harness")).unwrap();
    w
}

fn wish(w: &World) -> String {
    let made = w.jellymoon(&["wish", "--date", DAY, "--say", SAID, "--project", "alpha"]);
    made["wish"]["id"].as_str().unwrap().to_string()
}

fn run(w: &World, id: &str) -> Value {
    let logs = w.at("logs");
    w.jellymoon(&["run", "--wish", id, "--date", DAY, "--logs", logs.to_str().unwrap()])
}

/// The pieces of a line the made-up window printed for `placed`.
fn placed(line: &str) -> Vec<Piece> {
    let pieces: Vec<Value> = serde_json::from_str(line).expect(line);
    pieces.into_iter().map(Piece).collect()
}

/// Whether a piece is a card of the column: a note, or what the person typed.
fn card(piece: &Piece) -> bool {
    piece.kind() == "note" || piece.kind() == "quote" && piece.id().starts_with("typed-")
}

/// The ids of the cards of the made wishes, down the column.
fn cards(placed: &[Piece]) -> Vec<&str> {
    let of_a_wish = |id: &&str| id.rsplit('-').next().is_some_and(|wish| wish.starts_with('m'));
    placed.iter().filter(|piece| card(piece)).map(Piece::id).filter(of_a_wish).collect()
}

fn overlap(a: [f64; 4], b: [f64; 4]) -> bool {
    a[0] < b[0] + b[2] && b[0] < a[0] + a[2] && a[1] < b[1] + b[3] && b[1] < a[1] + a[3]
}

/// One column: each card's top is under the bottom of the card before it, no card is over another, and no
/// character, big or small, stands on a card.
fn one_column(placed: &[Piece], what: &str) {
    let cards: Vec<&Piece> = placed.iter().filter(|piece| card(piece)).collect();
    for pair in cards.windows(2) {
        let (over, under) = (pair[0].rect(), pair[1].rect());
        assert!(
            under[1] >= over[1] + over[3],
            "{what}: {} is not under {}: {over:?} {under:?}",
            pair[1].id(),
            pair[0].id()
        );
    }
    let stands = |piece: &&Piece| ["character", "hero"].contains(&piece.kind());
    for one in placed.iter().filter(stands) {
        for card in &cards {
            assert!(!overlap(one.rect(), card.rect()), "{what}: {one:?} stands on {card:?}");
        }
    }
    let wide: Vec<f64> = cards.iter().filter(|card| card.kind() == "note").map(|card| card.rect()[2]).collect();
    assert!(wide.windows(2).all(|pair| (pair[0] - pair[1]).abs() < 0.02), "{what}: cards of one width: {wide:?}");
}

/// The character of the wish `id` stands at the end of what was typed for it: beside the bubble and right of it.
fn at_its_bubble(placed: &[Piece], id: &str, what: &str) {
    let typed = format!("typed-{id}");
    let bubble = placed.iter().find(|piece| piece.kind() == "quote" && piece.id() == typed).expect(what).rect();
    let who = placed.iter().find(|piece| piece.kind() == "character" && piece.id() == id).expect(what).rect();
    assert!(who[0] >= bubble[0] + bubble[2], "{what}: at the bubble's end: {bubble:?} {who:?}");
    assert!(who[1] < bubble[1] + bubble[3] && bubble[1] < who[1] + who[3], "{what}: beside it: {bubble:?} {who:?}");
    assert_eq!(speaker(&typed), id, "a tap on what was typed is a tap on the wish's character");
}

/// Writes what the run says it did into its folder, a result the edits of `steps` pass.
fn result(w: &World) {
    let pieces = json!([
        {"type": "did", "text": "It edited a.txt."},
        {"type": "change", "file": "a.txt", "before": ["a.txt as it was"], "after": ["edited by the run"]},
    ]);
    let run = w.kept()["run"].as_str().unwrap().to_string();
    let file = w.at("jm").join("runs").join(run).join("result.json");
    fs::write(&file, json!({"pieces": pieces}).to_string()).unwrap();
    #[cfg(unix)]
    fs::set_permissions(&file, std::os::unix::fs::PermissionsExt::from_mode(0o600)).unwrap();
}

#[test]
fn one_wish_reads_down_the_column_from_the_typed_words_to_the_kept_change() {
    let w = day();
    w.play("steps");
    assert_eq!(wish(&w), "m1");
    let home = w.at("jm");
    let (mut working, mut lines) = (Vec::new(), Vec::new());
    thread::scope(|scope| {
        let going = scope.spawn(|| run(&w, "m1"));
        let first = |(_, steps): &(String, jellymoon_core::Steps)| steps.count == 1;
        wait_until("the first step", || jellymoon_core::doing(&home, DAY).iter().any(first));
        working = w.placed();
        let held = format!("hold:{}", w.at("tmp").join("ended").display());
        let acts = [&held, "look", "placed", "choose:work-m1:1", "placed", "run", "placed"];
        let mut window = w.taps("logs", &acts);
        fs::write(w.at("rec").join("go"), "").unwrap();
        assert_eq!(going.join().unwrap()["outcome"], "completed");
        result(&w);
        fs::write(w.at("tmp").join("ended"), "").unwrap();
        lines = window.ended();
    });
    // While it works: the person's words, then what the run is doing.
    let ids = cards(&working);
    assert_eq!(ids[..2], ["typed-m1", "steps-m1"], "{ids:?}");
    let typed = working.iter().find(|piece| piece.id() == "typed-m1").unwrap();
    assert_eq!((typed.kind(), typed.said().replace('\u{a0}', " ")), ("quote", SAID.to_string()), "the words, whole");
    one_column(&working, "working");
    at_its_bubble(&working, "m1", "working");

    let [_, ended, _, at_once, kept, last] = &lines[..] else { panic!("six lines: {lines:#?}") };
    // Ended: what the run says it did, then the change that waits, under the same words.
    let ended = placed(ended);
    assert_eq!(cards(&ended), ["typed-m1", "told-m1", "work-m1"]);
    one_column(&ended, "ended");
    at_its_bubble(&ended, "m1", "ended");
    let told = ended.iter().find(|piece| piece.kind() == "note" && piece.id() == "told-m1").unwrap().rect();
    let level = |piece: &&Piece| piece.kind() == "hero" && (piece.rect()[1] - told[1]).abs() < 0.02;
    let hero = ended.iter().find(level).expect("the big character at the first card after the typed words");
    assert!(hero.rect()[0] + hero.rect()[2] <= told[0], "left of the card");
    // Keep it: what is being done, then the answer, each in the work's place.
    let at_once = placed(at_once);
    assert_eq!(cards(&at_once).first().copied(), Some("typed-m1"));
    assert!(cards(&at_once).contains(&"busy-m1"), "{:?}", cards(&at_once));
    assert!(kept != "null", "core's answer is shown");
    let last = placed(last);
    assert_eq!(cards(&last), ["typed-m1", "said-m1"]);
    let said = last.iter().find(|piece| piece.kind() == "note" && piece.id() == "said-m1").unwrap();
    assert!(said.said().starts_with("The change was kept."), "{}", said.said());
    one_column(&last, "kept");
    at_its_bubble(&last, "m1", "kept");
    // A tap on the wish's character, which is the tap on its bubble, opens the wish as before.
    assert!(w.taps("logs", &["poke:m1"]).showed()[0].is_some(), "a tap opens the wish");
}

#[test]
fn the_older_wish_and_all_that_came_of_it_stand_over_the_newer() {
    let w = day();
    w.play("edits");
    for id in ["m1", "m2"] {
        assert_eq!(wish(&w), id);
        assert_eq!(run(&w, id)["outcome"], "completed");
    }
    let lines = w.taps("logs", &["look", "placed", "poke:m1", "placed", "size:960x640", "placed"]).ended();
    let [_, newest, _, both, small] = &lines[..] else { panic!("five lines: {lines:#?}") };
    let newest = placed(newest);
    assert_eq!(cards(&newest), ["typed-m1", "typed-m2", "work-m2"], "the newest wish's work, under its own words");
    for (what, line) in [("both", both), ("small", small)] {
        let page = placed(line);
        let ids = cards(&page);
        let second = ids.iter().position(|id| *id == "typed-m2").expect("the newer wish's words");
        assert_eq!(ids[0], "typed-m1", "{what}: {ids:?}");
        assert!(second > 1, "{what}: the older wish's work is under its words: {ids:?}");
        assert!(ids[..second].iter().all(|id| id.ends_with("-m1")), "{what}: {ids:?}");
        assert!(ids[second..].iter().all(|id| id.ends_with("-m2")), "{what}: {ids:?}");
        one_column(&page, what);
        at_its_bubble(&page, "m1", what);
        at_its_bubble(&page, "m2", what);
        // Every option of the work on show is placed, whole: a tap finds it at any size.
        let options = page.iter().filter(|piece| piece.kind() == "option" && piece.id() == "work-m1").count();
        assert_eq!(options, 3, "{what}");
    }
}

/// A day with one made wish and `count` notes of it, each of three long sentences, and its entry.
fn long_day(count: usize) -> (Scene, Entry) {
    let body = |n: usize| {
        let line =
            |part: &str| format!("Note {n} {part}: the uploader now waits a little and tries the same file again.");
        ["begins", "goes on", "ends"].map(line).to_vec()
    };
    let note = |n: usize| Note::Text {
        id: format!("file-{n}-m1"),
        title: format!("The change, part {n}"),
        body: body(n),
        options: vec![format!("Open part {n}")],
        quoted: String::new(),
        field: None,
    };
    let made = Note::Whole { title: MADE.to_string(), rows: vec![(Mark::Star, SAID.to_string())] };
    let scene = Scene {
        title: "what I wished for today".to_string(),
        hero: Some(Character { id: "me".to_string(), ..Character::default() }),
        characters: vec![Character { id: "m1".to_string(), stars: 1, ..Character::default() }],
        notes: (1..=count).map(note).chain([made]).collect(),
        ..Scene::default()
    };
    let options = ["alpha", "beta"].map(String::from).to_vec();
    (scene, Entry { id: "wish".to_string(), hint: "What do you want done?".to_string(), options, ..Entry::default() })
}

#[test]
fn eight_notes_are_all_placed_whole_and_the_entry_is_at_the_window_s_bottom_whatever_the_column_holds() {
    let theme = Theme::sticker();
    let measure = BundledMeasure::new(&theme);
    for (w, h) in SIZES {
        let (viewport, what) = (Viewport { w, h }, format!("{w}x{h}"));
        let band = entry_top(&theme, viewport);
        let mut entries = Vec::new();
        for count in [1, 8] {
            let (scene, entry) = long_day(count);
            let pieces = layout(&column(&scene), &theme, viewport, &measure);
            // Every note, with every word of it and its option: none left out, none cut short.
            let notes: Vec<_> = pieces.iter().filter(|piece| piece.piece == PieceKind::Note).collect();
            assert_eq!(notes.len(), count, "{what}");
            for (n, placed) in notes.iter().enumerate() {
                let said: Vec<&str> = placed.text.iter().flat_map(|run| run.text.split_whitespace()).collect();
                let Note::Text { title, body, .. } = &scene.notes[n] else { panic!("a text note") };
                let whole = [title.as_str()].into_iter().chain(body.iter().map(String::as_str)).collect::<Vec<_>>();
                assert_eq!(said.join(" "), whole.join(" "), "{what}: note {n} of {count}");
                assert_eq!(placed.id, format!("file-{}-m1", n + 1), "{what}: in the scene's order");
            }
            assert_eq!(pieces.iter().filter(|piece| piece.piece == PieceKind::Option).count(), count, "{what}");
            // The entry is the window's: inside it, under the column's part of it, and the lowest thing shown.
            let entry = layout_entry(&entry, &theme, viewport, &measure);
            assert!(entry.iter().any(|piece| piece.piece == PieceKind::Entry), "{what}: a field to type in");
            assert!(entry.iter().filter(|piece| piece.piece == PieceKind::Chip).count() == 2, "{what}: its chips");
            for piece in &entry {
                assert!(piece.piece.in_band(), "{what}: {piece:?}");
                assert!(piece.rect.y >= band && piece.rect.bottom() <= h, "{what}: in the band: {piece:?}");
                assert!(piece.rect.x >= 0.0 && piece.rect.right() <= w, "{what}: in the window: {piece:?}");
            }
            // However far the column goes, moved to its end its last card is over the band.
            let most = scroll_most(&pieces, &theme, viewport);
            let bottom = notes.iter().map(|piece| piece.rect.bottom()).fold(0.0, f32::max);
            assert!(pieces.iter().all(|piece| !piece.piece.in_band()), "{what}: the column holds none of the band");
            assert!(bottom - most <= band, "{what}: the last card is over the entry: {bottom} {most} {band}");
            assert!(count < 8 || most > 0.0, "{what}: eight notes are longer than the window");
            entries.push(entry);
        }
        assert_eq!(entries[0], entries[1], "{what}: the entry is where it was, whatever the column holds");
    }
}
