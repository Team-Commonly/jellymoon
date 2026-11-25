//! The sharper wish folds (ADR 0025, decision 12; `docs/development/app.md`, "The sharper wish folds"). Under a
//! question that waits, the sharper wish is one line of Jellymoon's own, with a chevron: a tap on it puts the
//! note under it, every row, and a second tap folds it again. The question's note, its field and the two ways
//! round it are as they were, and so is what each keeps and what the run is given. With no question nothing is
//! folded. Carried by the made-up window of `support/tapper.rs`, a process of its own: `column` prints the scene
//! as the window gives it to the kit's view, `pieces` what the kit places of it, and `placed` what `layout`
//! prints, which folds nothing and so is the page as it was before the fold. The made-up agent of F39's tests
//! answers the shaping call. Nothing here starts the real Claude Code or opens a window.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::fs;

use jellymoon_app::shaping::{
    ANSWER_HINT, GO, GO_TYPED, LABELS, READING, SAID as SHARPER_ID, SHAPE, SHAPING, SHARPER, TYPED,
};
use jellymoon_app::wishing::UNKEPT;
use serde_json::{Value, json};
use sticker_kit::{Field, Note, Scene, ToldRow};
use support::day::DAY;
use support::fold_line;
use support::wished::{SAID, World, world};

const FOLD: &str = "shape-fold";
const WISH: &str = "Make the uploader try a failed upload twice more before it gives up.";
const DONE: &str = "A failed upload is tried three times in all.";
const LEFT: &str = "How a file is read, and what is shown while it uploads.";
const ASK: &str = "Should it wait between tries?";
/// What a run is given of a wish kept from that shape, and with an answer: the texts of `shape_first.rs`.
const SHAPED: &str = "The wish: Make the uploader try a failed upload twice more before it gives up. | Done when: A \
                      failed upload is tried three times in all. | Not touching: How a file is read, and what is \
                      shown while it uploads. | The person's own words: “make the uploader retry twice”";
const ANSWERED: &str = "two seconds, and say so";
const WITH_ANSWER: &str = " | Asked of the person: Should it wait between tries? | The person's answer: “two \
                           seconds, and say so” | The person's own words: “";

/// A day with `alpha` on offer on which the made-up agent answers a shaping call with a sharper wish, which
/// asks when `asks`.
fn ready(asks: bool) -> World {
    let w = world();
    fs::create_dir(w.at("harness")).unwrap();
    says(&w, asks);
    w
}

fn says(w: &World, asks: bool) {
    let (why, ask) = ("The uploader gives up after one failure today.", if asks { ASK } else { "" });
    let sharper =
        json!({"clear": false, "wish": WISH, "done_when": DONE, "not_touching": LEFT, "why": why, "ask": ask});
    let printed = json!({"type": "result", "is_error": false, "result": sharper.to_string()});
    fs::write(w.at("agent").join("shape"), format!("says\n{printed}")).unwrap();
}

fn bin(w: &World) -> String {
    format!("bin:{}", w.install().parent().unwrap().display())
}

fn parsed(lines: Vec<String>) -> Vec<Value> {
    lines.iter().map(|line| serde_json::from_str(line).expect(line)).collect()
}

/// The window's lines for Enter in the entry with `alpha` chosen, core's answer, and the acts after them: the
/// lines of `then` begin at the third.
fn acted(w: &World, then: &[&str]) -> Vec<Value> {
    let first = [bin(w), format!("enter:wish@alpha:{SAID}"), "answer".to_string()];
    let acts: Vec<&str> = first.iter().map(String::as_str).chain(then.iter().copied()).collect();
    parsed(w.taps("logs", &acts).ended())
}

fn scene(line: &Value) -> Scene {
    serde_json::from_value(line.clone()).expect("a scene")
}

