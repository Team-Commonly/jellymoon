//! F64: a folder dropped on the window is a project (ADR 0020, decision 4). The rules are core's, proved at
//! `jellymoon project add` (`crates/jellymoon/tests/f62.rs`). Here is the app's part: a folder given to core by
//! the made-up window of F42's tests (`support/wisher.rs`), what `jellymoon-app layout` then places, and, on the
//! Mac, a drop and the folder picker played on GPUI's test platform (`f64/window.rs`). The folder's path is never
//! drawn. Nothing here starts the real Claude Code, and no window is opened on a screen.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;
#[cfg(all(target_os = "macos", feature = "window"))]
#[path = "f64/window.rs"]
mod window;

use std::fs;
use std::path::Path;

use jellymoon_app::starting::{ADDED, CHOOSE, UNADDED};
use sticker_kit::{Note, Scene};
use support::Piece;
use support::day::called;
use support::wished::{World, quiet, showed};

fn words(placed: &[Piece], kind: &str) -> Vec<String> {
    placed.iter().filter(|piece| piece.kind() == kind).map(Piece::said).collect()
}

/// Whether anything the scene draws holds the folder's path, or the world's folder it is in.
fn draws(scene: &Scene, folder: &Path, w: &World) -> bool {
    let drawn = serde_json::to_string(scene).unwrap();
    let paths = [folder.to_str().unwrap(), w.day.dir.path().to_str().unwrap()];
    paths.iter().any(|path| drawn.contains(path) || drawn.contains(&path.replace('\\', "\\\\")))
}

fn notes_of(scene: &Scene) -> Vec<(&str, &str)> {
    fn plain(note: &Note) -> Option<(&str, &str)> {
        match note {
            Note::Plain { title, body } if [ADDED, UNADDED].contains(&title.as_str()) => {
                Some((title.as_str(), body.as_str()))
            }
            _ => None,
        }
    }
    scene.notes.iter().filter_map(plain).collect()
}

/// A folder that is a project to core: a `.git` in it, as a listed project is looked for.
fn repo(w: &World, name: &str) -> std::path::PathBuf {
    let folder = w.at("work").join(name);
    fs::create_dir_all(folder.join(".git")).unwrap();
    folder
}

#[test]
fn a_folder_given_is_listed_by_core_named_in_one_note_and_has_a_chip_at_once() {
    for logs in [false, true] {
        let w = quiet();
        if !logs {
            fs::remove_dir(w.at("logs")).unwrap();
        }
        let folder = repo(&w, "garden");
        let scene = showed(w.gives(folder.to_str().unwrap(), "folder"));
        assert_eq!(notes_of(&scene[0]), [(ADDED, "“garden” is ready: type what you want done.")], "logs {logs}");
        assert!(!draws(&scene[0], &folder, &w), "logs {logs}: the path was drawn");
        let placed = w.placed();
        assert_eq!(words(&placed, "chip"), ["garden"], "logs {logs}");
        assert_eq!(words(&placed, "action"), [CHOOSE], "logs {logs}");
        assert!(!called(&w.day), "logs {logs}");
    }
}

#[test]
fn a_folder_core_refuses_is_one_note_in_core_s_words_and_its_path_is_never_drawn() {
    let w = quiet();
    let plain = w.at("work").join("not-a-repo");
    fs::create_dir(&plain).unwrap();
    let gone = w.at("work").join("gone");
    for (folder, why) in
        [(&plain, "That folder is not the top of a git repo."), (&gone, "That folder is not a folder that is there.")]
    {
        let scene = showed(w.gives(folder.to_str().unwrap(), "folder"));
        assert_eq!(notes_of(&scene[0]), [(UNADDED, why)]);
        assert!(!draws(&scene[0], folder, &w), "the path was drawn");
    }
    let list = w.jellymoon(&["project", "list"]);
    assert!(list["projects"].as_array().unwrap().is_empty());
}
