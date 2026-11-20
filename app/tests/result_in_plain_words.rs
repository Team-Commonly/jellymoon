//! The result speaks in plain words, with the lines behind a tap (ADR 0023, the line of 2026-11-20).
//! What a run says it did comes onto the page by a look with no file's name and no changed line: one sentence of
//! Jellymoon's stands for every change core checked, and the work's note counts its files and names none. A tap on
//! `Show me` brings the rows of the changes and the names, as they were before. The wish is made and run by the
//! `jellymoon` command with the made-up agent of F39's tests, its result is written into the run's folder by hand,
//! and the looks and taps are carried by the made-up window of F47 (`support/tapper.rs`). Nothing here starts the
//! real Claude Code or opens a window.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::fs;

use serde_json::{Value, json};
use sticker_kit::{Note, Scene, ToldRow};
use support::Piece;
use support::day::DAY;
use support::wished::{SAID, World, quiet};

const CHECKED: &str = "The lines it quotes match the copy.";
const NOTHING_RUN: &str = "Nothing was run to try this. A run reads and edits files, and no more.";
const UNCHECKED: &str = "Something the run said could not be checked, so it is not here.";
const READY: &str = "It is in a copy: Keep it puts it in; Toss it throws it away.";
const COUNTED: &str = "1 new file, 1 changed, 1 removed.";
const DID: &str = "It made the greeting shorter.";
const DECIDED: &str = "It kept the old name.";
const UNSURE: &str = "It could not try the new line.";
/// What no first view holds: the names of the work's files, its changed lines, and the labels of a change's row.
const BEHIND: [&str; 8] =
    ["a.txt", "b.txt", "n.txt", "as it was", "edited by the run", "new from the run", "Changed in", "New file"];

fn did() -> Value {
    json!({"type": "did", "text": DID})
}

/// A change the edits of `edits` pass, and one no check passes: `README.md` is no file of the work.
fn change() -> Value {
    json!({"type": "change", "file": "a.txt", "before": ["a.txt as it was"], "after": ["edited by the run"]})
}

fn unchecked() -> Value {
    json!({"type": "change", "file": "README.md", "after": ["Free shipping from 50."]})
}

