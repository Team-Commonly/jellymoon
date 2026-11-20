//! F86: a person sees what a run says it did over the choice to keep it or toss it, and the run's words are never
//! Jellymoon's (ADR 0023, decisions 3 to 6). The wish is made and run by the `jellymoon` command with the made-up
//! agent of F39's tests, and its result is written into the run's folder by hand, which core checks as it checks
//! the run's own (`crates/jellymoon/tests/f85.rs`). What `layout` prints is read here, and the taps are carried by
//! the made-up window of F47 (`support/tapper.rs`). `layout` prints the note as it first comes onto the page: a
//! change has no row there, and one sentence says core checked it (`tests/result_in_plain_words.rs`). Nothing here
//! starts the real Claude Code or opens a window.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::fs;

use serde_json::{Value, json};
use sticker_kit::{Note, Scene};
use support::Piece;
use support::wished::World;
use support::worked::{finished, tree};

const TITLE: &str = "What it says it did";
const NOTHING_RUN: &str = "Nothing was run to try this. A run reads and edits files, and no more.";
const CHECKED: &str = "The lines it quotes match the copy.";
const UNCHECKED: &str = "Something the run said could not be checked, so it is not here.";
const CHOICES: [&str; 3] = ["Show me", "Keep it", "Toss it"];
const SIZES: [&str; 2] = ["1440x900", "960x640"];

/// What the run did, as it says: a text with the shape of a command and of a page, which are text here.
const DID: &str = "It edited a.txt and made n.txt. $(touch ran) <b>bold</b>";

/// The pieces of a result the edits of `edits` pass: what it did, one change, one made file, and its doubts.
fn pieces() -> Value {
    json!([
        {"type": "did", "text": DID},
        {"type": "change", "file": "a.txt", "before": ["a.txt as it was"], "after": ["edited by the run"]},
        {"type": "change", "file": "n.txt", "after": ["new from the run"]},
        {"type": "decided", "text": "It kept the old name."},
        {"type": "unsure", "text": "It could not try the new line."}
    ])
}

/// A piece no check passes: `README.md` is no file of the work.
fn unchecked() -> Value {
    json!({"type": "change", "file": "README.md", "after": ["Free shipping from 50."]})
}

/// Writes `pieces` as the run's result into its folder, as anyone who can write Jellymoon's home can.
fn result(w: &World, pieces: &Value) {
    let run = w.kept()["run"].as_str().unwrap().to_string();
    let file = w.at("jm").join("runs").join(run).join("result.json");
    fs::write(&file, json!({"pieces": pieces}).to_string()).unwrap();
    // Core takes no file its group or others can write to, which a new file is under a umask of 002.
    #[cfg(unix)]
    fs::set_permissions(&file, std::os::unix::fs::PermissionsExt::from_mode(0o600)).unwrap();
}

fn of<'a>(placed: &'a [Piece], id: &str) -> Vec<&'a Piece> {
    placed.iter().filter(|piece| piece.id() == id).collect()
}

fn note<'a>(placed: &'a [Piece], id: &str) -> Option<&'a Piece> {
    of(placed, id).into_iter().find(|piece| piece.kind() == "note")
}

/// The texts of the told note's sheet, a run each, and the texts of its quote pieces, each whole.
fn told(placed: &[Piece]) -> (Vec<String>, Vec<String>) {
    let sheet = note(placed, "told-m1").expect("a told note");
    let own = sheet.runs().iter().map(|run| run["text"].as_str().unwrap().to_string()).collect();
    let quotes = of(placed, "told-m1").iter().filter(|piece| piece.kind() == "quote").map(|q| q.said()).collect();
    (own, quotes)
}

/// The pieces of the work's note, each with its place measured from the note's top, to a tenth: a hundredth
/// tells apart two layouts of the same note whose sums of floats round either side of it (17.33 and 17.34). A
/// sheet's and a tape's index is the note's place in the scene, which is one on with the told note before it.
fn work(placed: &[Piece]) -> Vec<String> {
    let top = note(placed, "work-m1").expect("the work's note").rect()[1];
    let from_top = |value: &mut Value| {
        let y = value["y"].as_f64().unwrap();
        value["y"] = json!(((y - top) * 10.0).round() / 10.0);
    };
    let moved = |piece: &&Piece| {
        let mut value = piece.0.clone();
        from_top(&mut value);
        value["text"].as_array_mut().unwrap().iter_mut().for_each(from_top);
        if ["note", "tape"].contains(&piece.kind()) {
            value.as_object_mut().unwrap().remove("index");
        }
        value.to_string()
    };
    of(placed, "work-m1").iter().map(moved).collect()
}

