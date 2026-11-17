//! The field of a note in the view, headless: no window is shown. Keys and clicks go through GPUI's own
//! dispatch, as in `tests/entry_draw.rs`. A field is typed in as the entry is, holds its own words, and gives
//! them once at Enter with its note's id; someone else's words under a note take no click.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::cell::RefCell;
use std::rc::Rc;

use gpui::{Entity, Modifiers, TestAppContext, TestDispatcher, VisualTestContext, point, px, size};
use sticker_kit::draw::headless::text_system;
use sticker_kit::draw::{EntryMade, Page, Tap, load_fonts};
use sticker_kit::{Entry, Field, Note, PieceKind, Placed, Scene, Theme};

type Seen<T> = Rc<RefCell<Vec<T>>>;

fn note(id: &str, quoted: &str, offer: &str) -> Note {
    let field = Some(Field { offer: offer.into(), hint: "Type here".into(), most: 12, key: String::new() });
    let (title, body) = ("A row".into(), vec!["what the note says".into()]);
    Note::Text { id: id.into(), title, body, options: Vec::new(), quoted: quoted.into(), field }
}

/// A note whose field has a key, and one with no field at all.
fn keyed(id: &str, key: &str) -> Note {
    let field = Some(Field { offer: "More of it".into(), hint: "Type here".into(), most: 12, key: key.into() });
    Note::Text { id: id.into(), title: "A row".into(), body: vec![], options: vec![], quoted: String::new(), field }
}

fn bare(id: &str) -> Note {
    let (title, quoted) = ("A row".into(), String::new());
    Note::Text { id: id.into(), title, body: vec![], options: vec![], quoted, field: None }
}

fn show(page: &Entity<Page>, cx: &mut VisualTestContext, notes: Vec<Note>) {
    page.update(cx, |page, cx| page.set_scene(scene(notes), cx));
    cx.run_until_parked();
}

/// The words in the field of a note and whether it waits. None for a note with no field.
fn field(page: &Entity<Page>, cx: &mut VisualTestContext, id: &str) -> Option<(String, bool)> {
    page.read_with(cx, |page, _| page.field(id).map(|field| (field.text().to_string(), field.waiting())))
}

fn has(words: &str) -> Option<(String, bool)> {
    Some((words.to_string(), false))
}

fn scene(notes: Vec<Note>) -> Scene {
    Scene { notes, ..Scene::sample() }
}

fn both() -> Vec<Note> {
    vec![note("asks", "Which of the two?", ""), note("more", "", "More of it")]
}

fn open(cx: &mut TestAppContext) -> (Entity<Page>, &mut VisualTestContext, Seen<EntryMade>, Seen<Tap>) {
    cx.update(|cx| load_fonts(cx)).expect("bundled fonts load");
    let (made, taps) = (Seen::<EntryMade>::default(), Seen::<Tap>::default());
    let (entered, tapped) = (made.clone(), taps.clone());
    let entry =
        Entry { id: "wish".into(), hint: "wish for something".into(), options: vec!["garden".into()], most: 12 };
    let (page, cx) = cx.add_window_view(move |_, _| {
        Page::new(scene(both()), Theme::sticker())
            .reduce_motion(true)
            .with_entry(entry)
            .on_entry(move |made, _, _| entered.borrow_mut().push(made.clone()))
            .on_tap(move |tap, _, _| tapped.borrow_mut().push(tap.clone()))
    });
    cx.simulate_resize(size(px(1440.0), px(900.0)));
    cx.run_until_parked();
    (page, cx, made, taps)
}

fn context() -> TestAppContext {
    TestAppContext::build_with_text_system(TestDispatcher::new(0), None, text_system())
}

fn piece(page: &Entity<Page>, cx: &mut VisualTestContext, kind: PieceKind, id: &str) -> Placed {
    let found = page.read_with(cx, |page, _| page.placed().iter().find(|p| p.piece == kind && p.id == id).cloned());
    found.unwrap_or_else(|| panic!("no {kind:?} of {id} is placed"))
}

fn click(cx: &mut VisualTestContext, at: (f32, f32)) {
    cx.simulate_click(point(px(at.0), px(at.1)), Modifiers::default());
    cx.run_until_parked();
}

fn press(page: &Entity<Page>, cx: &mut VisualTestContext, kind: PieceKind, id: &str) {
    let at = piece(page, cx, kind, id).rect.center();
    click(cx, at);
}

