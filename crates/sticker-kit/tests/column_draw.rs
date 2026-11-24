//! The column in the view, headless: no window is shown. A turn of the wheel moves the column and never the entry,
//! the column is shown over the entry's band only, a page opens at the column's end and stays there while it grows,
//! and a click on what a person typed is a tap on the character it is of. A click means what was seen: a choice
//! that has just moved in the column, or has just come where another piece took a tap, takes none for `SETTLE`.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::cell::RefCell;
use std::rc::Rc;

use gpui::{
    Entity, Modifiers, ScrollDelta, ScrollEvent, TestAppContext, TestDispatcher, VisualTestContext, point, px, size,
};
use sticker_kit::draw::headless::text_system;
use sticker_kit::draw::{Page, SETTLE, Tap, load_fonts};
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

/// A day that fits its window, so that nothing of it is moved by the column holding its end: these cards, each
/// with one line, under the header alone.
fn small(cards: &[usize]) -> Scene {
    let short = |n: &usize| match card(*n) {
        Note::Text { id, title, options, quoted, field, .. } => {
            Note::Text { id, title, body: vec!["One line.".into()], options, quoted, field }
        }
        other => other,
    };
    Scene { characters: Vec::new(), notes: cards.iter().map(short).collect(), ..long(0) }
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

/// The tap on the second option of the card `id`, with its words as the scene holds them.
fn no(id: &str) -> Tap {
    Tap::Option { note_id: id.into(), index: 1, words: "No".into() }
}

/// The page's time for what settles: a test cannot wait for a clock.
fn at_time(page: &Entity<Page>, cx: &mut VisualTestContext, seconds: f32) {
    page.update(cx, |page, _| page.set_settle_time(Some(seconds)));
}

fn show(page: &Entity<Page>, cx: &mut VisualTestContext, scene: Scene) {
    page.update(cx, |page, cx| page.set_scene(scene, cx));
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
    assert_eq!(*taps.borrow(), [no("card-5")], "the newest card is in reach");

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
    let tap = |index: usize| {
        let words = if index == 1 { "Toss it".into() } else { String::new() };
        Tap::Option { note_id: "work-garden".into(), index, words }
    };
    assert_eq!(*taps.borrow(), [tap(1), tap(PICTURE_PLACE), tap(PICTURE_PLACE)], "the picture's place is no option's");
}

#[test]
fn a_choice_a_card_pushed_down_the_column_takes_no_tap_until_it_has_settled() {
    let mut cx = context();
    let (page, cx, taps) = open(&mut cx, small(&[0]));
    let was = shown(&page, cx, PieceKind::Option, "card-0", 1);
    at_time(&page, cx, 10.0);
    // An older wish's result comes above the card: the choice is somewhere else under the pointer.
    show(&page, cx, small(&[7, 0]));
    assert_eq!(scrolled(&page, cx), (0.0, 0.0), "the column still fits");
    let now = shown(&page, cx, PieceKind::Option, "card-0", 1);
    assert!(now.y > was.y + 40.0 && now.bottom() <= entry_top(&Theme::sticker(), SIZE), "{was:?} {now:?}");
    click(cx, now.center());
    at_time(&page, cx, 10.0 + SETTLE.as_secs_f32() - 0.05);
    click(cx, now.center());
    assert_eq!(*taps.borrow(), [], "a choice that has just moved took a tap");
    at_time(&page, cx, 10.0 + SETTLE.as_secs_f32() + 0.05);
    click(cx, now.center());
    assert_eq!(*taps.borrow(), [no("card-0")], "settled, it takes the tap, with the words that were drawn");
    // The click that was no tap was not kept for later: one tap, not three.
    cx.run_until_parked();
    assert_eq!(taps.borrow().len(), 1);
}

#[test]
fn a_turn_of_the_wheel_moves_a_choice_in_the_window_and_it_takes_a_tap_at_once() {
    let mut cx = context();
    let (page, cx, taps) = open(&mut cx, long(6));
    at_time(&page, cx, 10.0);
    let was = shown(&page, cx, PieceKind::Option, "card-4", 1);
    wheel(cx, 120.0);
    let now = shown(&page, cx, PieceKind::Option, "card-4", 1);
    assert!(now.y == was.y + 120.0 && now.bottom() <= entry_top(&Theme::sticker(), SIZE), "{was:?} {now:?}");
    click(cx, now.center());
    assert_eq!(*taps.borrow(), [no("card-4")], "the wheel is the person's own move");
    // The same scene given again, as a look gives it: the choice is where it was in the column.
    show(&page, cx, long(6));
    click(cx, now.center());
    assert_eq!(*taps.borrow(), [no("card-4"), no("card-4")], "a place in the window is not what is kept");
}

#[test]
fn a_card_at_the_end_slides_an_older_choice_up_the_window_and_it_takes_no_tap_until_it_has_settled() {
    let mut cx = context();
    let (page, cx, taps) = open(&mut cx, long(6));
    let was = shown(&page, cx, PieceKind::Option, "card-5", 1);
    at_time(&page, cx, 10.0);
    // The newest wish's result comes at the end, and the reader is there: every older card goes up the window.
    show(&page, cx, long(7));
    let now = shown(&page, cx, PieceKind::Option, "card-5", 1);
    assert!(now.y < was.y - 40.0 && now.y > 0.0, "{was:?} {now:?}");
    click(cx, now.center());
    assert_eq!(*taps.borrow(), [], "a choice that slid up under the pointer took a tap");
    at_time(&page, cx, 10.0 + SETTLE.as_secs_f32() + 0.05);
    click(cx, now.center());
    assert_eq!(*taps.borrow(), [no("card-5")], "settled, it takes the tap");
}

#[test]
fn a_card_that_comes_above_a_column_held_at_its_end_leaves_a_choice_in_its_place_and_it_takes_a_tap_at_once() {
    let mut cx = context();
    let (page, cx, taps) = open(&mut cx, long(6));
    let was = shown(&page, cx, PieceKind::Option, "card-5", 1);
    at_time(&page, cx, 10.0);
    let mut above = long(6);
    above.notes.insert(2, card(9));
    show(&page, cx, above);
    let now = shown(&page, cx, PieceKind::Option, "card-5", 1);
    assert!((now.y - was.y).abs() < 1.0, "{was:?} {now:?}");
    click(cx, now.center());
    assert_eq!(*taps.borrow(), [no("card-5")], "it did not move under the pointer");
}

#[test]
fn a_card_that_comes_at_the_column_s_end_where_nothing_took_a_tap_takes_one_at_once() {
    let mut cx = context();
    let (page, cx, taps) = open(&mut cx, small(&[0]));
    at_time(&page, cx, 10.0);
    show(&page, cx, small(&[0, 1]));
    assert_eq!(scrolled(&page, cx), (0.0, 0.0), "the column still fits: nothing of it moved in the window");
    let option = shown(&page, cx, PieceKind::Option, "card-1", 1);
    click(cx, option.center());
    assert_eq!(*taps.borrow(), [no("card-1")], "a new card is not dead");
}

#[test]
fn a_card_that_comes_under_the_pointer_where_another_choice_stood_takes_no_tap_until_it_has_settled() {
    let mut cx = context();
    let (page, cx, taps) = open(&mut cx, small(&[0]));
    let was = shown(&page, cx, PieceKind::Option, "card-0", 1);
    at_time(&page, cx, 10.0);
    show(&page, cx, small(&[9]));
    let now = shown(&page, cx, PieceKind::Option, "card-9", 1);
    assert_eq!(now, was, "the new card's choice is where the other card's stood");
    click(cx, now.center());
    assert_eq!(*taps.borrow(), [], "a choice that came under the pointer took a tap");
    at_time(&page, cx, 10.0 + SETTLE.as_secs_f32() + 0.05);
    click(cx, now.center());
    assert_eq!(*taps.borrow(), [no("card-9")]);
}
