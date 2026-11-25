//! The day folds (ADR 0030, decision 12; `docs/development/app.md`, "A read day opens folded"): what the reading
//! put on a day's page is first one tally at the column's top, a tap on it opens the day under it, every piece
//! as it was, and a second tap folds it again. A made wish is never folded, and nothing that waits for a tap is.
//! The days are read by the made-up harness of F27's tests and shown through the made-up window of F47
//! (`support/tapper.rs`), a process of its own: `column` prints the scene as `Showing::column` gives it to the
//! view, `pieces` what the kit places of it, and `placed` what `layout` prints, which folds nothing. Nothing here
//! starts the real Claude Code or opens a window.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::fs;

use serde_json::{Value, json};
use sticker_kit::{Mark, Note, Picture, Scene, State, TallyRow};
use support::day::{DAY, FIRST, SECOND, answer, called, noon, reading, result, said, shown, write_log};
use support::wished::{SAID, World, quiet};

/// A window tall enough that every note of the day is placed.
const TALL: &str = "size:1440x3000";
const THIRD: &str = "draw the logo";
const READING: &str = "This is the model's reading, unedited.";

/// A day typed in `alpha` and in `beta` and read as `reading`, kept before any window opens on it.
fn read_day(reading: &str) -> World {
    let w = quiet();
    answer(&w.day, &result(reading), 0);
    let folder = |name: &str| w.at("work").join(name).to_str().unwrap().to_string();
    let alpha = [said(&noon(0), &folder("alpha"), FIRST), said(&noon(1), &folder("alpha"), SECOND)];
    write_log(&w.day, "-work-alpha/s1.jsonl", &alpha);
    write_log(&w.day, "-work-beta/s2.jsonl", &[said(&noon(2), &folder("beta"), THIRD)]);
    shown(&w.day, &[]);
    assert!(called(&w.day), "the day was to be read once, before any window");
    fs::create_dir(w.at("harness")).unwrap();
    w
}

/// A reading with a wish in every state, three things that came true, and a clash nobody chose on.
fn busy() -> String {
    let wish = |id: &str, said: &str, project: &str, state: &str, stars: &[&str]| {
        json!({"id": id, "label": format!("Label of {id}"), "said": said, "project": project, "state": state,
            "stars": stars})
    };
    json!({
        "wishes": [
            wish("a", FIRST, "alpha", "working", &[]),
            wish("b", SECOND, "alpha", "needs_you", &["no retry left", "fails fast"]),
            wish("c", THIRD, "beta", "done", &["a logo"]),
        ],
        "clashes": [{"a": "a", "b": "b", "quote_a": FIRST, "quote_b": "never retry uploads", "settled": false}],
    })
    .to_string()
}

fn column(line: &str) -> Scene {
    serde_json::from_str::<Option<Scene>>(line).expect(line).expect("a column")
}

fn pieces(line: &str) -> Vec<Value> {
    serde_json::from_str(line).expect(line)
}

fn id(note: &Note) -> &str {
    match note {
        Note::Text { id, .. }
        | Note::Choice { id, .. }
        | Note::Told { id, .. }
        | Note::Said { id, .. }
        | Note::Drawn { id, .. } => id,
        Note::Plain { title, .. } | Note::List { title, .. } | Note::Whole { title, .. } => title,
    }
}

fn ids(column: &Scene) -> Vec<&str> {
    column.notes.iter().map(id).collect()
}

/// The tally of a column: its rows, and whether it is open. None of a column with none.
fn tally(column: &Scene) -> Option<(Vec<TallyRow>, bool)> {
    column.notes.iter().find_map(|note| match note {
        Note::Drawn { id, picture: Picture::Tally { title, rows, under, open, .. }, words, options, field, .. } => {
            assert_eq!((id.as_str(), title.as_str(), under.as_str()), ("fold", "today so far", READING));
            assert!(words.is_empty() && options.is_empty() && field.is_none(), "a picture and nothing else");
            Some((rows.clone(), *open))
        }
        _ => None,
    })
}

fn row(label: &str, marks: &[Mark]) -> TallyRow {
    TallyRow { label: label.into(), marks: marks.to_vec() }
}

