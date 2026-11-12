//! The theme door's numbers, each held at its edge: the last value inside loads and the first one outside is refused.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;

use sticker_kit::theme::{
    MAX_ART_ATTRIBUTES, MAX_ART_BYTES, MAX_ART_COMMANDS, MAX_ART_FLAT, MAX_ART_IDLE, MAX_ART_PATH_BYTES,
    MAX_ART_WRITTEN, MAX_THEME_BYTES, art_cost, svg_refusal,
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

// The door has no reading of a number of its own. A unit or an exponent is what the renderer takes it for, and
// the picture is held at the size that comes of it.
#[test]
fn a_number_with_a_unit_is_what_the_renderer_takes_it_for() {
    let sized = |size: &str| {
        let rect = r#"<rect width="0.25in" height="1e1"/>"#;
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
// made of no curve, unless it follows a close (a move is drawn first). The door holds a run of them, and of no
// arc that is drawn: a scalloped edge is forty of those in a row.
#[test]
fn a_run_of_segments_the_parser_draws_nothing_for_is_held_to_a_length() {
    assert_eq!(MAX_ART_IDLE, 32);
    let flat = "a1 1 0 0 0 1e-17 0";
    // What is written before, the segment, whether the parser draws nothing for it, and whether the door counts it.
    let table = [
        ("M0 0L1 1z", "z", true, true),
        ("M0 0", flat, true, true),
        // An arc whose ends are closer than a millionth of its radius is taken for one: the parser's own line is
        // nearer 1e-16, and the door does not do an arc's sums.
        ("M0 0", "a1 1 0 0 0 1e-7 0", false, true),
        ("M0 0", "a9 2 0 0 0 0 8e-6", false, true),
        ("M5 5H6V7", "A1e3 1e3 0 0 0 6.0001 7.0001A1e3 1e3 0 0 0 6 7", false, true),
        // An arc that goes somewhere is drawn, and is not counted.
        ("M0 0", "a1 1 0 0 0 1 1", false, false),
        ("M0 0", "a1 1 0 0 0 2e-6 0", false, false),
        ("M0 0", "A1 1 0 0 0 1 1", false, false),
        ("M0 0", "A1 1 0 0 0 1 1A1 1 0 0 0 0 0", false, false),
        // An arc of no radius, or to where it starts, is a line to the parser.
        ("M0 0", "a0 0 0 0 0 1 1", false, false),
        ("M0 0", "a1 1 0 0 0 0 0", false, false),
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
    let said = Some(
        "a path holds more than 32 closes, or arcs that go nowhere, in a row, which the parser draws nothing for"
            .to_string(),
    );
    for (before, segment, nothing, held) in table {
        assert_eq!(drawn_for(before, segment) == 0, nothing, "{before} {segment}");
        assert!(held || !nothing, "{before} {segment}: drawn as nothing, and not held");
        let run = |times: usize| refusal(&format!("{before}{}", segment.repeat(times)));
        // A segment of two arcs is two of a run.
        let each = segment.matches(['a', 'A', 'z']).count().max(1);
        if held {
            let (most, over) = (MAX_ART_IDLE / each, MAX_ART_IDLE / each + 1);
            assert_eq!((run(most), run(over)), (None, said.clone()), "{before} {segment}");
        } else if segment != "z" && segment != flat {
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
    assert_eq!(refusal("M0 0L1 1 L ! 1e9 L1e9 1e9 a1e9 1 0 0 0 1 1"), None);
}

// What a review found refused when every arc was counted, each an ordinary shape for a sticker: a badge of forty
// scallops, a cloud of 33 bumps and a wave of 36 half turns. Every arc of them is drawn.
#[test]
fn a_scalloped_edge_is_a_run_of_arcs_that_are_drawn_and_is_taken() {
    let badge = format!("M20 2{}z", "a1.5 1.5 0 0 1 2 1".repeat(40));
    let cloud = format!("M4 30{}z", "a0.5 0.5 0 0 1 1 0".repeat(33));
    let wave = format!("M2 20{}", "a0.5 0.5 0 0 1 1 0a0.5 0.5 0 0 0 1 0".repeat(18));
    for d in [&badge, &cloud, &wave] {
        let arcs = d.matches('a').count();
        assert!(arcs > MAX_ART_IDLE && drawn_for("", d) > arcs, "{d}");
        assert_eq!(svg_refusal(&svg(&format!(r#"<path d="{d}"/>"#))), None, "{d}");
    }
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
        r#"<circle cy="1e7" r="1"/>"#,
        r#"<rect y="-1e7" width="9" height="9"/>"#,
        r#"<rect width="9" height="1e7"/>"#,
        r#"<rect width="9" height="9" ry="1e7"/>"#,
        r#"<ellipse rx="1e7" ry="1"/>"#,
        r#"<line x1="1e7" y1="1" x2="2" y2="2"/>"#,
        r#"<line x1="1" y1="1e7" x2="2" y2="2"/>"#,
        r#"<line x1="1" y1="1" x2="2" y2="-1e7"/>"#,
        r#"<path d="M0 0c1e7 1 2 2 3 3"/>"#,
        r#"<path d="M0 0c1 1e7 2 2 3 3"/>"#,
        r#"<path d="M0 0c1 1 2 1e7 3 3"/>"#,
        r#"<path d="M0 0s1e7 1 2 2"/>"#,
        r#"<path d="M0 0q1 1e7 2 2"/>"#,
        r#"<path d="M0 0v1e7"/>"#,
        r##"<path d="M0 0h9" stroke="#123" stroke-width="1000001"/>"##,
        r##"<g transform="scale(1e-6)"><path d="M0 0c1e6 0 0 1e6 1e6 1e6" stroke="#123" stroke-width="2e9"/></g>"##,
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
    let commands = Some("its paths hold more than 2048 commands".to_string());
    let closed = |paths: usize| svg_refusal(&svg(&format!(r#"<path d="M0 0{}"/>"#, "z".repeat(31)).repeat(paths)));
    assert_eq!((closed(64), closed(65)), (None, commands.clone()));
    let points = |threes: usize| {
        svg_refusal(&svg(&format!(r#"<polyline points="{}" fill="none"/>"#, "0,0 9,9 0,9 ".repeat(threes))))
    };
    assert_eq!(points(683), commands, "a pair of `points` is a command");
    assert_eq!(points(682), Some("its paths hold more than 3072 numbers".into()), "the count on the tree is nearer");
}

// A `viewBox` is what every percent is a part of, and what the picture is scaled by. PR 147's review: a `viewBox`
// of 1e36 with `rx="100%"` was a second for one ellipse, and no written number of it was over the limit.
#[test]
fn a_view_box_is_four_written_numbers_and_no_shape_is_a_part_of_it() {
    let large = Some("it writes a number over 1000000".to_string());
    // One the parser cannot read is none to it.
    let view = |view: &str| {
        svg_refusal(&format!(r#"<svg xmlns="http://www.w3.org/2000/svg" width="8" height="8" viewBox="{view}"/>"#))
    };
    for over in ["1e7 0 8 8", "0 -1e7 8 8", "0 0 1e36 8", "0 0 8 1000001", "0,0,1e36,1e36"] {
        assert_eq!(view(over), large, "{over}");
    }
    assert_eq!((view("-1e6 1e6 1e6 1e6"), view("1e7 0 8"), view("0 0 1e7 -8")), (None, None, None));
    // Only the picture's own size may be a percent, up to all of what holds it.
    for (inside, name) in [
        (r#"<ellipse rx="100%" ry="1"/>"#, "rx"),
        (r#"<ellipse rx="1" ry="1%"/>"#, "ry"),
        (r#"<rect width="9" height="9" rx="50%"/>"#, "rx"),
        (r#"<rect width="50%" height="9"/>"#, "width"),
        (r#"<circle cx="50%" r="1"/>"#, "cx"),
        (r##"<path d="M0 0h9" stroke="#123" stroke-width="1%"/>"##, "stroke-width"),
    ] {
        assert_eq!(svg_refusal(&svg(inside)), Some(format!("its `{name}` is a percent")), "{inside}");
    }
    let whole =
        |size: &str| svg_refusal(&format!(r#"<svg xmlns="http://www.w3.org/2000/svg" {size} viewBox="0 0 8 8"/>"#));
    assert_eq!(whole(r#"width="100%" height="50%""#), None, "the picture's own size is a part of what holds it");
    assert_eq!(svg_refusal(&svg(r#"<g width="100%"/>"#)), Some("its `width` is a percent".into()), "and no other");
    assert_eq!(whole(r#"width="101%" height="8""#), Some("its `width` is a percent".into()));
    assert_eq!(whole(r#"width="8" height="-1%""#), Some("its `height` is a percent".into()));
}

// The parser makes a radius larger until the arc reaches its end, by as much as the other radius is larger than
// it: a flat arc between far ends was 81 curves at radii of 1000000 and 1, and more the flatter. So an arc's radii
// are within 1000 times of each other. That is no count of an arc's curves: the ends of an arc are as far apart as
// the moves before it add up to, and the most a review found is pinned here. The count of commands and the trial
// hold what a path comes to.
#[test]
fn the_radii_of_an_arc_are_held_to_each_other() {
    assert_eq!(MAX_ART_FLAT, 1000.0);
    let far = format!("M0 0{}", "m1e6 1e6".repeat(2040));
    assert_eq!(drawn_for(&far, "A1e-3 1 45 0 1 -1e6 -1e6"), 80, "the most curves of one arc anyone has written");
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
}

// The XML parser compares every attribute and every namespace declaration of an element with every other. A root of
// 64 KiB of declarations was accepted with nothing to count, and took a third of a second to read and more to draw.
// So the `=` between one `<` and the next are counted on the text, before that parser reads it.
#[test]
fn the_attributes_one_element_writes_are_held_to_a_count_before_the_xml_is_read() {
    assert_eq!(MAX_ART_ATTRIBUTES, 40);
    let crowded = Some("an element writes more than 40 attributes".to_string());
    let declared = |count: usize| {
        let names: String = (0..count).map(|nth| format!(r#" xmlns:n{nth}="u""#)).collect();
        svg_refusal(&format!(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 8 8"{names}/>"#))
    };
    assert_eq!((declared(MAX_ART_ATTRIBUTES - 2), declared(MAX_ART_ATTRIBUTES - 1)), (None, crowded.clone()));
    // The two pictures a review measured: 64 KiB of declarations, and of attributes nobody knows.
    let started = std::time::Instant::now();
    assert_eq!(declared(5000), crowded);
    let unknown: String = (0..9000).map(|nth| format!(r#" a{nth}="1""#)).collect();
    assert_eq!(svg_refusal(&svg(&format!("<rect{unknown}/>"))), crowded);
    eprintln!("two crowded elements refused in {:?}", started.elapsed());
    // Every element has its own count, and one may write every attribute there is.
    let every = r##"<path id="a" fill="#123" stroke="#123" stop-color="#123" fill-rule="evenodd" stroke-linecap="round" stroke-linejoin="round" gradientUnits="userSpaceOnUse" version="1.1" width="1" height="1" x="1" y="1" x1="1" y1="1" x2="1" y2="1" cx="1" cy="1" r="1" rx="1" ry="1" d="M0 0h9v9z" points="0,0" transform="scale(1)" gradientTransform="scale(1)" fill-opacity="1" stroke-width="1" stroke-opacity="1" offset="0" stop-opacity="1"/>"##;
    assert_eq!(svg_refusal(&svg(&every.repeat(3))), None);
    // A `=` in a value or in a title is counted with its element's: the count errs to the refusal.
    let title = |signs: usize| svg_refusal(&svg(&format!("<title>{}</title>", "=".repeat(signs))));
    assert_eq!((title(MAX_ART_ATTRIBUTES), title(MAX_ART_ATTRIBUTES + 1)), (None, crowded));
}

// The renderer draws a picture at twice its size cut to whole pixels: each side by itself has to come to one.
#[test]
fn each_side_of_a_picture_is_held_to_the_smallest_by_itself() {
    let sized = |wide: &str, high: &str| {
        let root = format!(r#"<svg xmlns="http://www.w3.org/2000/svg" width="{wide}" height="{high}">"#);
        svg_refusal(&format!(r#"{root}<rect width="1" height="1"/></svg>"#))
    };
    assert_eq!(sized("0.5", "0.5"), None);
    assert_eq!(sized("8", "0.4"), Some("the renderer sizes it 8 by 0.4: a picture is at least 0.5 a side".into()));
    assert_eq!(sized("0.4", "8"), Some("the renderer sizes it 0.4 by 8: a picture is at least 0.5 a side".into()));
}
