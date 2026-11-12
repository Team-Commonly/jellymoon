//! The theme door's numbers, each held at its edge: the last value inside loads and the first one outside is refused.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;

use sticker_kit::theme::{
    MAX_ART_BYTES, MAX_ART_COMMANDS, MAX_ART_FLAT, MAX_ART_IDLE, MAX_ART_PATH_BYTES, MAX_ART_WRITTEN, MAX_THEME_BYTES,
    art_cost, svg_refusal,
};
use sticker_kit::{Theme, ThemeError};

fn svg(inside: &str) -> String {
    format!(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 40 40">{inside}</svg>"#)
}

/// `text` padded with a comment to `bytes` bytes.
fn padded(text: &str, open: &str, close: &str, bytes: u64) -> String {
    let room = bytes as usize - text.len() - open.len() - close.len();
    format!("{text}{open}{}{close}", "x".repeat(room))
}

#[test]
fn a_theme_file_and_an_art_file_are_read_up_to_their_size_and_not_one_byte_over() {
    assert_eq!((MAX_THEME_BYTES, MAX_ART_BYTES), (256 * 1024, 64 * 1024));
    let folder = std::env::temp_dir().join(format!("sticker-kit-limits-{}", std::process::id()));
    fs::create_dir_all(&folder).unwrap();
    let theme = folder.join("theme.toml");
    for (over, fine) in [(0, true), (1, false)] {
        let text = padded("[art]\nstar = \"\"\n", "# ", "\n", MAX_THEME_BYTES + over);
        assert_eq!(Theme::from_toml(&text).is_ok(), fine, "theme text, {over} over");
        fs::write(&theme, &text).unwrap();
        let loaded = Theme::load(&theme);
        assert_eq!(loaded.is_ok(), fine, "theme file, {over} over");
        assert!(fine || loaded == Err(ThemeError::Read("it is over 256 KiB".into())));

        fs::write(folder.join("star.svg"), padded(&svg(""), "<!-- ", " -->", MAX_ART_BYTES + over)).unwrap();
        fs::write(&theme, "[art]\nstar = \"star.svg\"\n").unwrap();
        let loaded = Theme::load(&theme);
        assert_eq!(loaded.is_ok(), fine, "art file, {over} over");
        let refused = ThemeError::RefusedSvg { key: "art.star".into(), reason: "it is over 64 KiB".into() };
        assert!(fine || loaded == Err(refused));
    }
    let _ = fs::remove_dir_all(folder);
}

#[test]
fn each_range_of_its_own_holds_at_both_ends() {
    let edges = [
        ("tilt", "note_tape", "-45.0", "-45.5"),
        ("tilt", "note_tape", "45.0", "45.5"),
        ("tilt", "list_note", "45", "46"),
        ("metrics", "hero_size", "800", "801"),
        ("metrics", "character_size", "400", "401"),
        ("metrics", "rim", "40", "40.5"),
        ("motion", "damping", "200", "201"),
    ];
    for (table, key, inside, outside) in edges {
        assert!(Theme::from_toml(&format!("[{table}]\n{key} = {inside}\n")).is_ok(), "{key} = {inside}");
        match Theme::from_toml(&format!("[{table}]\n{key} = {outside}\n")) {
            Err(ThemeError::OutOfRange { key: named, .. }) => assert_eq!(named, format!("{table}.{key}")),
            other => panic!("{key} = {outside} was not refused: {other:?}"),
        }
    }
}

// The door has no reading of a number of its own. A unit, a `%` or an exponent is what the renderer takes it for,
// and the picture is held at the size that comes of it.
#[test]
fn a_number_with_a_unit_is_what_the_renderer_takes_it_for() {
    let sized = |size: &str| {
        let rect = r#"<rect width="50%" height="1e1"/>"#;
        art_cost(&format!(r#"<svg xmlns="http://www.w3.org/2000/svg" {size}>{rect}</svg>"#)).map(|cost| cost.paint)
    };
    assert_eq!(sized(r#"width="1in" height="2in""#), Ok(96 * 192 + 16 * 20), "an inch is 96 pixels to the renderer");
    assert_eq!(sized(r#"width="40px" height="1e1""#), Ok(400 + 16 * 20));
    assert_eq!(
        sized(r#"width="3in" height="1in""#),
        Err("the renderer sizes it 288 by 96: a picture is at most 256 a side".into())
    );
    let commands =
        r#"<path d="M1 1L2 2H3V4C1 1 2 2 3 3S1 1 2 2Q1 1 2 2T3 3A4 4 0 0 1 5 5Zm1 1l1 1h1v1c1 1 1 1 1 1z"/>"#;
    assert_eq!(svg_refusal(&svg(commands)), None, "every path command");
}

// What the parser spends on a file before its points can be counted is bounded by the bytes its paths are written
// in: 64 KiB of arcs took it 0.36 s.
#[test]
fn the_paths_and_lists_of_points_of_a_file_are_held_to_a_length_together() {
    assert_eq!(MAX_ART_PATH_BYTES, 16 * 1024);
    for over in [0, 1] {
        let d = format!("M0 0h9v9{}", " ".repeat(MAX_ART_PATH_BYTES / 2 - 8));
        let points = format!("1,1 9,9 1,9{}", " ".repeat(MAX_ART_PATH_BYTES / 2 - 11 + over));
        assert_eq!(d.len() + points.len(), MAX_ART_PATH_BYTES + over);
        let shapes = format!(r#"<path d="{d}"/><polygon points="{points}"/>"#);
        let want = (over == 1).then(|| "its `d` and `points` values are over 16384 bytes together".to_string());
        assert_eq!(svg_refusal(&svg(&shapes)), want, "{over} over");
    }
    let arcs = format!(r##"<path d="M0 0{}" stroke="#000"/>"##, "a4096 4096 0 1 1 1 1".repeat(3200));
    let started = std::time::Instant::now();
    assert!(svg_refusal(&svg(&arcs)).is_some());
    assert!(started.elapsed().as_millis() < 100, "refused before it is parsed as a picture: {:?}", started.elapsed());
}

// An attribute is on the list by its whole name. One under a prefix is another attribute to XML, whatever its last
// part says, and the parser reads some of them (`xlink:href`, `xml:space`).
#[test]
fn an_attribute_under_a_prefix_is_not_the_one_on_the_list() {
    let prefixed = |attribute: &str| {
        let root = r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:o="urn:other" viewBox="0 0 8 8">"#;
        svg_refusal(&format!(r#"{root}<rect width="8" height="8" {attribute}/></svg>"#))
    };
    assert_eq!(prefixed(r##"fill="#123456""##), None);
    assert_eq!(prefixed(r##"o:fill="#123456""##), Some("it contains the attribute `fill`".into()));
    assert_eq!(prefixed(r#"o:d="M0 0""#), Some("it contains the attribute `d`".into()));
    assert_eq!(prefixed(r#"xml:id="a""#), Some("it contains the attribute `id`".into()));
}

/// How many segments the renderer's path parser draws for `segment` written after `before`.
fn drawn_for(before: &str, segment: &str) -> usize {
    let drawn = |d: &str| svgtypes::SimplifyingPathParser::from(d).map_while(Result::ok).count();
    drawn(&format!("{before}{segment}")) - drawn(before)
}

// The renderer's path parser calls itself once for every segment it draws nothing for, so a path of nothing but
// closes ended the program: `M0 0L1 1` and 4000 `z` overflowed a stack of 2 MiB. Every case of it is in this table,
// read off `svgtypes` 0.16.2 (`SimplifyingPathParser::next`): a close after a close, and an arc so flat that it is
// made of no curve, unless it follows a close (a move is drawn first). The door holds a run of them.
#[test]
fn a_run_of_segments_the_parser_draws_nothing_for_is_held_to_a_length() {
    assert_eq!(MAX_ART_IDLE, 32);
    let flat = "a1 1 0 0 0 1e-17 0";
    // What is written before, the segment, whether the parser draws nothing for it, and whether the door counts it.
    let table = [
        ("M0 0L1 1z", "z", true, true),
        ("M0 0", flat, true, true),
        // An arc that is drawn is counted too: the door does no arc's sums, and takes it for one that may be flat.
        ("M0 0", "a1 1 0 0 0 1 1", false, true),
        ("M0 0", "A1 1 0 0 0 1 1", false, true),
        // An arc of no radius, or to where it starts, is a line to the parser.
        ("M0 0", "a0 0 0 0 0 1 1", false, true),
        ("M0 0", "a1 1 0 0 0 0 0", false, true),
        ("M0 0L1 1", "z", false, false),
        ("M0 0L1 1z", flat, false, false),
        ("M0 0", "m0 0", false, false),
        ("M0 0", "l0 0", false, false),
        ("M0 0", "h0", false, false),
        ("M0 0", "v0", false, false),
        ("M0 0", "c0 0 0 0 0 0", false, false),
        ("M0 0", "s0 0 0 0", false, false),
        ("M0 0", "q0 0 0 0", false, false),
        ("M0 0", "t0 0", false, false),
        ("M0 0L1 1z", "l0 0", false, false),
    ];
    let refusal = |d: &str| svg_refusal(&svg(&format!(r#"<path d="{d}"/>"#)));
    let said =
        Some("a path holds more than 32 closes or arcs in a row, which the parser may draw nothing for".to_string());
    for (before, segment, nothing, held) in table {
        assert_eq!(drawn_for(before, segment) == 0, nothing, "{before} {segment}");
        assert!(held || !nothing, "{before} {segment}: drawn as nothing, and not held");
        let run = |times: usize| refusal(&format!("{before}{}", segment.repeat(times)));
        if held {
            assert_eq!((run(MAX_ART_IDLE), run(MAX_ART_IDLE + 1)), (None, said.clone()), "{before} {segment}");
        } else if !matches!(segment.as_bytes()[0], b'z' | b'a') {
            assert_eq!(run(200), None, "{before} {segment}");
        }
    }
    // The first close of a run is drawn, and so is an arc after a close: neither is counted.
    let closes = |times: usize| refusal(&format!("M0 0L1 1{}", "z".repeat(times)));
    assert_eq!((closes(MAX_ART_IDLE + 1), closes(MAX_ART_IDLE + 2)), (None, said.clone()));
    assert_eq!(refusal(&format!("M0 0L1 1{}", format!("z{flat}").repeat(200))), None);
    let arcs = |times: usize| refusal(&format!("M0 0L1 1z{}", flat.repeat(times)));
    assert_eq!((arcs(MAX_ART_IDLE + 1), arcs(MAX_ART_IDLE + 2)), (None, said.clone()));
    // The run is of one path, and anything drawn between ends it.
    let broken = format!("M0 0L1 1z{}l0 0{}", "z".repeat(MAX_ART_IDLE), flat.repeat(MAX_ART_IDLE));
    assert_eq!(svg_refusal(&svg(&format!(r#"<path d="{broken}"/>"#).repeat(9))), None);
    // What a review found: the file is pinned, and refused before the parser reads it.
    assert_eq!(closes(4000), said);
    // The parser stops at the first segment it cannot read, and so does the door: what follows is not drawn.
    assert_eq!(refusal(&format!("M0 0L1 1z ! {}", "z".repeat(4000))), None);
}

// An arc is made of more curves the larger its radius, all at once, and so are a circle, an ellipse and a rounded
// corner: `r="3e38"` was four million curves from one element. A written number is held to a size, and the paths of
// a file to a count of commands.
#[test]
fn what_a_path_or_a_shape_writes_is_held_to_a_size_and_a_count() {
    assert_eq!((MAX_ART_WRITTEN, MAX_ART_COMMANDS), (1_000_000.0, 2048));
    let large = Some("it writes a number over 1000000".to_string());
    for inside in [
        r#"<circle r="1000001"/>"#,
        r#"<circle r="3e38"/>"#,
        r#"<circle cx="-1e7" r="1"/>"#,
        r#"<ellipse rx="1" ry="1e7"/>"#,
        r#"<rect width="9" height="9" rx="1e7"/>"#,
        r#"<rect width="1e7mm" height="9"/>"#,
        r#"<rect x="1e7" y="1" width="9" height="9"/>"#,
        r#"<line x1="1" y1="1" x2="1e7" y2="2"/>"#,
        r#"<path d="M0 0a1e24 1e24 0 1 1 1 1"/>"#,
        r#"<path d="M0 0L1 1e7"/>"#,
        r#"<path d="M0 0h-1e7"/>"#,
        r#"<path d="M0 0c1 1 2 2 1e7 3"/>"#,
        r#"<polygon points="0,0 1,1 1e7,0"/>"#,
    ] {
        assert_eq!(svg_refusal(&svg(inside)), large, "{inside}");
    }
    // At the size itself a shape is read, and refused later for its reach.
    let far = Some("a shape reaches more than 4096 past the picture's edge".to_string());
    for inside in [r#"<circle r="1000000"/>"#, r#"<path d="M0 0a1e6 1e6 0 1 1 1 1"/>"#] {
        assert_eq!(svg_refusal(&svg(inside)), far, "{inside}");
    }
    // The parser makes a radius larger until the arc reaches its end, by as much as the other radius is larger
    // than it: a flat arc between far ends was 81 curves at radii of 1000000 and 1, and more the flatter. So an
    // arc's radii are within 1000 times of each other, and an arc is then 64 curves at most.
    assert_eq!(MAX_ART_FLAT, 1000.0);
    let flat = Some("an arc's radii are more than 1000 times apart".to_string());
    for radii in ["1e6 1", "1e6 999", "1 1001", "9 1e-4", "-1e6 1"] {
        assert_eq!(svg_refusal(&svg(&format!(r#"<path d="M0 0L9 9a{radii} 45 1 0 1 1z"/>"#))), flat, "{radii}");
    }
    for d in [
        "M0 0A1e6 1e6 0 1 1 1 1",
        "M-1e6 -1e6A1 1 0 1 1 1e6 1e6",
        "M-1e6 1e6A1e6 1e3 45 1 0 1e6 -1e6",
        "M-1e6 1e6A1 1e3 45 1 1 1e6 -1e6",
        "M-1e6 -1e6A1e-3 1 135 0 1 1e6 1e6",
    ] {
        assert!((2..=65).contains(&drawn_for("", d)), "{d}: {}", drawn_for("", d));
        let said = svg_refusal(&svg(&format!(r#"<path d="{d}"/>"#)));
        assert_ne!(said, flat, "{d}");
    }
    // An arc of no radius is a line to the parser, and is taken as one.
    assert_eq!(svg_refusal(&svg(r#"<path d="M0 0L9 9a0 5 0 1 0 1 1a5 0 0 1 0 1 1z"/>"#)), None);
    // An arc's turn and its two flags are not sizes.
    assert_eq!(svg_refusal(&svg(r#"<path d="M0 0a9 9 3000000 1 1 1 1"/>"#)), None);

    let commands = Some("its paths hold more than 2048 commands".to_string());
    let closed = |paths: usize| svg_refusal(&svg(&format!(r#"<path d="M0 0{}"/>"#, "z".repeat(31)).repeat(paths)));
    assert_eq!((closed(64), closed(65)), (None, commands.clone()));
    let points = |threes: usize| {
        svg_refusal(&svg(&format!(r#"<polyline points="{}" fill="none"/>"#, "0,0 9,9 0,9 ".repeat(threes))))
    };
    assert_eq!(points(683), commands, "a pair of `points` is a command");
    assert_eq!(points(682), Some("its paths hold more than 3072 numbers".into()), "the count on the tree is nearer");
}
