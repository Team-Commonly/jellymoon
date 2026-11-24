//! The column in the view, headless: no window is shown. A turn of the wheel moves the column and never the entry,
//! the column is shown over the entry's band only, a page opens at the column's end and stays there while it grows,
//! and a click on what a person typed is a tap on the character it is of.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::cell::RefCell;
use std::rc::Rc;

use gpui::{
    Entity, Modifiers, ScrollDelta, ScrollEvent, TestAppContext, TestDispatcher, VisualTestContext, point, px, size,
};
use sticker_kit::draw::headless::text_system;
use sticker_kit::draw::{Page, Tap, load_fonts};
use sticker_kit::{
    Entry, Kept, Note, PICTURE_PLACE, Picture, PieceKind, Placed, Rect, Scene, Theme, Viewport, entry_top, scroll_most,
};

type Taps = Rc<RefCell<Vec<Tap>>>;

const SIZE: Viewport = Viewport { w: 1440.0, h: 900.0 };

fn card(n: usize) -> Note {
    Note::Text {
        id: format!("card-{n}"),
        title: format!("Card {n}"),
        body: vec!["What came of it, in a line or two of plain words.".into(); 3],
        options: vec!["Yes".into(), "No".into()],
        quoted: String::new(),
        field: None,
    }
}

/// A day longer than its window: a note of the day, then the words typed for `garden` and `count` cards of it.
fn long(count: usize) -> Scene {
    let mut scene = Scene::sample();
    scene.characters.truncate(4);
    scene.notes = vec![Note::Plain { title: "Of the day".into(), body: "A note of no wish.".into() }];
    scene.notes.push(Note::Said { id: "typed-garden".into(), text: "plant the herbs by the door".into() });
    scene.notes.extend((0..count).map(card));
    scene
}

fn open(cx: &mut TestAppContext, scene: Scene) -> (Entity<Page>, &mut VisualTestContext, Taps) {
    cx.update(|cx| load_fonts(cx)).expect("bundled fonts load");
    let taps = Taps::default();
    let seen = taps.clone();
    let entry = Entry { id: "wish".into(), hint: "wish for something".into(), ..Entry::default() };
    let (page, cx) = cx.add_window_view(move |_, _| {
        Page::new(scene, Theme::sticker())
            .reduce_motion(true)
            .with_entry(entry)
            .on_tap(move |tap, _, _| seen.borrow_mut().push(tap.clone()))
    });
    cx.simulate_resize(size(px(SIZE.w), px(SIZE.h)));
    cx.run_until_parked();
    (page, cx, taps)
}

fn context() -> TestAppContext {
    TestAppContext::build_with_text_system(TestDispatcher::new(0), None, text_system())
}

/// Where a piece is in the window now.
fn shown(page: &Entity<Page>, cx: &mut VisualTestContext, kind: PieceKind, id: &str, index: usize) -> Rect {
    let find = |p: &&Placed| p.piece == kind && p.id == id && p.index == index;
    let found = page.read_with(cx, |page, _| page.placed().iter().find(find).map(|piece| page.shown_at(piece)));
    found.unwrap_or_else(|| panic!("no {kind:?} of {id} is placed"))
}

fn scrolled(page: &Entity<Page>, cx: &mut VisualTestContext) -> (f32, f32) {
    page.read_with(cx, |page, _| (page.scrolled(), scroll_most(page.placed(), page.theme(), SIZE)))
}

fn click(cx: &mut VisualTestContext, at: (f32, f32)) {
    cx.simulate_click(point(px(at.0), px(at.1)), Modifiers::default());
    cx.run_until_parked();
}

/// A turn of the wheel over the page, `dy` logical pixels: down the window when positive.
fn wheel(cx: &mut VisualTestContext, dy: f32) {
    let delta = ScrollDelta::Pixels(point(px(0.0), px(dy)));
    cx.simulate_event(ScrollEvent { position: point(px(700.0), px(300.0)), delta, ..Default::default() });
    cx.run_until_parked();
}

#[test]
fn a_page_opens_at_its_column_s_end_and_the_wheel_moves_the_column_and_never_the_entry() {
    let mut cx = context();
    let (page, cx, taps) = open(&mut cx, long(6));
    let (at, most) = scrolled(&page, cx);
    let band = entry_top(&Theme::sticker(), SIZE);
    assert!(most > 200.0 && at == most, "longer than its window, and opened at its end: {at} of {most}");
    // The last card is over the entry's band, and the entry is in it.
    let last = shown(&page, cx, PieceKind::Note, "card-5", 7);
    let field = shown(&page, cx, PieceKind::Entry, "wish", 0);
    assert!(last.bottom() <= band && field.y >= band && field.bottom() <= SIZE.h, "{last:?} {band} {field:?}");
    let option = shown(&page, cx, PieceKind::Option, "card-5", 1);
    click(cx, option.center());
    assert_eq!(*taps.borrow(), [Tap::Option { note_id: "card-5".into(), index: 1 }], "the newest card is in reach");

    // The wheel shows what is higher: the column moves by the turn, and the entry stays.
    wheel(cx, 200.0);
    assert_eq!(scrolled(&page, cx).0, most - 200.0);
    assert_eq!(shown(&page, cx, PieceKind::Note, "card-5", 7).y, last.y + 200.0);
    assert_eq!(shown(&page, cx, PieceKind::Entry, "wish", 0), field, "the entry is the window's");
    // A card moved under the band's top is not there to tap: a click on where it would be is the entry's, or nobody's.
    let hidden = shown(&page, cx, PieceKind::Option, "card-5", 1);
    assert!(hidden.y >= band, "the option is under the band now: {hidden:?}");
    click(cx, hidden.center());
    assert_eq!(taps.borrow().len(), 1, "a card under the entry took a tap");
    // To the top and no further; to the end and no further.
    wheel(cx, 1e7);
    assert_eq!(scrolled(&page, cx).0, 0.0);
    let who = shown(&page, cx, PieceKind::Character, "kite", 3);
    click(cx, who.center());
    assert_eq!(taps.borrow().last(), Some(&Tap::Character { id: "kite".into() }), "a row at the column's top");
    wheel(cx, -1e7);
    assert_eq!(scrolled(&page, cx).0, most);
}