fn keys(cx: &mut VisualTestContext, keystrokes: &str) {
    cx.simulate_keystrokes(keystrokes);
    cx.run_until_parked();
}

/// What is typed in the entry and in the two fields, and whether any of them has the keys.
fn typed(page: &Entity<Page>, cx: &mut VisualTestContext) -> (String, String, String, bool) {
    cx.update(|window, cx| {
        let page = page.read(cx);
        let field = |id: &str| page.field(id).map(|field| field.text().to_string()).unwrap_or_default();
        (page.entry().text().to_string(), field("asks"), field("more"), page.entry_focused(window))
    })
}

fn answer(page: &Entity<Page>, cx: &mut VisualTestContext, id: &str, kept: bool) {
    cx.update(|window, cx| page.update(cx, |page, cx| page.answer_field(id, kept, window, cx)));
    cx.run_until_parked();
}

fn made(id: &str, text: &str) -> EntryMade {
    EntryMade { id: id.into(), text: text.into(), option: None }
}

#[test]
fn a_click_gives_a_note_s_field_the_keys_and_enter_gives_its_words_once_with_the_note_s_id() {
    let mut cx = context();
    let (page, cx, entered, _) = open(&mut cx);
    keys(cx, "x");
    assert_eq!(typed(&page, cx), (String::new(), String::new(), String::new(), false), "no field has the keys yet");
    let field = piece(&page, cx, PieceKind::Field, "asks");
    click(cx, field.rect.center());
    keys(cx, "t h e space o l d");
    assert_eq!(typed(&page, cx), (String::new(), "the old".into(), String::new(), true));

    // Enter gives the words with the note's id and no option; a second Enter, and every key, is nothing while
    // the field waits for its answer.
    keys(cx, "enter enter x backspace enter");
    assert_eq!(*entered.borrow(), [made("asks", "the old")]);
    assert_eq!(typed(&page, cx).1, "the old", "the words stay until they are taken");
    assert!(page.read_with(cx, |page, _| page.field("asks").is_some_and(|field| field.waiting())));

    // Not taken: the words stay, and the field takes keys again. Taken: it is empty.
    answer(&page, cx, "asks", false);
    keys(cx, "!");
    assert_eq!(typed(&page, cx).1, "the old!");
    keys(cx, "enter");
    assert_eq!(entered.borrow().last(), Some(&made("asks", "the old!")));
    answer(&page, cx, "asks", true);
    assert_eq!(typed(&page, cx), (String::new(), String::new(), String::new(), true));
    // An answer for a note with no field changes nothing.
    answer(&page, cx, "nobody", true);
    assert_eq!(entered.borrow().len(), 2);
}

#[test]
fn each_field_and_the_entry_hold_their_own_words_and_a_tap_on_the_offer_gives_its_field_the_keys() {
    let mut cx = context();
    let (page, cx, entered, taps) = open(&mut cx);
    let offer = piece(&page, cx, PieceKind::Offer, "more");
    click(cx, offer.rect.center());
    keys(cx, "a b");
    assert_eq!(typed(&page, cx), (String::new(), String::new(), "ab".into(), true));
    press(&page, cx, PieceKind::Field, "asks");
    keys(cx, "c d");
    press(&page, cx, PieceKind::Entry, "wish");
    keys(cx, "e f");
    assert_eq!(typed(&page, cx), ("ef".into(), "cd".into(), "ab".into(), true));
    // The cap is each field's own: twelve characters here.
    press(&page, cx, PieceKind::Field, "more");
    keys(cx, "end 1 2 3 4 5 6 7 8 9 0 1 2");
    assert_eq!(typed(&page, cx).2, "ab1234567890");
    keys(cx, "enter");
    assert_eq!(*entered.borrow(), [made("more", "ab1234567890")]);
    // The other field and the entry take keys while that one waits.
    press(&page, cx, PieceKind::Field, "asks");
    keys(cx, "!");
    assert_eq!(typed(&page, cx).1, "cd!");
    // Escape gives the keys back, and so does a press anywhere else on the page.
    keys(cx, "escape z");
    assert_eq!(typed(&page, cx), ("ef".into(), "cd!".into(), "ab1234567890".into(), false));
    press(&page, cx, PieceKind::Field, "asks");
    click(cx, (4.0, 4.0));
    assert!(!typed(&page, cx).3);
    // Someone else's words, the field and the offer are no tap: nothing was tapped in all of this.
    press(&page, cx, PieceKind::Quote, "asks");
    assert!(taps.borrow().is_empty(), "{:?}", taps.borrow());
}

