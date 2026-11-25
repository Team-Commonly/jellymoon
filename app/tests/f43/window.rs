//! The window's wiring with no window on a screen: GPUI's test platform opens the page `window.rs` makes, a click
//! and keys go through GPUI's own dispatch, and the work the window runs off its thread is run by the test's
//! executor. What is held here is what `showing.rs` cannot hold: that the window passes core's answer on to the
//! entry. On the Mac only, as the kit's draw tests are. No run is started: the day has no installed command.

use std::ops::Deref;

use gpui::{
    ClipboardItem, Entity, Keystroke, Modifiers, TestAppContext, TestDispatcher, VisualTestContext, point, px, size,
};
use jellymoon_app::showing::Showing;
use jellymoon_app::window::page;
use sticker_kit::draw::headless::text_system;
use sticker_kit::draw::{Page, load_fonts};
use sticker_kit::{Entry, PieceKind, Scene, Theme, Viewport};

use super::{DAY, wished_alone};
use crate::support::wished::SAID;

fn context() -> TestAppContext {
    TestAppContext::build_with_text_system(TestDispatcher::new(0), None, text_system())
}

fn opened(cx: &mut TestAppContext, shown: Showing, entry: Entry) -> (Entity<Page>, &mut VisualTestContext) {
    cx.update(|cx| load_fonts(cx)).expect("bundled fonts load");
    let viewport = Viewport { w: 1440.0, h: 900.0 };
    let window = cx.update(|cx| page(Scene::sample(), Theme::sticker(), viewport, shown, cx)).expect("a page");
    let view = window.root(cx).expect("the page is the window's root");
    let cx = VisualTestContext::from_window(*window.deref(), cx).into_mut();
    cx.update(|_, cx| view.update(cx, |page, cx| page.set_entry(Some(entry), cx)));
    cx.simulate_resize(size(px(viewport.w), px(viewport.h)));
    cx.run_until_parked();
    (view, cx)
}

/// The entry's words and whether it waits, once the window has done all it does.
fn entry(page: &Entity<Page>, cx: &mut VisualTestContext) -> (String, bool) {
    cx.run_until_parked();
    page.read_with(cx, |page, _| (page.entry().text().to_string(), page.entry().waiting()))
}

/// A click on the field, `words` pasted into it, and Enter.
fn enters(page: &Entity<Page>, cx: &mut VisualTestContext, words: &str) -> (String, bool) {
    let field = page.read_with(cx, |page, _| page.placed().iter().find(|p| p.piece == PieceKind::Entry).cloned());
    let (x, y) = field.expect("the entry is placed").rect.center();
    cx.simulate_click(point(px(x), px(y)), Modifiers::default());
    cx.write_to_clipboard(ClipboardItem::new_string(words.into()));
    cx.simulate_keystrokes("cmd-v enter");
    entry(page, cx)
}

/// One key, and nothing after it: what the window left for its other thread has not begun.
fn key(cx: &mut VisualTestContext, key: &'static str) {
    cx.update(|window, cx| window.dispatch_keystroke(Keystroke::parse(key).expect("a key"), cx));
}

// Typed ahead: a person presses Enter and types the next wish at once, before core has answered.
#[test]
fn what_is_typed_before_core_has_answered_is_all_in_the_entry_and_the_wish_is_made_with_its_own_words() {
    const FIRST: &str = "tidy the uploader";
    let (world, on) = wished_alone();
    let shown = Showing::of(Some(on));
    let wish = shown.offering().unwrap()().expect("alpha was typed in today");
    let mut cx = context();
    let (page, cx) = opened(&mut cx, shown, wish);
    let field = page.read_with(cx, |page, _| page.placed().iter().find(|p| p.piece == PieceKind::Entry).cloned());
    let (x, y) = field.expect("the entry is placed").rect.center();
    cx.simulate_click(point(px(x), px(y)), Modifiers::default());
    cx.write_to_clipboard(ClipboardItem::new_string(FIRST.into()));
    let held = |cx: &mut VisualTestContext| {
        page.read_with(cx, |page, _| (page.entry().text().to_string(), page.entry().waiting()))
    };
    "cmd-v enter n e x t space o n e".split(' ').for_each(|one| key(cx, one));
    assert_eq!(held(cx), ("next one".to_string(), true), "a key was dropped while core was asked");
    assert_eq!(entry(&page, cx), ("next one".to_string(), false), "core kept the wish: what was typed since stays");
    let said: Vec<String> = jellymoon_core::made(&world.home, DAY).into_iter().map(|wish| wish.said).collect();
    assert_eq!(said, [SAID, FIRST], "the wish was made with other words than its own");
}

// Entered ahead: the next wish is typed and its Enter pressed before core has answered the first. The kit's entry
// remembers that Enter, and the window's answer gives it to the window's own handler.
#[test]
fn an_enter_pressed_before_core_has_answered_is_not_lost_and_both_wishes_are_made_in_their_order() {
    const FIRST: &str = "tidy the uploader";
    let (world, on) = wished_alone();
    let shown = Showing::of(Some(on));
    let wish = shown.offering().unwrap()().expect("alpha was typed in today");
    let mut cx = context();
    let (page, cx) = opened(&mut cx, shown, wish);
    let field = page.read_with(cx, |page, _| page.placed().iter().find(|p| p.piece == PieceKind::Entry).cloned());
    let (x, y) = field.expect("the entry is placed").rect.center();
    cx.simulate_click(point(px(x), px(y)), Modifiers::default());
    cx.write_to_clipboard(ClipboardItem::new_string(FIRST.into()));
    "cmd-v enter n e x t space o n e enter".split(' ').for_each(|one| key(cx, one));
    let held = page.read_with(cx, |page, _| (page.entry().text().to_string(), page.entry().waiting()));
    assert_eq!(held, ("next one".to_string(), true), "the second Enter was given before core had answered");
    assert_eq!(entry(&page, cx), (String::new(), false), "both were kept, and the entry is answered for both");
    let said: Vec<String> = jellymoon_core::made(&world.home, DAY).into_iter().map(|wish| wish.said).collect();
    assert_eq!(said, [SAID, FIRST, "next one"], "the second wish has other words than its Enter was pressed on");
}

#[test]
fn the_window_answers_an_enter_on_a_page_with_no_day_and_the_words_stay() {
    let mut cx = context();
    let wish = Entry {
        id: "wish".into(),
        hint: "wish".into(),
        options: vec!["alpha".into()],
        most: 200,
        action: String::new(),
        go: String::new(),
        loud: false,
    };
    let (page, cx) = opened(&mut cx, Showing::of(None), wish);
    assert_eq!(enters(&page, cx, "tea at four"), ("tea at four".to_string(), false), "answered, and not kept");
}

#[test]
fn the_window_tells_the_entry_what_core_said_of_the_wish() {
    let (_world, on) = wished_alone();
    let shown = Showing::of(Some(on));
    let mut wish = shown.offering().unwrap()().expect("alpha was typed in today");
    assert_eq!(wish.options.first().map(String::as_str), Some("alpha"), "{wish:?}");
    // A chip for a project the day does not have: core refuses a wish for it.
    wish.options.insert(0, "nowhere".into());
    let mut cx = context();
    let (page, cx) = opened(&mut cx, shown, wish);
    assert_eq!(enters(&page, cx, SAID), (SAID.to_string(), false), "core refused the wish: its words stay");
    cx.simulate_keystrokes("tab enter");
    assert_eq!(entry(&page, cx), (String::new(), false), "core kept the wish: the entry is empty");
}
