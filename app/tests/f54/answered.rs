//! An answer typed under a wish whose run asks, carried as the window carries it (`support/tapper.rs`): core's
//! `answer_run` is given the words on another thread and the run goes on with the made-up agent. What core
//! refuses is drawn in core's words, the field keeps what was typed, and nothing is written.

use std::fs;

use serde_json::json;

use super::support::day::DAY;
use super::support::worked::tree;
use super::{
    ANSWER_HINT, AT_WORK, MORE, MORE_HINT, NEEDS, QUESTION, UNANSWERED, UNSETTLED, WORK, acted, answer_key, at_once,
    key, plain, quoted, ready, text, typed,
};
use crate::support::wished::{SAID, wait_until};

const ANSWER: &str = "the newer one, please";

/// The note of a wish from Enter in its answer's field until core has answered: core's words for a wish at
/// work over the wish's own, with no question and no field.
fn at_work() -> super::Text<'static> {
    (AT_WORK, vec!["“make the uploader retry twice”"], "", vec![], None)
}

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
    // The review of F54's notes, note 6: the field is the wish's and the question's, by its key, so the kit
    // keeps a half-typed answer while the note is not on show (`crates/sticker-kit/tests/field_draw.rs`).
    assert_eq!(key(&opened, &note), Some(answer_key(&wish, QUESTION).as_str()));
    let (kept, shown) = typed(&printed[1]);
    assert!(kept, "core took the answer, so the field is emptied");
    // The review of F54, finding 5: from Enter the scene on show says the wish is being worked on.
    assert_eq!(at_once(&printed[1]).as_ref().and_then(|scene| text(scene, &note)), Some(at_work()));
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
    // Another question is another field: what was typed for the first is not there for it.
    assert_eq!(key(&shown, &note), Some(answer_key(&wish, again).as_str()));
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
        assert_eq!(at_once(&printed[1]).as_ref().and_then(|scene| text(scene, &note)), Some(at_work()), "{why}");
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
    // The review of F54's notes, note 5: the run is known to ask nothing now, so the question is not put
    // back. The wish's note is as core says the wish stands: its work waits.
    assert_eq!((text(&shown, &note), text(&shown, &format!("work-{wish}")).map(|work| work.0)), (None, Some(WORK)));
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

// The review of F54, finding 2: core's `Err` does not say whether the harness was given the answer. Here it was:
// the run went on and ended, and then the day was busy for longer than a run waits.
#[test]
fn an_answer_that_was_given_before_core_failed_empties_the_field_and_says_what_failed() {
    let w = ready();
    let wish = w.ran("asks", 0, "asks");
    let run = w.wish(&wish)["run"].as_str().unwrap().to_string();
    w.plays("edits", 0);
    let (starts, note) = (w.starts(), format!("wish-{wish}"));
    let (go, at) = (w.at("rec").join("go.step"), w.at("rec").join("go.at"));
    let lock = w.at("jm").join("pages").join(format!("{DAY}.ruled.lock"));
    // A test's build of core waits before the record takes the run's end, until the file is there.
    let vars = [
        ("JELLYMOON_TEST_NOTE_WAIT_MS", "60000".into()),
        ("JELLYMOON_TEST_GO", go.clone().into()),
        ("JELLYMOON_TEST_END_WAIT_MS", "300".into()),
    ];
    let mut window = w.taps_with("logs", &[&format!("poke:{wish}"), &format!("type:{note}:{ANSWER}")], &vars);
    wait_until("the end of the answered run", || at.exists());
    fs::write(&lock, "").unwrap(); // another change to the day, which does not finish in time
    fs::write(&go, "").unwrap();
    let printed = window.ended();
    fs::remove_file(&lock).unwrap();
    let (kept, shown) = typed(&serde_json::from_str(&printed[1]).unwrap());
    assert!(kept, "the harness has the answer: the field is emptied, so Enter cannot give it twice");
    let why = format!(
        "run {run} has ended and its record says how; the page of {DAY} was busy, and shows it when it is next drawn."
    );
    assert_eq!(plain(&shown).first(), Some(&(UNSETTLED, why.as_str())));
    assert_eq!(plain(&shown).iter().filter(|(title, _)| *title == UNANSWERED).count(), 0);
    // The note is the wish's as core says it stands, by the run's record: its work waits, and nothing is asked.
    let work = text(&shown, &format!("work-{wish}")).expect("the work that waits");
    assert_eq!((text(&shown, &note), work.0, work.2), (None, WORK, ""));
    assert_eq!((&w.wish(&wish)["answer"], w.starts()), (&json!(ANSWER), starts + 1));
}