#[test]
fn a_field_keeps_its_words_while_its_note_has_it_and_goes_with_it() {
    let mut cx = context();
    let (page, cx, entered, _) = open(&mut cx);
    press(&page, cx, PieceKind::Field, "more");
    keys(cx, "k e p t");
    // The scene changes round the note: the words stay, and the field still has the keys.
    let mut notes = both();
    notes.insert(0, Note::Plain { title: "Said".into(), body: "something new".into() });
    page.update(cx, |page, cx| page.set_scene(scene(notes), cx));
    cx.run_until_parked();
    keys(cx, "!");
    assert_eq!(typed(&page, cx), (String::new(), String::new(), "kept!".into(), true));
    // The note loses its field: the field goes, and an Enter has nothing to give.
    let gone = vec![note("asks", "Which of the two?", "")];
    page.update(cx, |page, cx| page.set_scene(scene(gone), cx));
    cx.run_until_parked();
    keys(cx, "enter");
    let fields = page.read_with(cx, |page, _| (page.field("asks").is_some(), page.field("more").is_some()));
    assert_eq!((fields, entered.borrow().len()), ((true, false), 0));
    let placed = page.read_with(cx, |page, _| page.placed().to_vec());
    assert_eq!(placed.iter().filter(|piece| piece.piece == PieceKind::Field).count(), 1);
}

#[test]
fn a_field_answered_with_no_window_waits_no_more() {
    let mut cx = context();
    let (page, cx, entered, _) = open(&mut cx);
    press(&page, cx, PieceKind::Field, "asks");
    keys(cx, "o l d enter");
    assert!(page.read_with(cx, |page, _| page.field("asks").is_some_and(|field| field.waiting())));
    // The answer comes with no window to tell: not kept, the words stay and the field takes keys again.
    page.update(cx, |page, cx| page.answer_field_closed("asks", false, cx));
    cx.run_until_parked();
    assert!(page.read_with(cx, |page, _| page.field("asks").is_some_and(|field| !field.waiting())));
    keys(cx, "! enter");
    assert_eq!(entered.borrow().last(), Some(&made("asks", "old!")));
    // Kept, it is empty, and the other field and the entry are as they were.
    page.update(cx, |page, cx| page.answer_field_closed("asks", true, cx));
    page.update(cx, |page, cx| page.answer_field_closed("nobody", true, cx));
    cx.run_until_parked();
    assert_eq!(typed(&page, cx), (String::new(), String::new(), String::new(), true));
    assert!(page.read_with(cx, |page, _| page.field("asks").is_some_and(|field| !field.waiting())));
}

// The review of F54, finding 6: one note's id can carry a field for one thing and then for another.
#[test]
fn a_field_s_words_are_dropped_when_its_offer_changes_and_stay_when_only_its_hint_does() {
    let mut cx = context();
    let (page, cx, _, _) = open(&mut cx);
    press(&page, cx, PieceKind::Field, "asks");
    keys(cx, "h a l f");
    // The same note, the same field, another hint: the words are still the field's.
    let mut hinted = both();
    if let Note::Text { field: Some(field), .. } = &mut hinted[0] {
        field.hint = "Type it here".into();
    }
    show(&page, cx, hinted);
    assert_eq!(field(&page, cx, "asks"), has("half"));
    // The same note with a field under another offer: what was typed for the first is not typed for this one.
    show(&page, cx, vec![note("asks", "", "More of it"), note("more", "", "More of it")]);
    assert_eq!((field(&page, cx, "asks"), field(&page, cx, "more")), (has(""), has("")));
    keys(cx, "x");
    assert_eq!(typed(&page, cx), (String::new(), String::new(), String::new(), false), "the new field has no keys");
}

