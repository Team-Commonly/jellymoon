//! F91: the wish is shaped before a run (ADR 0025). The rules of the call and the check of its answer are core's,
//! proved at `jellymoon shape` (`crates/jellymoon/tests/f91.rs`). Here is the app's part: what the page shows
//! between Enter and a run, carried as the window carries it by a made-up window in a process of its own
//! (`support/tapper.rs`). The made-up agent of F39's tests stands in place of Claude Code: asked to shape, it
//! answers what a test typed beside it, and that the wish is clear when a test typed nothing. No window is
//! opened and the real Claude Code is never started.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::ffi::OsString;
use std::fs;

use jellymoon_app::shaping::{
    ANSWER_HINT, GO, GO_TYPED, LABELS, READING, SAID as SHARPER_ID, SHAPE, SHAPING, SHARPER, TYPED,
};
use jellymoon_app::wishing::{UNKEPT, UNSTARTED};
use serde_json::{Value, json};
use sticker_kit::{Field, Note, Scene, ToldRow};
use support::day::{DAY, answer, noon, result, said, shown, write_log};
use support::wished::{SAID, World, quiet, wait_until, world};

const WISH: &str = "Make the uploader try a failed upload twice more before it gives up.";
const DONE: &str = "A failed upload is tried three times in all.";
const LEFT: &str = "How a file is read, and what is shown while it uploads.";
const WHY: &str = "The uploader gives up after one failure today.";
const ASK: &str = "Should it wait between tries?";
/// What a run is given of a wish kept from that shape: core's labels, and the person's own words last.
const SHAPED: &str = "The wish: Make the uploader try a failed upload twice more before it gives up. | Done when: A \
                      failed upload is tried three times in all. | Not touching: How a file is read, and what is \
                      shown while it uploads. | The person's own words: “make the uploader retry twice”";
/// An answer to that shape's question, and what stands for it in what a run is given: the question under a
/// label of core's and the answer in the person's quote marks, both before the person's own words.
const ANSWERED: &str = "two seconds, and say so";
const WITH_ANSWER: &str = " | Asked of the person: Should it wait between tries? | The person's answer: “two \
                           seconds, and say so” | The person's own words: “";

fn ready() -> World {
    let w = world();
    fs::create_dir(w.at("harness")).unwrap();
    w
}

/// Has the made-up agent answer a shaping call with `answer` as the model's own text.
fn says(w: &World, answer: &Value) {
    let printed = json!({"type": "result", "is_error": false, "result": answer.to_string()});
    fs::write(w.at("agent").join("shape"), format!("says\n{printed}")).unwrap();
}

fn sharper() -> Value {
    json!({"clear": false, "wish": WISH, "done_when": DONE, "not_touching": LEFT, "why": WHY, "ask": ASK})
}

/// The window's lines for Enter in the entry with `alpha` chosen, and the acts after it.
fn acted(w: &World, typed: &[&str], then: &[&str], more: &[(&str, OsString)]) -> Vec<Value> {
    let mut acts = vec![format!("bin:{}", w.install().parent().unwrap().display())];
    for typed in typed {
        acts.extend([format!("enter:wish@alpha:{typed}"), "answer".to_string()]);
    }
    acts.extend(then.iter().map(|act| act.to_string()));
    let acts: Vec<&str> = acts.iter().map(String::as_str).collect();
    let lines = w.taps_with("logs", &acts, more).ended();
    lines.iter().map(|line| serde_json::from_str(line).expect(line)).collect()
}

fn scene(line: &Value) -> Scene {
    serde_json::from_value(line.clone()).expect("a scene")
}

/// The notes of a scene that are of a wish being read or of a sharper wish.
fn shaping(scene: &Scene) -> Vec<Note> {
    let ours = |note: &&Note| match note {
        Note::Text { id, .. } | Note::Told { id, .. } => [SHAPING, SHARPER_ID, SHAPE].contains(&id.as_str()),
        _ => false,
    };
    scene.notes.iter().filter(ours).cloned().collect()
}

