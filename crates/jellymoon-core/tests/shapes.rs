//! The wire shapes are the contract between `jellymoon` and the journal app's shell. Each example under `fixtures/` is
//! what a command prints, byte for byte; each file under `fixtures/refused/` breaks the shape in one way and is refused.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::path::PathBuf;

use jellymoon_core::{
    FileChange, FileKind, Harness, HookState, Kept, PageData, Ran, ReaderDay, Ruling, RunOutcome, Shown, ShownFile,
    WishState, Wished, WorkState,
};

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
fn a_ruling_and_what_a_hook_command_did_round_trip_byte_for_byte() {
    let example = fixture("ruling.json");
    let ruling: Ruling = serde_json::from_str(&example).unwrap();
    assert_eq!((format!("{ruling}\n"), ruling.stands.as_str()), (example, "never retry uploads, fail fast"));
    let example = fixture("hook-state.json");
    let state: HookState = serde_json::from_str(&example).unwrap();
    assert_eq!((format!("{state}\n"), state.installed, state.changed), (example, true, false));
}

#[test]
fn what_a_made_wish_prints_round_trips_byte_for_byte() {
    let example = fixture("wished.json");
    let wished: Wished = serde_json::from_str(&example).unwrap();
    assert_eq!(format!("{wished}\n"), example);
    assert_eq!((wished.wish.id.as_str(), wished.wish.state, wished.run), ("m1", WishState::Working, None));
    // With `--run` the same line names the run the wish was given.
    let example = fixture("wished-run.json");
    let wished: Wished = serde_json::from_str(&example).unwrap();
    assert_eq!(format!("{wished}\n"), example);
    assert_eq!(wished.run.as_deref(), Some("r0123456789abcdef"));
    let more = example.replace("\"stars\":[]", "\"stars\":[],\"label\":\"x\"");
    assert!(serde_json::from_str::<Wished>(&more).is_err(), "a made wish with a field nobody knows is refused");
}

#[test]
fn what_a_run_prints_round_trips_byte_for_byte_for_each_outcome() {
    let outcomes = [
        ("ran-completed.json", RunOutcome::Completed, true, true),
        ("ran-failed.json", RunOutcome::Failed, true, false),
        ("ran-cancelled.json", RunOutcome::Cancelled, true, false),
        ("ran-not-started.json", RunOutcome::NotStarted, false, false),
    ];
    for (name, outcome, started, star) in outcomes {
        let example = fixture(name);
        let ran: Ran = serde_json::from_str(&example).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(format!("{ran}\n"), example, "{name}");
        assert_eq!((ran.outcome, ran.run.is_some(), ran.star.is_some()), (outcome, started, star), "{name}");
    }
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

#[test]
fn what_a_look_at_a_runs_work_prints_round_trips_byte_for_byte() {
    let example = fixture("shown.json");
    let shown: Shown = serde_json::from_str(&example).unwrap();
    assert_eq!(format!("{shown}\n"), example);
    let kinds: Vec<_> = shown.files.iter().map(|file| (file.change, file.kind)).collect();
    let want = [
        (FileChange::Made, FileKind::Text),
        (FileChange::Changed, FileKind::Other),
        (FileChange::Made, FileKind::Link),
        (FileChange::Removed, FileKind::Text),
        (FileChange::Made, FileKind::Repo),
    ];
    assert_eq!((kinds, shown.work, shown.files[0].obeyed), (want.to_vec(), WorkState::Waits, true));
    for state in ["kept", "tossed"] {
        let stood: Shown = serde_json::from_str(&example.replace("waits", state)).unwrap();
        assert_ne!(stood.work, WorkState::Waits);
    }
    for broken in [example.replace("\"more\":0", "\"more\":0,\"diff\":\"x\""), example.replace("waits", "burned")] {
        assert!(serde_json::from_str::<Shown>(&broken).is_err(), "{broken}");
    }
    let example = fixture("shown-file.json");
    let file: ShownFile = serde_json::from_str(&example).unwrap();
    assert_eq!(format!("{file}\n"), example);
    assert_eq!((file.n, file.text.lines().count(), file.cut), (1, 3, false));
}

#[test]
fn what_a_keep_prints_round_trips_byte_for_byte() {
    let example = fixture("kept.json");
    let kept: Kept = serde_json::from_str(&example).unwrap();
    assert_eq!(format!("{kept}\n"), example);
    assert_eq!(
        (kept.kept.map(|id| id.len()), kept.onto.as_deref(), kept.settings.len(), kept.fits),
        (Some(40), Some("main"), 2, None)
    );
    // Asked first: whether it fits is said, and of work that would not be kept there is no commit and no branch.
    let example = fixture("kept-asked.json");
    let asked: Kept = serde_json::from_str(&example).unwrap();
    assert_eq!(format!("{asked}\n"), example);
    assert_eq!((asked.kept, asked.onto, asked.settings.len(), asked.fits), (None, None, 0, Some(false)));
    assert!(serde_json::from_str::<Kept>(&example.replace("\"fits\"", "\"forced\"")).is_err());
    // Work that landed on no branch a record names: a commit, and no branch.
    let example = fixture("kept-loose.json");
    let loose: Kept = serde_json::from_str(&example).unwrap();
    assert_eq!(format!("{loose}\n"), example);
    assert_eq!((loose.kept.map(|id| id.len()), loose.onto, loose.fits), (Some(40), None, None));
    // Work kept past a `.gitattributes` git could not remove: the file is named, and no other answer has the name.
    let example = fixture("kept-stayed.json");
    let stayed: Kept = serde_json::from_str(&example).unwrap();
    assert_eq!(format!("{stayed}\n"), example);
    assert_eq!((stayed.stayed.len(), kept.stayed.len()), (1, 0));
}
