//! The wire shapes of a hearing (F55): the events `jellymoon hear` reads, the lines it prints and the settings of
//! voice. Each fixture is an example byte for byte, and each of its lines is in `docs/development/hear.md`.
//! `hear-lines.jsonl` is what the binary prints for `hear-events.jsonl` (`crates/jellymoon/tests/f55.rs`).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::path::PathBuf;

use jellymoon_core::{
    DayOptions, EarCannot, Error, HearEvent, HearOptions, Hearing, Jelly, JellyState, KeyMove, Said, Voice,
};

fn read(path: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(path);
    // A checkout on Windows may write line breaks its own way.
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display())).replace("\r\n", "\n")
}

fn fixture(name: &str) -> String {
    read(&format!("tests/fixtures/{name}"))
}

#[test]
fn every_event_round_trips_byte_for_byte() {
    let example = fixture("hear-events.jsonl");
    let events: Vec<HearEvent> = example.lines().map(|line| serde_json::from_str(line).unwrap()).collect();
    let written: String = events.iter().map(|event| serde_json::to_string(event).unwrap() + "\n").collect();
    assert_eq!(written, example);
    let cannot = Said::Cannot { why: EarCannot::MicrophoneRefused };
    assert_eq!(events[1], HearEvent::Key { key: KeyMove::Down });
    assert_eq!((&events[3], &events[10]), (&HearEvent::Tick { tick: 1200 }, &HearEvent::Said(cannot)));
    assert_eq!(events[2], HearEvent::Said(Said::Partial { words: "make the up".to_string() }));
    // Every reason the ear has is one a line can name, and none is known under another name.
    for why in EarCannot::ALL {
        let name = serde_json::to_string(&why).unwrap();
        let event: HearEvent = serde_json::from_str(&format!(r#"{{"said":"cannot","why":{name}}}"#)).unwrap();
        assert_eq!(event, HearEvent::Said(Said::Cannot { why }));
    }
    assert!(serde_json::from_str::<HearEvent>(r#"{"said":"cannot","why":"NoEar"}"#).is_err());
}

#[test]
fn every_line_round_trips_byte_for_byte_and_the_examples_show_all_seven_states() {
    let mut states = Vec::new();
    for name in ["hear-lines.jsonl", "hear-runs.jsonl"] {
        let example = fixture(name);
        let lines: Vec<Jelly> = example.lines().map(|line| serde_json::from_str(line).unwrap()).collect();
        assert_eq!(lines.iter().map(|line| format!("{line}\n")).collect::<String>(), example, "{name}");
        states.extend(lines.iter().map(|line| line.state));
    }
    let all = [
        JellyState::Quiet,
        JellyState::Listening,
        JellyState::Heard,
        JellyState::Working,
        JellyState::Asks,
        JellyState::Waits,
        JellyState::Cannot,
    ];
    assert!(all.iter().all(|state| states.contains(state)), "{states:?}");
    // A line with a name nobody knows, or without one of the five every line has, is no line.
    let quiet = serde_json::to_string(&Jelly::quiet()).unwrap();
    assert_eq!(format!("{quiet}\n"), fixture("hear-lines.jsonl").lines().next().unwrap().to_string() + "\n");
    assert!(serde_json::from_str::<Jelly>(&quiet.replace("\"left\":0", "\"left\":0,\"clock\":1")).is_err());
    assert!(serde_json::from_str::<Jelly>(&quiet.replace(",\"left\":0", "")).is_err());
    assert!(serde_json::from_str::<Jelly>(&quiet.replace("quiet", "asleep")).is_err());
}

#[test]
fn the_settings_of_voice_round_trip_byte_for_byte() {
    let example = fixture("voice.json");
    let voice: Voice = serde_json::from_str(&example).unwrap();
    assert_eq!(
        (format!("{voice}\n"), voice.key.as_str(), voice.language.as_str()),
        (example, "Control+Shift+Space", "default")
    );
}

// The doc's examples are the fixtures' own lines, and it says every sentence the ear's refusals are said in.
#[test]
fn the_doc_shows_every_line_of_the_fixtures_and_every_sentence() {
    let doc = read("../../docs/development/hear.md");
    for name in ["hear-events.jsonl", "hear-lines.jsonl", "hear-runs.jsonl", "voice.json"] {
        for line in fixture(name).lines() {
            assert!(doc.contains(line), "docs/development/hear.md lacks this line of {name}: {line}");
        }
    }
    for why in EarCannot::ALL {
        let name = serde_json::to_string(&why).unwrap();
        assert!(doc.contains(why.says()) && doc.contains(name.trim_matches('"')), "{name}");
    }
}

#[test]
fn each_sentence_of_the_ear_is_one_plain_line_of_its_own() {
    let says: Vec<&str> = EarCannot::ALL.iter().map(|why| why.says()).collect();
    for (at, sentence) in says.iter().enumerate() {
        let plain = sentence.chars().all(|c| c == ' ' || c.is_ascii_graphic()) && !sentence.contains("  ");
        assert!(plain && !sentence.is_empty() && sentence.len() <= 200 && !sentence.ends_with('.'), "{sentence}");
        assert!(!says[..at].contains(sentence), "{sentence} is said twice");
    }
}

#[test]
fn a_hearing_begins_only_with_a_home_that_is_a_full_path_and_a_day_that_is_one() {
    let home = tempfile::tempdir().unwrap();
    let options = |home: Option<PathBuf>, date: &str| {
        let read = DayOptions { logs: PathBuf::from("no-logs"), date: Some(date.to_string()), skip: Vec::new(), home };
        HearOptions { read, apart: None }
    };
    assert!(matches!(Hearing::begin(options(None, "2026-03-10")), Err(Error::Home)));
    assert!(matches!(Hearing::begin(options(Some(PathBuf::from("jm")), "2026-03-10")), Err(Error::Home)));
    assert!(matches!(Hearing::begin(options(Some(home.path().to_path_buf()), "2026-3-10")), Err(Error::Date)));
    // An event that changes what is shown gives the line, and one that changes nothing gives none.
    let mut hearing = Hearing::begin(options(Some(home.path().to_path_buf()), "2026-03-10")).unwrap();
    assert_eq!(hearing.shows(), &Jelly::quiet());
    let down = HearEvent::Key { key: KeyMove::Down };
    let listening = Jelly { state: JellyState::Listening, ..Jelly::quiet() };
    assert_eq!((hearing.event(down.clone()), hearing.event(down)), (Some(listening.clone()), None));
    assert_eq!((hearing.line(b"not an event\n"), hearing.line(b" \r\n"), hearing.skipped()), (None, None, 1));
    assert_eq!(hearing.shows(), &listening);
    assert_eq!(fs::read_dir(home.path()).unwrap().count(), 0, "a hearing wrote to its home");
}
