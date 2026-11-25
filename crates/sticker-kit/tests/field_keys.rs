//! A field that asks for the keys (`Field::wants_keys`), headless: no window is shown, and keys and clicks go
//! through GPUI's own dispatch, as in `tests/field_draw.rs`. It takes the keys once, when it first stands on the
//! page, and never out of typed words; a field that does not ask never takes them.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use gpui::{Entity, Modifiers, TestAppContext, TestDispatcher, VisualTestContext, point, px, size};
use sticker_kit::draw::headless::text_system;
use sticker_kit::draw::{Page, load_fonts};
use sticker_kit::{Entry, Field, Note, PieceKind, Scene, Theme};

/// A note with a field under the question it quotes: `asks` says whether the field asks for the keys.
fn note(id: &str, asks: bool) -> Note {
    let field = Field { hint: "Type here".into(), most: 40, wants_keys: asks, ..Field::default() };
    let (title, quoted) = ("A row".into(), "Which of the two?".into());
    Note::Text { id: id.into(), title, body: vec![], options: vec![], quoted, field: Some(field) }
}

fn open(cx: &mut TestAppContext, notes: Vec<Note>) -> (Entity<Page>, &mut VisualTestContext) {
    cx.update(|cx| load_fonts(cx)).expect("bundled fonts load");
    let entry = Entry { id: "wish".into(), hint: "wish for something".into(), most: 40, ..Entry::default() };
    let scene = Scene { notes, ..Scene::sample() };
    let (page, cx) =
        cx.add_window_view(move |_, _| Page::new(scene, Theme::sticker()).reduce_motion(true).with_entry(entry));
    cx.simulate_resize(size(px(1440.0), px(900.0)));
    cx.run_until_parked();
    (page, cx)
}

fn context() -> TestAppContext {
    TestAppContext::build_with_text_system(TestDispatcher::new(0), None, text_system())
}

fn show(page: &Entity<Page>, cx: &mut VisualTestContext, notes: Vec<Note>) {
    page.update(cx, |page, cx| page.set_scene(Scene { notes, ..Scene::sample() }, cx));
    cx.run_until_parked();
}

fn keys(cx: &mut VisualTestContext, keystrokes: &str) {
    cx.simulate_keystrokes(keystrokes);
    cx.run_until_parked();
}

/// A click on the entry, or on the field of the note `id`.
fn click(page: &Entity<Page>, cx: &mut VisualTestContext, id: Option<&str>) {
    let is = |piece: &&sticker_kit::Placed| match id {
        Some(id) => piece.piece == PieceKind::Field && piece.id == id,
        None => piece.piece == PieceKind::Entry,
    };
    let at = page.read_with(cx, |page, _| page.placed().iter().find(is).map(|piece| page.shown_at(piece).center()));
    let (x, y) = at.expect("the piece is placed");
    cx.simulate_click(point(px(x), px(y)), Modifiers::default());
    cx.run_until_parked();
}

/// What is typed in the entry, in the field of `asks` and in the field of `more`.
fn typed(page: &Entity<Page>, cx: &mut VisualTestContext) -> (String, String, String) {
    page.read_with(cx, |page, _| {
        let field = |id: &str| page.field(id).map(|field| field.text().to_string()).unwrap_or_default();
        (page.entry().text().to_string(), field("asks"), field("more"))
    })
}

fn held(entry: &str, asks: &str, more: &str) -> (String, String, String) {
    (entry.into(), asks.into(), more.into())
}

#[test]
fn a_field_that_asks_has_the_keys_when_it_first_stands_and_one_that_does_not_ask_never_takes_them() {
    let mut cx = context();
    // From the first frame, and on a page that was already drawn.
    let (page, cx) = open(&mut cx, vec![note("asks", true)]);
    keys(cx, "o l d");
    assert_eq!(typed(&page, cx), held("", "old", ""), "typed with no click");
    assert!(cx.update(|window, cx| page.read(cx).entry_focused(window)));
    let mut cx = context();
    let (page, cx) = open(&mut cx, vec![]);
    show(&page, cx, vec![note("asks", true)]);
    keys(cx, "n e w");
    assert_eq!(typed(&page, cx), held("", "new", ""));

    let mut cx = context();
    let (page, cx) = open(&mut cx, vec![note("more", false)]);
    keys(cx, "a");
    show(&page, cx, vec![]);
    show(&page, cx, vec![note("more", false)]);
    keys(cx, "b");
    assert_eq!(typed(&page, cx), held("ab", "", ""), "a field that does not ask took the keys");
}

#[test]
fn words_typed_in_the_entry_or_where_the_keys_are_keep_the_keys_there() {
    let mut cx = context();
    let (page, cx) = open(&mut cx, vec![]);
    keys(cx, "n e x t");
    show(&page, cx, vec![note("asks", true)]);
    keys(cx, "space o n e");
    assert_eq!(typed(&page, cx), held("next one", "", ""), "the keys were taken out of the entry's words");
    // It had its turn: an entry that is empty again does not lose the keys to it at a later drawing.
    keys(cx, &["backspace"; 8].join(" "));
    show(&page, cx, vec![note("asks", true), note("more", false)]);
    keys(cx, "x");
    assert_eq!(typed(&page, cx), held("x", "", ""));

    // Another field has the keys, with words in it.
    let mut cx = context();
    let (page, cx) = open(&mut cx, vec![note("more", false)]);
    click(&page, cx, Some("more"));
    keys(cx, "m");
    show(&page, cx, vec![note("asks", true), note("more", false)]);
    keys(cx, "n");
    assert_eq!(typed(&page, cx), held("", "", "mn"), "the keys were taken out of another field's words");
}

#[test]
fn it_takes_the_keys_once_and_a_click_or_escape_moves_them_for_good() {
    let mut cx = context();
    let (page, cx) = open(&mut cx, vec![note("asks", true)]);
    keys(cx, "a");
    // The entry is empty when it is clicked, and at every drawing after.
    click(&page, cx, None);
    show(&page, cx, vec![note("asks", true)]);
    page.update(cx, |_, cx| cx.notify());
    cx.run_until_parked();
    keys(cx, "b");
    assert_eq!(typed(&page, cx), held("b", "a", ""), "the field took the keys back from the entry");
    // A click on the field gives it the keys as ever, and Escape gives them to the entry, where they stay.
    click(&page, cx, Some("asks"));
    keys(cx, "c escape backspace");
    show(&page, cx, vec![note("asks", true), note("more", false)]);
    keys(cx, "d");
    assert_eq!(typed(&page, cx), held("d", "ac", ""), "the field took the keys back after Escape");
}

#[test]
fn when_the_asking_note_is_gone_the_next_letters_are_in_the_entry_and_a_new_one_asks_again() {
    let mut cx = context();
    let (page, cx) = open(&mut cx, vec![note("asks", true)]);
    keys(cx, "a");
    show(&page, cx, vec![]);
    keys(cx, "b");
    assert_eq!(typed(&page, cx), held("b", "", ""), "the keys went nowhere when the note left");
    keys(cx, "backspace");
    // Another question is another field: it stands for the first time.
    show(&page, cx, vec![note("asks", true)]);
    keys(cx, "c");
    assert_eq!(typed(&page, cx), held("", "c", ""));
}