/// The rows of [`pieces`] as `layout` prints them, each the note's own words in it and what it quotes, in the
/// order they are read. A change has no row before `Show me`.
fn rows(decided: &[String], unsure: &[String]) -> Vec<(&'static str, Vec<String>)> {
    let one = |text: &str| vec![text.to_string()];
    vec![
        ("", one(DID)),
        ("Decided", [&one("It kept the old name.")[..], decided].concat()),
        ("Not sure about", [&one("It could not try the new line.")[..], unsure].concat()),
    ]
}

/// Asserts the told note reads, in order: the title, the rows that were drawn, each whole with its label over
/// its quotes, a count of the rows left out when any is, and the note's own sentences. The rows drawn are the
/// first ones: none is passed over for a later one. How many rows were drawn.
fn reads(placed: &[Piece], rows: &[(&str, Vec<String>)], end: &[&str]) -> usize {
    let (own, quotes) = told(placed);
    let quoted = |n: usize| rows[..n].iter().flat_map(|(_, quotes)| quotes.clone()).collect::<Vec<String>>();
    let n = (0..=rows.len()).find(|n| quoted(*n) == quotes).expect("the quotes are the first rows', whole");
    let drawn: Vec<&(&str, Vec<String>)> = rows[..n].iter().collect();
    let labels = drawn.iter().map(|(label, _)| label).filter(|label| !label.is_empty());
    let mut words: Vec<String> = [TITLE].iter().chain(labels).map(|s| s.to_string()).collect();
    if drawn.len() < rows.len() {
        words.push(format!("+{} more", rows.len() - drawn.len()));
    }
    words.extend(end.iter().map(|s| s.to_string()));
    assert_eq!(own.join(" "), words.join(" "), "the note's own words");
    let quoted: Vec<String> = drawn.iter().flat_map(|(_, quotes)| quotes.clone()).collect();
    assert_eq!(quotes, quoted, "every text of a drawn row, whole, and no other");
    drawn.len()
}

#[test]
fn layout_prints_the_told_note_before_the_work_s_with_every_text_of_the_run_a_quote_and_nothing_else() {
    let (w, repo) = finished("edits");
    result(&w, &pieces());
    let before = w.everything(&repo);
    let placed = w.placed();
    let first = |id: &str| placed.iter().position(|piece| piece.id() == id).unwrap();
    assert!(first("told-m1") < first("work-m1"), "the told note comes first");
    let end = [CHECKED, NOTHING_RUN];
    assert!(reads(&placed, &rows(&[], &[]), &end) >= 1, "what the run did is read first");
    let small = w.placed_with(&["--size", SIZES[1]]);
    assert!(reads(&small, &rows(&[], &[]), &end) >= 1, "{}: what the run did is read first", SIZES[1]);
    // No piece names a file the run did not name in its own sentence, or holds a line of the work.
    let held = |word: &str| placed.iter().chain(&small).any(|piece| piece.0.to_string().contains(word));
    assert!(!["b.txt", "as it was", "edited by the run", "new from the run", "Changed in"].into_iter().any(held));
    // Every run of a quote is in the tone for quoted words, and no text of the run is on the sheet.
    let quoted = of(&placed, "told-m1").into_iter().filter(|piece| piece.kind() == "quote");
    assert!(quoted.flat_map(Piece::roles).all(|role| role == "quoted"));
    assert!(note(&placed, "told-m1").unwrap().roles().iter().all(|role| role != "quoted"));
    // Nothing of it is to act on.
    assert!(of(&placed, "told-m1").iter().all(|piece| ["note", "quote", "tape"].contains(&piece.kind())));
    // No file of the work was opened as a program: the command in its words was not run.
    assert_eq!(w.everything(&repo), before, "a layout wrote something");
    let ran = |files: support::worked::Files| files.keys().any(|path| path.ends_with("ran"));
    assert!(!ran(tree(w.day.dir.path())), "a text of the run was run");
}

