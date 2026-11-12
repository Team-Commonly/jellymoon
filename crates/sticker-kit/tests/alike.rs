//! The door reads a picture with the renderer's parser, so there is one reading of a file and not two. Each case
//! here is a picture the door's own reading once took another way than the renderer. It is held against `usvg`
//! called as GPUI calls it: whatever the door takes, it counted at the renderer's size and point for point. No
//! GPUI: `tests/art_draw.rs` rasterises the same and compares the sizes.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use sticker_kit::theme::{ArtCost, MAX_ART_SIDE, art_cost};
use usvg::{Group, Node, Options, Tree};

/// The picture's size and the points of its paths, as the renderer reads it: from the bytes, a DTD allowed.
fn renderer(svg: &str) -> Option<(f32, f32, usize)> {
    fn points(group: &Group) -> usize {
        let of = |node: &Node| match node {
            Node::Group(group) => points(group),
            Node::Path(path) => path.data().points().len(),
            _ => 0,
        };
        group.children().iter().map(of).sum()
    }
    let tree = Tree::from_data(svg.as_bytes(), &Options::default()).ok()?;
    Some((tree.size().width(), tree.size().height(), points(tree.root())))
}

/// What the door says of a picture. One it takes, the renderer reads at that size and with those points.
fn door(svg: &str) -> Result<ArtCost, String> {
    let cost = art_cost(svg);
    if let Ok(cost) = &cost {
        let (wide, high, points) = renderer(svg).expect("the renderer reads what the door takes");
        assert!(wide <= MAX_ART_SIDE && high <= MAX_ART_SIDE, "{svg}: taken at {wide} by {high}");
        assert_eq!(cost.numbers, 2 * points, "{svg}");
        let area = wide.ceil() as u64 * high.ceil() as u64;
        assert!(cost.paint == 0 || cost.paint >= area, "{svg}: paint on another size");
    }
    cost
}

/// What a fill of 8 by 8 costs on a picture of 8 by 8: its 64 pixels, and 16 for each of the 16 rows its two
/// sides cross. A triangle on the same picture crosses as many.
const RECT: u64 = 64 + 16 * 16;

