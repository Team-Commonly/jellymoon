//! An answer typed under a wish whose run asks, carried as the window carries it (`support/tapper.rs`): core's
//! `answer_run` is given the words on another thread and the run goes on with the made-up agent. What core
//! refuses is drawn in core's words, the field keeps what was typed, and nothing is written.

use std::fs;

use serde_json::json;

use super::support::worked::tree;
use super::{
    ANSWER_HINT, MORE, MORE_HINT, NEEDS, QUESTION, UNANSWERED, WORK, acted, plain, quoted, ready, text, typed,
};
use crate::support::wished::{SAID, wait_until};

const ANSWER: &str = "the newer one, please";

/// The note of a wish that asks `question`, as a tap on its character opens it.
fn asks(question: &str) -> super::Text<'_> {
    (NEEDS, vec!["“make the uploader retry twice”"], question, vec![], Some(("", ANSWER_HINT)))
}

#[test]
fn an_answer_typed_under_the_question_is_given_to_core_and_the_page_shows_what_core_says_then() {
    let w = ready();
    let wish = w.ran("asks", 0, "asks");
    assert_eq!(quoted(SAID), asks(QUESTION).1[0]);
    // The answered run goes on with the agent's edits, so its work waits when it ends.
    w.plays("edits", 0);
    let (starts, note) = (w.starts(), format!("wish-{wish}"));
    let printed = acted(&w, &[&format!("poke:{wish}"), &format!("type:{note}:{ANSWER}")]);
    let opened = super::scene(&printed[0]);
    assert_eq!(text(&opened, &note), Some(asks(QUESTION)));
    let (kept, shown) = typed(&printed[1]);
    assert!(kept, "core took the answer, so the field is emptied");
    assert_eq!(plain(&shown).iter().filter(|(title, _)| *title == UNANSWERED).count(), 0);
    // Core was given the words, the agent was started once more, and the run ended with its work.
    let made = w.wish(&wish);
    assert_eq!((&made["answer"], made.get("question"), w.starts()), (&json!(ANSWER), None, starts + 1), "{made}");
    // The note is the wish's as core says it stands now: its work waits, and a follow-up may be made of it.
    assert_eq!(text(&shown, &note), None);
    let work = text(&shown, &format!("work-{wish}")).expect("the work that waits");
    assert_eq!((work.0, work.2, work.4), (WORK, "", Some((MORE, MORE_HINT))));
    // The answer is the person's and core's: no note holds it.
    assert!(!printed[1].to_string().contains(ANSWER), "{}", printed[1]);
}

#[test]
fn while_an_answered_run_goes_on_the_wish_is_drawn_as_core_says_and_has_no_field() {
    let w = ready();
    let wish = w.ran("asks", 0, "asks");
    w.plays("slow", 0);
    let note = format!("wish-{wish}");
    let mut window = w.taps("logs", &[&format!("poke:{wish}"), &format!("type:{note}:{ANSWER}")]);
    wait_until("the answered run", || w.at("rec").join("started").exists());
    // The run is going: core keeps no question with the wish, and a page drawn now has nothing to type in.
    let placed = w.placed();
    let ours =
        |kind: &str| super::of(&placed, kind, &note).len() + super::of(&placed, kind, &format!("work-{wish}")).len();
    assert_eq!((ours("field"), ours("quote"), ours("offer")), (0, 0, 0));
    assert_eq!(w.wish(&wish)["state"], "working");
    // The helper is ended before the test's folder goes.
    fs::write(w.at("rec").join("go"), "").unwrap();
    let printed = window.ended();
    let (kept, shown) = typed(&serde_json::from_str(&printed[1]).unwrap());
    assert!(kept && text(&shown, &format!("work-{wish}")).is_some(), "{}", printed[1]);
}