/// The notes of a scene that are of a wish being read, of a sharper wish or of its fold, in the scene's order.
fn ours(scene: &Scene) -> Vec<Note> {
    let ours = |note: &&Note| match note {
        Note::Text { id, .. } | Note::Told { id, .. } | Note::Drawn { id, .. } => {
            [SHAPING, SHARPER_ID, SHAPE, FOLD].contains(&id.as_str())
        }
        _ => false,
    };
    scene.notes.iter().filter(ours).cloned().collect()
}

/// The question's note, as it was before the fold: its quote, its field with the keys, and the two ways round it.
fn question() -> Note {
    let (most, hint) = (jellymoon_core::MADE_SAID_MAX, ANSWER_HINT.into());
    let field = Field { hint, most, wants_keys: true, first: true, ..Field::default() };
    let options = ["Skip the question", "Use my own words"].map(str::to_string).to_vec();
    let (id, title, quoted) = (SHAPE.into(), "Only you know".into(), ASK.into());
    Note::Text { id, title, body: Vec::new(), options, quoted, field: Some(field) }
}

/// The sharper wish's note as it was before the fold: every word of the model's a quote under a label.
fn sharper() -> Note {
    let rows = LABELS.iter().zip([WISH, DONE, LEFT]);
    let rows =
        rows.map(|(label, text)| ToldRow { label: label.to_string(), quotes: vec![text.into()], ..ToldRow::default() });
    Note::Told { id: SHARPER_ID.into(), title: SHARPER.into(), rows: rows.collect(), end: Vec::new() }
}

/// The pieces of the note `id` among printed pieces, and how far down each piece and each run of its text is.
/// A piece's place among the scene's notes is taken out with them: a note under one more card is the same note.
fn of(pieces: &Value, id: &str) -> (Vec<Value>, Vec<f64>) {
    let mut downs = Vec::new();
    let mut own: Vec<Value> = pieces.as_array().unwrap().iter().filter(|piece| piece["id"] == id).cloned().collect();
    for piece in &mut own {
        if piece["kind"] != "option" {
            piece["index"].take();
        }
        downs.push(piece["y"].take().as_f64().unwrap());
        let runs = piece["text"].as_array_mut().unwrap();
        downs.extend(runs.iter_mut().map(|run| run["y"].take().as_f64().unwrap()));
    }
    (own, downs)
}

/// Whether the pieces `now` are the pieces `was`, all of them lower by one and the same distance: that distance.
fn lower((now, at): &(Vec<Value>, Vec<f64>), (was, was_at): &(Vec<Value>, Vec<f64>)) -> f64 {
    assert!(!was.is_empty(), "no piece to compare");
    assert_eq!(now, was);
    let by = at[0] - was_at[0];
    assert!(at.iter().zip(was_at).all(|(a, b)| (a - b - by).abs() < 0.011), "{at:?} {was_at:?}");
    by
}

/// Every word placed among printed pieces, the runs joined with spaces.
fn words(pieces: &Value) -> String {
    let runs = pieces.as_array().unwrap().iter().flat_map(|piece| piece["text"].as_array().unwrap());
    runs.map(|run| run["text"].as_str().unwrap()).collect::<Vec<&str>>().join(" ")
}

fn nothing_kept(w: &World) -> bool {
    let wishes = w.at("jm").join("pages").join(format!("{DAY}.wishes.json"));
    !wishes.exists() && w.starts() == 0 && wishes.with_file_name(format!("{DAY}.shape.json")).exists()
}