// The review of F54, finding 7: only a made wish has a run to answer. A kept page file is a file anyone with
// the folder can write, and a question it gives a wish of the reading is no question of a run.
#[test]
fn a_question_a_kept_page_file_gives_a_wish_of_the_reading_is_not_drawn_and_has_no_field() {
    let w = ready();
    let file = w.at("jm").join("pages").join(format!("{DAY}.json"));
    let mut page: serde_json::Value = serde_json::from_str(&fs::read_to_string(&file).unwrap()).unwrap();
    let read = "a".to_string();
    let wish = json!({"id": read, "label": "The uploader", "said": "look at the uploader", "project": "alpha",
        "state": "needs_you", "stars": [], "question": QUESTION});
    page["wishes"].as_array_mut().expect("the page's wishes").push(wish);
    fs::write(&file, page.to_string()).unwrap();
    let before = tree(&w.at("jm"));
    let note = format!("wish-{read}");
    let printed = acted(&w, &[&format!("poke:{read}"), &format!("type:{note}:{ANSWER}")]);
    let opened = super::scene(&printed[0]);
    let opened = text(&opened, &note).expect("the wish's note");
    assert_eq!((opened.2, opened.4), ("", None), "{}", printed[0]);
    assert_eq!(printed[1], json!(null));
    let placed = w.placed();
    assert_eq!((super::of(&placed, "field", &note).len(), super::of(&placed, "quote", &note).len()), (0, 0));
    assert!(tree(&w.at("jm")) == before, "an Enter under a wish of the reading reached core");
}

/// The first two notes a printed scene holds: the title of the first and the id of the second.
fn first_two(printed: &serde_json::Value) -> (&str, &str) {
    let notes = &printed["scene"]["notes"];
    (notes[0]["title"].as_str().unwrap_or(""), notes[1]["id"].as_str().unwrap_or(""))
}

// The review of F54's notes, note 1: during the wait another little one is tapped, which takes the wish's note
// off the page. A refusal then has no note to put the question back into, and puts it under what core said.
#[test]
fn a_refused_answer_whose_wish_is_no_longer_open_is_put_under_the_refusal_with_its_question_and_its_field() {
    let w = ready();
    let wish = w.ran("asks", 0, "asks");
    let other = w.wishes();
    let note = format!("wish-{wish}");
    let acts = [format!("poke:{wish}"), format!("enter:{note}:{}", "w".repeat(601)), format!("poke:{other}")];
    let mut acts: Vec<&str> = acts.iter().map(String::as_str).collect();
    acts.push("answer");
    let printed = acted(&w, &acts);
    let between = super::scene(&printed[2]);
    assert!(text(&between, &note).is_none() && text(&between, &format!("wish-{other}")).is_some(), "{}", printed[2]);
    let (kept, shown) = typed(&printed[3]);
    assert_eq!((kept, plain(&shown).first().map(|said| said.0)), (false, Some(UNANSWERED)));
    assert_eq!(text(&shown, &note), Some(asks(QUESTION)), "the words typed have no field to stay in");
    assert_eq!(key(&shown, &note), Some(answer_key(&wish, QUESTION).as_str()));
    // The second review of F54's notes, note 1: the refusal is the page's first note, and the question is the
    // note directly under it, as where the scene did not change during the wait.
    assert_eq!(first_two(&printed[3]), (UNANSWERED, note.as_str()), "{}", printed[3]);
    // An answer that was given has no words to keep: the wish that went off the page is left to the next look,
    // also when its run asks again and its note has a field to answer in.
    w.plays("asks", 11);
    let acts = [format!("poke:{wish}"), format!("enter:{note}:{ANSWER}"), format!("poke:{other}")];
    let mut acts: Vec<&str> = acts.iter().map(String::as_str).collect();
    acts.push("answer");
    let printed = acted(&w, &acts);
    let (kept, shown) = typed(&printed[3]);
    assert!(kept && text(&shown, &note).is_none() && text(&shown, &format!("work-{wish}")).is_none(), "{}", printed[3]);
}

