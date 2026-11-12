//! What art costs to draw is counted at the door, on the tree the renderer draws: two numbers for every point of
//! its paths, and its fills and strokes times its size. Each count is held at its edge, for one file and for a
//! theme's files together. No GPUI: the counts are the same everywhere.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;

use sticker_kit::theme::{
    ArtCost, MAX_ART_NUMBERS, MAX_ART_PAINT, MAX_ART_REACH, MIN_ART_SIDE, MIN_ART_STROKE, ROW_PAINT, art_cost,
};
use sticker_kit::{Mood, Theme, ThemeError, art};

/// One fill, and the 13 points the renderer draws a circle with. It is above the picture, so no edge of it crosses
/// a row: it costs its fill and nothing more.
const CIRCLE: &str = r#"<circle cy="-9" r="4"/>"#;

/// How the door says what paint is.
const PAINT: &str = "fills and strokes x width x height, and 16 for each row an edge crosses; x 4 for a gradient";

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
    assert_eq!(said, format!("its paint ({PAINT}) is 4259840, over 4194304"));
    assert_eq!(art_cost(&sized(128, &CIRCLE.repeat(99))).map(|cost| cost.paint), Ok(99 * 128 * 128));
    // A stroke is painted as a fill is, and a shape with neither is not painted.
    let both = r##"<circle cy="-9" r="4" stroke="#123456" stroke-width="2"/>"##;
    assert_eq!(art_cost(&sized(256, &both.repeat(32))).map(|cost| cost.paint), Ok(MAX_ART_PAINT));
    assert!(art_cost(&sized(256, &both.repeat(33))).is_err());
    assert_eq!(art_cost(&sized(256, r#"<circle r="4" fill="none"/>"#)), Ok(ArtCost { numbers: 26, paint: 0 }));
    // A side is at least half a pixel: under it the renderer has no pixel to draw, and refuses the picture.
    assert_eq!(MIN_ART_SIDE, 0.5);
    let tiny = |side: &str| {
        let root = format!(r#"<svg xmlns="http://www.w3.org/2000/svg" width="{side}" height="30">"#);
        art_cost(&format!(r#"{root}<rect y="-9" width="30" height="2"/></svg>"#)).map(|cost| cost.paint)
    };
    assert_eq!(tiny("0.5"), Ok(30));
    for side in ["0.49", "1e-9"] {
        let said = tiny(side).unwrap_err();
        assert!(said.ends_with(" by 30: a picture is at least 0.5 a side"), "{said}");
    }

    let stops = r##"<stop offset="0" stop-color="#fff"/><stop offset="1" stop-color="#000"/>"##;
    let gradient = |shapes: usize| {
        let filled = r#"<circle cy="-9" r="4" fill="url(#g)"/>"#.repeat(shapes);
        sized(256, &format!(r#"<linearGradient id="g">{stops}</linearGradient>{CIRCLE}{filled}"#))
    };
    let paint = |shapes: usize| art_cost(&gradient(shapes)).map(|cost| cost.paint);
    assert_eq!(paint(1), Ok(5 * 256 * 256), "a gradient counts four times, where it is painted with");
    assert!(paint(15).is_ok() && paint(16).is_err());

    let rooted = |root: &str| {
        format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" {root}>{CIRCLE}<g><rect y="-2" width="1" height="1"/></g></svg>"#
        )
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
        r#"<path d="M0 0L137 4"/>"#.to_string(),
        // Past the bottom alone, and past the top alone.
        r#"<rect y="130" width="8" height="7"/>"#.to_string(),
        r#"<rect y="-129" width="8" height="7"/>"#.to_string(),
    ] {
        assert_eq!(scaled(&inside), far, "{inside}");
    }
    // A stroke reaches as far as the path's box and its drawn width, whatever a group does to it. Under a slant
    // the renderer's own box of a stroke is the path's and no more, and this was taken: 64 short lines under a
    // stroke a million pixels wide.
    for slant in ["matrix(1000 0.0001 0 1000 0 0)", "matrix(1000 0 0.0001 1000 0 0)", "rotate(0.001) scale(1000)"] {
        let slanted = sized(256, &format!(r#"<g transform="{slant}">{}</g>"#, WIDE.repeat(64)));
        assert_eq!(art_cost(&slanted), far, "{slant}");
    }
    // A mitre's point is as far from the path as two of the stroke's widths, and no farther.
    let wide = |width: u32| {
        scaled(&format!(r##"<path d="M0 0L8 8" fill="none" stroke="#000" stroke-width="{width}"/>"##)).is_ok()
    };
    assert!(wide(64) && !wide(65));
    assert!(scaled(r#"<rect y="129" width="8" height="7"/>"#).is_ok());
    for near in [r#"<circle cx="4" cy="4" r="132"/>"#, r#"<g transform="translate(-120 130)"><circle r="4"/></g>"#] {
        assert!(scaled(near).is_ok(), "{near}: {:?}", scaled(near));
    }
}

/// A short line under a stroke a thousand wide.
const WIDE: &str = r##"<path d="M0.1 0.1L0.15 0.15" fill="none" stroke="#123456" stroke-opacity="0.3" stroke-width="1000" stroke-linecap="round"/>"##;

// A stroke drawn a pixel wide or under is a hairline to the renderer, ten times slower by the line: 511 lines under
// a stroke of 0.016 on this picture took 0.07 s, and 1.64 s under one of 0.015. (504 lines as tall as the picture
// are now all the paint of a file.)
#[test]
fn a_stroke_is_no_thinner_than_a_drawn_pixel() {
    assert_eq!(MIN_ART_STROKE, 0.51);
    let lines: String = (1..505).map(|i| format!("L{} {}", (i * 3) % 8, (i % 2) * 8)).collect();
    let stroked = |width: &str, inside: &str| {
        let path = format!(r##"<path d="M0 0{inside}" fill="none" stroke="#123456" stroke-width="{width}"/>"##);
        scaled(&path).map(|cost| cost.numbers)
    };
    for width in ["0.016", "0.03", "1", "8"] {
        assert_eq!(stroked(width, &lines), Ok(1010), "{width}");
    }
    for width in ["0.015", "0.01", "0.0001", "1e-9"] {
        let said = stroked(width, &lines).unwrap_err();
        assert!(said.starts_with("a stroke is dashed, or drawn ") && said.ends_with(", thinner than 0.51"), "{said}");
    }
    assert!(stroked("0.01", &"C1 1 2 2 3 3".repeat(170)).is_err());
    assert_eq!(stroked("0", &lines), Ok(1010), "a stroke of no width is not drawn");
    // The width is the drawn one: a group that makes the picture smaller makes its strokes thinner.
    let grouped = |scale: &str| {
        let line = r##"<path d="M0 0L8 8" stroke="#000" stroke-width="1"/>"##;
        scaled(&format!(r#"<g transform="scale({scale})">{line}</g>"#))
    };
    assert!(grouped("0.02").is_ok() && grouped("0.01").is_err() && grouped("1 0.01").is_err());
    // The edge itself, on a picture drawn a pixel to a unit.
    let line = |width: &str| {
        art_cost(&sized(64, &format!(r##"<path d="M0 0L8 8" stroke="#000" stroke-width="{width}"/>"##))).is_ok()
    };
    assert_eq!(["0.51", "0.509", "0.5", "0.49"].map(line), [true, false, false, false]);
}

/// The stops of a gradient named `g`.
const GRADIENT: &str = r##"<linearGradient id="g" x2="0.02" gradientTransform="rotate(33)"><stop offset="0" stop-color="#f00"/><stop offset="0.5" stop-color="#0f0"/><stop offset="1" stop-color="#00f"/></linearGradient>"##;

/// `points` points from the left of a picture of 256 to its right, each at the top or the bottom in turn: every
/// edge crosses every row.
fn ragged(points: usize) -> String {
    (0..points).map(|i| format!("{:.2},{} ", i as f32 * 256.0 / points as f32, (i % 2) * 256)).collect()
}

// The renderer walks every edge of a shape down every row it crosses, and paints a row in the pieces the edges
// cut it into. A fill was counted by the picture's area alone, so eight ragged shapes as tall as the picture were
// all the paint of a file and no more, and took 7.6 s to draw. An edge is counted by the rows it crosses.
#[test]
fn a_fill_and_a_stroke_are_counted_by_the_rows_their_edges_cross() {
    assert_eq!(ROW_PAINT, 16);
    let area = 256 * 256;
    let paint = |inside: &str| art_cost(&sized(256, &format!("{GRADIENT}{inside}"))).map(|cost| cost.paint);
    // Down 256 rows, up 256, and no row to close it.
    assert_eq!(paint(r#"<path d="M0 0L8 256L16 0z"/>"#), Ok(area + 16 * 512));
    assert_eq!(paint(r#"<path d="M0 0L8 256L16 0"/>"#), Ok(area + 16 * 512));
    assert_eq!(paint(r#"<path d="M0 0L8 256"/>"#), Ok(area + 16 * 512), "a fill closes what is left open");
    let along = r##"<path d="M0 9h200" fill="none" stroke="#123"/>"##;
    assert_eq!(paint(along), Ok(area), "an edge along a row crosses none");
    // A stroke has two sides, a gradient is four times a colour, and both are eight.
    let stroked = |paint: &str| format!(r#"<path d="M0 0L8 256L16 0z" fill="none" stroke="{paint}"/>"#);
    assert_eq!(paint(&stroked("#123")), Ok(area + 2 * 16 * 512));
    assert_eq!(paint(r#"<path d="M0 0L8 256L16 0z" fill="url(#g)"/>"#), Ok(4 * (area + 16 * 512)));
    assert_eq!(paint(&stroked("url(#g)")), Ok(4 * (area + 2 * 16 * 512)));
    // Only the rows of the picture are counted, where the path is drawn, and a curve by the points that pull it.
    assert_eq!(paint(r#"<path d="M0 -100L8 356L16 -100z"/>"#), Ok(area + 16 * 512));
    assert_eq!(paint(r#"<path d="M0 -100L8 -1L16 -100z"/>"#), Ok(area));
    assert_eq!(paint(r#"<g transform="scale(2)"><path d="M0 0L4 128L8 0z"/></g>"#), Ok(area + 16 * 512));
    assert_eq!(paint(r#"<g transform="rotate(90)"><path d="M0 0L8 -256L16 0z"/></g>"#), Ok(area + 16 * 32));
    assert_eq!(paint(r#"<path d="M0 0Q8 256 16 0z"/>"#), Ok(area + 16 * 512));
    assert_eq!(paint(r#"<path d="M0 0L8 256L16 0zM20 0L28 100z"/>"#), Ok(area + 16 * 712), "every piece of a path");

    // The picture a review measured at 7.6 s in a debug build (14 s on a busy machine), and its three neighbours.
    let shapes = |count: usize, points: usize, paint: &str| {
        sized(
            256,
            &format!("{GRADIENT}{}", format!(r#"<polyline points="{}" {paint}/>"#, ragged(points)).repeat(count)),
        )
    };
    let see_through = r##"fill="#123456" fill-opacity="0.5""##;
    let slow = [
        shapes(8, 192, r#"fill="url(#g)" fill-opacity="0.5" fill-rule="evenodd" stroke="url(#g)" stroke-opacity="0.5" stroke-width="0.52""#),
        shapes(8, 192, r#"fill="url(#g)" fill-opacity="0.5" fill-rule="evenodd" stroke="url(#g)" stroke-opacity="0.5" stroke-width="3""#)
            .replace("linearGradient", "radialGradient"),
        shapes(32, 48, &format!(r##"{see_through} stroke="#123456" stroke-opacity="0.5" stroke-width="2.6" stroke-linejoin="round" stroke-linecap="round""##)),
        shapes(1, 1535, see_through),
    ];
    for (nth, svg) in slow.iter().enumerate() {
        let said = art_cost(svg).unwrap_err();
        assert!(said.starts_with(&format!("its paint ({PAINT}) is ")), "{nth}: {said}");
    }
    // What is left of the last one inside the count: 1008 edges, each down every row, and that is all of it.
    let whole = art_cost(&shapes(1, 1008, see_through)).map(|cost| cost.paint);
    assert_eq!(whole, Ok(area + 16 * 256 * 1008));
    assert_eq!(whole, Ok(MAX_ART_PAINT));
    assert!(art_cost(&shapes(1, 1010, see_through)).is_err());
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
    let paint = format!("with the art before it, paint ({PAINT}) is 4194560, over 4194304");
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
/// Counted here as the door counts, on the renderer's tree: every point, every fill and stroke, and every row an
/// edge crosses.
fn counted(svg: &str) -> ArtCost {
    use usvg::tiny_skia_path::PathSegment;
    fn rows(path: &usvg::Path, high: f32) -> u64 {
        let to = path.abs_transform();
        let row = |point: usvg::tiny_skia_path::Point| (to.ky * point.x + to.sy * point.y + to.ty).clamp(0.0, high);
        let (mut began, mut on, mut rows) = (0.0_f32, 0.0_f32, 0.0_f32);
        for segment in path.data().segments() {
            let through = match segment {
                PathSegment::MoveTo(point) => {
                    rows += (began - on).abs();
                    (began, on) = (row(point), row(point));
                    continue;
                }
                PathSegment::LineTo(point) => [point; 3],
                PathSegment::QuadTo(pull, point) => [pull, point, point],
                PathSegment::CubicTo(one, two, point) => [one, two, point],
                PathSegment::Close => {
                    rows += (began - on).abs();
                    on = began;
                    continue;
                }
            };
            for point in through {
                rows += (row(point) - on).abs();
                on = row(point);
            }
        }
        (rows + (began - on).abs()).ceil() as u64
    }
    fn walk(group: &usvg::Group, area: u64, high: f32, cost: &mut ArtCost) {
        for node in group.children() {
            match node {
                usvg::Node::Group(group) => walk(group, area, high, cost),
                usvg::Node::Path(path) => {
                    cost.numbers += 2 * path.data().points().len();
                    let fill = path.fill().map(|fill| (fill.paint(), 1));
                    let paints = fill.into_iter().chain(path.stroke().map(|stroke| (stroke.paint(), 2)));
                    for (paint, sides) in paints {
                        let times = if matches!(paint, usvg::Paint::Color(_)) { 1 } else { 4 };
                        cost.paint += times * (area + ROW_PAINT * sides * rows(path, high));
                    }
                }
                _ => {}
            }
        }
    }
    let tree = usvg::Tree::from_str(svg, &usvg::Options::default()).unwrap();
    let mut cost = ArtCost::default();
    let area = tree.size().width().ceil() as u64 * tree.size().height().ceil() as u64;
    walk(tree.root(), area, tree.size().height(), &mut cost);
    cost
}

#[test]
fn the_kit_s_own_art_is_inside_each_count() {
    let both =
        sized(256, r##"<path d="M0 0Q8 300 16 0zM20 9C1 1 2 2 30 40" stroke="url(#none) #123" stroke-width="3"/>"##);
    assert_eq!(Ok(counted(&both)), art_cost(&both), "counted here as the door counts");
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
        // A theme that draws what the kit draws, without its blur, is one the door takes: 74% and 61% of the counts.
        // The rows its edges cross are 163440 of that paint.
        assert_eq!(all, ArtCost { numbers: 2260, paint: 2_551_696 }, "{}", theme.name);
        assert!(all.numbers < MAX_ART_NUMBERS && all.paint < MAX_ART_PAINT);
    }
}
