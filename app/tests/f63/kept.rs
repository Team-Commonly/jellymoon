//! The kept page of a day with no session logs is no earlier reading: when the day can be read, or cannot be, it
//! is as if nothing were kept. And a page says that no logs were found only when it holds nothing a reading gives.

use std::fs;

use serde_json::json;

use super::{FOUND_NO_LOGS, Logs, NO_LOGS, UNREAD, list, nothing_typed, of, words};
use crate::support::day::{DAY, answer, called, kept, noon, notes, said, write_log};
use crate::support::wished::World;
use crate::support::{Piece, pieces, wish};

const EARLIER: &str = "This page is from an earlier reading.";

/// A day with a project on offer and no logs, drawn once: its no-logs page is kept.
fn kept_with_no_logs() -> World {
    let w = nothing_typed(Logs::Absent);
    list(&w, "alpha");
    w.placed();
    assert_eq!(fs::read_to_string(kept(&w.day)).unwrap(), NO_LOGS);
    w
}

/// The first note says the day could not be read, with `why`, and nothing on the page is of a reading.
fn unread(placed: &[Piece], why: &str) {
    let said = notes(placed);
    assert!(said[0].contains(UNREAD) && said[0].contains(why), "{said:?}");
    assert!(said.iter().all(|note| !note.contains(EARLIER) && !note.contains("Reading your day again")), "{said:?}");
    assert!(words(placed, "footer").iter().all(|footer| !footer.contains(FOUND_NO_LOGS)), "{placed:?}");
}

#[test]
fn a_kept_page_of_a_day_with_no_logs_is_no_earlier_reading() {
    // The project is forgotten: core has no page for the day any more, and the reader's answer is the note.
    let w = kept_with_no_logs();
    w.jellymoon(&["project", "forget", "alpha"]);
    unread(&w.placed(), "No Claude Code session logs were found on this computer");

    // Logs come, with one turn of the day, and the reading fails: the day is read for the first time.
    let w = kept_with_no_logs();
    let folder = w.at("work").join("alpha");
    fs::create_dir(w.at("logs")).unwrap();
    write_log(&w.day, "-work-alpha/s1.jsonl", &[said(&noon(0), folder.to_str().unwrap(), "look at the uploader")]);
    answer(&w.day, "not a reply", 1);
    unread(&w.placed(), "Claude Code could not do the reading");
    assert!(called(&w.day));

    // The logs are named under a regular file: where the system says so, they cannot be read.
    let w = kept_with_no_logs();
    let under = w.at("gitconfig").join("logs");
    let placed = w.placed_with(&["--logs", under.to_str().unwrap()]);
    assert!(notes(&placed).iter().all(|note| !note.contains(EARLIER)), "{:?}", notes(&placed));
    if cfg!(unix) {
        unread(&placed, "Your Claude Code session logs could not be read");
    }
}

#[test]
fn a_model_that_is_no_name_is_said_in_cores_words_on_a_day_with_no_logs() {
    let w = nothing_typed(Logs::Absent);
    list(&w, "alpha");
    let said = notes(&w.placed_with(&["--model", "-p"]));
    assert!(said[0].contains(UNREAD) && said[0].contains("--model must be a model's name"), "{said:?}");
    assert!(!said[0].contains("No Claude Code session logs"), "{said:?}");
    assert!(!kept(&w.day).exists() && !called(&w.day));
}

#[test]
fn a_page_that_says_no_logs_and_holds_a_reading_is_drawn_as_the_reading() {
    let read = json!({"turns": 3, "projects": ["alpha"], "no_logs": true});
    let page =
        json!({"day": DAY, "wishes": [wish("w1", "Retry uploads", "working", &[])], "clashes": [], "read": read});
    let reading = "Read from 3 things you typed in 1 project (alpha). This is the model's reading, unedited.";
    // As a file named with `--page`.
    let placed = pieces(&page, &[]);
    assert_eq!((of(&placed, "character").len(), words(&placed, "footer")), (1, vec![reading.to_string()]));
    // As the kept page of a day with no logs and a project on offer: it is the earlier reading, and stays.
    let w = nothing_typed(Logs::Absent);
    list(&w, "alpha");
    fs::create_dir_all(kept(&w.day).parent().unwrap()).unwrap();
    fs::write(kept(&w.day), format!("{page}\n")).unwrap();
    let placed = w.placed();
    assert!(notes(&placed)[0].contains(EARLIER), "{:?}", notes(&placed));
    assert_eq!((of(&placed, "character").len(), words(&placed, "footer")), (1, vec![reading.to_string()]));
    assert_eq!(fs::read_to_string(kept(&w.day)).unwrap(), format!("{page}\n"));

    // A page that says no logs and holds only a clash, or only a project read, is no page of a day with no logs.
    for (clashes, projects) in
        [(json!([crate::support::clash("a", "b", "do it", "do not", false)]), json!([])), (json!([]), json!(["alpha"]))]
    {
        let read = json!({"turns": 0, "projects": projects, "no_logs": true});
        let placed = pieces(&json!({"day": DAY, "wishes": [], "clashes": clashes, "read": read}), &[]);
        assert!(words(&placed, "footer").iter().all(|footer| footer.starts_with("Read from 0 things")), "{placed:?}");
    }
}