// The second review of F54's notes, note 4: only a note with a field to answer in is put under the refusal.
// Here the run ended, so the wish's note is its work, with nothing to answer in: a wish the person closed is
// not opened again.
#[test]
fn a_refused_answer_whose_run_asks_no_more_does_not_open_again_a_wish_that_went_off_the_page() {
    let w = ready();
    let wish = w.ran("edits", 0, "completed");
    let other = w.wishes();
    let note = format!("wish-{wish}");
    let stale = json!({"type": "text", "id": note, "title": NEEDS, "body": [quoted(SAID)], "quoted": QUESTION,
        "field": {"hint": ANSWER_HINT, "most": 0}});
    let left = w.at("tmp").join("left.json");
    fs::write(&left, stale.to_string()).unwrap();
    let acts = [format!("left:{}", left.display()), format!("enter:{note}:{ANSWER}"), format!("poke:{other}")];
    let mut acts: Vec<&str> = acts.iter().map(String::as_str).collect();
    acts.push("answer");
    let printed = acted(&w, &acts);
    let (kept, shown) = typed(&printed[2]);
    // Core's refusal is the first note, and the note the person opened meanwhile is the one under it.
    assert_eq!((kept, first_two(&printed[2])), (false, (UNANSWERED, format!("wish-{other}").as_str())));
    assert!(text(&shown, &note).is_none() && text(&shown, &format!("work-{wish}")).is_none(), "{}", printed[2]);
}

// The review of F54's notes, note 2: the look after core's error cannot read the day. That is no proof the
// answer was given, so the words stay. Here the run did get the answer; the next look says how it ended.
#[cfg(unix)]
#[test]
fn an_answer_is_held_not_given_when_the_look_after_core_s_error_cannot_read_the_day() {
    use std::os::unix::fs::PermissionsExt;
    let w = ready();
    let wish = w.ran("asks", 0, "asks");
    w.plays("edits", 0);
    let note = format!("wish-{wish}");
    let (go, at) = (w.at("rec").join("go.step"), w.at("rec").join("go.at"));
    let pages = w.at("jm").join("pages");
    let (lock, made) = (pages.join(format!("{DAY}.ruled.lock")), pages.join(format!("{DAY}.wishes.json")));
    let vars = [
        ("JELLYMOON_TEST_NOTE_WAIT_MS", "60000".into()),
        ("JELLYMOON_TEST_GO", go.clone().into()),
        ("JELLYMOON_TEST_END_WAIT_MS", "300".into()),
    ];
    let mut window = w.taps_with("logs", &[&format!("poke:{wish}"), &format!("type:{note}:{ANSWER}")], &vars);
    wait_until("the end of the answered run", || at.exists());
    fs::write(&lock, "").unwrap();
    // Core reads a kept file only when its owner alone can write to it: the day's made wishes are unreadable.
    let mode = fs::metadata(&made).unwrap().permissions();
    fs::set_permissions(&made, fs::Permissions::from_mode(0o666)).unwrap();
    fs::write(&go, "").unwrap();
    let printed = window.ended();
    fs::set_permissions(&made, mode).unwrap();
    fs::remove_file(&lock).unwrap();
    let (kept, shown) = typed(&serde_json::from_str(&printed[1]).unwrap());
    assert!(!kept, "nothing says the answer was given: the words stay");
    assert_eq!(plain(&shown).first().map(|said| said.0), Some(UNANSWERED), "{:?}", plain(&shown));
    assert_eq!(plain(&shown).iter().filter(|(title, _)| *title == UNSETTLED).count(), 0);
    assert_eq!(text(&shown, &note), Some(asks(QUESTION)));
}