/// A typical result (`shown-told-typical.json`): a did of two lines, one change, a decided, nothing dropped.
const TYPICAL: &str = include_str!("../../crates/jellymoon-core/tests/fixtures/shown-told-typical.json");

/// The pieces placed of the opened wish once `Show me` was tapped: the rows of the changes come with that tap.
fn shown(w: &World) -> Vec<Piece> {
    let lines = w.taps("logs", &["poke:m1", "choose:work-m1:0", "run", "placed"]).ended();
    serde_json::from_str::<Vec<Value>>(&lines[3]).unwrap().into_iter().map(Piece).collect()
}

#[test]
fn at_1440x900_a_typical_result_reads_what_it_did_and_after_show_me_the_whole_change_before_the_work_s_note() {
    let (w, _) = finished("edits");
    let typical: Value = serde_json::from_str(TYPICAL).unwrap();
    let said = typical["result"]["pieces"].as_array().unwrap();
    let text = |at: usize| said[at]["text"].as_str().unwrap().to_string();
    // The fixture's did and decided, and the day's change, which core checks against the work.
    let change =
        json!({"type": "change", "file": "a.txt", "before": ["a.txt as it was"], "after": ["edited by the run"]});
    result(&w, &json!([said[0], change, said[2]]));
    let first = [("", vec![text(0)]), ("Decided", vec![text(2)])];
    assert_eq!(reads(&w.placed(), &first, &[CHECKED, NOTHING_RUN]), 2, "what it did and what it decided are read");
    let placed = shown(&w);
    let rows = vec![
        ("", vec![text(0)]),
        ("Changed in a.txt before after", vec!["a.txt as it was".to_string(), "edited by the run".to_string()]),
        ("Decided", vec![text(2)]),
    ];
    assert!(reads(&placed, &rows, &[NOTHING_RUN]) >= 2, "what it did and the whole change are read");
    let top = note(&placed, "work-m1").expect("the work's note").rect()[1];
    let quotes = of(&placed, "told-m1").into_iter().filter(|piece| piece.kind() == "quote");
    assert!(quotes.map(|quote| quote.rect()[1]).all(|y| y < top), "read before the work's note");
    let options: Vec<String> =
        of(&placed, "work-m1").iter().filter(|piece| piece.kind() == "option").map(|o| o.said()).collect();
    assert_eq!(options, CHOICES);
}

#[test]
fn at_960x640_a_did_too_long_for_the_room_draws_no_row_at_all_and_the_work_s_note_is_whole() {
    let (w, _) = finished("edits");
    let small = |w: &World| w.placed_with(&["--size", SIZES[1]]);
    let alone = small(&w);
    // A did of 231 characters, under core's most, does not fit what the work's note leaves: no row is drawn, and
    // none after it in its place.
    let long = [DID; 4].join(" ");
    let mut said = pieces().as_array().unwrap().clone();
    said[0] = json!({"type": "did", "text": long});
    result(&w, &Value::Array(said));
    let mut rows = rows(&[], &[]);
    rows[0].1 = vec![long];
    let placed = small(&w);
    assert_eq!(reads(&placed, &rows, &[CHECKED, NOTHING_RUN]), 0, "a row was drawn");
    assert_eq!(work(&placed), work(&alone), "the work's note is whole");
    let options: Vec<String> =
        of(&placed, "work-m1").iter().filter(|piece| piece.kind() == "option").map(|o| o.said()).collect();
    assert_eq!(options, CHOICES);
}

#[test]
fn the_work_s_note_is_the_note_it_is_without_a_result_with_its_three_choices_at_both_sizes() {
    let (w, _) = finished("edits");
    for size in SIZES {
        let alone = w.placed_with(&["--size", size]);
        result(&w, &pieces());
        let placed = w.placed_with(&["--size", size]);
        assert!(note(&placed, "told-m1").is_some(), "{size}: a told note");
        assert_eq!(work(&placed), work(&alone), "{size}");
        let options: Vec<String> =
            of(&placed, "work-m1").iter().filter(|piece| piece.kind() == "option").map(|o| o.said()).collect();
        assert_eq!(options, CHOICES, "{size}");
        fs::remove_file(w.at("jm").join("runs").join(w.kept()["run"].as_str().unwrap()).join("result.json")).unwrap();
    }
}