fn rooted(root: &str) -> String {
    format!(r#"<svg xmlns="http://www.w3.org/2000/svg" {root}><circle cx="4096" cy="4096" r="4096"/></svg>"#)
}

fn path(d: &str) -> Result<usize, String> {
    let svg = format!(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64"><path d="{d}"/></svg>"#);
    door(&svg).map(|cost| cost.numbers)
}

// The door's own reader took `<?XML ` for a declaration and read on inside it, where the renderer reads an
// instruction as one piece up to `?>`. The door counted the picture written inside the instruction, 8 by 8 and
// empty, and the renderer drew the one after it: 8192 pixels a side, 9.7 s.
#[test]
fn a_picture_inside_a_processing_instruction_is_not_the_one_the_door_reads() {
    let hidden = r#"<?XML <svg viewBox="0 0 8 8"/> <!-- ?><svg xmlns="http://www.w3.org/2000/svg"><circle cx="4096" cy="4096" r="4096"/></svg><!-- -->"#;
    let said = door(hidden).unwrap_err();
    assert_eq!(said, "the renderer sizes it 8192 by 8192: a picture is at most 256 a side");
    let honest = |inside: &str| {
        let root = r#"<svg xmlns="http://www.w3.org/2000/svg" width="256" height="256" viewBox="0 0 8 8">"#;
        door(&format!("{root}<?XML <!-- ?>{inside}<!-- --></svg>")).unwrap_err()
    };
    assert_eq!(honest(&r#"<circle cx="4" cy="4" r="4096"/>"#.repeat(600)), "it has more than 100 elements");
    assert_eq!(honest(r#"<text x="1" y="5">hi</text>"#), "it contains the element `<text`");
    assert_eq!(honest(r#"<image href="a.png" width="4" height="4"/>"#), "it contains the element `<image`");
    assert_eq!(honest(r#"<circle r="4096"/>"#), "a shape reaches more than 4096 past the picture's edge");
    for name in ["xml", "XML", "Xml", "php"] {
        let inside = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 8 8"><?{name} x ?><rect width="8" height="8"/></svg>"#
        );
        assert_eq!(door(&inside).map(|cost| cost.paint).ok(), renderer(&inside).map(|_| RECT), "{name}");
    }
}

// The renderer reads a name only as SVG writes it. A picture that wrote `viewbox` was counted as 8 by 8 and drawn
// as large as what it held. Now a name is on the list as SVG writes it, or it is refused.
#[test]
fn a_name_in_another_case_than_svg_writes_it_is_refused() {
    for (root, name) in [
        (r#"viewbox="0 0 8 8""#, "viewbox"),
        (r#"VIEWBOX="0 0 8 8""#, "VIEWBOX"),
        (r#"WIDTH="8" HEIGHT="8""#, "WIDTH"),
        (r#"width="8" Height="8""#, "Height"),
        (r#"viewBox="0 0 256 256" viewbox="0 0 8 8""#, "viewbox"),
    ] {
        assert_eq!(door(&rooted(root)), Err(format!("it contains the attribute `{name}`")), "{root}");
    }
    let inside =
        |inside: &str| door(&format!(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 8 8">{inside}</svg>"#));
    assert_eq!(inside(r#"<Circle r="4"/>"#), Err("it contains the element `<Circle`".into()));
    assert_eq!(inside(r#"<lineargradient id="g"/>"#), Err("it contains the element `<lineargradient`".into()));
    assert_eq!(inside(r#"<circle R="4"/>"#), Err("it contains the attribute `R`".into()));
    let units = r#"<radialGradient id="g" gradientunits="userSpaceOnUse"/>"#;
    assert_eq!(inside(units), Err("it contains the attribute `gradientunits`".into()));
    let own = r##"<linearGradient id="g" gradientUnits="userSpaceOnUse" gradientTransform="rotate(4)" x2="8"><stop offset="0" stop-color="#fff"/><stop offset="1" stop-color="#000"/></linearGradient><path d="M0 0L8 8H0Z" fill="url(#g)"/>"##;
    assert_eq!(inside(own), Ok(ArtCost { numbers: 6, paint: 4 * RECT }), "the names with a capital");
}

// The renderer takes a `viewBox` it cannot read for none at all, and then sizes the picture to what is in it. The
// door has no reading of its own of a `viewBox`: every way of writing one is sized as the renderer sizes it.
#[test]
fn a_view_box_however_written_is_sized_as_the_renderer_sizes_it() {
    let odd = ["- - 8 8", "0.0.0.0 0 8 8", "0 0.256.256 8 8", "0,,0 8 8", "0 0 -8 8", "0 0 8", "0 0 0 8", "x"];
    for view in odd {
        let said = door(&rooted(&format!(r#"viewBox="{view}""#))).unwrap_err();
        let sized = said.starts_with("the renderer sizes it 8192 by 8192") || said.contains("the renderer reads");
        // The renderer reads `0 0.256.256 8 8` as four numbers, the second and third under 1: a picture 0.256 wide,
        // which it has no pixel for.
        assert!(sized || said.ends_with("a picture is at least 0.5 a side"), "{view}: {said}");
    }
    let read =
        ["0 0 8 8", "0,0,8,8", "0, 0, 8, 8", "0 0  8 8", " 0 0 8 8 ", "0 0 8. 8", "0 0 .8e1 8", "+0 0 8 8", "0\t0 8 8"];
    for view in read {
        let svg =
            format!(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="{view}"><rect width="8" height="8"/></svg>"#);
        assert_eq!(door(&svg), Ok(ArtCost { numbers: 8, paint: RECT }), "{view}");
    }
    // A file that states an attribute twice is not XML, to the renderer or to the door.
    for root in [r#"viewBox="0 0 256 256" viewBox="0 0 8 8""#, r#"width="8" height="8" width="9""#] {
        let said = door(&rooted(root)).unwrap_err();
        assert!(said.starts_with("it is not XML the renderer reads: ") && renderer(&rooted(root)).is_none(), "{said}");
    }
    // A side the picture does not state is the `viewBox`'s, and with no `viewBox` as large as what is in it.
    for (root, side) in [(r#"width="8""#, 8192), (r#"height="8""#, 8192), ("", 8192)] {
        let said = door(&rooted(root)).unwrap_err();
        assert!(said.contains(&side.to_string()), "{root}: {said}");
    }
    let small = r#"<svg xmlns="http://www.w3.org/2000/svg"><rect x="10" y="10" width="30" height="20"/></svg>"#;
    assert!(door(small).is_ok(), "a picture that states no size is counted at the size it is drawn");
}

// The renderer reads an arc's flags as one character each, so `11 2` is to it two flags and a 2, and to a reader
// of numbers an 11 and a 2: 51 arcs passed the door's own reader for 36. The count is now the renderer's points.
#[test]
fn a_path_is_counted_as_the_renderer_reads_it() {
    let apart = path("M0 0a4 4 0 1 1 2 2").unwrap();
    for d in ["M0 0a4 4 0 11 2 2", "M0 0a4 4 0 112 2", "M0 0a4 4 0 1,1,2,2", "M0 0A4 4 0 1 1 2 2"] {
        assert_eq!(path(d), Ok(apart), "{d}");
    }
    // Every other way of writing a flag, a number or a subpath, read however the renderer reads it: `door`
    // compares each point for point.
    for d in [
        "M0 0a4 4 0 01 2 2",
        "M0 0a4 4 0 1.0 1 2 2",
        "M0 0a4 4 0 +1 1 2 2",
        "M0 0a4 4 0 2 1 2 2L9 9",
        "M0 0a4 4 0 0.5 1 2 2",
        "M0 0a4 4 0 -1 1 2 2",
        "M0 0A4 4 0 012 2",
        "M50 50 0 0zl20 20",
        "M40 0h-40zh10h10",
        "5 5L1 1",
        "M0 0z5 5",
        "M0 0 # L1 1",
        "M0 0L4in 4",
        "M0 0L1e1 4.5.5-3T9 9t1 1",
        "M0 0Q0 30 9 9T20 0 0 0 20 0 0 0",
    ] {
        let _ = path(d);
    }
    assert_eq!(path("M0 0L8 8 # L1 1"), Ok(4), "the renderer draws a path up to what it cannot read");
}

// What is left different between the door's reading and the renderer's is what the door turns off: a DTD, which the
// renderer allows and the door refuses, so no entity is expanded.
#[test]
fn a_file_with_a_dtd_is_refused_though_the_renderer_would_read_it() {
    let entity = r#"<!DOCTYPE svg [<!ENTITY c "<circle r='4'/>">]><svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 8 8">&c;</svg>"#;
    assert!(renderer(entity).is_some_and(|(wide, _, points)| wide == 8.0 && points == 13), "the renderer expands it");
    let said = door(entity).unwrap_err();
    assert!(said.starts_with("it is not XML the renderer reads: ") && !said.contains('\n'), "{said}");
    let plain = r#"<!DOCTYPE svg><svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 8 8"><circle r="4"/></svg>"#;
    assert!(door(plain).is_err() && renderer(plain).is_some());
    // A character reference is read by the one parser, so it is what the renderer reads too.
    let referred =
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 8 8"><rect width="&#56;" height="8"/></svg>"#;
    assert_eq!(door(referred).map(|cost| cost.paint), Ok(RECT));
}

// The door's parser is the renderer's only while the two are one package. The kit asks for the version GPUI builds
// and no other (`Cargo.toml`), so a GPUI that moves to another leaves two in the lock file, and this fails.
#[test]
fn the_door_and_the_renderer_build_one_version_of_the_parser() {
    let lock = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("Cargo.lock");
    let lock = std::fs::read_to_string(lock).unwrap();
    let versions = |name: &str| -> Vec<&str> {
        let named = format!("name = \"{name}\"");
        let packages = lock.split("[[package]]").map(|package| package.lines().filter(|line| !line.is_empty()));
        packages.filter_map(|mut lines| (lines.next()? == named).then(|| lines.next()).flatten()).collect()
    };
    assert_eq!(versions("usvg"), ["version = \"0.46.0\""], "one parser, the one the door was measured on");
    // A kept answer is for the parser it was given by: another version tries every picture again.
    assert_eq!(versions("usvg"), [format!("version = \"{}\"", sticker_kit::theme::USVG)]);
    assert_eq!(versions("resvg"), versions("usvg"), "the renderer draws with the parser's own version");
    // The door walks a path with the library the parser reads one with, before the parser does (`svg_text.rs`).
    assert_eq!(versions("svgtypes"), ["version = \"0.16.2\""], "one reader of a path, the one whose calls were read");
}
