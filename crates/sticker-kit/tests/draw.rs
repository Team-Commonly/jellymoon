//! The drawing half, headless: no window is shown.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::cell::RefCell;
use std::rc::Rc;

use gpui::{Modifiers, TestAppContext, TestDispatcher, VisualTestContext, point, px, size};
use sticker_kit::draw::headless::text_system;
use sticker_kit::draw::{GlyphMeasure, Page, Tap, load_fonts};
use sticker_kit::{BundledMeasure, Measure, PieceKind, Placed, Role, Scene, Theme};

type Taps = Rc<RefCell<Vec<Tap>>>;

/// A page in a test window of the default size, with motion off so the
/// frame is still.
fn open(cx: &mut TestAppContext, theme: Theme) -> (gpui::Entity<Page>, &mut VisualTestContext, Taps) {
    cx.update(|cx| load_fonts(cx)).expect("bundled fonts load");
    let taps = Taps::default();
    let seen = taps.clone();
    let (page, cx) = cx.add_window_view(move |_, _| {
        Page::new(Scene::sample(), theme)
            .reduce_motion(true)
            .on_tap(move |tap, _, _| seen.borrow_mut().push(tap.clone()))
    });
    cx.simulate_resize(size(px(1440.0), px(900.0)));
    cx.run_until_parked();
    (page, cx, taps)
}

fn context() -> TestAppContext {
    TestAppContext::build_with_text_system(TestDispatcher::new(0), None, text_system())
}

fn placed(page: &gpui::Entity<Page>, cx: &mut VisualTestContext) -> Vec<Placed> {
    // Each piece where it is in the window: the column's are moved up by how far the page is scrolled.
    let shown = |page: &Page, piece: &Placed| Placed { rect: page.shown_at(piece), ..piece.clone() };
    page.read_with(cx, |page, _| page.placed().iter().map(|piece| shown(page, piece)).collect())
}

/// Turns the wheel to the column's top, where a piece is painted at the place the layout gave it.
fn to_top(cx: &mut VisualTestContext) {
    let up = gpui::ScrollDelta::Pixels(point(px(0.0), px(1e7)));
    cx.simulate_event(gpui::ScrollEvent { position: point(px(700.0), px(300.0)), delta: up, ..Default::default() });
    cx.run_until_parked();
}

// The tests marked `cfg(debug_assertions)`, here and in `tests/entry_draw.rs`, ask `cx.debug_bounds` where a piece
// was painted, and GPUI keeps those bounds in a debug build only: in a release test build they are left out.
#[cfg(debug_assertions)]
#[test]
fn the_view_builds_for_both_themes() {
    for theme in [Theme::sticker(), Theme::plain()] {
        let mut cx = context();
        let (page, cx, _) = open(&mut cx, theme);
        let placed = placed(&page, cx);
        assert_eq!(placed.iter().filter(|p| p.piece == PieceKind::Character).count(), 8);
        assert!(cx.debug_bounds("Title:title:0:0").is_some(), "the title was laid out");
    }
}

#[test]
fn a_click_on_an_option_reaches_the_callback() {
    let mut cx = context();
    let (page, cx, taps) = open(&mut cx, Theme::sticker());
    let placed = placed(&page, cx);
    let option =
        placed.iter().find(|p| p.piece == PieceKind::Option && p.index == 1).expect("the second option is placed");
    let (x, y) = option.rect.center();
    cx.simulate_click(point(px(x), px(y)), Modifiers::default());
    cx.run_until_parked();

    // The page opened at its column's end, where the choice is: the small ones stand at its top.
    to_top(cx);
    let placed = self::placed(&page, cx);
    let who = placed.iter().find(|p| p.piece == PieceKind::Character && p.id == "kite").expect("a character is placed");
    let (x, y) = who.rect.center();
    cx.simulate_click(point(px(x), px(y)), Modifiers::default());
    cx.run_until_parked();

    assert_eq!(
        *taps.borrow(),
        [Tap::Option { note_id: "picnic-place".into(), index: 1 }, Tap::Character { id: "kite".into() },]
    );
}