#[test]
fn rows_that_do_not_fit_are_left_out_and_counted_and_none_is_cut() {
    let (w, _) = finished("edits");
    let long = |n: usize| format!("{n} {}", "a long word of doubt".repeat(7));
    let doubts: Vec<Value> = (0..3).map(|n| json!({"type": "unsure", "text": long(n)})).collect();
    let decided: Vec<Value> = (3..6).map(|n| json!({"type": "decided", "text": long(n)})).collect();
    let all = [pieces().as_array().unwrap().clone(), decided, doubts].concat();
    result(&w, &Value::Array(all));
    let rows = rows(&(3..6).map(long).collect::<Vec<_>>(), &(0..3).map(long).collect::<Vec<_>>());
    for size in SIZES {
        let drawn = reads(&w.placed_with(&["--size", size]), &rows, &[CHECKED, NOTHING_RUN]);
        assert!(size == SIZES[0] || drawn < rows.len(), "{size}: no row was left out");
    }
}

#[test]
fn a_dropped_piece_ends_the_note_with_the_sentence_that_something_could_not_be_checked() {
    let (w, _) = finished("edits");
    // Core checks the first two changes only, so the one that fails stands in the place of n.txt's.
    let mut said = pieces().as_array().unwrap().clone();
    said[2] = unchecked();
    result(&w, &Value::Array(said));
    for size in SIZES {
        reads(&w.placed_with(&["--size", size]), &rows(&[], &[]), &[CHECKED, NOTHING_RUN, UNCHECKED]);
    }
    // Every piece dropped, and no result at all: no told note, and the work's note as it is.
    result(&w, &json!([unchecked()]));
    let placed = w.placed();
    assert!(of(&placed, "told-m1").is_empty() && note(&placed, "work-m1").is_some());
    fs::remove_file(w.at("jm").join("runs").join(w.kept()["run"].as_str().unwrap()).join("result.json")).unwrap();
    let placed = w.placed();
    assert!(of(&placed, "told-m1").is_empty() && note(&placed, "work-m1").is_some());
}

/// The ids of the notes of a scene, in their order.
fn ids(scene: &Option<Scene>) -> Vec<String> {
    let id = |note: &Note| match note {
        Note::Text { id, .. } | Note::Told { id, .. } => Some(id.clone()),
        _ => None,
    };
    scene.as_ref().unwrap().notes.iter().filter_map(id).collect()
}

#[test]
fn a_tap_on_the_told_note_finds_nothing_and_work_that_touches_files_tools_obey_is_kept_only_at_the_second_yes() {
    let (w, repo) = finished("crowd");
    result(&w, &pieces());
    let before = w.everything(&repo);
    let go = w.at("go");
    let hold = format!("hold:{}", go.display());
    let acts: [&str; 7] = ["poke:m1", "placed", "poke:told-m1", "choose:told-m1:0", "choose:work-m1:1", "run", &hold];
    let mut window = w.taps("logs", &[&acts[..], &["choose:sure-m1:1", "run"]].concat());
    window.printed(6);
    assert_eq!(w.everything(&repo), before, "a tap before the second yes changed something");
    fs::write(&go, "").unwrap();
    let lines = window.ended();
    let [opened, placed, poked, chose, _, asks, _, kept] = &lines[..] else { panic!("eight lines: {lines:?}") };
    let scene = |line: &str| serde_json::from_str::<Option<Scene>>(line).unwrap();
    assert_eq!(ids(&scene(opened)), ["told-m1", "work-m1"]);
    let placed: Vec<Piece> = serde_json::from_str::<Vec<Value>>(placed).unwrap().into_iter().map(Piece).collect();
    assert!(of(&placed, "told-m1").iter().all(|piece| ["note", "quote", "tape"].contains(&piece.kind())));
    assert_eq!((poked.as_str(), chose.as_str()), ("null", "null"), "a tap on the told note did something");
    let asks = scene(asks);
    let Some(Note::Text { title, body, .. }) = asks.as_ref().unwrap().notes.first() else { panic!("{asks:?}") };
    assert_eq!(title, "This work touches files your tools obey.");
    assert!(["CLAUDE.md", ".claude/settings.json"].iter().all(|name| body.iter().any(|text| text == name)));
    let kept = scene(kept);
    let Some(Note::Text { title, .. }) = kept.as_ref().unwrap().notes.first() else { panic!("{kept:?}") };
    assert_eq!(title, "The change was kept.");
    assert!(repo.join("CLAUDE.md").is_file());
}
