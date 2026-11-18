//! F63: a day nothing was typed on has a page of its made wishes and an entry (ADR 0020, decision 5). The rules
//! are core's, proved at `jellymoon page` (`crates/jellymoon/tests/f63.rs`). Here is the app's part: what
//! `jellymoon-app layout` places on such a day, over logs that are not there, that hold nothing, or that hold only
//! another day; the chips of the projects core offers; and a wish entered on such a day, carried by the made-up
//! window of F42's tests (`support/wisher.rs`). A project is listed with `jellymoon project add`, as a person
//! types it, and a run is the made-up agent's: nothing here starts the real Claude Code, and no window is opened.
#![allow(clippy::unwrap_used, clippy::expect_used)]

#[path = "f63/chips.rs"]
mod chips;
#[path = "f63/made.rs"]
mod made;
#[path = "f63/kept.rs"]
mod no_logs_kept;
mod support;

use std::fs;
use std::time::{Duration, SystemTime};

use support::Piece;
use support::day::{DAY, called, kept, notes, said, write_log};
use support::wished::{World, quiet};

/// What `jellymoon page` prints of a day with logs, nothing typed and nothing made: the page kept of it.
const EMPTY: &str = "{\"day\":\"2026-03-10\",\"wishes\":[],\"clashes\":[],\"read\":{\"turns\":0,\"projects\":[]}}\n";
/// The same of a day with no logs at all, which has a page since something is made or on offer.
pub const NO_LOGS: &str =
    "{\"day\":\"2026-03-10\",\"wishes\":[],\"clashes\":[],\"read\":{\"turns\":0,\"projects\":[],\"no_logs\":true}}\n";
pub const NOTHING: &str = "Nothing wished for yet today.";
pub const UNREAD: &str = "Today could not be read.";
pub const NO_PAGE: &str = "There is no page for this day yet.";
pub const HINT: &str = "What do you want done? Enter starts it.";
pub const NEW_HINT: &str = "Name a new project, or drop a folder here";
pub const START: &str = "Start something new";
pub const READ_NOTHING: &str = "Nothing typed on this day was read, so there is no reading.";
pub const FOUND_NO_LOGS: &str = "No Claude Code session logs were found, so nothing was read.";

/// What the day's session logs are.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Logs {
    /// No folder of them at all.
    Absent,
    /// A folder that holds nothing.
    Empty,
    /// A folder whose one session was typed in `alpha` two days before.
    OtherDay,
}

pub const EVERY: [Logs; 3] = [Logs::Absent, Logs::Empty, Logs::OtherDay];

/// A world where nothing was typed on the day, with its logs as `logs` says.
pub fn nothing_typed(logs: Logs) -> World {
    let w = quiet();
    match logs {
        Logs::Absent => fs::remove_dir(w.at("logs")).unwrap(),
        Logs::Empty => {}
        Logs::OtherDay => {
            let folder = w.at("work").join("alpha");
            let line = said("2026-03-08T12:00:00.000Z", folder.to_str().unwrap(), "look at the uploader");
            write_log(&w.day, "-work-alpha/s1.jsonl", &[line]);
        }
    }
    w
}

/// Lists the repo `work/<name>` as a project Jellymoon knows, as a person types it.
pub fn list(w: &World, name: &str) {
    let repo = w.at("work").join(name);
    let added = w.jellymoon(&["project", "add", "--repo", repo.to_str().unwrap(), "--date", DAY]);
    assert_eq!(added["name"], name, "{added}");
}

pub fn of<'a>(placed: &'a [Piece], kind: &str) -> Vec<&'a Piece> {
    placed.iter().filter(|piece| piece.kind() == kind).collect()
}

/// The words on the pieces of a kind, in the order placed.
pub fn words(placed: &[Piece], kind: &str) -> Vec<String> {
    of(placed, kind).iter().map(|piece| piece.said()).collect()
}

pub fn values(placed: &[Piece]) -> Vec<&serde_json::Value> {
    placed.iter().map(|piece| &piece.0).collect()
}

#[test]
fn a_day_with_nothing_typed_nothing_made_and_no_project_on_offer_is_drawn_as_it_was() {
    for logs in [Logs::Empty, Logs::OtherDay] {
        let w = nothing_typed(logs);
        let placed = w.placed();
        for kind in ["more", "character"] {
            assert!(of(&placed, kind).is_empty(), "{logs:?}: a {kind} was placed");
        }
        assert_eq!(
            (words(&placed, "entry"), words(&placed, "chip")),
            (vec![NEW_HINT.to_string()], vec![START.to_string()])
        );
        let said = notes(&placed);
        assert!(said.len() == 1 && said[0].contains(NOTHING), "{logs:?}: {said:?}");
        assert_eq!(words(&placed, "footer"), [READ_NOTHING], "{logs:?}");
        assert_eq!(fs::read_to_string(kept(&w.day)).unwrap(), EMPTY, "{logs:?}");
        assert!(!called(&w.day), "{logs:?}: a reading call was made");
    }
    // No logs at all, and nothing to draw: the day has no page, and none is kept.
    let w = nothing_typed(Logs::Absent);
    let placed = w.placed();
    for kind in ["more", "character"] {
        assert!(of(&placed, kind).is_empty(), "a {kind} was placed");
    }
    assert_eq!(
        (words(&placed, "entry"), words(&placed, "chip")),
        (vec![NEW_HINT.to_string()], vec![START.to_string()])
    );
    let said = notes(&placed);
    assert!(said.len() == 1 && said[0].contains(NO_PAGE) && said[0].contains("No Claude Code session logs"));
    assert!(!said[0].contains(UNREAD), "{said:?}");
    assert!(!said[0].contains(NOTHING), "{said:?}");
    assert!(!kept(&w.day).exists() && !called(&w.day));
}

