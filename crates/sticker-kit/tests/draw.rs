//! The drawing half, headless: no window is shown.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

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

/// One way to spend all that a theme's art may cost, over its five files.
struct Heavy {
    name: &'static str,
    /// See-through fills over the whole picture, in the five files together: four points each.
    covers: u64,
    /// How the ragged shape in each file is painted.
    paint: &'static str,
    /// How many fills and strokes of the picture that is, a gradient being four, and what the door counts for a row
    /// under one of the shape's edges.
    areas: u64,
    row: u64,
    /// The shape is written as curves.
    curved: bool,
}

/// What a review and the kit's own measuring found dearest, a count's worth of each: paint that covers the picture,
/// and edges that cross its rows under each kind of paint. The second is the one a review measured at 7.6 s when a
/// row cost nothing.
const HEAVY: [Heavy; 5] = [
    Heavy { name: "see-through fills", covers: 52, paint: SEE_THROUGH, areas: 1, row: 16, curved: false },
    Heavy { name: "a ragged fill", covers: 0, paint: SEE_THROUGH, areas: 1, row: 16, curved: false },
    Heavy {
        name: "a ragged gradient",
        covers: 0,
        paint: r#"fill="url(#g)" fill-opacity="0.5" fill-rule="evenodd""#,
        areas: 4,
        row: 64,
        curved: false,
    },
    Heavy {
        name: "a ragged fill under a round stroke",
        covers: 0,
        paint: r##"fill="#abcdef" fill-opacity="0.2" stroke="#123456" stroke-opacity="0.3" stroke-width="2.6" stroke-linejoin="round" stroke-linecap="round""##,
        areas: 2,
        row: 48,
        curved: false,
    },
    Heavy {
        name: "ragged curves under the thinnest stroke",
        covers: 0,
        paint: r##"fill="#abcdef" fill-opacity="0.2" stroke="#123456" stroke-opacity="0.3" stroke-width="0.51""##,
        areas: 2,
        row: 48,
        curved: true,
    },
];

const SEE_THROUGH: &str = r##"fill="#abcdef" fill-opacity="0.2""##;

