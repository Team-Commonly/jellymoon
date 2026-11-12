//! The door accepts only what it reads the way the renderer does, and refuses the rest. Each case here is a picture
//! the two read differently: the door would count one size, or one path, and the renderer would draw another. No
//! GPUI: `tests/draw.rs` rasterises the same roots and compares the sizes.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use sticker_kit::theme::{MAX_ART_REACH, art_cost};

fn rooted(root: &str) -> String {
    format!(r#"<svg xmlns="http://www.w3.org/2000/svg" {root}><circle cx="4096" cy="4096" r="4096"/></svg>"#)
}

/// The numbers the door counts in one path, or why it refuses the picture.
fn path(d: &str) -> Result<usize, String> {
    let svg = format!(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 8 8"><path d="{d}"/></svg>"#);
    art_cost(&svg).map(|cost| cost.numbers)
}

// The renderer reads `viewBox`, `width` and `height` in that case only. A picture that wrote `viewbox` was counted
// as 8 by 8 and drawn as large as what it held, 8192 by 8192.
#[test]
fn a_name_in_another_case_than_svg_writes_it_is_refused() {
    let said = |written: &str, svg: &str| {
        Err(format!("it writes `{written}` for `{svg}`: the renderer reads a name only as SVG writes it"))
    };
    for (root, written, svg) in [
        (r#"viewbox="0 0 8 8""#, "viewbox", "viewBox"),
        (r#"VIEWBOX="0 0 8 8""#, "VIEWBOX", "viewBox"),
        (r#"WIDTH="8" HEIGHT="8""#, "WIDTH", "width"),
        (r#"width="8" Height="8""#, "Height", "height"),
        (r#"viewBox="0 0 256 256" viewbox="0 0 8 8""#, "viewbox", "viewBox"),
    ] {
        assert_eq!(art_cost(&rooted(root)), said(written, svg), "{root}");
    }
    let inside = |inside: &str| {
        art_cost(&format!(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 8 8">{inside}</svg>"#))
    };
    assert_eq!(inside(r#"<Circle r="4"/>"#), said("Circle", "circle"));
    assert_eq!(inside(r#"<lineargradient id="g"/>"#), said("lineargradient", "linearGradient"));
    assert_eq!(inside(r#"<circle R="4"/>"#), said("R", "r"));
    assert_eq!(
        inside(r#"<radialGradient id="g" gradientunits="userSpaceOnUse"/>"#),
        said("gradientunits", "gradientUnits")
    );
    let own = r##"<linearGradient id="g" gradientUnits="userSpaceOnUse" gradientTransform="rotate(4)"/><radialGradient id="h"/><path d="M0 0L1 1Z" fill="url(#g)"/>"##;
    assert!(inside(own).is_ok(), "the five names with a capital, and a path's letters in either case");
}

// The renderer takes a `viewBox` it cannot read for none at all, and then sizes the picture to what is in it.
#[test]
fn a_view_box_is_four_plain_numbers_and_the_svg_states_each_size_once() {
    for view in [
        "- - 8 8",
        "0.0.0.0 0 8 8",
        "0 0.256.256 8 8",
        "0,,0 8 8",
        "0 0  8 8",
        " 0 0 8 8",
        "0 0 8 8 ",
        "0 0 8. 8",
        "0 0 .8e1 8",
        "+0 0 8 8",
        "0 0 -8 8",
        "0\t0 8 8",
    ] {
        let said = art_cost(&rooted(&format!(r#"viewBox="{view}""#))).unwrap_err();
        let shape = format!("`viewbox=\"{view}\"` is not four plain numbers, the last two a size from 8 to 256");
        assert!(said == shape || said.contains("plain digits"), "{view}: {said}");
    }
    for view in ["0 0 8 8", "0,0,8,8", "0, 0, 8, 8", "-128 -128.5 256 8.5"] {
        assert!(art_cost(&rooted(&format!(r#"viewBox="{view}""#))).is_ok(), "{view}");
    }
    for (root, name) in [
        (r#"viewBox="0 0 256 256" viewBox="0 0 8 8""#, "viewBox"),
        (r#"width="8" height="8" width="9""#, "width"),
        (r#"height="8" viewBox="0 0 8 8" height="8""#, "height"),
    ] {
        assert_eq!(art_cost(&rooted(root)), Err(format!("its `<svg` states `{name}` twice")), "{root}");
    }
    for side in [".5", "8.", "08.5.5", "+8", " 8"] {
        let said = art_cost(&rooted(&format!(r#"width="{side}" height="8""#))).unwrap_err();
        assert!(said.contains("is not a size from 1 to 256") || said.contains("plain digits"), "{side}: {said}");
    }
}

// A curve that reaches far from a small picture is what was slow to draw (`README.md`), so the door follows the pen
// and holds every place a path names, and an arc's radius, to 512.
#[test]
fn a_path_stays_within_reach_of_its_picture_with_the_pen_followed_through_its_steps() {
    assert_eq!(MAX_ART_REACH, 512.0);
    for near in [
        "M512 -512L-512 512",
        "M0 0h256h256",
        "M500 0a512 512 0 1 1 12 0",
        "M0 0c512 512 -512 512 0 0",
        "M400 0h-400zh100",
        "M0 0m256 0 256 0",
        "M-300 0C-512 0 0 0 0 0s512 0 512 0",
        "M0 0Q0 500 0 0T0 0",
        "M0 0C-300 0 0 0 200 0S0 0 0 0",
        "M0 0Q-300 0 200 0L200 0T0 0",
        "M0 0Q-300 0 200 0S0 0 0 0",
    ] {
        assert!(path(near).is_ok(), "{near}: {:?}", path(near));
    }
    let far = "its path goes more than 512 from the picture's corner, or has an arc of a larger radius";
    for d in [
        "M513 0",
        "M0 0L0 -512.5",
        "M0 0h256h256h1",
        "M0 0v-300v-300",
        "M300 0l100 0 100 0 100 0",
        "M0 0A513 4 0 1 1 2 2",
        "M0 0a4 4096 0 1 1 2 2",
        "M300 300c0 0 0 0 0 0C0 0 600 0 1 1",
        "M300 0c300 0 0 0 1 1",
        "M0 0m400 0 200 0",
        "M400 0h-400zh100h100",
        "M0 0a4 4 0 1 1 300 300a4 4 0 1 1 300 300",
        // The handle a `t` or an `s` does not write is the last one mirrored through the pen: 700 in the first
        // two, and in the third further out at each `t` of a row.
        "M0 0Q-300 0 200 0T0 0",
        "M0 0C0 0 -300 0 200 0S0 0 0 0",
        "M0 0Q0 300 0 0T200 0 0 0 200 0 0 0",
    ] {
        assert_eq!(path(d), Err(far.to_string()), "{d}");
    }
}

// The renderer reads an arc's flags as one character each, so `11 2` is to it two flags and a 2, and to a reader
// of numbers an 11 and a 2: 51 arcs passed for 36.
#[test]
fn a_path_the_renderer_could_read_another_way_is_refused() {
    assert_eq!(path("M0 0a4 4 0 1 1 2 2A4 4 0 0,1 2 2"), Ok(2 + 14 * 2));
    let flags = "an arc's two flags are each `0` or `1`, apart from the numbers beside them";
    for d in [
        "M0 0a4 4 0 11 2 2",
        "M0 0a4 4 0 1 12 2",
        "M0 0A4 4 0 012 2",
        "M0 0a4 4 0 2 1 2 2",
        "M0 0a4 4 0 0.5 1 2 2",
        "M0 0a4 4 0 -1 1 2 2",
    ] {
        assert_eq!(path(d), Err(flags.to_string()), "{d}");
    }
    for d in ["5 5L1 1", "M0 0z5 5"] {
        assert_eq!(path(d), Err("its path holds a number that no command takes".to_string()), "{d}");
    }
    assert_eq!(path("M0 0 # L1 1"), Err("its path holds `#` where a number is read".to_string()));
}
