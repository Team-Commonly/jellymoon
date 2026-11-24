//! The page's window on GPUI's test platform, with no window on a screen, for F64's and F66's tests: the view
//! `window.rs` makes, its first entry the one core offers, and a drop, a click and keys played through GPUI's own
//! dispatch. On the Mac only, as the kit's draw tests are.

use std::ops::Deref;
use std::path::PathBuf;

use gpui::{
    ClipboardItem, Entity, ExternalPaths, FileDropEvent, Modifiers, TestAppContext, TestDispatcher, VisualTestContext,
    point, px, size,
};
use jellymoon_app::showing::Showing;
use jellymoon_app::window::page;
use sticker_kit::draw::headless::text_system;
use sticker_kit::draw::{Page, load_fonts};
use sticker_kit::{PieceKind, Scene, Theme, Viewport};

pub fn context() -> TestAppContext {
    TestAppContext::build_with_text_system(TestDispatcher::new(0), None, text_system())
}

/// The page's window on `shown`, with the entry core offers it, as the window's first work gives it.
pub fn opened(cx: &mut TestAppContext, shown: Showing) -> (Entity<Page>, &mut VisualTestContext) {
    cx.update(|cx| load_fonts(cx)).expect("bundled fonts load");
    let entry = shown.offering().expect("a day")();
    let viewport = Viewport { w: 1440.0, h: 900.0 };
    let scene = shown.scene().unwrap_or_else(Scene::sample);
    let window = cx.update(|cx| page(scene, Theme::sticker(), viewport, shown, cx)).expect("a page");
    let view = window.root(cx).expect("the page is the window's root");
    let cx = VisualTestContext::from_window(*window.deref(), cx).into_mut();
    // The window is still: a card that opens moves every card under it, and a page that moves them there over a
    // moment takes a tap on a piece where it is drawn, not yet where the layout has it. A click here is on the
    // layout's place.
    cx.update(|_, cx| {
        view.update(cx, |page, cx| {
            page.set_reduce_motion(true, cx);
            page.set_entry(entry, cx);
        });
    });
    cx.simulate_resize(size(px(viewport.w), px(viewport.h)));
    cx.run_until_parked();
    (view, cx)
}

/// The middle of the first placed piece of `kind`.
fn middle(page: &Entity<Page>, cx: &mut VisualTestContext, kind: PieceKind) -> (f32, f32) {
    let is = |piece: &&sticker_kit::Placed| piece.piece == kind;
    let piece = page.read_with(cx, |page, _| page.placed().iter().find(is).map(|piece| page.shown_at(piece)));
    piece.unwrap_or_else(|| panic!("no {kind:?} is placed")).center()
}

/// A click on the piece of `kind`.
pub fn click(page: &Entity<Page>, cx: &mut VisualTestContext, kind: PieceKind) {
    let (x, y) = middle(page, cx, kind);
    cx.simulate_click(point(px(x), px(y)), Modifiers::default());
    cx.run_until_parked();
}

/// A click on the field and `words` pasted into it, and Enter when `enter`.
pub fn types(page: &Entity<Page>, cx: &mut VisualTestContext, words: &str, enter: bool) {
    click(page, cx, PieceKind::Entry);
    cx.write_to_clipboard(ClipboardItem::new_string(words.into()));
    cx.simulate_keystrokes(if enter { "cmd-v enter" } else { "cmd-v" });
    cx.run_until_parked();
}

/// `folder` dragged from outside the app and dropped on the page, at its bottom left corner, where no piece is.
pub fn drop_on(cx: &mut VisualTestContext, folder: PathBuf) {
    let position = point(px(10.0), px(890.0));
    cx.simulate_event(FileDropEvent::Entered { position, paths: ExternalPaths([folder].into_iter().collect()) });
    cx.simulate_event(FileDropEvent::Submit { position });
    cx.simulate_event(FileDropEvent::Exited);
    cx.run_until_parked();
}

/// The entry as the window holds it: its id, its options, the chosen one, its words, and whether it has the keys.
pub fn held(page: &Entity<Page>, cx: &mut VisualTestContext) -> (String, Vec<String>, Option<usize>, String, bool) {
    cx.run_until_parked();
    let (id, options, chosen, text) = page.read_with(cx, |page, _| {
        let entry = page.placed().iter().find(|piece| piece.piece == PieceKind::Entry).map(|piece| piece.id.clone());
        let chips = page.placed().iter().filter(|piece| piece.piece == PieceKind::Chip);
        let options = chips.map(|chip| chip.text.iter().map(|run| run.text.clone()).collect()).collect();
        (entry.unwrap_or_default(), options, page.entry().chosen(), page.entry().text().to_string())
    });
    let keys = cx.update(|window, cx| page.read(cx).entry_focused(window));
    (id, options, chosen, text, keys)
}