#[cfg(debug_assertions)]
#[test]
fn label_text_is_painted_where_the_layout_put_it() {
    for theme in [Theme::sticker(), Theme::plain()] {
        let mut cx = context();
        let (page, cx, _) = open(&mut cx, theme);
        to_top(cx);
        let labels: Vec<Placed> = placed(&page, cx).into_iter().filter(|p| p.piece == PieceKind::Label).collect();
        assert_eq!(labels.len(), 8);
        for label in labels {
            let selector: &'static str = Box::leak(format!("Label:{}:{}:0", label.id, label.index).into_boxed_str());
            let painted = cx.debug_bounds(selector).expect("the label was painted");
            let want = label.text[0].rect;
            let got = [
                f32::from(painted.origin.x),
                f32::from(painted.origin.y),
                f32::from(painted.size.width),
                f32::from(painted.size.height),
            ];
            for (got, want) in got.iter().zip([want.x, want.y, want.w, want.h]) {
                assert!((got - want).abs() <= 1.0, "{}: painted {painted:?}, laid out {want:?}", label.id);
            }
        }
    }
}

/// Why the view lays out with shaped widths and not the bundled advances: advances carry no kerning. Everyday
/// labels agree within 3 pixels; capitals that kern drift by a tenth of their width.
#[test]
fn bundled_advances_run_a_little_wider_than_shaped_text() {
    let theme = Theme::sticker();
    let mut cx = context();
    let (_, cx, _) = open(&mut cx, theme.clone());
    cx.update(|window, _| {
        let shaped = GlyphMeasure { text: window.text_system(), theme: &theme };
        let bundled = BundledMeasure::new(&theme);
        let gap = |text: &str| bundled.width(text, Role::Label, 14.0) - shaped.width(text, Role::Label, 14.0);
        let everyday = [
            "Plant the herb garden",
            "Bake sourdough",
            "Write to grandma",
            "Login page",
            "Sort the 本棚 shelf",
            "0123456789",
        ];
        for text in everyday {
            assert!((-0.5..=3.0).contains(&gap(text)), "{text}: {} px apart", gap(text));
        }
        assert!(gap("AVATAR WAVE Type To Yo") > 10.0, "kerned capitals now agree: one measurer could serve both");
    });
}

/// Draws the sample scene headless. What bounds the time is counted elsewhere, in numbers that are the same on every
/// machine; `backstop` is ten times what the draw took in a debug build on a Mac, so only a hang trips it.
fn drawn(theme: Theme, name: &str, backstop: u64) {
    let out = std::env::temp_dir().join(format!("sticker-kit-{name}-{}.png", std::process::id()));
    let started = std::time::Instant::now();
    sticker_kit::draw::headless::snapshot(Scene::sample(), theme, 1440.0, 900.0, &out).expect("it is drawn");
    let took = started.elapsed();
    eprintln!("{name}: {took:?}");
    assert!(out.is_file() && took.as_secs() < backstop, "{name} took {took:?}");
    let _ = std::fs::remove_file(&out);
}

/// The largest sizes a theme may ask for, on the smallest canvas at the largest scale: the hero comes out 3600 px
/// wide. A unit test beside `Sprites` holds what it is drawn from: an image of 800 at most.
#[test]
fn a_theme_at_the_top_of_its_size_ranges_is_drawn() {
    let mut theme = Theme::sticker();
    let m = &mut theme.metrics;
    (m.width, m.height, m.max_scale) = (200.0, 200.0, 10.0);
    (m.hero_size, m.character_size, m.rim) = (800.0, 400.0, 40.0);
    assert_eq!(theme.check(), Ok(()));
    drawn(theme, "largest sizes", 110);
}