#[test]
fn a_project_on_offer_gives_the_day_an_entry_and_its_chip_and_no_wish() {
    for logs in EVERY {
        let w = nothing_typed(logs);
        list(&w, "alpha");
        let placed = w.placed();
        assert_eq!(words(&placed, "entry"), [HINT], "{logs:?}: one entry, with its hint");
        assert_eq!(words(&placed, "chip"), ["alpha"], "{logs:?}");
        assert!(of(&placed, "character").is_empty() && of(&placed, "more").is_empty(), "{logs:?}");
        // No wish and no clash is made up, and the day is not said to be unreadable.
        let said = notes(&placed);
        assert!(said.len() == 1 && said[0].contains(NOTHING), "{logs:?}: {said:?}");
        let (page, footer) = if logs == Logs::Absent { (NO_LOGS, FOUND_NO_LOGS) } else { (EMPTY, READ_NOTHING) };
        assert_eq!(words(&placed, "footer"), [footer], "{logs:?}");
        assert_eq!(fs::read_to_string(kept(&w.day)).unwrap(), page, "{logs:?}");
        // The same day again: the same pieces from the same kept page, which is not written a second time.
        let long_ago = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000_000);
        fs::File::options().write(true).open(kept(&w.day)).unwrap().set_modified(long_ago).unwrap();
        assert_eq!(values(&w.placed()), values(&placed), "{logs:?}");
        assert_eq!(fs::read_to_string(kept(&w.day)).unwrap(), page, "{logs:?}");
        assert_eq!(fs::metadata(kept(&w.day)).unwrap().modified().unwrap(), long_ago, "{logs:?}: kept again");
        assert!(!called(&w.day), "{logs:?}: a reading call was made");
    }
}

#[test]
fn a_page_that_read_something_stays_when_the_logs_are_gone_and_a_made_wish_alone_has_a_page() {
    // A typed day, read: its page is kept. The logs go: the kept page is the earlier reading, and is not replaced.
    let w = support::wished::world();
    list(&w, "alpha");
    let (read, was) = (w.placed(), fs::read_to_string(kept(&w.day)).unwrap());
    fs::remove_dir_all(w.at("logs")).unwrap();
    let placed = w.placed();
    assert!(notes(&placed)[0].contains("This page is from an earlier reading."), "{:?}", notes(&placed));
    assert_eq!(words(&placed, "footer"), words(&read, "footer"));
    assert_eq!(of(&placed, "character").len(), of(&read, "character").len());
    assert_eq!(fs::read_to_string(kept(&w.day)).unwrap(), was);

    // A wish made for a project that is then forgotten: the day has its page for the wish, and no entry.
    let w = nothing_typed(Logs::Absent);
    list(&w, "alpha");
    w.jellymoon(&["wish", "--date", DAY, "--say", "tidy the uploader", "--project", "alpha"]);
    w.jellymoon(&["project", "forget", "alpha"]);
    let placed = w.placed();
    assert_eq!(
        (of(&placed, "character").len(), words(&placed, "entry")),
        (1, vec![NEW_HINT.to_string()]),
        "{placed:?}"
    );
    assert_eq!(words(&placed, "footer"), [format!("{FOUND_NO_LOGS} 1 made wish is in your own words.")]);
    assert_eq!(fs::read_to_string(kept(&w.day)).unwrap(), NO_LOGS);
    assert!(!called(&w.day));
}

#[test]
fn a_project_that_is_forgotten_takes_the_entry_with_it_and_logs_that_come_are_read() {
    let w = nothing_typed(Logs::Absent);
    list(&w, "alpha");
    assert_eq!(words(&w.placed(), "chip"), ["alpha"]);
    assert_eq!(fs::read_to_string(kept(&w.day)).unwrap(), NO_LOGS);
    // Logs that hold nothing: they were read, and the kept page says so from then on.
    fs::create_dir(w.at("logs")).unwrap();
    let placed = w.placed();
    assert_eq!(
        (words(&placed, "chip"), words(&placed, "footer")),
        (vec!["alpha".to_string()], vec![READ_NOTHING.to_string()])
    );
    assert_eq!(fs::read_to_string(kept(&w.day)).unwrap(), EMPTY);
    // Nothing is on offer any more: no entry, and the page is the empty day's.
    w.jellymoon(&["project", "forget", "alpha"]);
    let placed = w.placed();
    assert_eq!(
        (words(&placed, "entry"), words(&placed, "chip")),
        (vec![NEW_HINT.to_string()], vec![START.to_string()])
    );
    assert!(notes(&placed)[0].contains(NOTHING) && !called(&w.day));
}