#[test]
fn under_a_question_the_sharper_wish_is_one_line_a_tap_puts_every_row_under_it_and_a_second_folds_it() {
    let w = ready(true);
    let tap = "tap:shape-fold";
    let printed = acted(&w, &["column", "pieces", "placed", tap, "column", "pieces", tap, "column"]);
    let [_, answer, folded, few, was, tapped, open, all, _, again] = &printed[..] else { panic!("{printed:#?}") };
    // Folded: the question's note, then the line, and no row of the sharper wish.
    assert_eq!(ours(&scene(folded)), [question(), fold_line(false)]);
    assert!(of(few, SHARPER_ID).0.is_empty(), "no piece of the sharper wish is placed");
    let said = words(few);
    for hidden in [SHARPER, "The wish", "Done when", "Not touching", "twice more", "three times", "How a file"] {
        assert!(!said.contains(hidden), "{hidden:?} is placed while folded: {said}");
    }
    let line: Vec<&Value> = few.as_array().unwrap().iter().filter(|piece| piece["id"] == FOLD).collect();
    let pictures = line.iter().filter(|piece| piece["kind"] == "picture");
    let pictures: Vec<String> = pictures.map(|one| words(&json!([one]))).collect();
    assert_eq!(pictures, ["your wish, sharper"], "one picture, with Jellymoon's own words and no other");
    assert_eq!(words(&json!(line)), "your wish, sharper", "and no other word on its card");
    let acts = |piece: &&Value| ["option", "field", "offer", "ask", "quote"].contains(&piece["kind"].as_str().unwrap());
    assert!(!line.iter().any(acts), "nothing to choose, type in or read on it: {line:?}");
    // The question's note is the one `layout` prints, which folds nothing: its quote, its field, its two options.
    let asked = of(few, SHAPE);
    let kinds = |kind: &str| asked.0.iter().filter(|piece| piece["kind"] == kind).count();
    assert_eq!((kinds("field"), kinds("option"), kinds("quote")), (1, 2, 1), "{:?}", asked.0);
    let question_down = lower(&asked, &of(was, SHAPE));
    let top = |pieces: &Value, id: &str| of(pieces, id).1[0];
    assert!(top(few, SHAPE) < top(few, FOLD), "the question first, then the line");
    // The tap changes nothing that is kept, and asks core nothing: the scene on show is the one that stood.
    assert_eq!((tapped, ours(&scene(tapped))), (&answer["scene"], vec![question(), sharper()]));
    assert!(nothing_kept(&w), "a tap on the line keeps no wish, starts no run and leaves the shape waiting");
    // Unfolded: the line, turned, and under it the note as it was, every row and every word.
    assert_eq!(ours(&scene(open)), [question(), fold_line(true), sharper()]);
    assert_eq!(of(all, SHAPE), asked, "the question's note does not move");
    let unfolded = of(all, SHARPER_ID);
    let quotes = unfolded.0.iter().filter(|piece| piece["kind"] == "quote").count();
    assert_eq!(quotes, 3, "{:?}", unfolded.0);
    let down = lower(&unfolded, &of(was, SHARPER_ID));
    assert!(down > question_down, "under the line: {down} {question_down}");
    assert!(top(all, SHAPE) < top(all, FOLD) && top(all, FOLD) < top(all, SHARPER_ID), "question, line, note");
    assert_eq!(again, folded, "a second tap folds it again");
    assert!(nothing_kept(&w));
}

#[test]
fn the_answer_and_both_ways_round_keep_what_they_kept_and_give_the_run_the_same_form_folded_or_unfolded() {
    let with_answer = SHAPED.replace(" | The person's own words: “", WITH_ANSWER);
    let ways = [
        (format!("type:shape:{ANSWERED}"), "start", json!(with_answer), with_answer.as_str()),
        ("choose:shape:0:Skip the question".to_string(), "run", json!(SHAPED), SHAPED),
        ("choose:shape:1:Use my own words".to_string(), "run", Value::Null, SAID),
    ];
    for unfolded in [false, true] {
        for (way, then, shaped, given_to_the_run) in &ways {
            let w = ready(true);
            let acts = unfolded.then_some("tap:shape-fold").into_iter().chain([way.as_str(), then, "column"]);
            let printed = acted(&w, &acts.collect::<Vec<&str>>());
            let kept = w.kept();
            assert_eq!((&kept["said"], &kept["shaped"]), (&json!(SAID), shaped), "{way}, {unfolded}: {kept}");
            w.ended();
            let given = fs::read_to_string(w.at("rec").join("stdin")).unwrap();
            assert_eq!((w.starts(), given), (1, format!("{given_to_the_run}\n")), "{way}, {unfolded}");
            assert_eq!(ours(&scene(printed.last().unwrap())), [], "the pair and its line are gone");
        }
    }
}

