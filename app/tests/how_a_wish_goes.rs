//! How a wish goes (ADR 0027, decision 10; `docs/development/app.md`, "What is shown first"): a day with no wish
//! shows one strip of three sketches, each with its caption, and one line under it. It takes no tap, and it is
//! gone when the day has a wish, a reading's or a made one. The days are shown through the made-up window of F47
//! (`support/tapper.rs`), a process of its own: `column` prints the scene as `Showing::column` gives it to the
//! view, `pieces` what the kit places of it, and `tap:how` a tap on the strip. Nothing here starts the real
//! Claude Code or opens a window.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::collections::BTreeSet;
use std::fs;

use jellymoon_app::scene::{HOW, ruled};
use serde_json::{Value, json};
use sticker_kit::{Note, Panel, Picture, Scene, Shows};
use support::day::{DAY, FIRST, SECOND, answer, called, noon, reading, result, said, shown, write_log};
use support::wished::{SAID, World, quiet};

/// The window's smallest size (`jellymoon_app::SMALLEST`).
const NARROW: &str = "size:960x640";
const SAYS: [&str; 3] = ["say a wish", "it gets made", "keep or toss"];
const TYPE_ONE: &str = "Type a wish in the field at the bottom.";
const FOLDER_FIRST: &str = "First choose a folder to work in, or name a new project.";
const OLD: &str = "Nothing wished for yet today.";

/// A day nothing was typed on, with the folder the made-up window's harness is looked for in.
fn empty() -> World {
    let w = quiet();
    fs::create_dir(w.at("harness")).unwrap();
    w
}

/// The folder `work/alpha` chosen: a project Jellymoon knows.
fn chosen(w: &World) {
    let repo = w.at("work").join("alpha");
    let added = w.jellymoon(&["project", "add", "--repo", repo.to_str().unwrap(), "--date", DAY]);
    assert_eq!(added["name"], "alpha", "{added}");
}

fn column(line: &str) -> Scene {
    serde_json::from_str::<Option<Scene>>(line).expect(line).expect("a column")
}

fn pieces(line: &str) -> Vec<Value> {
    serde_json::from_str(line).expect(line)
}

/// The strip as the page is to hold it: the three sketches in the order a wish goes, each with its caption.
fn strip(line: &str) -> Note {
    let shows = [Shows::Speech, Shows::Sheet, Shows::Folder];
    let panels = shows.into_iter().zip(SAYS).map(|(shows, says)| Panel { shows, caption: says.into() }).collect();
    let (picture, words) = (Picture::Strip { panels }, vec![line.to_string()]);
    Note::Drawn { id: HOW.into(), picture, words, open: true, options: Vec::new(), field: None }
}

fn id(note: &Note) -> &str {
    match note {
        Note::Text { id, .. } | Note::Choice { id, .. } | Note::Told { id, .. } => id,
        Note::Said { id, .. } | Note::Drawn { id, .. } => id,
        Note::Plain { title, .. } | Note::List { title, .. } | Note::Whole { title, .. } => title,
    }
}

fn ids(column: &Scene) -> Vec<&str> {
    column.notes.iter().map(id).collect()
}

/// What the pieces of the note `how` say, kind by kind, and how the captions lie: each right of the one before.
fn placed(pieces: &[Value], wide: f64) -> Vec<(String, String)> {
    let own: Vec<&Value> = pieces.iter().filter(|piece| piece["id"] == HOW).collect();
    let said = |piece: &Value| {
        let runs = piece["text"].as_array().unwrap().iter().map(|run| run["text"].as_str().unwrap());
        runs.collect::<Vec<&str>>().join(" ")
    };
    for piece in &own {
        let (x, w) = (piece["x"].as_f64().unwrap(), piece["w"].as_f64().unwrap());
        assert!(x >= 0.0 && x + w <= wide, "{} of the strip is out of a window {wide} wide", piece["kind"]);
        if piece["kind"] == "picture" {
            let lefts: Vec<f64> =
                piece["text"].as_array().unwrap().iter().map(|run| run["x"].as_f64().unwrap()).collect();
            assert!(lefts.len() == 3 && lefts.windows(2).all(|pair| pair[0] < pair[1]), "one row: {lefts:?}");
        }
    }
    let mut kinds: Vec<(String, String)> =
        own.iter().map(|piece| (piece["kind"].as_str().unwrap().to_string(), said(piece))).collect();
    kinds.sort();
    kinds
}