fn text(id: &str, title: &str, quoted: &str, options: &[&str]) -> Note {
    let options = options.iter().map(|option| option.to_string()).collect();
    Note::Text { id: id.into(), title: title.into(), body: Vec::new(), options, quoted: quoted.into(), field: None }
}

/// The note of the two choices. When the sharper wish `asks`, it is the question's: titled as Jellymoon names a
/// question, the question its quote, the field it is answered in first, under no offer, and the two ways round
/// the question worded as what they do; `said` is not on it. With no question the typed words are its quote.
fn choice(said: &str, asks: bool) -> Note {
    let (most, hint) = (jellymoon_core::MADE_SAID_MAX, ANSWER_HINT.into());
    let field = Field { hint, most, wants_keys: true, first: true, ..Field::default() };
    let options = if asks { ["Skip the question", "Use my own words"] } else { [GO, GO_TYPED] };
    let (title, quoted) = if asks { ("Only you know", ASK) } else { (TYPED, said) };
    let (id, title, quoted) = (SHAPE.into(), title.into(), quoted.into());
    let options = options.map(str::to_string).to_vec();
    Note::Text { id, title, body: Vec::new(), options, quoted, field: asks.then_some(field) }
}

/// The two notes of the sharper wish that asks, whatever words it was made of: every word of the model's is a
/// quote, and its reason is not among them. The question's note stands first: it is what the person is asked.
fn pair() -> Vec<Note> {
    let rows = LABELS.iter().zip([WISH, DONE, LEFT]);
    let rows =
        rows.map(|(label, text)| ToldRow { label: label.to_string(), quotes: vec![text.into()], ..ToldRow::default() });
    let sharper = Note::Told { id: SHARPER_ID.into(), title: SHARPER.into(), rows: rows.collect(), end: Vec::new() };
    vec![choice("", true), sharper]
}

fn wishes(w: &World) -> std::path::PathBuf {
    w.at("jm").join("pages").join(format!("{DAY}.wishes.json"))
}

fn given(w: &World, name: &str) -> String {
    fs::read_to_string(w.at("rec").join(name)).unwrap()
}

/// The wish went on as a typed wish does: kept with the typed words and no prompt of a shape, and nothing of a
/// reading or a sharper wish left on the page.
fn as_typed(w: &World, printed: &[Value]) {
    assert_eq!(shaping(&scene(&printed[0])), [text(SHAPING, READING, SAID, &[])], "the page says so at once");
    assert_eq!(printed[1]["kept"], true);
    assert_eq!(shaping(&scene(&printed[1]["scene"])), []);
    let kept = w.kept();
    assert_eq!((&kept["said"], &kept["shaped"], &kept["id"]), (&json!(SAID), &Value::Null, &json!("m1")));
}

#[test]
fn enter_says_at_once_that_the_wish_is_read_and_clear_words_go_on_as_a_typed_wish() {
    let w = ready();
    let printed = acted(&w, &[SAID], &["start"], &[]);
    as_typed(&w, &printed);
    assert_eq!(printed[2], Value::Null, "the run was started, and the asking had nothing to say");
    w.ended();
    assert_eq!((w.starts(), given(&w, "stdin")), (1, format!("{SAID}\n")));
    assert!(given(&w, "shaping/stdin").ends_with(&format!("\nThe wish: {SAID}\n")), "the agent was asked to shape");
}

