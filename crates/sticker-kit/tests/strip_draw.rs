//! A strip in the view, headless: no window is shown. A click anywhere on it is no tap, while the picture of
//! another drawn note on the same page takes its own.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::cell::RefCell;
use std::rc::Rc;

use gpui::{Modifiers, TestAppContext, TestDispatcher, point, px, size};
use sticker_kit::draw::headless::text_system;
use sticker_kit::draw::{Page, Tap, load_fonts};
use sticker_kit::{Kept, Note, PICTURE_PLACE, Panel, Picture, PieceKind, Scene, Shows, Theme, Viewport};

const SIZE: Viewport = Viewport { w: 1440.0, h: 900.0 };

fn drawn(id: &str, picture: Picture) -> Note {
    let words = vec!["One line under it.".into()];
    Note::Drawn { id: id.into(), picture, words, open: true, options: Vec::new(), field: None }
}

#[test]
fn a_click_on_a_strip_is_no_tap_and_one_on_another_note_s_picture_is_that_note_s() {
    let panel = |shows, caption: &str| Panel { shows, caption: caption.into() };
    let panels = vec![panel(Shows::Speech, "ask"), panel(Shows::Sheet, "made"), panel(Shows::Folder, "kept")];
    let sheets = Picture::Sheets { made: 1, changed: 0, removed: 0, state: Kept::In };
    let notes = vec![drawn("how", Picture::Strip { panels }), drawn("work", sheets)];
    let scene = Scene { notes, characters: Vec::new(), ..Scene::sample() };
    let mut cx = TestAppContext::build_with_text_system(TestDispatcher::new(0), None, text_system());
    cx.update(|cx| load_fonts(cx)).expect("bundled fonts load");
    let taps: Rc<RefCell<Vec<Tap>>> = Rc::default();
    let seen = taps.clone();
    let (page, cx) = cx.add_window_view(move |_, _| {
        let page = Page::new(scene, Theme::sticker()).reduce_motion(true);
        page.on_tap(move |tap, _, _| seen.borrow_mut().push(tap.clone()))
    });
    cx.simulate_resize(size(px(SIZE.w), px(SIZE.h)));
    cx.run_until_parked();
    let of = |id: &str, kind: PieceKind| {
        let find = |page: &Page| {
            let own = page.placed().iter().filter(|piece| piece.id == id && piece.piece == kind);
            own.map(|piece| page.shown_at(piece)).collect::<Vec<_>>()
        };
        page.read_with(cx, |page, _| find(page))
    };
    assert!(of("how", PieceKind::Ask).is_empty() && of("work", PieceKind::Ask).len() == 1, "no mark on a strip");
    let strip = of("how", PieceKind::Picture)[0];
    // Each sketch, each arrow and each caption of it: a click at fifteen places down and across the picture.
    let places = (1..=15).map(|n| (strip.x + strip.w * n as f32 / 16.0, strip.y + strip.h * (1 + n % 3) as f32 / 4.0));
    let other = of("work", PieceKind::Picture)[0].center();
    for at in places.chain([other]) {
        cx.simulate_click(point(px(at.0), px(at.1)), Modifiers::default());
        cx.run_until_parked();
    }
    let work = Tap::Option { note_id: "work".into(), index: PICTURE_PLACE, words: String::new() };
    assert_eq!(*taps.borrow(), [work], "the strip took none of its fifteen clicks");
}
