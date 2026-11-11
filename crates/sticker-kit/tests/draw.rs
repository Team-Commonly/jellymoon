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
    page.read_with(cx, |page, _| page.placed().to_vec())
}

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

    let who = placed.iter().find(|p| p.piece == PieceKind::Character && p.id == "kite").expect("a character is placed");
    let (x, y) = who.rect.center();
    cx.simulate_click(point(px(x), px(y)), Modifiers::default());
    cx.run_until_parked();

    assert_eq!(
        *taps.borrow(),
        [Tap::Option { note_id: "picnic-place".into(), index: 1 }, Tap::Character { id: "kite".into() },]
    );
}

#[test]
fn label_text_is_painted_where_the_layout_put_it() {
    for theme in [Theme::sticker(), Theme::plain()] {
        let mut cx = context();
        let (page, cx, _) = open(&mut cx, theme);
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

/// Art as heavy as the door lets through: the largest size, the most elements and depth, the largest numbers, and
/// a path of many thousand segments. Every shape covers the whole picture.
fn heaviest_art() -> String {
    use sticker_kit::theme::{MAX_ART_DEPTH, MAX_ART_ELEMENTS, MAX_ART_SIDE};
    let mut svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{MAX_ART_SIDE}" height="{MAX_ART_SIDE}" viewBox="0 0 1 1">"#
    );
    let zigzag: String = (0..60_000).map(|i| format!("L{} {}", (i * 37) % 4096, (i * 91) % 4096)).collect();
    svg +=
        &format!(r##"<path d="M0 0{zigzag}" fill="none" stroke="#123456" stroke-width="3" stroke-linejoin="round"/>"##);
    for _ in 1..MAX_ART_DEPTH {
        svg += r#"<g transform="translate(4096 -4096) rotate(4096)">"#;
    }
    for i in 0..MAX_ART_ELEMENTS - MAX_ART_DEPTH - 1 {
        svg += &format!(
            r##"<circle cx="{}" cy="4096" r="4096" fill="#abcdef" fill-opacity="0.2" stroke="#000" stroke-width="4096"/>"##,
            i * 3
        );
    }
    svg + &"</g>".repeat(MAX_ART_DEPTH - 1) + "</svg>"
}

#[test]
fn art_at_the_door_s_limits_is_drawn_in_seconds() {
    let folder = std::env::temp_dir().join(format!("sticker-kit-heavy-{}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    let art = heaviest_art();
    assert!(art.len() < 1024 * 1024);
    let mut theme = String::from("[art]\n");
    for piece in ["hero", "character", "star", "bubble", "badge"] {
        std::fs::write(folder.join(format!("{piece}.svg")), &art).unwrap();
        theme += &format!("{piece} = \"{piece}.svg\"\n");
    }
    std::fs::write(folder.join("theme.toml"), theme).unwrap();
    let loaded = Theme::load(&folder.join("theme.toml")).expect("the art is inside the door");
    assert!(loaded.warnings.is_empty(), "{:?}", loaded.warnings);
    let started = std::time::Instant::now();
    let out = folder.join("heavy.png");
    sticker_kit::draw::headless::snapshot(Scene::sample(), loaded.theme, 1440.0, 900.0, &out).expect("it is drawn");
    let took = started.elapsed();
    eprintln!("heaviest art: {took:?}");
    assert!(out.is_file() && took.as_secs() < 60, "it took {took:?}");
    let _ = std::fs::remove_dir_all(&folder);
}
