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

use jellymoon_app::shaping::{GO, GO_TYPED, LABELS, READING, SAID as SHARPER_ID, SHAPE, SHAPING, SHARPER, TYPED};
use jellymoon_app::wishing::{UNKEPT, UNSTARTED};
use serde_json::{Value, json};
use sticker_kit::{Note, Scene, ToldRow};
use support::day::DAY;
use support::wished::{SAID, World, world};

const WISH: &str = "Make the uploader try a failed upload twice more before it gives up.";
const DONE: &str = "A failed upload is tried three times in all.";
const LEFT: &str = "How a file is read, and what is shown while it uploads.";
const WHY: &str = "The uploader gives up after one failure today.";
const ASK: &str = "Should it wait between tries?";
/// What a run is given of a wish kept from that shape: core's labels, and the person's own words last.
const SHAPED: &str = "The wish: Make the uploader try a failed upload twice more before it gives up. | Done when: A \
                      failed upload is tried three times in all. | Not touching: How a file is read, and what is \
                      shown while it uploads. | The person's own words: “make the uploader retry twice”";

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

/// The two notes of the sharper wish for the words `said`: every word of the model's is a quote.
fn pair(said: &str) -> Vec<Note> {
    // The question only the person can answer stands before the model's reason.
    let rows = LABELS.iter().zip([WISH, DONE, LEFT, ASK, WHY]);
    let rows =
        rows.map(|(label, text)| ToldRow { label: label.to_string(), quotes: vec![text.into()], ..ToldRow::default() });
    let sharper = Note::Told { id: SHARPER_ID.into(), title: SHARPER.into(), rows: rows.collect(), end: Vec::new() };
    vec![sharper, text(SHAPE, TYPED, said, &[GO, GO_TYPED])]
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
    assert_eq!((&shown.notes[..2], shaping(&shown)), (&pair(SAID)[..], pair(SAID)));
    assert!(!wishes(&w).exists() && w.starts() == 0, "no wish is kept and no run is started");
    // A look leaves the notes: it shows nothing new, or a scene that still begins with them.
    assert!(printed[2].is_null() || scene(&printed[2]).notes[..2] == pair(SAID)[..], "{}", printed[2]);
    // A row whose text is empty is left out, and a question is asked only when there is one.
    let mut answer = sharper();
    (answer["ask"], answer["not_touching"]) = (json!(""), json!(" "));
    says(&w, &answer);
    let printed = acted(&w, &[SAID], &[], &[]);
    let Some(Note::Told { rows, .. }) = scene(&printed[1]["scene"]).notes.first().cloned() else { panic!("no note") };
    assert_eq!(rows.iter().map(|row| row.label.as_str()).collect::<Vec<_>>(), ["The wish", "Done when", "Why"]);
    assert_eq!(LABELS[3..], ["Only you know", "Why"]);
}

#[test]
fn a_tap_on_either_choice_keeps_the_wish_and_starts_its_run_on_the_shape_or_on_the_typed_words() {
    for (index, shaped, given_to_the_run) in [(0, json!(SHAPED), SHAPED), (1, Value::Null, SAID)] {
        let w = ready();
        says(&w, &sharper());
        let printed = acted(&w, &[SAID], &[&format!("choose:shape:{index}"), "run"], &[]);
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
    let printed = acted(&w, &[SAID, next], &[], &[]);
    assert_eq!(shaping(&scene(&printed[2])), [text(SHAPING, READING, next, &[])], "the old pair goes at once");
    assert_eq!(shaping(&scene(&printed[3]["scene"])), pair(next), "one pair, of the new words");
    assert!(!wishes(&w).exists() && w.starts() == 0, "no wish is made of the old words");
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
