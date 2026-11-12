//! What art costs to draw is counted at the door, on the tree the renderer draws: two numbers for every point of
//! its paths, and its fills and strokes times its size. Each count is held at its edge, for one file and for a
//! theme's files together. No GPUI: the counts are the same everywhere.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;

use sticker_kit::theme::{ArtCost, MAX_ART_NUMBERS, MAX_ART_PAINT, MAX_ART_REACH, MIN_ART_STROKE, art_cost};
use sticker_kit::{Mood, Theme, ThemeError, art};

/// One fill, and the 13 points the renderer draws a circle with.
const CIRCLE: &str = r#"<circle r="4"/>"#;

/// A picture `side` wide and high.
fn sized(side: u32, inside: &str) -> String {
    format!(r#"<svg xmlns="http://www.w3.org/2000/svg" width="{side}" height="{side}">{inside}</svg>"#)
}

/// One path of `numbers` numbers: half as many points, round a square.
fn path(numbers: usize) -> String {
    let steps: String = ["h1", "v1", "h-1", "v-1"].into_iter().cycle().take(numbers / 2 - 1).collect();
    format!(r#"<path d="M0 0{steps}"/>"#)
}

/// A picture drawn 256 a side for 8 of its own units, 32 pixels to one.
fn scaled(inside: &str) -> Result<ArtCost, String> {
    let root = r#"<svg xmlns="http://www.w3.org/2000/svg" width="256" height="256" viewBox="0 0 8 8">"#;
    art_cost(&format!("{root}{inside}</svg>"))
}

#[test]
fn an_art_file_is_held_to_its_fills_and_strokes_times_its_size() {
    assert_eq!(MAX_ART_PAINT, 64 * 256 * 256);
    assert_eq!(art_cost(&sized(256, &CIRCLE.repeat(64))), Ok(ArtCost { numbers: 64 * 26, paint: MAX_ART_PAINT }));
    let said = art_cost(&sized(256, &CIRCLE.repeat(65))).unwrap_err();
    assert_eq!(said, "its paint (fills and strokes x width x height, x 4 for a gradient) is 4259840, over 4194304");
    assert_eq!(art_cost(&sized(128, &CIRCLE.repeat(99))).map(|cost| cost.paint), Ok(99 * 128 * 128));
    // A stroke is painted as a fill is, and a shape with neither is not painted.
    let both = r##"<circle r="4" stroke="#123456" stroke-width="2"/>"##;
    assert_eq!(art_cost(&sized(256, &both.repeat(32))).map(|cost| cost.paint), Ok(MAX_ART_PAINT));
    assert!(art_cost(&sized(256, &both.repeat(33))).is_err());
    assert_eq!(art_cost(&sized(256, r#"<circle r="4" fill="none"/>"#)), Ok(ArtCost { numbers: 26, paint: 0 }));

    let stops = r##"<stop offset="0" stop-color="#fff"/><stop offset="1" stop-color="#000"/>"##;
    let gradient = |shapes: usize| {
        let filled = r#"<circle r="4" fill="url(#g)"/>"#.repeat(shapes);
        sized(256, &format!(r#"<linearGradient id="g">{stops}</linearGradient>{CIRCLE}{filled}"#))
    };
    let paint = |shapes: usize| art_cost(&gradient(shapes)).map(|cost| cost.paint);
    assert_eq!(paint(1), Ok(5 * 256 * 256), "a gradient counts four times, where it is painted with");
    assert!(paint(15).is_ok() && paint(16).is_err());

    let rooted = |root: &str| {
        format!(r#"<svg xmlns="http://www.w3.org/2000/svg" {root}>{CIRCLE}<g><rect width="1" height="1"/></g></svg>"#)
    };
    for (root, paint) in [
        (r#"width="10" height="20.5""#, 2 * 10 * 21),
        (r#"viewBox="0 0 64 32""#, 2 * 64 * 32),
        (r#"width="100px" viewBox="0 0 64 32""#, 2 * 100 * 32),
    ] {
        assert_eq!(art_cost(&rooted(root)).map(|cost| cost.paint), Ok(paint), "{root}");
    }
}

#[test]
fn an_art_file_is_held_to_the_points_of_its_paths_whatever_shape_wrote_them() {
    assert_eq!(MAX_ART_NUMBERS, 3072);
    let two = |more: &str| sized(64, &format!("{}{}{more}", path(1536), path(1528)));
    let full = two(r#"<polygon points="1,1 2,2 3,1 2,0"/>"#);
    assert_eq!(art_cost(&full).map(|cost| cost.numbers), Ok(MAX_ART_NUMBERS));
    let over = art_cost(&two(r#"<polygon points="1,1 2,2 3,1 2,0 1,0"/>"#)).unwrap_err();
    assert_eq!(over, "its paths hold more than 3072 numbers");
    // A shape that writes no path is drawn as one, and counted as it is drawn. A circle or a rounded corner far
    // larger than its picture was once free, and slow.
    let numbers = |inside: &str| art_cost(&sized(64, inside)).map(|cost| cost.numbers);
    assert_eq!(numbers(r#"<rect width="4" height="4"/>"#), Ok(8));
    assert_eq!(numbers(r##"<line x2="4" y2="4" stroke="#000"/>"##), Ok(4));
    assert_eq!(numbers(CIRCLE), Ok(26));
    assert_eq!(numbers(r#"<ellipse rx="3" ry="2"/>"#), Ok(26));
    assert_eq!(numbers(r#"<rect width="4" height="4" rx="1"/>"#), Ok(34));
    assert_eq!(numbers(r#"<polyline points="0,0 4,4 0,4"/>"#), Ok(6));
    assert_eq!(numbers(&CIRCLE.repeat(99)), Ok(99 * 26), "a file of nothing but circles is inside the count");
}

// A shape far larger than its picture is walked all along for the little of it that shows: 99 ellipses of 4096
// under a thin stroke took 4.5 s on a picture of 8. A shape is held to a reach, with its stroke, wherever a group
// moves it.
#[test]
fn a_shape_stays_within_reach_of_its_picture_with_its_stroke() {
    assert_eq!(MAX_ART_REACH, 4096.0);
    let far = Err("a shape reaches more than 4096 past the picture's edge".to_string());
    let stroke = r##"fill="#abcdef" fill-opacity="0.2" stroke="#123456" stroke-width="0.03""##;
    for inside in [
        format!(r#"<ellipse cx="4" cy="4" rx="4096" ry="4096" {stroke}/>"#).repeat(99),
        format!(r#"<circle cx="4" cy="4" r="4096" {stroke}/>"#).repeat(30),
        format!(r#"<rect width="4096" height="4096" rx="4096" ry="4096" {stroke}/>"#),
        r##"<line x2="8" y2="8" stroke="#123456" stroke-width="4096" stroke-linecap="round"/>"##.to_string(),
        format!(r#"<polyline points="{}" {stroke}/>"#, "4096,-4096 -4096,4096 ".repeat(150)),
        r#"<circle cx="4" cy="4" r="137"/>"#.to_string(),
        r#"<g transform="translate(140 0)"><circle cx="4" cy="4" r="4"/></g>"#.to_string(),
        r#"<g transform="scale(40)"><circle cx="4" cy="4" r="4"/></g>"#.to_string(),
        r#"<path d="M0 0L1e30 4"/>"#.to_string(),
    ] {
        assert_eq!(scaled(&inside), far, "{inside}");
    }
    for near in [r#"<circle cx="4" cy="4" r="132"/>"#, r#"<g transform="translate(-120 130)"><circle r="4"/></g>"#] {
        assert!(scaled(near).is_ok(), "{near}: {:?}", scaled(near));
    }
}

// A stroke drawn a pixel wide or under is a hairline to the renderer, ten times slower by the line: 511 lines under
// a stroke of 0.016 on this picture took 0.07 s, and 1.64 s under one of 0.015.
#[test]
fn a_stroke_is_no_thinner_than_a_drawn_pixel() {
    assert_eq!(MIN_ART_STROKE, 0.51);
    let lines: String = (1..512).map(|i| format!("L{} {}", (i * 3) % 8, (i % 2) * 8)).collect();
    let stroked = |width: &str, inside: &str| {
        let path = format!(r##"<path d="M0 0{inside}" fill="none" stroke="#123456" stroke-width="{width}"/>"##);
        scaled(&path).map(|cost| cost.numbers)
    };
    for width in ["0.016", "0.03", "1", "8"] {
        assert_eq!(stroked(width, &lines), Ok(1024), "{width}");
    }
    for width in ["0.015", "0.01", "0.0001", "1e-9"] {
        let said = stroked(width, &lines).unwrap_err();
        assert!(said.starts_with("a stroke is dashed, or drawn ") && said.ends_with(", thinner than 0.51"), "{said}");
    }
    assert!(stroked("0.01", &"C1 1 2 2 3 3".repeat(170)).is_err());
    assert_eq!(stroked("0", &lines), Ok(1024), "a stroke of no width is not drawn");
    // The width is the drawn one: a group that makes the picture smaller makes its strokes thinner.
    let grouped = |scale: &str| {
        let line = r##"<path d="M0 0L8 8" stroke="#000" stroke-width="1"/>"##;
        scaled(&format!(r#"<g transform="scale({scale})">{line}</g>"#))
    };
    assert!(grouped("0.02").is_ok() && grouped("0.01").is_err() && grouped("1 0.01").is_err());
}

#[test]
fn a_theme_s_art_files_together_are_held_to_the_same_two_counts() {
    let folder = std::env::temp_dir().join(format!("sticker-kit-cost-{}", std::process::id()));
    fs::create_dir_all(&folder).unwrap();
    let theme = folder.join("theme.toml");
    fs::write(&theme, "[art]\nhero = \"hero.svg\"\nstar = \"star.svg\"\nbadge = \"badge.svg\"\n").unwrap();
    let write = |hero: &str, star: &str, badge: &str| {
        for (name, svg) in [("hero", hero), ("star", star), ("badge", badge)] {
            fs::write(folder.join(format!("{name}.svg")), svg).unwrap();
        }
        Theme::load(&theme).map(|_| ()).map_err(|refused| match refused {
            ThemeError::RefusedSvg { key, reason } => format!("{key}: {reason}"),
            other => panic!("{other:?}"),
        })
    };
    let half = sized(256, &CIRCLE.repeat(32));
    let small = |shapes: usize| sized(16, &CIRCLE.repeat(shapes));
    assert_eq!(write(&half, &small(0), &half), Ok(()));
    let paint = "with the art before it, paint (fills and strokes x width x height, x 4 for a gradient) is 4194560, over 4194304";
    assert_eq!(write(&half, &small(1), &half), Err(format!("art.badge: {paint}")));
    assert_eq!(write(&half, &half, &small(1)), Err(format!("art.badge: {paint}")));

    let paths = |numbers: usize| sized(16, &path(numbers));
    assert_eq!(write(&paths(1536), &paths(1500), &paths(36)), Ok(()));
    let numbers = "with the art before it, paths hold more than 3072 numbers";
    assert_eq!(write(&paths(1536), &paths(1500), &paths(38)), Err(format!("art.badge: {numbers}")));
    assert_eq!(write(&paths(1536), &paths(1538), &small(1)), Err(format!("art.star: {numbers}")));

    // Files changed after the theme was loaded are read again when it is drawn, and held to the same total: a
    // file that would take the theme past it is not given, so the kit's own art is drawn in its place.
    assert_eq!(write(&small(1), &small(1), &small(1)), Ok(()));
    let loaded = Theme::load(&theme).unwrap().theme;
    // One pass gives every piece: each file is read once and there is one sum, so no file can be changed between
    // the reading of one piece and the summing of the next.
    let given = |loaded: &Theme| {
        let pieces: Vec<&str> = loaded.art_svgs().into_iter().map(|(piece, _)| piece).collect();
        ["hero", "star", "badge"].map(|key| pieces.contains(&key))
    };
    assert_eq!(given(&loaded), [true, true, true]);
    let full = sized(256, &CIRCLE.repeat(64));
    for name in ["hero", "star"] {
        fs::write(folder.join(format!("{name}.svg")), &full).unwrap();
    }
    assert_eq!(given(&loaded), [true, false, false], "each alone is inside the counts; together they are not");
    fs::write(folder.join("hero.svg"), paths(3072 - 26)).unwrap();
    assert_eq!(given(&loaded), [true, false, true], "a file past the total is passed over, not counted");
    fs::write(folder.join("badge.svg"), paths(28)).unwrap();
    assert_eq!(given(&loaded), [true, false, false], "the numbers are summed as the paint is");
    let _ = fs::remove_dir_all(folder);
}

/// The same two counts for the kit's own art, which is built in code and does not come through the door (it blurs).
/// Counted here as the door counts, on the renderer's tree: every point, and every fill and stroke.
fn counted(svg: &str) -> ArtCost {
    fn walk(group: &usvg::Group, cost: &mut ArtCost) {
        for node in group.children() {
            match node {
                usvg::Node::Group(group) => walk(group, cost),
                usvg::Node::Path(path) => {
                    cost.numbers += 2 * path.data().points().len();
                    let fill = path.fill().map(usvg::Fill::paint);
                    let paints = fill.into_iter().chain(path.stroke().map(usvg::Stroke::paint));
                    cost.paint +=
                        paints.map(|paint| if matches!(paint, usvg::Paint::Color(_)) { 1 } else { 4 }).sum::<u64>();
                }
                _ => {}
            }
        }
    }
    let tree = usvg::Tree::from_str(svg, &usvg::Options::default()).unwrap();
    let mut cost = ArtCost::default();
    walk(tree.root(), &mut cost);
    let area = tree.size().width().ceil() as u64 * tree.size().height().ceil() as u64;
    ArtCost { numbers: cost.numbers, paint: cost.paint * area }
}

#[test]
fn the_kit_s_own_art_is_inside_each_count() {
    for theme in [Theme::sticker(), Theme::plain()] {
        let (p, m) = (&theme.palette, &theme.metrics);
        let pieces = [
            art::character(p, theme.hue(0), Mood::Happy, false, m.hero_size, m.rim),
            art::character(p, theme.hue(1), Mood::Cross, true, m.character_size, m.rim),
            art::star(p, 10.0, m.rim),
            art::bubble(p, 11.0, m.rim),
            art::badge(p, 15.0, m.rim),
        ];
        let all = pieces.map(|svg| counted(&svg)).iter().fold(ArtCost::default(), |all, one| ArtCost {
            numbers: all.numbers + one.numbers,
            paint: all.paint + one.paint,
        });
        // A theme that draws what the kit draws, without its blur, is one the door takes: 74% and 57% of the counts.
        assert_eq!(all, ArtCost { numbers: 2260, paint: 2_388_256 }, "{}", theme.name);
        assert!(all.numbers < MAX_ART_NUMBERS && all.paint < MAX_ART_PAINT);
    }
}