#[test]
fn a_sharper_wish_is_shown_as_quotes_with_two_choices_and_nothing_is_kept() {
    let w = ready();
    says(&w, &sharper());
    let printed = acted(&w, &[SAID], &["look"], &[]);
    assert_eq!(printed[1]["kept"], true, "the entry is emptied: its words stand on the note");
    let shown = scene(&printed[1]["scene"]);
    assert_eq!((&shown.notes[..2], shaping(&shown)), (&pair()[..], pair()));
    assert!(!wishes(&w).exists() && w.starts() == 0, "no wish is kept and no run is started");
    // Core reads the model's reason and checks it, and the page does not draw it.
    assert_eq!(LABELS[..], ["The wish", "Done when", "Not touching"]);
    assert!(!printed[1].to_string().contains(WHY), "{}", printed[1]);
    // A look leaves the notes: it shows nothing new, or a scene that still begins with them.
    assert!(printed[2].is_null() || scene(&printed[2]).notes[..2] == pair()[..], "{}", printed[2]);
    // A row whose text is empty is left out. With no question there is no field: the note of the choices has
    // the typed words as its quote, and Enter under it is nothing.
    let mut answer = sharper();
    (answer["ask"], answer["not_touching"]) = (json!(""), json!(" "));
    says(&w, &answer);
    let printed = acted(&w, &[SAID], &["placed", "type:shape:two seconds"], &[]);
    let unasked = scene(&printed[1]["scene"]);
    let Some(Note::Told { rows, .. }) = unasked.notes.first() else { panic!("no note") };
    assert_eq!(rows.iter().map(|row| row.label.as_str()).collect::<Vec<_>>(), ["The wish", "Done when"]);
    assert_eq!(unasked.notes[1], choice(SAID, false));
    assert_eq!((acted_on(&printed[2]), &printed[3]), (vec!["option", "option"], &Value::Null));
    assert!(!wishes(&w).exists() && w.starts() == 0, "no wish is kept and no run is started");
}

#[test]
fn a_question_is_the_quote_of_the_note_it_is_answered_in_and_the_typed_words_are_on_neither_note() {
    let w = ready();
    says(&w, &sharper());
    let shown = scene(&acted(&w, &[SAID], &[], &[])[1]["scene"]);
    let [Note::Text { id, title, quoted, options, field: Some(field), .. }, told] = &shown.notes[..2] else {
        panic!("{:?}", shown.notes)
    };
    let Note::Told { rows, .. } = told else { panic!("the sharper wish is under the question: {told:?}") };
    let rows: Vec<_> = rows.iter().map(|row| (row.label.as_str(), row.quotes.join("|"))).collect();
    assert_eq!(rows, [("The wish", WISH.to_string()), ("Done when", DONE.into()), ("Not touching", LEFT.into())]);
    assert_eq!((id.as_str(), title.as_str(), quoted.as_str()), (SHAPE, "Only you know", ASK));
    assert_eq!((options.join("|").as_str(), field.offer.as_str()), ("Skip the question|Use my own words", ""));
    assert_eq!((field.hint.as_str(), ANSWER_HINT, field.first), ("Type your answer, then Enter", &*field.hint, true));
    assert!(!format!("{:?}", &shown.notes[..2]).contains(SAID), "{:?}", shown.notes);
}

/// The kinds of the pieces a person acts on under the note of the choices, in the order they are placed.
fn acted_on(placed: &Value) -> Vec<&str> {
    let ours = |piece: &&Value| piece["id"] == SHAPE && ["option", "offer", "field"].contains(&kind(piece));
    placed.as_array().unwrap().iter().filter(ours).map(kind).collect()
}

fn kind(piece: &Value) -> &str {
    piece["kind"].as_str().unwrap()
}

#[test]
fn enter_in_the_field_keeps_the_wish_with_the_answer_and_starts_its_run_on_both() {
    let w = ready();
    says(&w, &sharper());
    let typed = format!("type:shape:{ANSWERED}");
    let printed = acted(&w, &[SAID], &["placed", "type:shape:  ", &typed, "start"], &[]);
    assert_eq!(acted_on(&printed[2]), ["field", "option", "option"], "the field stands before the ways round it");
    // An empty field gives nothing, and the pair stands: the Enter after it finds the field placed still.
    assert!(printed[3].is_null(), "{}", printed[3]);
    let (at_once, after) = (scene(&printed[4]["at_once"]), scene(&printed[4]["scene"]));
    assert_eq!((shaping(&at_once), shaping(&after)), (vec![], vec![]), "both notes go at once, as at a tap");
    assert_eq!(printed[4]["kept"], true, "the field is told its words were taken");
    let kept = w.kept();
    let given_to_the_run = SHAPED.replace(" | The person's own words: “", WITH_ANSWER);
    assert_eq!((&kept["said"], &kept["shaped"]), (&json!(SAID), &json!(given_to_the_run)), "{kept}");
    assert_eq!(printed[5], Value::Null, "the run was started, and the asking had nothing to say");
    w.ended();
    assert_eq!((w.starts(), given(&w, "stdin")), (1, format!("{given_to_the_run}\n")));
    assert!(!wishes(&w).with_file_name(format!("{DAY}.shape.json")).exists(), "the shape waits no more");
}