// The review of F54, finding 1: a note that stands in another's place for a while has no field, and the words
// typed under the first are there when it is back.
#[test]
fn a_keyed_field_keeps_its_words_while_no_note_has_it_and_under_another_note_with_its_key() {
    let mut cx = context();
    let (page, cx, entered, _) = open(&mut cx);
    show(&page, cx, vec![keyed("work", "k")]);
    press(&page, cx, PieceKind::Field, "work");
    keys(cx, "n e x t");
    // No note has the field: nothing is drawn of it, and no key is taken for it.
    show(&page, cx, vec![bare("busy")]);
    keys(cx, "x");
    let fields = page.read_with(cx, |page, _| page.placed().iter().filter(|p| p.piece == PieceKind::Field).count());
    assert_eq!((field(&page, cx, "work"), field(&page, cx, "busy"), fields), (None, None, 0));
    assert!(!typed(&page, cx).3, "a field that is not drawn has the keys");
    show(&page, cx, vec![keyed("work", "k")]);
    assert_eq!(field(&page, cx, "work"), has("next"));
    // Another note with the same key and offer has the same field; one with another key has its own.
    show(&page, cx, vec![keyed("stands", "k"), keyed("other", "j")]);
    assert_eq!((field(&page, cx, "stands"), field(&page, cx, "other")), (has("next"), has("")));
    // An answer names the note Enter was pressed under, and finds the field under the note that has it now.
    press(&page, cx, PieceKind::Field, "stands");
    keys(cx, "enter");
    assert_eq!(*entered.borrow(), [made("stands", "next")]);
    show(&page, cx, vec![keyed("work", "k")]);
    assert_eq!(field(&page, cx, "work"), Some(("next".to_string(), true)));
    answer(&page, cx, "work", true);
    assert_eq!(field(&page, cx, "work"), Some(("next".to_string(), true)), "an answer for another Enter was taken");
    answer(&page, cx, "stands", true);
    assert_eq!(field(&page, cx, "work"), has(""));
    // A field with no key goes with its note, words and all, as before.
    show(&page, cx, both());
    press(&page, cx, PieceKind::Field, "more");
    keys(cx, "g o n e");
    show(&page, cx, vec![bare("busy")]);
    show(&page, cx, both());
    assert_eq!(field(&page, cx, "more"), has(""));
}

// The review of F54, finding 5: a note may lose its field while the field waits for its answer.
#[test]
fn a_field_that_waits_is_kept_apart_while_its_note_has_no_field_and_a_refusal_leaves_its_words() {
    let mut cx = context();
    let (page, cx, entered, _) = open(&mut cx);
    press(&page, cx, PieceKind::Field, "asks");
    keys(cx, "o l d enter");
    show(&page, cx, vec![bare("asks")]);
    keys(cx, "x enter");
    assert_eq!((field(&page, cx, "asks"), entered.borrow().len()), (None, 1));
    assert!(!typed(&page, cx).3, "the keys are free while the note has no field");
    // Not kept: the words are there when the note has its field again, and the field takes keys.
    answer(&page, cx, "asks", false);
    show(&page, cx, both());
    assert_eq!(field(&page, cx, "asks"), has("old"));
    press(&page, cx, PieceKind::Field, "asks");
    keys(cx, "! enter");
    assert_eq!(entered.borrow().last(), Some(&made("asks", "old!")));
    // Kept, with no window to tell: the field that comes back is empty and does not wait.
    show(&page, cx, vec![bare("asks")]);
    page.update(cx, |page, cx| page.answer_field_closed("asks", true, cx));
    show(&page, cx, both());
    assert_eq!(field(&page, cx, "asks"), has(""));
    // Answered while it was apart and not shown again at the next scene, it is gone.
    press(&page, cx, PieceKind::Field, "asks");
    keys(cx, "l o s t enter");
    show(&page, cx, vec![bare("asks")]);
    answer(&page, cx, "asks", false);
    show(&page, cx, vec![bare("asks")]);
    show(&page, cx, both());
    assert_eq!(field(&page, cx, "asks"), has(""));
}

#[test]
fn at_most_eight_fields_are_kept_apart_and_the_oldest_goes_first() {
    let mut cx = context();
    let (page, cx, _, _) = open(&mut cx);
    let count = sticker_kit::draw::MAX_KEPT_FIELDS + 1;
    for n in 0..count {
        show(&page, cx, vec![keyed("work", &format!("k{n}"))]);
        press(&page, cx, PieceKind::Field, "work");
        keys(cx, "w");
    }
    show(&page, cx, vec![bare("busy")]);
    let kept = |n: usize, cx: &mut VisualTestContext| {
        show(&page, cx, vec![keyed("work", &format!("k{n}"))]);
        let words = field(&page, cx, "work");
        show(&page, cx, vec![bare("busy")]);
        words == has("w")
    };
    let kept: Vec<bool> = (0..count).map(|n| kept(n, cx)).collect();
    assert_eq!(kept, [false, true, true, true, true, true, true, true, true]);
}