#[test]
fn a_run_that_asks_again_is_drawn_with_its_new_question_and_takes_a_second_answer() {
    let w = ready();
    let wish = w.ran("asks", 0, "asks");
    w.plays("asks", 11);
    let note = format!("wish-{wish}");
    let again = "Did you mean 'retry' or 'resume'?";
    let printed = acted(&w, &[&format!("poke:{wish}"), &format!("type:{note}:{ANSWER}")]);
    let (kept, shown) = typed(&printed[1]);
    assert!(kept);
    assert_eq!(text(&shown, &note), Some(asks(again)));
    assert_eq!(w.wish(&wish)["question"], again);
    w.plays("edits", 0);
    let printed = acted(&w, &[&format!("poke:{wish}"), &format!("type:{note}:resume")]);
    let (kept, shown) = typed(&printed[1]);
    assert!(kept && text(&shown, &note).is_none() && text(&shown, &format!("work-{wish}")).is_some());
    assert_eq!(w.wish(&wish)["answer"], "resume");
}

#[test]
fn an_answer_core_refuses_is_drawn_in_core_s_words_and_the_field_keeps_what_was_typed() {
    let w = ready();
    let wish = w.ran("asks", 0, "asks");
    let note = format!("wish-{wish}");
    let (before, starts) = (tree(&w.at("jm")), w.starts());
    let refusals = [
        ("w".repeat(601), "an answer is at most 600 characters, and this one is longer."),
        ("\u{200b} \u{200b}".to_string(), "an answer needs words, and nothing is left of these once cleaned."),
    ];
    for (words, why) in refusals {
        let printed = acted(&w, &[&format!("poke:{wish}"), &format!("type:{note}:{words}")]);
        let (kept, shown) = typed(&printed[1]);
        assert!(!kept, "a refused answer stays in its field: {why}");
        assert_eq!(plain(&shown).first(), Some(&(UNANSWERED, why)));
        // The question and the field are still there to answer in.
        assert_eq!(text(&shown, &note), Some(asks(QUESTION)), "{why}");
        assert!(tree(&w.at("jm")) == before && w.starts() == starts, "a refused answer wrote or started: {why}");
    }
    // A second refusal takes the place of the first: one note says why, not two.
    let both = [format!("poke:{wish}"), format!("type:{note}:{}", "w".repeat(601)), format!("type:{note}: ")];
    let printed = acted(&w, &both.iter().map(String::as_str).collect::<Vec<_>>());
    let shown = typed(&printed[2]).1;
    assert_eq!(plain(&shown).iter().filter(|(title, _)| *title == UNANSWERED).count(), 1);
}

#[test]
fn an_answer_to_a_run_that_ended_meanwhile_is_refused_in_core_s_words() {
    // A page left open while the run was answered elsewhere: the note with the question is still on show.
    let w = ready();
    let wish = w.ran("edits", 0, "completed");
    let run = w.wish(&wish)["run"].as_str().unwrap().to_string();
    let note = format!("wish-{wish}");
    let stale = json!({"type": "text", "id": note, "title": NEEDS, "body": [quoted(SAID)], "quoted": QUESTION,
        "field": {"hint": ANSWER_HINT, "most": 0}});
    let left = w.at("tmp").join("left.json");
    fs::write(&left, stale.to_string()).unwrap();
    let before = tree(&w.at("jm"));
    let printed = acted(&w, &[&format!("left:{}", left.display()), &format!("type:{note}:{ANSWER}")]);
    let (kept, shown) = typed(&printed[0]);
    let why = format!("run {run} does not ask a question, so there is nothing to answer.");
    assert_eq!((kept, plain(&shown).first()), (false, Some(&(UNANSWERED, why.as_str()))));
    assert_eq!(text(&shown, &note), Some(asks(QUESTION)), "the field keeps its note, and so its words");
    assert!(tree(&w.at("jm")) == before, "a refused answer wrote something");
}

#[test]
fn enter_is_carried_only_from_a_field_that_is_on_show() {
    let w = ready();
    let wish = w.ran("asks", 0, "asks");
    let before = tree(&w.at("jm"));
    // No note is open, a note of another wish is named, and a wish that is being stopped has no field.
    let acts = [format!("type:wish-{wish}:{ANSWER}"), format!("poke:{wish}"), format!("type:wish-m9:{ANSWER}")];
    let printed = acted(&w, &acts.iter().map(String::as_str).collect::<Vec<_>>());
    assert_eq!((&printed[0], &printed[2]), (&json!(null), &json!(null)));
    assert!(tree(&w.at("jm")) == before, "an Enter from no field reached core");
}