#[test]
fn an_answer_core_refuses_keeps_nothing_and_leaves_the_pair_with_core_s_reason() {
    let w = ready();
    says(&w, &sharper());
    // The answer is over its length: 203 characters typed, 202 once cleaned.
    let long = format!("type:shape:{}", "no ".repeat(68));
    let printed = acted(&w, &[SAID], &[&long], &[]);
    let refused = &printed[2];
    assert_eq!(refused["kept"], false, "the field keeps its words");
    let (at_once, after) = (scene(&refused["at_once"]), scene(&refused["scene"]));
    assert_eq!((shaping(&at_once), shaping(&after)), (vec![], pair()), "the pair is back, to answer again");
    let why = "an answer is at most 200 characters, and this one is longer, so nothing was kept.";
    let says = |note: &Note| matches!(note, Note::Plain { title, body } if title == UNKEPT && body.ends_with(why));
    assert!(after.notes.iter().any(says), "{:?}", after.notes);
    let waits = wishes(&w).with_file_name(format!("{DAY}.shape.json"));
    assert!(!wishes(&w).exists() && w.starts() == 0 && waits.exists(), "nothing is kept, and the shape still waits");
}

#[test]
fn skip_the_question_goes_on_with_the_shape_and_use_my_own_words_with_the_typed_words_and_each_starts_its_run() {
    let ways = [("0:Skip the question", json!(SHAPED), SHAPED), ("1:Use my own words", Value::Null, SAID)];
    for (way, shaped, given_to_the_run) in ways {
        let w = ready();
        says(&w, &sharper());
        // The tap carries the words seen at its place: with other words there it is no tap, and nothing runs.
        let printed = acted(&w, &[SAID], &[&format!("choose:shape:{way}"), "run"], &[]);
        assert_eq!(shaping(&scene(&printed[2])), [], "both notes go at once");
        let after = scene(&printed[3]);
        let failed =
            |note: &Note| matches!(note, Note::Plain { title, .. } if [UNKEPT, UNSTARTED].contains(&title.as_str()));
        assert!(shaping(&after).is_empty() && !after.notes.iter().any(failed), "{:?}", after.notes);
        let kept = w.kept();
        assert_eq!((&kept["said"], &kept["shaped"]), (&json!(SAID), &shaped), "{kept}");
        w.ended();
        assert_eq!((w.starts(), given(&w, "stdin")), (1, format!("{given_to_the_run}\n")));
        assert!(!wishes(&w).with_file_name(format!("{DAY}.shape.json")).exists(), "the shape waits no more");
    }
}

#[test]
fn enter_while_a_sharper_wish_stands_shapes_the_new_words_in_its_place() {
    let w = ready();
    says(&w, &sharper());
    let next = "make the uploader retry three times";
    let printed = acted(&w, &[SAID, next], &["choose:shape:1", "run"], &[]);
    assert_eq!(shaping(&scene(&printed[2])), [text(SHAPING, READING, next, &[])], "the old pair goes at once");
    assert_eq!(shaping(&scene(&printed[3]["scene"])), pair(), "one pair");
    // The pair is of the new words: a tap keeps one wish, of those, and none of the old.
    let kept = w.kept();
    assert_eq!((&kept["said"], &kept["id"]), (&json!(next), &json!("m1")), "{kept}");
    w.ended();
}