#[test]
fn a_refused_answer_puts_the_pair_back_with_the_fold_as_it_was() {
    // The answer is over its length: 203 characters typed, 202 once cleaned.
    let long = format!("type:shape:{}", "no ".repeat(68));
    for unfolded in [false, true] {
        let w = ready(true);
        let acts = unfolded.then_some("tap:shape-fold").into_iter().chain([long.as_str(), "column"]);
        let printed = acted(&w, &acts.collect::<Vec<&str>>());
        let [.., refused, column] = &printed[..] else { panic!("{printed:#?}") };
        assert_eq!(refused["kept"], false, "the field keeps its words");
        assert_eq!(ours(&scene(&refused["scene"])), [question(), sharper()], "the same notes are held again");
        let rows = unfolded.then(sharper);
        let back: Vec<Note> = [question(), fold_line(unfolded)].into_iter().chain(rows).collect();
        assert_eq!(ours(&scene(column)), back, "unfolded: {unfolded}");
        assert!(nothing_kept(&w));
    }
}

#[test]
fn a_new_question_starts_folded_whatever_the_one_before_was() {
    let w = ready(true);
    // The made-up agent answers the next words with the same sharper wish, text for text.
    let next = "enter:wish@alpha:make the uploader retry three times";
    let printed = acted(&w, &["tap:shape-fold", "column", next, "answer", "column"]);
    assert_eq!(ours(&scene(&printed[3])), [question(), fold_line(true), sharper()]);
    assert_eq!(ours(&scene(&printed[5]["scene"])), [question(), sharper()], "the next pair stands");
    assert_eq!(ours(&scene(&printed[6])), [question(), fold_line(false)]);
}

// Each case in which the sharper wish, or the reading before it, stands with no question: nothing is folded, no
// line is placed, and a tap on the line's id is no tap.
#[test]
fn with_no_question_the_reading_the_sharper_wish_with_its_two_choices_and_a_refused_tap_are_as_they_were() {
    let w = ready(false);
    let (enter, go) = (format!("enter:wish@alpha:{SAID}"), w.at("tmp").join("go"));
    let (shape, hold) = (w.at("jm").join("pages").join(format!("{DAY}.shape.json")), format!("hold:{}", go.display()));
    let then = ["column", "pieces", "answer", "column", "pieces", "placed", "tap:shape-fold", "column"];
    let (bin, last) = (bin(&w), [hold.as_str(), "choose:shape:0", "run", "column"]);
    let acts: Vec<&str> = [bin.as_str(), enter.as_str()].into_iter().chain(then).chain(last).collect();
    let mut window = w.taps("logs", &acts);
    window.printed(9);
    // The shape waits no more: core refuses the tap, and nothing is kept.
    fs::remove_file(&shape).unwrap();
    fs::write(&go, "").unwrap();
    let printed = parsed(window.ended());
    let [_, reading, read, _, column, pieces, was, tapped, after, _, _, refused] = &printed[..] else {
        panic!("{printed:#?}")
    };
    // While the wish is read.
    let quoted = SAID.to_string();
    let (id, title, body, options) = (SHAPING.into(), READING.into(), Vec::new(), Vec::new());
    assert_eq!(ours(&scene(reading)), [Note::Text { id, title, body, options, quoted, field: None }]);
    assert!(of(read, FOLD).0.is_empty());
    // With its two choices: the sharper wish, then the typed words and the two choices, piece for piece.
    let (id, title, quoted) = (SHAPE.into(), TYPED.into(), SAID.to_string());
    let options = [GO, GO_TYPED].map(str::to_string).to_vec();
    let choice = Note::Text { id, title, body: Vec::new(), options, quoted, field: None };
    assert_eq!(ours(&scene(column)), [sharper(), choice]);
    assert_eq!((tapped, after), (&Value::Null, column), "no line, and no tap on one");
    assert!(of(pieces, FOLD).0.is_empty());
    let down = lower(&of(pieces, SHARPER_ID), &of(was, SHARPER_ID));
    let other = lower(&of(pieces, SHAPE), &of(was, SHAPE));
    assert!((other - down).abs() < 0.011, "both notes as `layout` prints them: {other} {down}");
    // After a refused tap: both notes are gone, core's reason stands, and no line is left.
    let after = scene(refused);
    assert_eq!(ours(&after), []);
    assert!(after.notes.iter().any(|note| matches!(note, Note::Plain { title, .. } if title == UNKEPT)), "{after:?}");
    assert!(!shape.with_file_name(format!("{DAY}.wishes.json")).exists() && w.starts() == 0);
}

