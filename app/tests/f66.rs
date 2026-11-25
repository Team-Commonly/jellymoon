//! F66: with no project on offer, the entry offers to start something new (ADR 0020, decision 3). The rules of a
//! new project are core's, proved at `jellymoon project new` (`crates/jellymoon/tests/f65.rs`). Here is the app's
//! part: what `jellymoon-app layout` places on a day with no project on offer, and a name carried to core by the
//! made-up window of F42's tests (`support/wisher.rs`), in a process of its own, where core starts git with the
//! test's variables. Nothing here starts the real Claude Code, and no window is opened on a screen.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;
#[cfg(all(target_os = "macos", feature = "window"))]
#[path = "f66/window.rs"]
mod window;

use std::fs;

use jellymoon_app::starting::{CHOOSE, MADE, NEW_HINT, START, UNMADE};
use jellymoon_app::wishing::{HINT, HINT_TAB};
use sticker_kit::{Note, Scene};
use support::Piece;
use support::day::{called, notes};
use support::wished::{World, quiet, showed};

pub fn of<'a>(placed: &'a [Piece], kind: &str) -> Vec<&'a Piece> {
    placed.iter().filter(|piece| piece.kind() == kind).collect()
}

pub fn words(placed: &[Piece], kind: &str) -> Vec<String> {
    of(placed, kind).iter().map(|piece| piece.said()).collect()
}

/// A world with nothing typed and nothing listed; with `logs` false, no session logs at all: a first run.
pub fn first_run(logs: bool) -> World {
    let w = quiet();
    if !logs {
        fs::remove_dir(w.at("logs")).unwrap();
    }
    w
}

/// The names `jellymoon project list` prints.
pub fn listed(w: &World) -> Vec<String> {
    let list = w.jellymoon(&["project", "list"]);
    list["projects"].as_array().unwrap().iter().map(|one| one["name"].as_str().unwrap().to_string()).collect()
}

fn first(scene: &Scene) -> Option<(&str, &str)> {
    match scene.notes.first()? {
        Note::Plain { title, body } => Some((title, body)),
        _ => None,
    }
}

#[test]
fn with_no_project_on_offer_the_entry_holds_one_choice_a_field_for_a_name_and_the_folder_picker() {
    for logs in [false, true] {
        let w = first_run(logs);
        let placed = w.placed();
        assert_eq!(words(&placed, "entry"), [NEW_HINT], "logs {logs}: the field for a name, and its hint");
        assert_eq!(words(&placed, "chip"), [START], "logs {logs}: one choice");
        assert_eq!(words(&placed, "action"), [CHOOSE], "logs {logs}: the folder picker, after the chip");
        let (chip, action) = (of(&placed, "chip")[0].rect(), of(&placed, "action")[0].rect());
        assert!(chip[0] + chip[2] <= action[0] && (chip[1] - action[1]).abs() < 0.01, "{chip:?} {action:?}");
        // The page says what to do here, in the window, and never sends the person away.
        let said = notes(&placed).join(" ");
        assert!(!said.contains("Ask one of your agents") && !said.contains("open Jellymoon again"), "{said}");
        // With no project the field takes a name: the page says to choose a folder first (the entry-first slice).
        // A day with no logs has the note that it has no page; one whose logs were read, the strip's line.
        let none = ("hoose the folder you want to work in", "a name for a new project");
        let own = if logs { ("hoose a folder to work in", "name a new project") } else { none };
        let (folder, name) = (said.find(own.0), said.find(own.1));
        assert!(folder.is_some() && folder < name, "logs {logs}: {said}");
        assert!(!called(&w.day), "logs {logs}: a reading call was made");
    }
    // The hint is whole at the smallest page, in both themes.
    let w = first_run(false);
    for theme in ["sticker", "plain"] {
        let placed = w.placed_with(&["--theme", theme, "--size", "960x640"]);
        assert_eq!(words(&placed, "entry"), [NEW_HINT], "{theme}");
        assert_eq!(words(&placed, "action"), [CHOOSE], "{theme}");
    }
}

#[test]
fn with_a_project_on_offer_there_is_no_choice_to_start_and_the_hint_says_tab_only_for_more_than_one() {
    let w = first_run(false);
    w.jellymoon(&["project", "add", "--repo", w.at("work").join("alpha").to_str().unwrap()]);
    let placed = w.placed_with(&["--size", "960x640"]);
    assert_eq!((words(&placed, "entry"), words(&placed, "chip")), (vec![HINT.to_string()], vec!["alpha".to_string()]));
    assert_eq!(words(&placed, "action"), [CHOOSE], "the folder picker is offered beside a project too");
    fs::create_dir_all(w.at("work").join("beta").join(".git")).unwrap();
    w.jellymoon(&["project", "add", "--repo", w.at("work").join("beta").to_str().unwrap()]);
    for theme in ["sticker", "plain"] {
        let placed = w.placed_with(&["--theme", theme, "--size", "960x640"]);
        assert_eq!(words(&placed, "entry"), [HINT_TAB], "{theme}: whole at the smallest page");
        assert_eq!(words(&placed, "chip"), ["alpha", "beta"], "{theme}");
        assert!(!words(&placed, "chip").contains(&START.to_string()));
    }
}

#[test]
fn a_name_goes_to_core_s_project_new_and_the_new_project_has_a_chip() {
    let w = first_run(false);
    let scene = showed(w.gives("garden", "name"));
    let (title, body) = first(&scene[0]).expect("one note");
    assert_eq!((title, body), (MADE, "“garden” is ready: type what you want done."));
    assert_eq!(listed(&w), ["garden"], "core made and listed it");
    assert!(w.at("jm").join("projects").join("garden").join(".git").is_dir());
    let placed = w.placed();
    assert_eq!((words(&placed, "chip"), words(&placed, "entry")), (vec!["garden".to_string()], vec![HINT.to_string()]));
    assert!(!called(&w.day));
}

#[test]
fn a_name_core_refuses_is_one_note_in_core_s_words_and_nothing_is_made() {
    let w = first_run(false);
    let scene = showed(w.gives(".hidden", "name"));
    let (title, body) = first(&scene[0]).expect("one note");
    assert_eq!(title, UNMADE);
    assert!(body.starts_with("A new project takes a name for the new project") && body.ends_with("nothing was made."));
    let notes = scene[0].notes.iter().filter(|note| matches!(note, Note::Plain { title, .. } if title == UNMADE));
    assert_eq!(notes.count(), 1);
    assert!(listed(&w).is_empty() && !w.at("jm").join("projects").join(".hidden").exists());
    assert_eq!(words(&w.placed(), "chip"), [START], "still the choice to start something new");
}