/// `column` with its tally taken off: what stood there before the day could fold.
fn untallied(mut column: Scene) -> Scene {
    column.notes.retain(|note| id(note) != "fold");
    column
}

/// `piece` with how far down it and every run of its text are taken out of it, and those, `by` higher.
fn raised(piece: &Value, by: f64) -> (Value, Vec<f64>) {
    let mut piece = piece.clone();
    let mut downs = vec![piece["y"].take()];
    downs.extend(piece["text"].as_array_mut().unwrap().iter_mut().map(|run| run["y"].take()));
    (piece, downs.iter().map(|down| down.as_f64().unwrap() - by).collect())
}

// The day of the brief's own shape: three wishes of the reading in two projects, one of each state but let go,
// three things that came true, and an open clash. Folded, the tally counts them and none of them is placed, but
// the clash, which waits for a tap; opened, each piece is the piece `layout` prints, further down.
#[test]
fn a_read_day_opens_folded_a_tap_opens_it_piece_for_piece_and_a_second_tap_folds_it_again() {
    let w = read_day(&busy());
    let acts = [TALL, "look", "column", "pieces", "placed", "tap:fold", "column", "pieces", "tap:fold", "column"];
    let lines = w.taps("logs", &acts).ended();
    let [_, first, folded, placed, _, open, opened, _, again] = &lines[..] else { panic!("nine lines: {lines:#?}") };
    let (first, open, again) = (column(first), column(open), column(again));
    // Folded: the counts are the counts of the reading's pieces, by project and by state.
    let asks = Mark::NeedsYou;
    let rows = vec![row("alpha", &[asks, asks, Mark::Busy, Mark::Star, Mark::Star]), row("beta", &[Mark::Star])];
    assert_eq!(tally(&first), Some((rows.clone(), false)), "{first:?}");
    assert!(first.characters.is_empty(), "no little one of the reading: {:?}", first.characters);
    assert!(!ids(&first).contains(&"came true today"), "{:?}", ids(&first));
    assert!(ids(&first).contains(&"clash-1"), "a clash nobody chose on waits for a tap: {:?}", ids(&first));
    let folded = pieces(folded);
    let kinds = |pieces: &[Value], kind: &str| pieces.iter().filter(|piece| piece["kind"] == kind).count();
    assert_eq!((kinds(&folded, "character"), kinds(&folded, "star"), kinds(&folded, "picture")), (0, 0, 1));
    assert!(folded.iter().any(|piece| piece["kind"] == "option" && piece["id"] == "clash-1"), "it can be tapped");
    // The sentence that this is the model's reading stands once, under the title, and not in the tally too.
    let said = |pieces: &[Value]| {
        let whole = |piece: &&Value| {
            let runs = piece["text"].as_array().unwrap().iter().map(|run| run["text"].as_str().unwrap());
            runs.collect::<Vec<&str>>().join(" ").contains("This is the model's reading, unedited.")
        };
        pieces.iter().filter(whole).map(|piece| piece["kind"].as_str().unwrap().to_string()).collect::<Vec<String>>()
    };
    assert_eq!(said(&folded), ["footer"]);
    // Opened: the tally stays, turned, and under it is every piece of the page as it was.
    assert_eq!(tally(&open), Some((rows, true)));
    let states: Vec<State> = open.characters.iter().map(|who| who.state).collect();
    assert_eq!(states, [State::Busy, State::NeedsYou, State::Idle]);
    let stars: usize = open.characters.iter().map(|who| usize::from(who.stars)).sum();
    assert_eq!(stars, 3, "a star in the tally for each star of the open page");
    // The paper is as long as its column and its holes go down it: a longer column has more of them.
    let whole =
        |line: &String| pieces(line).into_iter().filter(|piece| piece["kind"] != "hole").collect::<Vec<Value>>();
    let (opened, placed) = (whole(opened), whole(placed));
    let (own, rest): (Vec<Value>, Vec<Value>) = opened.into_iter().partition(|piece| piece["id"] == "fold");
    assert_eq!(kinds(&own, "picture"), 1);
    assert_eq!(rest.len(), placed.len(), "every piece `layout` prints, and no other");
    let top =
        |pieces: &[Value]| pieces.iter().find(|piece| piece["kind"] == "character").unwrap()["y"].as_f64().unwrap();
    let down = top(&rest) - top(&placed);
    // What ends over the tally is the header, which stays; everything else is the day, under it.
    let (card, bottom) =
        (own[0]["y"].as_f64().unwrap(), |piece: &Value| piece["y"].as_f64().unwrap() + piece["h"].as_f64().unwrap());
    assert!(down > 0.0, "the day is under its tally");
    for (now, was_piece) in rest.iter().zip(&placed) {
        if was_piece["kind"] == "paper" {
            continue; // the paper is as long as its column
        }
        let by = if bottom(was_piece) < card { 0.0 } else { down };
        let ((now, at), (was_piece, was_at)) = (raised(now, by), raised(was_piece, 0.0));
        assert_eq!(now, was_piece);
        assert!(at.iter().zip(&was_at).all(|(a, b)| (a - b).abs() < 0.011), "{now}: {at:?} {was_at:?}");
    }
    assert_eq!(again, first, "a second tap folds it again");
}