/// The keys are in a view, which the made-up window has none of: the page's own window on GPUI's test platform,
/// as in `a_question_takes_the_keys.rs`. On the Mac only; no window is opened on a screen.
#[cfg(all(target_os = "macos", feature = "window"))]
mod window {
    use gpui::{Modifiers, TestAppContext, point, px};
    use jellymoon_app::shaping::{SAID as SHARPER_ID, SHAPE};
    use jellymoon_app::showing::Showing;
    use sticker_kit::PieceKind;

    use crate::support::windowed::{context, opened};

    #[test]
    fn letters_typed_with_no_click_on_the_field_are_the_answer_folded_and_after_a_click_on_the_line() {
        // The world of `a_question_takes_the_keys.rs`: `alpha` added, its chip on the page that was read and kept.
        let w = crate::support::wished::quiet();
        w.jellymoon(&["project", "add", "--repo", w.at("work").join("alpha").to_str().unwrap()]);
        assert_eq!(w.placed().iter().filter(|piece| piece.kind() == "chip").count(), 1);
        super::says(&w, true);
        let shown = Showing::of(Some(w.on_show()));
        let mut cx: TestAppContext = context();
        let (page, cx) = opened(&mut cx, shown.clone());
        let keys = |cx: &mut gpui::VisualTestContext, keystrokes: &str| {
            cx.simulate_keystrokes(keystrokes);
            cx.run_until_parked();
        };
        keys(cx, "t i d y space i t enter");
        // What is typed in the entry and in the answer's field, and whether the sharper wish's rows are placed.
        let read = |cx: &mut gpui::VisualTestContext| {
            page.read_with(cx, |page, _| {
                let field = page.field(SHAPE).map(|field| field.text().to_string());
                let rows = page.placed().iter().any(|piece| piece.id == SHARPER_ID);
                (page.entry().text().to_string(), field, rows)
            })
        };
        keys(cx, "t w o");
        assert_eq!(read(cx), (String::new(), Some("two".into()), false), "folded, the letters are the answer");
        // A click on the line, where it is drawn: the rows come, and the next letters are still the answer.
        let line = |piece: &&sticker_kit::Placed| piece.piece == PieceKind::Picture && piece.id == super::FOLD;
        let at = page.read_with(cx, |page, _| page.placed().iter().find(line).map(|piece| page.shown_at(piece)));
        let (x, y) = at.expect("the line is placed").center();
        cx.simulate_click(point(px(x), px(y)), Modifiers::default());
        cx.run_until_parked();
        keys(cx, "space s e c");
        assert_eq!(read(cx), (String::new(), Some("two sec".into()), true), "unfolded, they still are");
        keys(cx, "enter");
        let kept = w.kept();
        assert_eq!((kept["said"].as_str(), kept["answered"].as_str()), (Some("tidy it"), Some("two sec")), "{kept}");
    }
}
