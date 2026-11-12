//! The run of a wish entered in the window. The world, with its made-up window, is `support/wished.rs`.

use std::fs;

use jellymoon_app::wishing::UNSTARTED;
use serde_json::Value;
use sticker_kit::{Mark, Note, Scene, State};

use super::{SAID, drawn, first};
use crate::support::day::called;
use crate::support::open_above;
use crate::support::wished::{World, showed, wait_until, world};

const STAR: &str = "a run reported it finished";

/// The rows of the list of what came true.
fn came_true(scene: &Scene) -> Vec<(String, String)> {
    let rows = scene.notes.iter().find_map(|note| match note {
        Note::List { title, rows } if title == "came true today" => Some(rows.clone()),
        _ => None,
    });
    rows.unwrap_or_default()
}

#[test]
fn a_wish_entered_starts_its_run_apart_and_the_page_shows_its_star_with_no_tap() {
    let w = world();
    w.install();
    if open_above(w.day.dir.path()).is_some() {
        return;
    }
    w.play("slow");
    let window = w.enters(0, "watch");
    wait_until("the agent", || w.at("rec").join("started").is_file());
    let going = w.kept();
    assert_eq!((&going["state"], going["run"].is_string()), (&Value::from("working"), true), "{going}");
    fs::write(w.at("rec").join("go"), "").unwrap();

    let scenes = showed(window);
    assert_eq!(scenes.len(), 2, "the wish at once, and the star: the asking had nothing to say");
    // At once: working, in the person's words, before any run was asked for.
    assert_eq!(drawn(&scenes[0], "m1"), Some((State::Busy, 0, Mark::Busy, SAID.to_string())));
    // The same page on show, looked at again: no tap, and never opened again.
    assert_eq!(drawn(&scenes[1], "m1"), Some((State::Idle, 1, Mark::Star, SAID.to_string())));
    let stars: Vec<String> = came_true(&scenes[1]).into_iter().map(|(star, _)| star).collect();
    assert!(stars.contains(&STAR.to_string()) && !came_true(&scenes[0]).iter().any(|(star, _)| star == STAR));
    assert_eq!((w.starts(), called(&w.day)), (1, false), "one run of the agent, and no reading call");
}

// The run is the installed command's, not the window's: the window is gone while the agent still waits, and the
// run ends, with its star, all the same.
#[test]
fn the_run_of_a_wish_goes_on_when_the_window_is_gone() {
    let w = world();
    w.install();
    if open_above(w.day.dir.path()).is_some() {
        return;
    }
    w.play("slow");
    let scenes = showed(w.enters(0, "leave"));
    assert_eq!(scenes.len(), 1, "{:?}", scenes.last().and_then(first));
    wait_until("the agent", || w.at("rec").join("started").is_file());
    assert_eq!(w.kept()["state"], "working", "the run ended with the window");
    fs::write(w.at("rec").join("go"), "").unwrap();
    wait_until("the star", || w.kept()["stars"] == serde_json::json!([STAR]));
    w.ended();
    assert_eq!((w.kept()["state"].as_str(), w.starts()), (Some("done"), 1));
}

/// What the page says of a wish that was entered and has no run: the wish is kept and drawn working, and one note.
fn unstarted(w: &World, option: usize, wish: &str) -> String {
    let scenes = showed(w.enters(option, "leave"));
    assert_eq!(scenes.len(), 2, "the wish at once, then why it has no run");
    assert!(drawn(&scenes[0], wish).is_some() && first(&scenes[0]).is_none_or(|(title, _)| title != UNSTARTED));
    assert_eq!(drawn(&scenes[1], wish).map(|drawn| drawn.0), Some(State::Busy));
    let said = scenes[1].notes.iter().filter(|note| matches!(note, Note::Plain { title, .. } if title == UNSTARTED));
    assert_eq!(said.count(), 1);
    let copies = fs::read_dir(w.at("jm").join("copies")).map_or(0, Iterator::count);
    assert_eq!((w.starts(), copies), (0, 0), "something was started");
    let (title, body) = first(&scenes[1]).unwrap();
    assert_eq!(title, UNSTARTED);
    // The note names its wish, then says why in core's words.
    body.strip_prefix(&format!("“{SAID}”: ")).expect(body).to_string()
}

// A folder switched off by a name above it is still offered, since only its name is on the page: the window
// passes the names on, and the run, which reads the folder, starts nothing.
#[test]
fn the_names_switched_off_reach_the_run_which_starts_nothing_in_a_folder_they_name() {
    let w = world();
    w.install();
    if open_above(w.day.dir.path()).is_some() {
        return;
    }
    let above = format!("work{}alph", std::path::MAIN_SEPARATOR);
    let scenes = showed(w.enters_with(0, "leave", &above));
    assert_eq!(scenes.len(), 2, "the wish at once, then why it has no run");
    assert_eq!(first(&scenes[1]).map(|(title, _)| title), Some(UNSTARTED));
    let copies = fs::read_dir(w.at("jm").join("copies")).map_or(0, Iterator::count);
    assert_eq!((w.starts(), copies, w.kept()["run"].is_null()), (0, 0, true), "a run was started in it");
}

#[test]
fn a_command_that_is_not_found_and_a_run_that_was_not_started_each_leave_one_note_that_says_why() {
    let w = world();
    assert_eq!(unstarted(&w, 0, "m1"), "the installed jellymoon command is not used: it was not found.");
    let command = w.install();
    #[cfg(unix)]
    {
        // A command others can write to is not the one `install.sh` put there, and is never started.
        fs::set_permissions(&command, std::os::unix::fs::PermissionsExt::from_mode(0o775)).unwrap();
        assert_eq!(unstarted(&w, 0, "m2"), "the installed jellymoon command is not used: others can write to it.");
        fs::set_permissions(&command, std::os::unix::fs::PermissionsExt::from_mode(0o755)).unwrap();
    }
    if open_above(w.day.dir.path()).is_some() {
        return;
    }
    // The command is started, finds no folder for the project, and says so: its line is the note.
    let wish = if cfg!(unix) { "m3" } else { "m2" };
    let no_repo = "the folder of that name typed in on 2026-03-10 is not one a run can copy: it is not there, or cannot \
                   be reached.";
    assert_eq!(unstarted(&w, 1, wish), no_repo);
    assert!(command.is_file());
}
