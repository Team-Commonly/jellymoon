//! The wire shapes are the contract between `jellymoon` and the journal app's shell. Each example under `fixtures/` is
//! what a command prints, byte for byte; each file under `fixtures/refused/` breaks the shape in one way and is refused.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::path::PathBuf;

use jellymoon_core::{Harness, PageData, ReaderDay, WishState};

fn fixture(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name);
    // A checkout on Windows may write line breaks its own way.
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display())).replace("\r\n", "\n")
}

#[test]
fn a_reader_day_round_trips_byte_for_byte() {
    for name in ["reader-day.json", "reader-day-empty.json"] {
        let example = fixture(name);
        let day: ReaderDay = serde_json::from_str(&example).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(format!("{day}\n"), example, "{name}");
    }
    let day: ReaderDay = serde_json::from_str(&fixture("reader-day.json")).unwrap();
    assert_eq!((day.turns.len(), day.turns[0].tool, day.turns[1].reply.as_str()), (2, Harness::ClaudeCode, ""));
}

#[test]
fn page_data_round_trips_byte_for_byte() {
    for name in ["page.json", "page-empty.json"] {
        let example = fixture(name);
        let page: PageData = serde_json::from_str(&example).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(format!("{page}\n"), example, "{name}");
    }
    let page: PageData = serde_json::from_str(&fixture("page.json")).unwrap();
    let states: Vec<WishState> = page.wishes.iter().map(|wish| wish.state).collect();
    assert_eq!(states, [WishState::LetGo, WishState::Done, WishState::NeedsYou, WishState::Working]);
    assert_eq!((page.clashes[0].settled, page.read.turns, page.read.projects.len()), (true, 5, 2));
}

#[test]
fn a_wrong_type_a_missing_field_and_an_unknown_state_or_tool_are_refused() {
    for (name, why) in [
        ("reader-day-missing-turns.json", "missing field `turns`"),
        ("reader-day-wrong-type.json", "invalid type"),
        ("reader-day-unknown-tool.json", "unknown variant `codex`"),
    ] {
        let refused = serde_json::from_str::<ReaderDay>(&fixture(&format!("refused/{name}"))).expect_err(name);
        assert!(refused.to_string().contains(why), "{name}: {refused}");
    }
    for (name, why) in [
        ("page-missing-read.json", "missing field `read`"),
        ("page-missing-stars.json", "missing field `stars`"),
        ("page-wrong-type.json", "invalid type"),
        ("page-unknown-state.json", "unknown variant `finished`"),
    ] {
        let refused = serde_json::from_str::<PageData>(&fixture(&format!("refused/{name}"))).expect_err(name);
        assert!(refused.to_string().contains(why), "{name}: {refused}");
    }
}
