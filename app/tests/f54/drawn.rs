//! What `layout` prints of a made wish a person may type to: the question of a run that asks, apart from
//! Jellymoon's own words, with a field under it, and `More like this` with a field under work that waits and
//! under work that was kept. `layout` has no tap, so it prints the note of every such wish.

use serde_json::Value;

use super::support::Piece;
use super::{ANSWER_HINT, DONE, MORE, MORE_HINT, NEEDS, QUESTION, WORK, World, of, quoted, ready};
use crate::support::wished::SAID;

/// Every run of text of the pieces of one kind that carry an id: its role and its words.
fn runs(placed: &[Piece], kind: &str, id: &str) -> Vec<(String, String)> {
    let said = |run: &Value| (run["role"].as_str().unwrap().to_string(), run["text"].as_str().unwrap().to_string());
    of(placed, kind, id).iter().flat_map(|piece| piece.runs().iter().map(said)).collect()
}

/// The question drawn under a note, its lines joined, and whether every line of it is in the tone for quoted
/// words and no line of the note itself is.
fn question(placed: &[Piece], id: &str) -> (String, bool) {
    let lines = runs(placed, "quote", id);
    let apart = lines.iter().all(|(role, _)| role == "quoted")
        && runs(placed, "note", id).iter().all(|(role, _)| role != "quoted");
    (lines.into_iter().map(|(_, text)| text).collect::<Vec<_>>().join(" "), apart)
}

/// Whether a rect lies inside another.
fn inside(inner: [f64; 4], outer: [f64; 4]) -> bool {
    inner[0] >= outer[0]
        && inner[1] >= outer[1]
        && inner[0] + inner[2] <= outer[0] + outer[2]
        && inner[1] + inner[3] <= outer[1] + outer[3]
}

/// The kinds of the pieces a person can act on that carry an id.
fn controls(placed: &[Piece], id: &str) -> Vec<String> {
    let acts = |piece: &&Piece| ["option", "field", "offer", "entry", "chip"].contains(&piece.kind());
    placed.iter().filter(|piece| piece.id() == id).filter(acts).map(|piece| piece.kind().to_string()).collect()
}

#[test]
fn layout_draws_a_wish_that_asks_with_its_question_apart_from_jellymoon_s_words_and_a_field() {
    let w = ready();
    let wish = w.ran("asks", 0, "asks");
    let id = format!("wish-{wish}");
    let placed = w.placed();
    let [note] = of(&placed, "note", &id)[..] else { panic!("one note of the wish: {placed:?}") };
    // Jellymoon's own words are the note's: how the wish stands, and the wish.
    let said = note.said();
    assert!(said.contains(NEEDS) && said.contains(&quoted(SAID)), "{said}");
    assert!(!said.contains(QUESTION), "the question is drawn among Jellymoon's own words: {said}");
    // The question is a piece of its own inside the note, every line in the tone for quoted words.
    let [quote] = of(&placed, "quote", &id)[..] else { panic!("one quoted piece: {placed:?}") };
    assert_eq!(question(&placed, &id), (QUESTION.to_string(), true));
    assert!(inside(quote.rect(), note.rect()), "{quote:?} is not in {note:?}");
    // One field under it, with its hint, and nothing else to act on: no choice and no offer.
    let [field] = of(&placed, "field", &id)[..] else { panic!("one field: {placed:?}") };
    assert_eq!(field.said(), ANSWER_HINT);
    assert!(inside(field.rect(), note.rect()) && field.rect()[1] >= quote.rect()[1] + quote.rect()[3]);
    assert_eq!(controls(&placed, &id), ["field"]);
    // The agent's own words are nowhere, and the run's answer is no part of a page.
    assert!(placed.iter().all(|piece| !piece.said().contains("zebra-quartz")));
}

#[test]
fn two_wishes_that_ask_are_each_drawn_with_their_own_question_and_their_own_field() {
    let w = ready();
    let (first, second) = (w.ran("asks", 0, "asks"), w.ran("asks", 9, "asks"));
    let placed = w.placed();
    let asked = [QUESTION, "Which uploader? The old one or the new one?"];
    for (wish, asked) in [first, second].iter().zip(asked) {
        let id = format!("wish-{wish}");
        assert_eq!(question(&placed, &id), (asked.to_string(), true), "{id}");
        assert_eq!(controls(&placed, &id), ["field"], "{id}");
    }
}