#[test]
fn a_bad_answer_a_call_that_takes_too_long_and_no_harness_go_on_as_a_typed_wish() {
    let w = ready();
    let mut answer = sharper();
    answer["more"] = json!("a seventh field");
    says(&w, &answer);
    let printed = acted(&w, &[SAID], &["start"], &[]);
    as_typed(&w, &printed);
    w.ended();
    assert_eq!((w.starts(), given(&w, "stdin")), (1, format!("{SAID}\n")));

    let w = ready();
    fs::write(w.at("agent").join("shape"), "forever\n").unwrap();
    let printed = acted(&w, &[SAID], &["start"], &[("JELLYMOON_TEST_SHAPE_LIMIT_MS", "300".into())]);
    as_typed(&w, &printed);
    w.ended();
    assert_eq!((w.starts(), given(&w, "stdin")), (1, format!("{SAID}\n")));

    let w = ready();
    fs::remove_file(w.at("agent").join(format!("claude{}", std::env::consts::EXE_SUFFIX))).unwrap();
    let printed = acted(&w, &[SAID], &["start"], &[]);
    as_typed(&w, &printed);
    assert_eq!(w.starts(), 0, "with no harness the wish is kept and no run starts, as it was");
}

/// A day that was read and holds no wish, with `alpha` on offer: its page says that nothing was wished for.
fn empty_day() -> World {
    let w = quiet();
    let typed = [said(&noon(0), w.at("work").join("alpha").to_str().unwrap(), "look at the uploader")];
    write_log(&w.day, "-work-alpha/s1.jsonl", &typed);
    answer(&w.day, &result(r#"{"wishes":[],"clashes":[]}"#), 0);
    shown(&w.day, &[]);
    fs::create_dir(w.at("harness")).unwrap();
    w
}

/// The window's lines for `acts`, with the installed command.
fn window(w: &World, acts: &[&str]) -> Vec<Value> {
    let bin = format!("bin:{}", w.install().parent().unwrap().display());
    let acts: Vec<&str> = [bin.as_str()].into_iter().chain(acts.iter().copied()).collect();
    let lines = w.taps("logs", &acts).ended();
    lines.iter().map(|line| serde_json::from_str(line).expect(line)).collect()
}

/// Whether the scene of a line has the note of a day with no wish: the strip that shows how a wish goes.
fn says_nothing(line: &Value) -> bool {
    let nothing = |note: &Note| matches!(note, Note::Drawn { id, .. } if id == jellymoon_app::scene::HOW);
    scene(line).notes.iter().any(nothing)
}

// Each `column` is the page as the window gives it to the kit's view (`Showing::column`).
#[test]
fn a_first_wish_that_is_being_read_or_made_is_not_under_a_note_that_nothing_was_wished_for() {
    let w = empty_day();
    // Its run waits for the test's word, so the wish is at work for as long as the window looks.
    w.play("steps");
    let enter = format!("enter:wish@alpha:{SAID}");
    let acts = ["column", enter.as_str(), "column", "look", "column", "answer", "column", "start", "look", "column"];
    let printed = window(&w, &acts);
    assert!(says_nothing(&printed[0]), "the empty day says so, as it did: {}", printed[0]);
    // From Enter on, while the wish is read, and after a look made meanwhile.
    for read in [&printed[2], &printed[4]] {
        assert_eq!(shaping(&scene(read)), [text(SHAPING, READING, SAID, &[])]);
        assert!(!says_nothing(read), "{read}");
    }
    // The wish is made, and then its run goes.
    assert_eq!((&printed[5]["kept"], w.kept()["id"].as_str()), (&json!(true), Some("m1")));
    assert!(!says_nothing(&printed[6]), "{}", printed[6]);
    let going = printed[9].to_string();
    assert!(going.contains("Your wish is being worked on.") && !says_nothing(&printed[9]), "{going}");
    fs::write(w.at("rec").join("go"), "").unwrap();
    w.ended();
}

#[test]
fn a_reading_that_kept_no_wish_leaves_the_page_saying_that_nothing_was_wished_for() {
    // No folder of `beta` was typed in: core refuses the wish, and the day has none.
    let w = empty_day();
    let printed = window(&w, &[&format!("enter:wish@beta:{SAID}"), "column", "answer", "column"]);
    assert!(!shaping(&scene(&printed[1])).is_empty() && !says_nothing(&printed[1]), "{}", printed[1]);
    assert_eq!(printed[2]["kept"], false);
    assert!(shaping(&scene(&printed[3])).is_empty() && says_nothing(&printed[3]), "{}", printed[3]);
    assert!(printed[3].to_string().contains(UNKEPT) && !wishes(&w).exists());
}

#[test]
fn a_sharper_wish_that_waits_and_the_choice_on_it_are_not_under_that_note_either() {
    let w = empty_day();
    says(&w, &sharper());
    let enter = format!("enter:wish@alpha:{SAID}");
    let printed = window(&w, &[enter.as_str(), "answer", "column", "choose:shape:1", "run", "column"]);
    assert_eq!(shaping(&scene(&printed[2])), pair());
    assert!(!says_nothing(&printed[2]), "{}", printed[2]);
    // The tap takes both notes away at once, and the wish is kept only after it: the scene shown at once.
    assert!(shaping(&scene(&printed[3])).is_empty() && !says_nothing(&printed[3]), "{}", printed[3]);
    assert!(!says_nothing(&printed[5]) && w.kept()["id"] == "m1", "{}", printed[5]);
    w.ended();
}

const NEXT: &str = "say hello in the readme";

/// Two wishes typed into the made-up window's entry, the kit's own state of one, with Enter after each: the
/// second Enter is pressed before core has answered the first. Then the first is answered, and `then`.
fn two_entered(w: &World, then: &[&str]) -> Vec<Value> {
    let typed = [SAID, NEXT].map(|typed| format!("press:wish@alpha:{typed}"));
    let printed = window(w, &[&[typed[0].as_str(), typed[1].as_str(), "answer"][..], then].concat());
    assert_eq!(printed[1], Value::Null, "the entry is owed an answer: its Enter gives nothing yet");
    printed
}

#[test]
fn a_wish_entered_while_the_one_before_is_read_is_kept_after_it_with_its_own_words_and_both_are_run() {
    let w = ready();
    let printed = two_entered(&w, &["answer", "start", "start"]);
    assert_eq!(shaping(&scene(&printed[2]["again"])), [text(SHAPING, READING, NEXT, &[])], "it is read at once");
    assert_eq!((&printed[2]["entry"], &printed[3]["again"]), (&json!(""), &Value::Null), "given once");
    assert_eq!(printed[4..], [Value::Null, Value::Null], "each run was started, with nothing to say");
    let kept: Value = serde_json::from_str(&fs::read_to_string(wishes(&w)).unwrap()).unwrap();
    let said = |at: usize| kept["wishes"][at]["said"].clone();
    assert_eq!((said(0), said(1)), (json!(SAID), json!(NEXT)), "in their order, each with its own words");
    for at in 0..2 {
        let lock = w.at("jm").join("runs").join(kept["wishes"][at]["run"].as_str().unwrap()).join("run.lock");
        wait_until("the end of the command", || !lock.exists());
    }
}

#[test]
fn an_enter_remembered_is_not_given_over_a_sharper_wish_that_asks_and_its_words_stay_in_the_entry() {
    let w = ready();
    says(&w, &sharper());
    let printed = two_entered(&w, &["column"]);
    assert_eq!((&printed[2]["again"], &printed[2]["entry"]), (&Value::Null, &json!(NEXT)), "{}", printed[2]);
    assert_eq!((shaping(&scene(&printed[2]["scene"])), shaping(&scene(&printed[3]))), (pair(), pair()));
    assert!(!wishes(&w).exists() && w.starts() == 0, "no wish is kept until the person chooses, and none is run");
}