/// One of five art files that together cost all the door lets a theme's art cost: both counts are spent to the last
/// unit. Each file holds its share of the fills that cover the picture, and one ragged shape that takes every point
/// left and every row left: it climbs from the top of the picture and comes back, as many times as it has points
/// for. `nth` moves every point, so no two files are the same picture.
fn heaviest_art(heavy: &Heavy, nth: u64) -> String {
    use sticker_kit::theme::{MAX_ART_DEPTH, MAX_ART_NUMBERS, MAX_ART_PAINT, ROW_PAINT};
    let area = 256 * 256;
    // A fifth of each count, and the remainder to the first files.
    let share = |all: u64| all / 5 + u64::from(nth < all % 5);
    // A cover's two sides cross every row.
    let covered = heavy.covers * (area + ROW_PAINT * 512);
    let rows = (MAX_ART_PAINT - covered - 5 * heavy.areas * area) / heavy.row;
    let (points, climb) = (share(MAX_ART_NUMBERS as u64 / 2 - 4 * heavy.covers), share(rows / 2));
    let x = |point: u64| format!("{:.2}", nth as f32 * 0.3 + point as f32 * 250.0 / points as f32);
    let spikes = (points - 1) / 2;
    let mut d = format!("M{} 0", x(0));
    for spike in 0..spikes {
        let (tip, high, base) =
            (x(2 * spike + 1), climb / spikes + u64::from(spike < climb % spikes), x(2 * spike + 2));
        d += &if heavy.curved { format!("Q{tip} {high} {base} 0") } else { format!("L{tip} {high}L{base} 0") };
    }
    if points.is_multiple_of(2) {
        d += &format!("L{} 0", x(points));
    }
    let cover = |i: u64| format!(r#"<rect x="-{}" y="-1" width="300" height="258" {SEE_THROUGH}/>"#, 1 + i + nth);
    let covers: String = (0..share(heavy.covers)).map(cover).collect();
    let gradient = r##"<linearGradient id="g" x2="0.02" gradientTransform="rotate(33)"><stop offset="0" stop-color="#f00"/><stop offset="0.5" stop-color="#0f0"/><stop offset="1" stop-color="#00f"/></linearGradient>"##;
    let root = format!(r#"<svg xmlns="http://www.w3.org/2000/svg" width="256" height="256">{gradient}"#);
    let (open, close) = ("<g>".repeat(MAX_ART_DEPTH - 2), "</g>".repeat(MAX_ART_DEPTH - 2));
    format!(r#"{root}{open}{covers}<path d="{d}z" {}/>{close}</svg>"#, heavy.paint)
}

/// Draws the sample scene headless. What bounds the time is counted elsewhere, in numbers that are the same on every
/// machine; `backstop` is ten times what the draw took in a debug build on a Mac, so only a hang trips it.
fn drawn(theme: Theme, name: &str, backstop: u64) {
    drawn_with(theme, Vec::new(), name, backstop);
}

/// [`drawn`] with the art a trial accepted for the theme.
fn drawn_with(theme: Theme, art: Vec<sticker_kit::theme::Accepted>, name: &str, backstop: u64) {
    use sticker_kit::draw::headless::snapshot_with;
    let out = std::path::Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("sticker-kit-{name}-{}.png", std::process::id()));
    let started = std::time::Instant::now();
    snapshot_with(Scene::sample(), (theme, art), None, 1440.0, 900.0, &out).expect("it is drawn");
    let took = started.elapsed();
    eprintln!("{name}: {took:?}");
    assert!(out.is_file() && took.as_secs() < backstop, "{name} took {took:?}");
    let _ = std::fs::remove_file(&out);
}

// Whatever a theme's art is made of, it costs no more to draw than this: each way of spending the two counts is
// read and drawn, and the time of each is printed (`README.md` states them).
#[test]
fn the_heaviest_art_a_theme_may_hold_is_counted_to_the_door_s_two_limits_and_drawn() {
    HEAVY.iter().for_each(heaviest);
}

fn heaviest(heavy: &Heavy) {
    use sticker_kit::theme::{ArtCost, MAX_ART_NUMBERS, MAX_ART_PAINT, art_cost};
    let name = format!("heaviest art, {}", heavy.name);
    let folder = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "sticker-kit-heavy-{}-{}",
        heavy.row,
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).unwrap();
    let (mut theme, mut all) = (String::from("[art]\n"), ArtCost::default());
    // What the five cost by themselves: the door's reading of each, and the renderer's drawing of it at its own size.
    let (cx, mut read, mut rasterised) = (context(), Duration::ZERO, Duration::ZERO);
    for (nth, piece) in ["hero", "character", "star", "bubble", "badge"].into_iter().enumerate() {
        let art = heaviest_art(heavy, nth as u64);
        let started = Instant::now();
        let cost = art_cost(&art).unwrap_or_else(|why| panic!("{name}: {why}"));
        read += started.elapsed();
        all = all.with(cost);
        let image = gpui::Image::from_bytes(gpui::ImageFormat::Svg, art.clone().into_bytes());
        let started = Instant::now();
        cx.update(|cx| image.to_image_data(cx.svg_renderer())).expect("it is drawn");
        rasterised += started.elapsed();
        std::fs::write(folder.join(format!("{piece}.svg")), &art).unwrap();
        theme += &format!("{piece} = \"{piece}.svg\"\n");
    }
    eprintln!("{name}: read at the door in {read:?}, rasterised in {rasterised:?}");
    let spent = ArtCost { numbers: MAX_ART_NUMBERS, paint: MAX_ART_PAINT };
    assert_eq!(all, spent, "{name}: both counts are spent to the last unit");
    std::fs::write(folder.join("theme.toml"), theme).unwrap();
    let started = Instant::now();
    let (loaded, art) = support::tried(&folder.join("theme.toml")).expect("the art is inside the door");
    eprintln!("{name}: five trials in {:?}", started.elapsed());
    assert!(loaded.warnings.is_empty(), "{:?}", loaded.warnings);
    assert_eq!(art.len(), 5, "{name}: every piece's picture is drawn");
    drawn_with(loaded.theme, art, &name, 40);

    let one_more = heaviest_art(heavy, 4).replacen("<path", r#"<circle r="1"/><path"#, 1);
    std::fs::write(folder.join("badge.svg"), one_more).unwrap();
    let refused = support::tried(&folder.join("theme.toml")).expect_err("one shape over the counts");
    assert!(refused.to_string().starts_with("the SVG for art.badge is refused: with the art before it, "));
    let _ = std::fs::remove_dir_all(&folder);
}

// The dearest theme a review found inside both counts (PR 147): 64 see-through round strokes 600 wide, each of
// 12 dots. A dot crosses no row, so each stroke is priced as one paint of the picture, and covers it twelve times.
#[test]
fn the_dearest_art_a_review_found_inside_the_counts_is_tried_and_drawn() {
    use sticker_kit::theme::{ArtCost, MAX_ART_NUMBERS, MAX_ART_PAINT, art_cost};
    let folder =
        std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("sticker-kit-dots-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).unwrap();
    let (mut theme, mut all) = (String::from("[art]\n"), ArtCost::default());
    let paint = r##"fill="none" stroke="#123456" stroke-opacity="0.3" stroke-width="600" stroke-linecap="round""##;
    for (nth, piece) in ["hero", "character", "star", "bubble", "badge"].into_iter().enumerate() {
        let dots =
            |path: usize| -> String { (0..12).map(|dot| format!("M{} {}h0", 20 * dot + nth, 4 * path)).collect() };
        let paths: String = (0..13 - nth / 4).map(|path| format!(r#"<path d="{}" {paint}/>"#, dots(path))).collect();
        let art = format!(r#"<svg xmlns="http://www.w3.org/2000/svg" width="256" height="256">{paths}</svg>"#);
        all = all.with(art_cost(&art).expect("inside the door"));
        std::fs::write(folder.join(format!("{piece}.svg")), &art).unwrap();
        theme += &format!("{piece} = \"{piece}.svg\"\n");
    }
    assert_eq!(all, ArtCost { numbers: MAX_ART_NUMBERS, paint: MAX_ART_PAINT }, "both counts to the last unit");
    std::fs::write(folder.join("theme.toml"), theme).unwrap();
    let started = Instant::now();
    let (loaded, art) = support::tried(&folder.join("theme.toml")).expect("the art is inside the door");
    eprintln!("heaviest art, round dots: five trials in {:?}", started.elapsed());
    assert_eq!(art.len(), 5);
    drawn_with(loaded.theme, art, "heaviest art, round dots", 40);
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
