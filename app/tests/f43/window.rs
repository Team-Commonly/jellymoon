//! The window's wiring with no window on a screen: GPUI's test platform opens the page `window.rs` makes, a click
//! and keys go through GPUI's own dispatch, and the work the window runs off its thread is run by the test's
//! executor. What is held here is what `showing.rs` cannot hold: that the window passes core's answer on to the
//! entry. On the Mac only, as the kit's draw tests are. No run is started: the day has no installed command.

use std::ops::Deref;

use gpui::{ClipboardItem, Entity, Modifiers, TestAppContext, TestDispatcher, VisualTestContext, point, px, size};
use jellymoon_app::showing::Showing;
use jellymoon_app::window::page;
use sticker_kit::draw::headless::text_system;
use sticker_kit::draw::{Page, load_fonts};
use sticker_kit::{Entry, PieceKind, Scene, Theme, Viewport};

use super::wished_alone;
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

#[test]
fn the_window_answers_an_enter_on_a_page_with_no_day_and_the_words_stay() {
    let mut cx = context();
    let wish = Entry { id: "wish".into(), hint: "wish".into(), options: vec!["alpha".into()], most: 200 };
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