#[test]
fn a_question_at_each_of_core_s_bounds_is_drawn_as_core_keeps_it_and_one_core_keeps_none_of_has_no_field() {
    // What core keeps of each of the made-up agent's questions (`crates/jellymoon/tests/f51/words.rs`).
    let kept = [
        (4, "Which uploader did you mean?".to_string()),
        (5, format!("Which {}uploa…", "uploader ".repeat(32))),
        (7, format!("{}…", "q".repeat(299))),
        (8, "Is [redacted] the key to use?".to_string()),
        (11, "Did you mean 'retry' or 'resume'?".to_string()),
    ];
    for (number, asked) in kept {
        let w = ready();
        let id = format!("wish-{}", w.ran("asks", number, "asks"));
        let placed = w.placed();
        let (drawn, apart) = question(&placed, &id);
        // A question of one long word is broken where a line ends, so its lines are held without their spaces.
        assert_eq!((drawn.replace(' ', ""), apart), (asked.replace(' ', ""), true), "question {number}");
        assert_eq!(controls(&placed, &id), ["field"], "question {number}");
        let note = of(&placed, "note", &id)[0].rect();
        let lines = of(&placed, "quote", &id)[0].runs().to_vec();
        assert!(lines.iter().all(|run| inside(super::support::rect(run), note)), "question {number} leaves its note");
    }
    // A question core keeps nothing of is a run that failed: the wish has no question and no field.
    for number in [2, 6, 10] {
        let w = ready();
        let wish = w.ran("asks", number, "failed");
        let placed = w.placed();
        // What the person typed for it is a quote of its own, `typed-<wish>`: not a question.
        let ours = |piece: &&Piece| piece.id().ends_with(&format!("-{wish}")) && !piece.id().starts_with("typed-");
        let kinds: Vec<&str> = placed.iter().filter(ours).map(Piece::kind).collect();
        assert!(!kinds.contains(&"field") && !kinds.contains(&"quote"), "question {number}: {kinds:?}");
    }
}

/// The offer and the field under a note, when it has them: the offer's words and the field's hint.
fn offered(placed: &[Piece], id: &str) -> Option<(String, String)> {
    let (offer, field) = (of(placed, "offer", id), of(placed, "field", id));
    assert!(offer.len() <= 1 && field.len() == offer.len(), "{id}: an offer and its field come together");
    Some((offer.first()?.said(), field.first()?.said()))
}

fn more() -> Option<(String, String)> {
    Some((MORE.to_string(), MORE_HINT.to_string()))
}

#[test]
fn layout_offers_more_like_this_under_work_that_waits_and_under_work_that_was_kept() {
    let w = ready();
    let wish = w.ran("edits", 0, "completed");
    let placed = w.placed();
    let work = format!("work-{wish}");
    assert!(of(&placed, "note", &work)[0].said().contains(WORK));
    assert_eq!(offered(&placed, &work), more());
    // The three choices of the work are still there, over the offer.
    let (options, offer) = (of(&placed, "option", &work), of(&placed, "offer", &work)[0]);
    assert_eq!(options.len(), 3);
    assert!(options.iter().all(|option| option.rect()[1] < offer.rect()[1]));
    assert!(of(&placed, "quote", &work).is_empty(), "work that waits quotes nobody");
    // The wish has that one note: work that waits is not drawn once more as a wish that stands.
    let stands = format!("wish-{wish}");
    assert!(placed.iter().all(|piece| piece.id() != stands), "the wish is drawn twice");

    // Kept, the work has no choice any more, and the wish's own note offers it.
    w.runs(&["keep"], &wish);
    let placed = w.placed();
    assert!(of(&placed, "note", &work).is_empty());
    let kept = format!("wish-{wish}");
    let said = of(&placed, "note", &kept)[0].said();
    assert!(said.contains(DONE) && said.contains(&quoted(SAID)), "{said}");
    assert_eq!((offered(&placed, &kept), controls(&placed, &kept)), (more(), vec!["offer".into(), "field".into()]));
}

fn nothing_offered(w: &World, wish: &str, what: &str) {
    let placed = w.placed();
    let ours = |piece: &&Piece| piece.id() == format!("wish-{wish}") || piece.id() == format!("work-{wish}");
    let offers: Vec<&str> = placed.iter().filter(ours).map(Piece::kind).filter(|kind| *kind == "offer").collect();
    assert!(offers.is_empty(), "{what} offers more like it");
    assert!(placed.iter().all(|piece| !piece.said().contains(MORE)), "{what}: the offer's words are drawn");
}

#[test]
fn nothing_is_offered_under_a_wish_with_no_run_or_whose_run_asks_failed_changed_nothing_or_was_tossed() {
    let w = ready();
    nothing_offered(&w, &w.wishes(), "a wish with no run");
    let w = ready();
    nothing_offered(&w, &w.ran("asks", 0, "asks"), "a wish whose run asks");
    let w = ready();
    nothing_offered(&w, &w.ran("exit-3", 0, "failed"), "a wish whose run failed");
    let w = ready();
    nothing_offered(&w, &w.ran("nothing", 0, "completed"), "a wish whose run changed nothing");
    let w = ready();
    let tossed = w.ran("edits", 0, "completed");
    w.runs(&["toss"], &tossed);
    nothing_offered(&w, &tossed, "work that was tossed");
}