// The first page, with no folder chosen and with one: the same strip in the same place, and under it the one
// line that says where to begin. A tap on it is no tap, and the narrowest window draws the same row.
#[test]
fn an_empty_day_shows_how_a_wish_goes_in_three_sketches_with_one_line_under_them_and_takes_no_tap() {
    for (folder, line) in [(false, FOLDER_FIRST), (true, TYPE_ONE)] {
        let w = empty();
        if folder {
            chosen(&w);
        }
        // The day is drawn once before the window, as `layout` draws it, so its page is kept.
        w.placed();
        let acts = ["column", "look", "column", "pieces", "tap:how", "column", NARROW, "pieces"];
        let lines = w.taps("logs", &acts).ended();
        let [waits, _, first, wide, tapped, after, narrow] = &lines[..] else { panic!("seven lines: {lines:#?}") };
        // Until the day's logs were looked at the window waits, and the waiting page has no strip.
        let waiting = ids(&column(waits)) == ["Reading your day…"];
        assert!(waiting || column(waits) == column(first), "{folder}: {waits}");
        assert_eq!(column(first).notes, [strip(line)], "{folder}: the strip alone");
        assert!(column(first).characters.is_empty() && !lines.concat().contains(OLD), "{folder}: {first}");
        assert_eq!((tapped.as_str(), column(after)), ("null", column(first)), "{folder}: a tap on it is no tap");
        for (placed_at, width) in [(wide, 1440.0), (narrow, 960.0)] {
            let want = [("note", line), ("picture", "say a wish it gets made keep or toss"), ("tape", "")];
            let want: Vec<(String, String)> =
                want.iter().map(|(kind, said)| (kind.to_string(), said.to_string())).collect();
            assert_eq!(placed(&pieces(placed_at), width), want, "{folder} at {width}: a card and its picture, no mark");
        }
    }
}

#[test]
fn the_first_made_wish_takes_the_strip_off_the_page() {
    let w = empty();
    chosen(&w);
    let made = w.jellymoon(&["wish", "--date", DAY, "--say", SAID, "--project", "alpha"]);
    assert_eq!(made["wish"]["id"], "m1", "{made}");
    let lines = w.taps("logs", &["column", "pieces"]).ended();
    let [first, placed] = &lines[..] else { panic!("two lines: {lines:#?}") };
    let first = column(first);
    assert!(ids(&first).contains(&"typed-m1") && !ids(&first).contains(&HOW), "{:?}", ids(&first));
    assert_eq!(first.characters.len(), 1, "the made wish's little one");
    assert!(!pieces(placed).iter().any(|piece| piece["id"] == HOW), "{placed}");
    assert!(!lines.concat().contains(SAYS[0]) && !lines.concat().contains(TYPE_ONE), "no word of the strip is left");
}

// A day the reading put wishes on opens folded into its tally, and a day being read says so: neither has the strip.
#[test]
fn a_read_day_shows_its_tally_and_a_day_being_read_says_so_and_neither_shows_the_strip() {
    let w = quiet();
    answer(&w.day, &result(&reading()), 0);
    let alpha = w.at("work").join("alpha");
    let typed = [FIRST, SECOND].map(|words| said(&noon(0), alpha.to_str().unwrap(), words));
    write_log(&w.day, "-work-alpha/s1.jsonl", &typed);
    shown(&w.day, &[]);
    assert!(called(&w.day), "the day was to be read once, before any window");
    fs::create_dir(w.at("harness")).unwrap();
    let lines = w.taps("logs", &["look", "column", "pieces"]).ended();
    let [_, read, placed] = &lines[..] else { panic!("three lines: {lines:#?}") };
    let read = column(read);
    let tally = |note: &Note| matches!(note, Note::Drawn { picture: Picture::Tally { .. }, .. });
    assert!(read.notes.iter().any(tally) && !ids(&read).contains(&HOW), "{:?}", ids(&read));
    assert!(!pieces(placed).iter().any(|piece| piece["id"] == HOW), "{placed}");
    assert!(!lines.concat().contains(SAYS[0]), "no caption of it either");
    // Typed in, and not read yet: the window's first scene waits for the reading.
    let w = quiet();
    let typed = [said(&noon(0), w.at("work").join("alpha").to_str().unwrap(), "look at the uploader")];
    write_log(&w.day, "-work-alpha/s1.jsonl", &typed);
    let on = w.on_show();
    assert!(on.page.is_none() && ids(&on.scene).contains(&"Reading your day…"), "{:?}", ids(&on.scene));
    assert!(!ids(&on.scene).contains(&HOW), "{:?}", ids(&on.scene));
}

// The page of F37 whose reading holds a clash and no wish: the strip stands over the clash, where the note it
// took the place of stood, and a made wish takes it away and leaves the clash first. The page is typed here,
// as in `f37.rs`, and put in the order of its column as the window puts it.
#[test]
fn over_a_clash_with_no_wish_the_strip_stands_first_and_a_made_wish_leaves_the_clash_first() {
    let wish = json!({"id": "m1", "label": SAID, "said": SAID, "project": "alpha", "state": "working", "stars": []});
    let clash = json!({"a": "w1", "b": "w2", "quote_a": "retry", "quote_b": "never retry", "settled": false});
    let page = |wishes: Vec<Value>| {
        let read = json!({"turns": 2, "projects": ["alpha"]});
        let data = json!({"day": DAY, "wishes": wishes, "clashes": [clash], "read": read});
        jellymoon_app::read::page(&data.to_string()).unwrap()
    };
    let shown = |wishes: Vec<Value>, made: usize| {
        jellymoon_app::drawing::column(&ruled(&page(wishes), &[], made), &BTreeSet::new())
    };
    let (none, one) = (shown(vec![], 0), shown(vec![wish], 1));
    assert_eq!((ids(&none), &none.notes[0]), (vec![HOW, "clash-1"], &strip(TYPE_ONE)));
    assert_eq!(ids(&one)[0], "clash-1", "{:?}", ids(&one));
    assert!(!ids(&one).contains(&HOW), "{:?}", ids(&one));
}