/// What it did, a change, a made file, and its doubts: every piece passes.
fn pieces() -> Value {
    let made = json!({"type": "change", "file": "n.txt", "after": ["new from the run"]});
    json!([did(), change(), made, {"type": "decided", "text": DECIDED}, {"type": "unsure", "text": UNSURE}])
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

/// A day nothing was typed on, with one wish, `m1`, whose run ended with work that waits and said `pieces`.
fn waiting(pieces: &Value) -> World {
    let w = quiet();
    let repo = w.at("work").join("alpha");
    let added = w.jellymoon(&["project", "add", "--repo", repo.to_str().unwrap(), "--date", DAY]);
    assert_eq!(added["name"], "alpha", "{added}");
    fs::create_dir(w.at("harness")).unwrap();
    let made = w.jellymoon(&["wish", "--date", DAY, "--say", SAID, "--project", "alpha"]);
    assert_eq!(made["wish"]["id"], "m1", "{made}");
    let logs = w.at("logs");
    let ran = w.jellymoon(&["run", "--wish", "m1", "--date", DAY, "--logs", logs.to_str().unwrap()]);
    assert_eq!(ran["outcome"], "completed", "{ran}");
    result(&w, pieces);
    w
}

fn scene(line: &str) -> Scene {
    serde_json::from_str::<Option<Scene>>(line).expect(line).unwrap_or_else(|| panic!("no scene was shown: {line}"))
}

fn ids(scene: &Scene) -> Vec<&str> {
    fn id(note: &Note) -> Option<&str> {
        match note {
            Note::Text { id, .. } | Note::Told { id, .. } => Some(id),
            _ => None,
        }
    }
    scene.notes.iter().filter_map(id).collect()
}

/// The rows and the last sentences of the note of what the run says it did.
fn told(scene: &Scene) -> (Vec<ToldRow>, Vec<String>) {
    let told = scene.notes.iter().find_map(|note| match note {
        Note::Told { id, title, rows, end } if id == "told-m1" && title == "What it says it did" => Some((rows, end)),
        _ => None,
    });
    let (rows, end) = told.unwrap_or_else(|| panic!("no told note: {scene:?}"));
    (rows.clone(), end.clone())
}

/// The texts of the work's note, with its choices under them.
fn work(scene: &Scene) -> Vec<String> {
    let work = scene.notes.iter().find_map(|note| match note {
        Note::Text { id, title, body, options, .. } if id == "work-m1" => Some((title, body, options)),
        _ => None,
    });
    let (title, body, options) = work.unwrap_or_else(|| panic!("no work's note: {scene:?}"));
    assert_eq!(title, "Your folder is not changed yet.");
    assert_eq!(options, &["Show me", "Keep it", "Toss it"]);
    body.clone()
}

fn row(label: &str, quote: &str) -> ToldRow {
    ToldRow { label: label.to_string(), quotes: vec![quote.to_string()], ..ToldRow::default() }
}

/// The checked sentence, which starts the paragraph that nothing was run.
fn checked() -> String {
    format!("{CHECKED} {NOTHING_RUN}")
}

fn own(texts: &[&str]) -> Vec<String> {
    texts.iter().map(|text| text.to_string()).collect()
}

/// What the first look of a window that just opened shows.
fn looked(w: &World) -> Scene {
    scene(&w.taps("logs", &["look"]).ended()[0])
}

#[test]
fn the_first_view_a_look_brings_names_no_file_and_holds_no_changed_line() {
    let w = waiting(&pieces());
    let lines = w.taps("logs", &["look", "placed"]).ended();
    let first = scene(&lines[0]);
    assert_eq!(ids(&first), ["told-m1", "work-m1"]);
    // The run's own sentence, what it decided and what it is not sure about; one sentence for every change.
    let rows = vec![row("", DID), row("Decided", DECIDED), row("Not sure about", UNSURE)];
    assert_eq!(told(&first), (rows, own(&[&checked()])));
    // The work's note counts its files, in core's sentence, and names none.
    assert_eq!(work(&first), own(&[READY, &format!("“{SAID}”"), COUNTED]));
    // Nowhere in the scene, and nowhere among the pieces placed of it.
    for (line, word) in lines.iter().flat_map(|line| BEHIND.map(|word| (line, word))) {
        assert!(!line.contains(word), "{word:?} is on the first view: {line}");
    }
    // A tap on the wish's character shows what the look showed: the lines come with `Show me` alone.
    let tapped = w.taps("logs", &["poke:m1"]).showed().remove(0).expect("a tap opens the wish");
    assert_eq!(tapped.notes, first.notes);
}

#[test]
fn show_me_brings_the_rows_of_the_changes_and_the_names_as_they_were() {
    let w = waiting(&pieces());
    let lines = w.taps("logs", &["look", "choose:work-m1:0", "run", "placed", "look"]).ended();
    let [_, _, shown, placed, later] = &lines[..] else { panic!("five lines: {lines:#?}") };
    let shown = scene(shown);
    assert_eq!(ids(&shown), ["told-m1", "work-m1", "file-1-m1"]);
    let changed = ToldRow {
        label: "Changed in".to_string(),
        name: "a.txt".to_string(),
        quotes: own(&["a.txt as it was", "edited by the run"]),
        marks: own(&["before", "after"]),
    };
    let made = ToldRow { name: "n.txt".to_string(), ..row("New file", "new from the run") };
    let rows = vec![row("", DID), changed, made, row("Decided", DECIDED), row("Not sure about", UNSURE)];
    assert_eq!(told(&shown), (rows, own(&[NOTHING_RUN])), "the rows, and no sentence in their place");
    assert_eq!(work(&shown), own(&[READY, &format!("“{SAID}”"), COUNTED, "a.txt, b.txt, n.txt"]));
    // The change is drawn: its lines are quote pieces of the told note.
    let placed: Vec<Piece> = serde_json::from_str::<Vec<Value>>(placed).unwrap().into_iter().map(Piece).collect();
    let quotes = placed.iter().filter(|piece| piece.kind() == "quote" && piece.id() == "told-m1");
    let quotes: Vec<String> = quotes.map(Piece::said).collect();
    assert!(quotes.contains(&"edited by the run".to_string()), "{quotes:?}");
    // A look leaves what the person asked to see.
    let after = serde_json::from_str::<Option<Scene>>(later).unwrap().unwrap_or_else(|| shown.clone());
    assert_eq!((told(&after), work(&after)), (told(&shown), work(&shown)));
}

#[test]
fn the_checked_sentence_is_said_only_when_a_change_passed_core_s_check() {
    // No change at all: nothing was checked, and nothing says so.
    let w = waiting(&json!([did()]));
    assert_eq!(told(&looked(&w)), (vec![row("", DID)], own(&[NOTHING_RUN])));
    // Its only change dropped: the sentence that something could not be checked, and not the checked one.
    result(&w, &json!([did(), unchecked()]));
    assert_eq!(told(&looked(&w)), (vec![row("", DID)], own(&[NOTHING_RUN, UNCHECKED])));
    // One change passed and one dropped: both sentences, and still no row of a change.
    result(&w, &json!([did(), change(), unchecked()]));
    assert_eq!(told(&looked(&w)), (vec![row("", DID)], own(&[&checked(), UNCHECKED])));
}
