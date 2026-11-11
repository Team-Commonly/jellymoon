//! The drawing half, headless: no window is shown.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::cell::RefCell;
use std::rc::Rc;

use gpui::{Modifiers, TestAppContext, TestDispatcher, VisualTestContext, point, px, size};
use sticker_kit::draw::headless::text_system;
use sticker_kit::draw::{GlyphMeasure, Page, Tap, load_fonts};
use sticker_kit::{BundledMeasure, Measure, Note, PieceKind, Placed, Role, Scene, Theme};

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

/// One of five art files that together are as heavy as the door lets a theme be: both counts spent to the last
/// unit, on what costs the most to draw. Every shape covers the whole picture with see-through paint, the circles are
/// far larger than it with strokes as wide, and each number is a line as tall as the picture under a thin stroke.
/// `nth` moves every point, so no two files are the same picture.
fn heaviest_art(nth: usize) -> String {
    use sticker_kit::theme::{MAX_ART_DEPTH, MAX_ART_NUMBERS, MAX_ART_PAINT, MAX_ART_SIDE, MIN_ART_VIEW};
    let side = MAX_ART_SIDE as u64;
    let (shapes, numbers) = (MAX_ART_PAINT / (side * side), MAX_ART_NUMBERS / 2);
    // A fifth of each count, and the remainder to the first files.
    let share = |all: u64| (all / 5 + u64::from((nth as u64) < all % 5)) as usize;
    let mut svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{side}" height="{side}" viewBox="0 0 {MIN_ART_VIEW} {MIN_ART_VIEW}">"#
    );
    for _ in 1..MAX_ART_DEPTH {
        svg += r#"<g transform="translate(4096 -4096) rotate(4096)">"#;
    }
    let paint = r##"fill="#abcdef" fill-opacity="0.2" stroke="#123456" stroke-opacity="0.3""##;
    let lines: String = (1..share(numbers as u64)).map(|i| format!("L{} {}", (i * 3 + nth) % 8, (i % 2) * 8)).collect();
    svg += &format!(r#"<path d="M0 0{lines}" {paint} stroke-width="0.03" stroke-linejoin="round"/>"#);
    for i in 1..share(shapes) {
        svg += &format!(r#"<circle cx="{}" cy="4096" r="4096" {paint} stroke-width="4096"/>"#, i * 3 + nth);
    }
    svg + &"</g>".repeat(MAX_ART_DEPTH - 1) + "</svg>"
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

#[test]
fn the_heaviest_art_a_theme_may_hold_is_counted_to_the_door_s_two_limits_and_drawn() {
    use sticker_kit::theme::{ArtCost, MAX_ART_NUMBERS, MAX_ART_PAINT, art_cost};
    let folder = std::env::temp_dir().join(format!("sticker-kit-heavy-{}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    let (mut theme, mut all) = (String::from("[art]\n"), ArtCost::default());
    for (nth, piece) in ["hero", "character", "star", "bubble", "badge"].into_iter().enumerate() {
        let art = heaviest_art(nth);
        let cost = art_cost(&art).expect("each file is inside the door");
        all = ArtCost { numbers: all.numbers + cost.numbers, paint: all.paint + cost.paint };
        std::fs::write(folder.join(format!("{piece}.svg")), &art).unwrap();
        theme += &format!("{piece} = \"{piece}.svg\"\n");
    }
    assert_eq!(all, ArtCost { numbers: MAX_ART_NUMBERS, paint: MAX_ART_PAINT }, "nothing heavier passes the door");
    std::fs::write(folder.join("theme.toml"), theme).unwrap();
    let loaded = Theme::load(&folder.join("theme.toml")).expect("the art is inside the door");
    assert!(loaded.warnings.is_empty(), "{:?}", loaded.warnings);
    drawn(loaded.theme, "heaviest art", 40);

    let one_more = heaviest_art(4).replacen("<circle", r#"<circle r="1"/><circle"#, 1);
    std::fs::write(folder.join("badge.svg"), one_more).unwrap();
    let refused = Theme::load(&folder.join("theme.toml")).expect_err("one shape over the count");
    assert!(refused.to_string().starts_with("the SVG for art.badge is refused: with the art before it, paint"));
    let _ = std::fs::remove_dir_all(&folder);
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

// A whole choice's options are written smaller when room is short. What is painted is the size the layout measured:
// each line is as wide as its run, so it ends inside its bubble. Painted at the role's own size it would run past.
#[test]
fn a_whole_option_written_smaller_is_painted_at_the_size_it_was_measured_at() {
    let options = vec!["请".repeat(200), "W".repeat(200)];
    let rows = (0..9).map(|i| (format!("row {i}"), "of nine".to_string())).collect();
    let (title, footer) = ("pick one".to_string(), "a footer".to_string());
    let mut scene = Scene::sample();
    scene.notes = vec![
        Note::List { title: "a list".into(), rows },
        Note::Choice { id: "a".into(), title, options, footer, struck_first: false, whole: true },
    ];
    for theme in [Theme::sticker(), Theme::plain()] {
        let mut cx = context();
        let (page, cx, _) = open(&mut cx, theme.clone());
        page.update(cx, |page, cx| page.set_scene(scene.clone(), cx));
        cx.simulate_resize(size(px(960.0), px(640.0)));
        cx.run_until_parked();
        let bubbles: Vec<Placed> = placed(&page, cx).into_iter().filter(|p| p.piece == PieceKind::Option).collect();
        assert_eq!(bubbles.len(), 2, "the choice is drawn");
        let own = Role::Option.size(&theme, 960.0 / 1440.0);
        assert!(bubbles.iter().all(|bubble| bubble.text.iter().all(|run| run.size < own)), "written smaller");
        for bubble in &bubbles {
            for (n, run) in bubble.text.iter().enumerate() {
                let selector: &'static str = Box::leak(format!("Option:a:{}:{n}", bubble.index).into_boxed_str());
                let painted = cx.debug_bounds(selector).expect("the line was painted");
                let (left, width) = (f32::from(painted.origin.x), f32::from(painted.size.width));
                assert!((width - run.rect.w).abs() <= 1.0, "painted {width} wide, measured {}", run.rect.w);
                assert!(left + width <= bubble.rect.x + bubble.rect.w, "the line ends inside its bubble");
            }
        }
    }
}
