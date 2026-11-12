//! Art rasterised by the renderer itself, headless: what the door counted is what is drawn, and what it takes is
//! drawn in the time of its counts. No window is shown.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::time::{Duration, Instant};

use gpui::{Image, ImageFormat, TestAppContext, TestDispatcher};
use sticker_kit::draw::headless::text_system;
use sticker_kit::theme::art_cost;

fn context() -> TestAppContext {
    TestAppContext::build_with_text_system(TestDispatcher::new(0), None, text_system())
}

/// The size in pixels the renderer draws a picture at, and how long it took, or `None` when it draws nothing.
fn rasterised(cx: &TestAppContext, svg: &str) -> Option<(u64, u64, Duration)> {
    let image = Image::from_bytes(ImageFormat::Svg, svg.as_bytes().to_vec());
    let started = Instant::now();
    let drawn = cx.update(|cx| image.to_image_data(cx.svg_renderer())).ok()?.size(0);
    Some((i32::from(drawn.width) as u64, i32::from(drawn.height) as u64, started.elapsed()))
}

/// The size the renderer rasterises a picture at is the size the door counted its paint on, at the renderer's own
/// two pixels a unit, however the picture states it or leaves it out. Each root holds one filled circle, so the
/// paint the door counts is the picture's width times its height.
#[test]
fn a_picture_is_rasterised_at_the_size_its_paint_was_counted_on() {
    let cx = context();
    let picture = |root: &str, r: u32| {
        format!(r#"<svg xmlns="http://www.w3.org/2000/svg" {root}><circle cx="{r}" cy="{r}" r="{r}"/></svg>"#)
    };
    let stated = [
        (r#"width="256" viewBox="0 0 8 256""#, Some((256, 256))),
        (r#"height="256" viewBox="0 0 256 8""#, Some((256, 256))),
        (r#"width="100px" viewBox="0 0 64 32""#, Some((100, 32))),
        (r#"viewBox="0 0 64 32""#, Some((64, 32))),
        (r#"viewBox="-4, -4.5, 64, 32""#, Some((64, 32))),
        (r#"width="10" height="20.5""#, Some((10, 21))),
        (r#"width="1in" height="50""#, Some((96, 50))),
        // With no `viewBox` a side left out is as large as what is in the picture, and the door counts it so: 60
        // here, and 600 below, which is refused. A `viewBox` the renderer cannot read is none to it.
        (r#"width="10""#, None),
        (r#"height="10""#, None),
        ("", None),
        (r#"viewBox="- - 8 8""#, None),
        (r#"viewBox="0.0.0.0 0 8 8""#, None),
        (r#"viewBox="0,,0 8 8""#, None),
        (r#"viewBox="0 0  8 8""#, None),
        (r#"viewBox="0 0 .8e1 8""#, None),
    ];
    for (root, size) in stated {
        for r in [30, 300] {
            let svg = picture(root, r);
            let Ok(cost) = art_cost(&svg) else {
                assert!(size.is_none() || r == 300, "{root}, a circle of {r}, was refused");
                continue;
            };
            let (wide, high, _) = rasterised(&cx, &svg).expect("what the door takes is drawn");
            // A side with a fraction is counted whole and drawn to the pixel: 20.5 is counted as 21 and drawn 41 high.
            let counted = (wide.div_ceil(2), high.div_ceil(2));
            // One fill of the picture, and 16 for each row the circle's edge crosses: twice its height at most.
            let rows = (cost.paint - counted.0 * counted.1) / 16;
            assert_eq!(cost.paint, counted.0 * counted.1 + 16 * rows, "{root}: drawn {wide} by {high}");
            assert!((1..=high).contains(&rows), "{root}: {rows} rows on a picture drawn {high} high");
            assert!(size.is_none_or(|size| size == counted), "{root}: drawn {wide} by {high}");
        }
    }
    let unstated = art_cost(&picture("", 30)).map(|cost| cost.paint / (60 * 60));
    assert_eq!(unstated, Ok(1), "a picture that states no size is counted at the size it is drawn");
}

// The renderer draws a picture at twice its size, cut to whole pixels, and refuses one that comes to none. The door
// took such a file, and the theme that held it loaded and drew nothing for the piece.
#[test]
fn a_picture_under_half_a_pixel_a_side_is_refused_as_the_renderer_refuses_it() {
    let cx = context();
    let sized = |side: &str| {
        let root = format!(r#"<svg xmlns="http://www.w3.org/2000/svg" width="{side}" height="{side}">"#);
        format!(r#"{root}<rect width="30" height="20"/></svg>"#)
    };
    for side in ["1e-9", "0.2", "0.49"] {
        assert!(rasterised(&cx, &sized(side)).is_none(), "{side} was drawn");
        let said = art_cost(&sized(side)).expect_err(side);
        assert!(said.ends_with(": a picture is at least 0.5 a side"), "{said}");
    }
    assert!(art_cost(&sized("0.5")).is_ok());
    assert_eq!(rasterised(&cx, &sized("0.5")).map(|(wide, high, _)| (wide, high)), Some((1, 1)));
}

/// `work` on a thread of its own with a stack of 256 KiB, an eighth of what a thread is given unasked.
fn on_a_small_stack<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new().stack_size(256 * 1024).spawn(work).unwrap().join().unwrap()
}

// The parser calls itself by what a value holds as well as by how deep elements nest, and a file that takes it too
// deep ends the program: nothing can catch that. So the worst files the door takes are read, and drawn, on a stack
// far smaller than any thread of the app has. This is a debug build, where a call takes the most stack.
#[test]
fn the_worst_files_the_door_takes_are_read_and_drawn_on_a_small_stack() {
    use sticker_kit::theme::{MAX_ART_COMMANDS, MAX_ART_DEPTH, MAX_ART_IDLE};
    let nested = |inside: &str| {
        let root = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64">"#;
        format!("{root}{}{inside}{}</svg>", "<g>".repeat(MAX_ART_DEPTH - 2), "</g>".repeat(MAX_ART_DEPTH - 2))
    };
    let path = |d: &str| nested(&format!(r##"<path d="{d}" stroke="#123456"/>"##));
    let (closes, flat) = ("z".repeat(MAX_ART_IDLE), "a1 1 0 0 0 1e-17 0".repeat(MAX_ART_IDLE));
    let worst = [
        ("the longest run of closes", path(&format!("M0 0L9 9L0 9z{closes}"))),
        ("the longest run of arcs drawn as nothing", path(&format!("M0 0{flat}L9 9L0 9z"))),
        ("a run after a run", path(&format!("M0 0L9 9L0 9z{closes}{flat}L9 0z{closes}"))),
        ("as many commands as a file holds", path(&format!("M0 0{}", "l3 3zzz".repeat(MAX_ART_COMMANDS / 4 - 1)))),
        ("an arc as large as is within reach", path(&"M9 9a2000 2000 0 1 1 1 1z".repeat(8))),
        ("shapes as deep as elements nest", nested(&r#"<rect width="9" height="9" rx="2"/>"#.repeat(80))),
    ];
    for (name, svg) in worst {
        let took = on_a_small_stack(move || {
            let cost = art_cost(&svg).unwrap_or_else(|why| panic!("{name}: {why}"));
            let (wide, high, took) = rasterised(&context(), &svg).expect("it is drawn");
            assert_eq!((wide, high), (128, 128), "{name}");
            (cost, took)
        });
        eprintln!("{name}, on 256 KiB: {took:?}");
    }
}

/// The pictures one review measured at seconds each, on a picture of 8 drawn 256 or 204: shapes that hold no path
/// numbers and reach far past the picture, and strokes under a drawn pixel. Each is refused. Its nearest kin that
/// the door takes is drawn at the size it was counted on, in the time printed: the clock here is a backstop.
#[test]
fn the_pictures_a_review_found_slow_are_refused_and_their_nearest_kin_are_drawn_fast() {
    let cx = context();
    let on = |side: u32, inside: &str| {
        let root =
            format!(r#"<svg xmlns="http://www.w3.org/2000/svg" width="{side}" height="{side}" viewBox="0 0 8 8">"#);
        format!("{root}{inside}</svg>")
    };
    let thin = r##"fill="#abcdef" fill-opacity="0.2" stroke="#123456" stroke-opacity="0.3" stroke-width="0.03""##;
    let ellipse = |r: u32| format!(r#"<ellipse cx="4" cy="4" rx="{r}" ry="{r}" {thin}/>"#);
    let lines: String = (1..505).map(|i| format!("L{} {}", (i * 3) % 8, (i % 2) * 8)).collect();
    let stroked = |d: &str, width: &str| {
        on(256, &format!(r##"<path d="M0 0{d}" fill="none" stroke="#123456" stroke-width="{width}"/>"##))
    };
    let far = |reach: u32| format!("{reach},-{reach} -{reach},{reach} ").repeat(150);
    let polyline = |reach: u32, width: &str| {
        on(
            256,
            &format!(r##"<polyline points="{}" fill="none" stroke="#123456" stroke-width="{width}"/>"##, far(reach)),
        )
    };
    let refused = [
        ("99 ellipses of 4096", on(204, &ellipse(4096).repeat(99))),
        ("one ellipse of 4096", on(204, &ellipse(4096))),
        ("30 circles of 4096", on(204, &format!(r#"<circle cx="4" cy="4" r="4096" {thin}/>"#).repeat(30))),
        (
            "a rounded rect of 4096",
            on(204, &format!(r#"<rect width="4096" height="4096" rx="4096" ry="4096" {thin}/>"#)),
        ),
        (
            "a line 4096 wide",
            on(204, r##"<line x2="8" y2="8" stroke="#123456" stroke-width="4096" stroke-linecap="round"/>"##),
        ),
        ("99 ellipses within reach", on(204, &ellipse(100).repeat(99))),
        ("504 lines at 0.015", stroked(&lines, "0.015")),
        ("504 lines at 0.01", stroked(&lines, "0.01")),
        ("504 lines at 0.0001", stroked(&lines, "0.0001")),
        ("505 lines at 0.016", stroked(&format!("{lines}L0 8"), "0.016")),
        ("50 ellipses within reach", on(204, &ellipse(100).repeat(50))),
        ("170 curves at 0.01", stroked(&"C1 1 2 2 3 3".repeat(170), "0.01")),
        ("300 points to 4096 at 0.01", polyline(4096, "0.01")),
        ("300 points to 4096 at 0.03", polyline(4096, "0.03")),
    ];
    for (name, svg) in &refused {
        assert!(art_cost(svg).is_err(), "{name} was taken");
    }
    let taken = [
        ("40 ellipses within reach", on(204, &ellipse(100).repeat(40))),
        ("one ellipse within reach", on(204, &ellipse(100))),
        ("504 lines at 0.016", stroked(&lines, "0.016")),
        ("504 lines at 0.03", stroked(&lines, "0.03")),
        ("170 curves at 0.016", stroked(&"C1 1 2 2 3 3".repeat(170), "0.016")),
        ("300 points to 100 at 0.016", polyline(100, "0.016")),
    ];
    for (name, svg) in &taken {
        let cost = art_cost(svg).unwrap_or_else(|why| panic!("{name}: {why}"));
        let (wide, high, took) = rasterised(&cx, svg).expect("it is drawn");
        eprintln!("{name}: {took:?}, numbers {}, paint {}", cost.numbers, cost.paint);
        assert!(cost.paint >= wide.div_ceil(2) * high.div_ceil(2), "{name}: drawn {wide} by {high}");
        assert!(took < Duration::from_secs(10), "{name} took {took:?}");
    }
}
