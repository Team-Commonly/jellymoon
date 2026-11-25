//! The answer stays with its wish (ADR 0025, decision 9). A wish that went on with the person's answer to the
//! sharper wish's question is made of more than its typed words, so the page draws the answer with them: on the
//! wish's row of the scene, and in its bubble of the column. Carried as `shape_first.rs` carries a sharper wish:
//! by the made-up window in a process of its own, with the made-up agent in place of Claude Code. No window is
//! opened and the real Claude Code is never started.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::fs;

use jellymoon_app::scene::MADE;
use serde_json::{Value, json};
use sticker_kit::{Note, Scene};
use support::wished::{SAID, World, world};

const ANSWERED: &str = "two seconds, and say so";

/// A day with `alpha` on offer, on which the made-up agent answers a shaping call with a sharper wish that asks.
fn asked() -> World {
    let w = world();
    fs::create_dir(w.at("harness")).unwrap();
    let sharper = json!({"clear": false, "wish": "Make the uploader try a failed upload twice more.",
        "done_when": "A failed upload is tried three times in all.", "not_touching": "How a file is read.",
        "why": "The uploader gives up after one failure today.", "ask": "Should it wait between tries?"});
    let printed = json!({"type": "result", "is_error": false, "result": sharper.to_string()});
    fs::write(w.at("agent").join("shape"), format!("says\n{printed}")).unwrap();
    w
}

/// The window's lines for Enter in the entry with `alpha` chosen, core's answer to it, and then `then`.
fn acted(w: &World, then: &[&str]) -> Vec<Value> {
    let (bin, enter) = (format!("bin:{}", w.install().parent().unwrap().display()), format!("enter:wish@alpha:{SAID}"));
    let acts: Vec<&str> = [bin.as_str(), enter.as_str(), "answer"].into_iter().chain(then.iter().copied()).collect();
    let lines = w.taps("logs", &acts).ended();
    lines.iter().map(|line| serde_json::from_str(line).expect(line)).collect()
}

fn scene(line: &Value) -> Scene {
    serde_json::from_value(line.clone()).expect("a scene")
}

/// The words of each row of the day's made wishes, as the scene holds them.
fn rows(scene: &Scene) -> Vec<String> {
    let made = |note: &Note| match note {
        Note::Whole { title, rows } if title == MADE => Some(rows.iter().map(|row| row.1.clone()).collect()),
        _ => None,
    };
    scene.notes.iter().find_map(made).unwrap_or_default()
}

/// What each bubble of a column holds: its id, the typed words, and the answer under them.
fn bubbles(column: &Scene) -> Vec<(String, String, String)> {
    let said = |note: &Note| match note {
        Note::Said { id, text, answer } => Some((id.clone(), text.clone(), answer.clone())),
        _ => None,
    };
    column.notes.iter().filter_map(said).collect()
}

#[test]
fn a_wish_that_went_on_with_an_answer_has_it_on_its_row_and_in_its_bubble_under_the_typed_words() {
    let w = asked();
    // Typed with blanks around it: what is kept and drawn is the answer as core cleaned it for the prompt.
    let printed = acted(&w, &[&format!("type:shape:  {ANSWERED} "), "column"]);
    assert_eq!(printed[2]["kept"], true, "{}", printed[2]);
    let kept = w.kept();
    assert_eq!((&kept["said"], &kept["answered"]), (&json!(SAID), &json!(ANSWERED)), "{kept}");
    assert!(kept["shaped"].as_str().unwrap().contains(&format!("The person's answer: “{ANSWERED}” | ")), "{kept}");
    assert_eq!(rows(&scene(&printed[2]["scene"])), [format!("{SAID}\n{ANSWERED}")], "under the typed words");
    assert_eq!(bubbles(&scene(&printed[3])), [("typed-m1".to_string(), SAID.to_string(), ANSWERED.to_string())]);
}

#[test]
fn a_wish_that_went_on_with_no_answer_has_the_row_and_the_bubble_it_always_had() {
    // `Go with this`, then `Go with what I typed`: a question that was not answered leaves nothing.
    for choice in 0..2 {
        let w = asked();
        let printed = acted(&w, &[&format!("choose:shape:{choice}"), "run", "column"]);
        assert!(w.kept()["answered"].is_null(), "{}", w.kept());
        assert_eq!(rows(&scene(&printed[3])), [SAID], "{choice}: the typed words and nothing else");
        assert_eq!(bubbles(&scene(&printed[4])), [("typed-m1".to_string(), SAID.to_string(), String::new())]);
        assert!(!printed[4].to_string().contains("\"answer\""), "{choice}: the bubble is as it was: {}", printed[4]);
        w.ended();
    }
}
