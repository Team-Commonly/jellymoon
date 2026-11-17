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
    let field = Some(Field { offer: offer.into(), hint: "Type here".into(), most: 12 });
    let (title, body) = ("A row".into(), vec!["what the note says".into()]);
    Note::Text { id: id.into(), title, body, options: Vec::new(), quoted: quoted.into(), field }
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