#[test]
fn a_column_that_grows_stays_at_its_end_for_a_reader_who_is_there_and_where_it_was_for_one_who_is_not() {
    let mut cx = context();
    let (page, cx, _) = open(&mut cx, long(4));
    let (at, most) = scrolled(&page, cx);
    assert_eq!(at, most);
    page.update(cx, |page, cx| page.set_scene(long(6), cx));
    cx.run_until_parked();
    let (at, longer) = scrolled(&page, cx);
    assert!(longer > most && at == longer, "the new cards are under the eyes: {at} of {longer}");
    // Reading higher up: the next card moves nothing.
    wheel(cx, 300.0);
    page.update(cx, |page, cx| page.set_scene(long(8), cx));
    cx.run_until_parked();
    let (at, longest) = scrolled(&page, cx);
    assert!(longest > longer && at == longer - 300.0, "{at} of {longest}");
    // A column that fits its window again is not left moved.
    page.update(cx, |page, cx| page.set_scene(Scene { characters: Vec::new(), notes: Vec::new(), ..long(0) }, cx));
    cx.run_until_parked();
    assert_eq!(scrolled(&page, cx), (0.0, 0.0));
}

#[test]
fn a_click_on_what_a_person_typed_is_a_tap_on_the_character_it_is_of() {
    let mut cx = context();
    let (page, cx, taps) = open(&mut cx, long(1));
    wheel(cx, 1e7);
    let bubble = shown(&page, cx, PieceKind::Quote, "typed-garden", 0);
    click(cx, bubble.center());
    let who = shown(&page, cx, PieceKind::Character, "garden", 0);
    assert!(who.x >= bubble.right(), "the character stands at the bubble's end: {bubble:?} {who:?}");
    click(cx, who.center());
    let garden = Tap::Character { id: "garden".into() };
    assert_eq!(*taps.borrow(), [garden.clone(), garden], "the bubble, then the character: the same tap");
    // Someone else's words under a note are a quote too, and take no tap: only what a person typed does.
    let mut scene = long(1);
    let Some(Note::Text { quoted, .. }) = scene.notes.last_mut() else { panic!("a text note") };
    *quoted = "words of the run".into();
    scene.notes.push(Note::Said { id: "typed-nobody".into(), text: "of no character".into() });
    page.update(cx, |page, cx| page.set_scene(scene, cx));
    cx.run_until_parked();
    wheel(cx, -1e7);
    for id in ["card-0", "typed-nobody"] {
        let band = entry_top(&Theme::sticker(), SIZE);
        assert!(shown(&page, cx, PieceKind::Quote, id, 0).bottom() <= band, "{id} is in view");
        let quote = shown(&page, cx, PieceKind::Quote, id, 0);
        click(cx, quote.center());
    }
    assert_eq!(taps.borrow().len(), 2, "a quote that is no character's took a tap");
    // In a column moved to its end, the character of the last words typed takes the tap where it is drawn.
    let mut scene = long(8);
    let last = scene.characters[1].id.clone();
    scene.notes.push(Note::Said { id: format!("typed-{last}"), text: "and water them".into() });
    page.update(cx, |page, cx| page.set_scene(scene, cx));
    cx.run_until_parked();
    wheel(cx, -1e7);
    assert!(scrolled(&page, cx).0 > 200.0, "the column is moved");
    let who = shown(&page, cx, PieceKind::Character, &last, 1);
    click(cx, who.center());
    assert_eq!(taps.borrow().last(), Some(&Tap::Character { id: last }), "a tap in a moved column");
    assert_eq!(taps.borrow().len(), 3);
}

#[test]
fn a_tap_on_a_drawn_note_s_option_is_a_text_note_s_and_one_on_its_picture_or_its_mark_is_the_note_s_own() {
    let mut cx = context();
    let mut scene = long(0);
    scene.notes.push(Note::Drawn {
        id: "work-garden".into(),
        picture: Picture::Sheets { made: 1, changed: 0, removed: 0, state: Kept::Waiting },
        words: vec!["Your folder is not changed yet.".into()],
        open: false,
        options: vec!["Keep it".into(), "Toss it".into()],
        field: None,
    });
    let (page, cx, taps) = open(&mut cx, scene);
    for (kind, index) in [(PieceKind::Option, 1), (PieceKind::Picture, 0), (PieceKind::Ask, 0)] {
        let at = shown(&page, cx, kind, "work-garden", index);
        click(cx, at.center());
    }
    let tap = |index: usize| Tap::Option { note_id: "work-garden".into(), index };
    assert_eq!(*taps.borrow(), [tap(1), tap(PICTURE_PLACE), tap(PICTURE_PLACE)], "the picture's place is no option's");
}
