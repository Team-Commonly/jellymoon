//! Open a wish: the page says once, in its legend, that a tap on a little one opens its wish; a wish of the reading
//! opens to what is true of it (the reading's take, never a run's state), where it was typed, that Jellymoon ran
//! nothing, and how to go on when its project has a chip; and the tap chooses that chip (`open_a_wish/window.rs`,
//! on the Mac). The taps are carried by the made-up window of F47 (`support/tapper.rs`). Nothing here starts the
//! real Claude Code or opens a window on a screen.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;
#[cfg(all(target_os = "macos", feature = "window"))]
#[path = "open_a_wish/window.rs"]
mod window;

use serde_json::json;
use sticker_kit::{Note, Scene};
use support::day::{DAY, answer, noon, result, said, write_log};
use support::wished::{SAID, World, quiet};
use support::{Piece, clash, page, pieces, wish};

const TAP: &str = "tap a jelly to open its wish";
const MARKS: [&str; 4] = ["★ finished", "working on it", "needs you", "let go"];
const RAN: &str = "Jellymoon did not run it.";
const NEXT: &str = "To go on with it here, type what you want next. It will start in";
const TYPED: [&str; 4] = ["tidy the uploader", "write the docs", "sort the old notes", "say hello"];

/// A folder name too long to be a project's: it is read, and has no chip.
fn long() -> String {
    format!("notes-{}", "x".repeat(100))
}

/// A day read from logs typed in `alpha`, `beta`, a folder whose name has no chip and no folder at all, with a wish
/// of the reading in each state: `w1` unfinished in `alpha`, `w2` done in `beta`, `w3` needing the person in the
/// long one, `w4` let go in `alpha`, and `w5` unfinished in no project. The page is read and kept.
pub fn read_day() -> World {
    let w = quiet();
    let at = |name: &str| w.at("work").join(name).to_str().unwrap().to_string();
    let folders = [at("alpha"), at("beta"), at(&long()), String::new()];
    for ((n, folder), typed) in [0_i8, 1, 2, 3].iter().zip(&folders).zip(TYPED) {
        write_log(&w.day, &format!("-work-{n}/s{n}.jsonl"), &[said(&noon(*n), folder, typed)]);
    }
    let one = |id: &str, state: &str, project: &str, said: &str| {
        let label = format!("Label {id}");
        json!({"id": id, "label": label, "said": said, "project": project, "state": state, "stars": []})
    };
    let wishes = [
        one("a", "working", "alpha", TYPED[0]),
        one("b", "done", "beta", TYPED[1]),
        one("c", "needs_you", &long(), TYPED[2]),
        one("d", "let_go", "alpha", TYPED[0]),
        one("e", "working", "", TYPED[3]),
    ];
    answer(&w.day, &result(&json!({"wishes": wishes, "clashes": []}).to_string()), 0);
    let placed = w.placed();
    let chips: Vec<String> = placed.iter().filter(|piece| piece.kind() == "chip").map(Piece::said).collect();
    assert_eq!(chips, ["alpha", "beta"], "the long name has no chip");
    assert_eq!(placed.iter().filter(|piece| piece.kind() == "character").count(), 5, "{placed:?}");
    w
}

/// The note of the wish `id` on a scene: its title and its texts.
fn note_of(scene: &Scene, id: &str) -> Option<(String, Vec<String>)> {
    scene.notes.iter().find_map(|note| match note {
        Note::Text { id: of, title, body, .. } if *of == format!("wish-{id}") => Some((title.clone(), body.clone())),
        _ => None,
    })
}

/// What the note of a wish of the reading says: its title, the words typed, where, and how to go on.
fn read_note(title: &str, typed: &str, project: &str, chip: bool) -> (String, Vec<String>) {
    let at = if project.is_empty() { String::new() } else { format!(", in {project}") };
    let mut body = vec![format!("“{typed}”"), format!("You typed this in Claude Code{at}. {RAN}")];
    body.extend(chip.then(|| format!("{NEXT} {project}.")));
    (title.to_string(), body)
}

fn legend(placed: &[Piece]) -> Vec<String> {
    placed.iter().filter(|piece| piece.kind() == "legend").map(Piece::said).collect()
}

fn overlap(a: [f64; 4], b: [f64; 4]) -> bool {
    a[0] < b[0] + b[2] && b[0] < a[0] + a[2] && a[1] < b[1] + b[3] && b[1] < a[1] + a[3]
}

#[test]
fn a_page_with_a_wish_says_once_that_a_jelly_opens_it_and_a_page_with_none_does_not() {
    let wishes = (1..=3).map(|n| wish(&format!("w{n}"), "a long label for a wish", "done", &["it came true"]));
    let with = page(wishes.collect(), vec![clash("w1", "w2", "add a retry", "never retry", false)]);
    for size in ["1440x900", "960x640"] {
        let placed = pieces(&with, &["--size", size]);
        assert_eq!(legend(&placed), [&MARKS[..], &[TAP]].concat(), "{size}");
        let said: Vec<String> = placed.iter().map(Piece::said).filter(|said| said.contains("jelly")).collect();
        assert_eq!(said, [TAP], "{size}: said once");
        let tap = placed.iter().find(|piece| piece.said() == TAP).unwrap();
        let paper = placed.iter().find(|piece| piece.kind() == "paper").unwrap().rect();
        assert!(tap.rect()[0] + tap.rect()[2] <= paper[0] + paper[2], "{size}: off the page");
        for note in placed.iter().filter(|piece| ["note", "option"].contains(&piece.kind())) {
            assert!(!overlap(tap.rect(), note.rect()), "{size}: the line is under {}", note.id());
        }
    }
    let mut read_none = page(vec![], vec![]);
    read_none["read"] = json!({ "turns": 5, "projects": ["alpha"] });
    assert_eq!(legend(&pieces(&read_none, &[])), MARKS, "read, and no wish: the marks only");
    for empty in [read_none, page(vec![], vec![])] {
        let placed = pieces(&empty, &[]);
        assert!(placed.iter().all(|piece| !piece.said().contains("jelly")), "{placed:?}");
    }
}

#[test]
fn a_tapped_wish_of_the_reading_says_what_is_true_of_it_and_a_made_wish_s_note_is_as_it_was() {
    let w = read_day();
    w.jellymoon(&["wish", "--date", DAY, "--say", SAID, "--project", "alpha"]);
    let ids = ["w1", "w2", "w3", "w4", "w5", "m1"];
    let acts: Vec<String> = ids.iter().map(|id| format!("poke:{id}")).collect();
    let showed = w.taps("logs", &acts.iter().map(String::as_str).collect::<Vec<_>>()).showed();
    let opened = |(id, scene): (&&str, &Option<Scene>)| note_of(scene.as_ref().expect("the wish opened"), id);
    let notes: Vec<_> = ids.iter().zip(&showed).map(|tapped| opened(tapped).unwrap()).collect();
    let long = long();
    assert_eq!(notes[0], read_note("This looks unfinished.", TYPED[0], "alpha", true));
    assert_eq!(notes[1], read_note("This looks done.", TYPED[1], "beta", true));
    assert_eq!(notes[2], read_note("This looks like it needs you.", TYPED[2], &long, false));
    assert_eq!(notes[3], read_note("This looks like you let it go.", TYPED[0], "alpha", true));
    assert_eq!(notes[4], read_note("This looks unfinished.", TYPED[3], "", false), "no project, no clause");
    let made = ("This wish is being worked on.".to_string(), vec![format!("“{SAID}”")]);
    assert_eq!(notes[5], made, "a made wish's note is word for word what it was");
}
