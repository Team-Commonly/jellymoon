//! Finished wishes tuck away (ADR 0030, decision 11; `docs/development/app.md`, "Every wish keeps its place"): a
//! made wish that is settled has no note on the page once the next wish is made, and a tap on it brings its notes
//! back. The wishes are typed through the made-up window of F47 (`support/tapper.rs`), a process of its own, and
//! run by the `jellymoon` command with the made-up agent of F39's tests, in a world where a finished change is
//! kept by itself, with the real git in a repo of the test's own. The window's `column` act prints the scene as
//! `Showing::column` gives it to the view. Nothing here starts the real Claude Code or opens a window.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::fs;
use std::path::Path;

use sticker_kit::{Note, Scene};
use support::day::DAY;
use support::wished::World;
use support::worked::landed;

/// A window tall enough that every note of the day is placed.
const TALL: &str = "size:1440x3000";
const UNDO: [&str; 1] = ["Undo it"];

fn column(line: &str) -> Scene {
    serde_json::from_str::<Option<Scene>>(line).expect(line).expect("a column")
}

fn id(note: &Note) -> &str {
    match note {
        Note::Text { id, .. }
        | Note::Choice { id, .. }
        | Note::Told { id, .. }
        | Note::Said { id, .. }
        | Note::Drawn { id, .. } => id,
        Note::Plain { title, .. } | Note::List { title, .. } | Note::Whole { title, .. } => title,
    }
}

/// What stands under the bubble of `wish` in a column: every note from its typed words to the next wish's. The
/// bubble itself has to be there.
fn under(column: &Scene, wish: &str) -> Vec<String> {
    let all: Vec<&str> = column.notes.iter().map(id).collect();
    let at = all.iter().position(|one| *one == format!("typed-{wish}")).expect("the wish's bubble");
    all[at + 1..].iter().take_while(|one| !one.starts_with("typed-")).map(|one| one.to_string()).collect()
}

fn choices(column: &Scene, named: &str) -> Vec<String> {
    column.notes.iter().find(|note| id(note) == named).map_or(Vec::new(), |note| match note {
        Note::Text { options, .. } | Note::Drawn { options, .. } => options.clone(),
        _ => Vec::new(),
    })
}

/// The run of the made wish `id`, with the made-up agent playing `script`: how it ended.
fn ran(w: &World, repo: &Path, id: &str, script: &str) -> String {
    w.play(script);
    let (logs, named) = (w.at("logs"), repo.to_str().unwrap());
    let ran = w.jellymoon(&["run", "--wish", id, "--date", DAY, "--repo", named, "--logs", logs.to_str().unwrap()]);
    ran["outcome"].as_str().unwrap().to_string()
}

#[test]
fn a_settled_wish_tucks_away_when_the_next_wish_is_made_and_a_tap_or_a_new_window_shows_what_is_owed() {
    let (w, repo) = landed("edits");
    let landing = w.at("tmp").join("landed");
    let hold = format!("hold:{}", landing.display());
    let [second, third] = ["type:wish@alpha:say hello", "type:wish@alpha:and nothing more"];
    let acts =
        [TALL, "look", "column", second, "column", "poke:m1", "column", third, "column", &hold, "look", "column"];
    let mut window = w.taps("logs", &acts);
    window.printed(8);
    // The second wish's change lands after the third wish was made.
    assert_eq!(ran(&w, &repo, "m2", "hello"), "completed");
    fs::write(&landing, "").unwrap();
    let lines = window.ended();
    let [_, alone, kept, made, _, back, _, again, _, landed] = &lines[..] else { panic!("ten lines: {lines:#?}") };
    // A wish lands, and its result is on the page.
    let alone = column(alone);
    assert!(under(&alone, "m1").contains(&"kept-m1".to_string()), "{alone:?}");
    // A second wish is made, by Enter in the entry: the first one's notes are gone, and its bubble is there.
    assert!(kept.contains(r#""kept":true"#), "{kept}");
    let made = column(made);
    assert_eq!(under(&made, "m1"), [""; 0], "{made:?}");
    // A tap on the first wish brings its notes back, as core says the wish stands: its change can be undone.
    let back = column(back);
    assert!(under(&back, "m1").contains(&"kept-m1".to_string()), "{back:?}");
    assert_eq!(choices(&back, "kept-m1"), UNDO);
    // The next made wish tucks it away again.
    let again = column(again);
    assert_eq!(under(&again, "m1"), [""; 0], "{again:?}");
    // The second wish settled after the third was made: its result stays on the page, and the first is as it was.
    let landed = column(landed);
    assert!(under(&landed, "m2").contains(&"kept-m2".to_string()), "{landed:?}");
    assert_eq!((choices(&landed, "kept-m2"), under(&landed, "m1")), (UNDO.map(String::from).to_vec(), Vec::new()));

    // A window opened on that day, once the third wish's run failed and a fourth changed nothing: the notes of
    // the newest wish, and of the wish that needs the person, and of no other.
    assert_ne!(ran(&w, &repo, "m3", "is-error"), "completed");
    let fourth = w.jellymoon(&["wish", "--date", DAY, "--say", "look again", "--project", "alpha"]);
    assert_eq!(fourth["wish"]["id"], "m4", "{fourth}");
    assert_eq!(ran(&w, &repo, "m4", "nothing"), "completed");
    let lines = w.taps("logs", &[TALL, "look", "column"]).ended();
    let opened = column(lines.last().expect("two lines"));
    assert_eq!((under(&opened, "m1"), under(&opened, "m2")), (Vec::<String>::new(), Vec::new()), "{opened:?}");
    assert!(under(&opened, "m3").contains(&"wish-m3".to_string()), "{opened:?}");
    assert!(under(&opened, "m4").contains(&"wish-m4".to_string()), "{opened:?}");
}