// A made wish is never folded: its typed words, its little one and its notes stand under the tally, the same
// whether the day is folded or open. And a new window on the same day starts folded, whatever the last one did.
#[test]
fn a_made_wish_stands_under_the_tally_folded_or_open_and_a_new_window_starts_folded() {
    let w = read_day(&reading());
    w.play("edits");
    let made = w.jellymoon(&["wish", "--date", DAY, "--say", SAID, "--project", "alpha"]);
    assert_eq!(made["wish"]["id"], "m1", "{made}");
    let (repo, logs) = (w.at("work").join("alpha"), w.at("logs"));
    let run = ["run", "--wish", "m1", "--date", DAY, "--repo", repo.to_str().unwrap(), "--logs"];
    let ran = w.jellymoon(&[&run[..], &[logs.to_str().unwrap()]].concat());
    assert_eq!(ran["outcome"], "completed", "{ran}");
    let lines = w.taps("logs", &[TALL, "look", "column", "tap:fold", "column"]).ended();
    let [_, folded, _, open] = &lines[..] else { panic!("four lines: {lines:#?}") };
    let (folded, open) = (column(folded), column(open));
    // The reading of `day.rs`: one wish let go and one done with one star, in one project, and a settled clash.
    let rows = vec![row("alpha", &[Mark::Star, Mark::Dimmed])];
    assert_eq!((tally(&folded), tally(&open)), (Some((rows.clone(), false)), Some((rows, true))));
    let made = |column: &Scene| {
        let all = ids(column);
        let at = all.iter().position(|one| *one == "typed-m1").expect("the made wish's typed words");
        let notes: Vec<Note> = column.notes[at..].iter().filter(|note| id(note) != "fold").cloned().collect();
        (notes, column.characters.iter().find(|who| who.id == "m1").cloned().expect("its little one"))
    };
    assert_eq!(made(&folded), made(&open), "the made wish is as it is, folded or open");
    assert!(made(&folded).0.len() > 1, "with its notes under it: {:?}", ids(&folded));
    assert_eq!(folded.characters.len(), 1, "and it is the only little one of a folded day");
    assert!(!ids(&folded).contains(&"clash-1"), "a settled clash is the reading's, and folded: {:?}", ids(&folded));
    assert_eq!(open.characters.len(), 3);
    let under = untallied(open);
    assert!(ids(&under).contains(&"came true today") && ids(&under).contains(&"clash-1"), "{:?}", ids(&under));
    let lines = w.taps("logs", &[TALL, "look", "column"]).ended();
    assert_eq!(column(lines.last().expect("two lines")), folded, "a window starts folded");
}

// A day the reading has nothing on has no tally: there is nothing to fold.
#[test]
fn a_day_with_nothing_read_has_no_tally() {
    let w = quiet();
    fs::create_dir(w.at("harness")).unwrap();
    let lines = w.taps("logs", &[TALL, "look", "column", "tap:fold", "pieces"]).ended();
    let [_, opened, tapped, placed] = &lines[..] else { panic!("four lines: {lines:#?}") };
    assert_eq!(tally(&column(opened)), None);
    assert_eq!(tapped, "null", "and no tap on one");
    assert!(!pieces(placed).iter().any(|piece| piece["kind"] == "picture" || piece["id"] == "fold"));
}
