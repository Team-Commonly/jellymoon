//! What art costs to draw is counted at the door: the numbers in its paths, and its shapes times its size. Each count
//! is held at its edge, for one file and for a theme's files together. No GPUI: the counts are the same everywhere.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;

use sticker_kit::theme::{ArtCost, MAX_ART_NUMBERS, MAX_ART_PAINT, MIN_ART_VIEW, art_cost};
use sticker_kit::{Mood, Theme, ThemeError, art};

const CIRCLE: &str = r#"<circle r="4"/>"#;

/// A picture `side` wide and high.
fn sized(side: u32, inside: &str) -> String {
    format!(r#"<svg xmlns="http://www.w3.org/2000/svg" width="{side}" height="{side}">{inside}</svg>"#)
}

/// One path of `numbers` numbers.
fn path(numbers: usize) -> String {
    format!(r#"<path d="M0 0{}"/>"#, "h1".repeat(numbers - 2))
}

#[test]
fn an_art_file_is_held_to_its_shapes_times_its_size() {
    assert_eq!(MAX_ART_PAINT, 64 * 256 * 256);
    assert_eq!(art_cost(&sized(256, &CIRCLE.repeat(64))), Ok(ArtCost { numbers: 0, paint: MAX_ART_PAINT }));
    let said = art_cost(&sized(256, &CIRCLE.repeat(65))).unwrap_err();
    assert_eq!(said, "its paint (shapes x width x height, x 4 with a gradient) is 4259840, over 4194304");
    assert_eq!(art_cost(&sized(128, &CIRCLE.repeat(99))).map(|cost| cost.paint), Ok(99 * 128 * 128));

    let gradient = |shapes: usize| sized(256, &format!(r#"<linearGradient id="g"/>{}"#, CIRCLE.repeat(shapes)));
    assert_eq!(art_cost(&gradient(16)).map(|cost| cost.paint), Ok(MAX_ART_PAINT), "a gradient counts four times");
    assert!(art_cost(&gradient(17)).is_err());

    let rooted = |root: &str| format!(r#"<svg xmlns="http://www.w3.org/2000/svg" {root}>{CIRCLE}<g><rect/></g></svg>"#);
    for (root, paint) in [
        (r#"width="10" height="20.5""#, 2 * 10 * 21),
        (r#"viewBox="0 0 64 32""#, 2 * 64 * 32),
        (r#"width="100px" viewBox="0 0 64 32""#, 2 * 100 * 32),
    ] {
        assert_eq!(art_cost(&rooted(root)).map(|cost| cost.paint), Ok(paint), "{root}");
    }
    // With no `viewBox` and a side not stated, the renderer draws the picture as large as what is in it.
    for root in ["", r#"width="10""#, r#"height="10""#] {
        let said = art_cost(&rooted(root)).unwrap_err();
        assert_eq!(said, "it states no size: a picture needs a `viewBox`, or a `width` and a `height`", "{root}");
    }
}

#[test]
fn an_art_file_is_held_to_the_numbers_in_its_paths_and_a_view_box_to_a_least_size() {
    assert_eq!((MAX_ART_NUMBERS, MIN_ART_VIEW), (1024, 8.0));
    let two = |more: &str| sized(64, &format!("{}{}{more}", path(512), path(508)));
    let full = two(r#"<polygon points="1,1 2 2"/>"#);
    assert_eq!(art_cost(&full).map(|cost| cost.numbers), Ok(MAX_ART_NUMBERS));
    let over = art_cost(&two(r#"<polygon points="1,1 2 2 3"/>"#)).unwrap_err();
    assert_eq!(over, "its paths hold more than 1024 numbers");
    assert!(art_cost(&two(r#"<circle cx="1" cy="2" r="3"/><rect x="1"/>"#)).is_ok(), "only paths and points count");
    // An arc is drawn as several curves: each of its numbers counts four times, in either case of its letter.
    let arcs = |d: &str| art_cost(&sized(64, &format!(r#"<path d="{d}"/>"#))).map(|cost| cost.numbers);
    assert_eq!(arcs("M0 0a4 4 0 1 1 2 2L1 1"), Ok(2 + 7 * 4 + 2));
    assert_eq!(arcs("M0 0A4 4 0 1 1 2 2 4.5.5 0 1 1 3 3h1"), Ok(2 + 14 * 4 + 1), "an arc goes on until a letter");
    let most = format!("M0 0{}{}", "a4 4 0 1 1 2 2".repeat(36), "h1".repeat(14));
    assert_eq!(arcs(&most), Ok(MAX_ART_NUMBERS));
    assert_eq!(arcs(&format!("{most}h1")), Err("its paths hold more than 1024 numbers".to_string()));

    let viewed = |view: &str| format!(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="{view}">{CIRCLE}</svg>"#);
    assert!(art_cost(&viewed("0 0 8 8")).is_ok());
    for view in ["0 0 7.5 8", "0 0 8 7", "0 0 1 1"] {
        let said = art_cost(&viewed(view)).unwrap_err();
        assert_eq!(said, format!("`viewbox=\"{view}\"` is not a size from 8 to 256"));
    }
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
    let paint = "with the art before it, paint (shapes x width x height, x 4 with a gradient) is 4194560, over 4194304";
    assert_eq!(write(&half, &small(1), &half), Err(format!("art.badge: {paint}")));
    assert_eq!(write(&half, &half, &small(1)), Err(format!("art.badge: {paint}")));

    let paths = |numbers: usize| sized(16, &path(numbers));
    assert_eq!(write(&paths(512), &paths(500), &paths(12)), Ok(()));
    let numbers = "with the art before it, paths hold more than 1024 numbers";
    assert_eq!(write(&paths(512), &paths(500), &paths(13)), Err(format!("art.badge: {numbers}")));
    assert_eq!(write(&paths(512), &paths(513), &small(1)), Err(format!("art.star: {numbers}")));

    // Files changed after the theme was loaded are read again when it is drawn, and held to the same total: a
    // file that would take the theme past it is not given, so the kit's own art is drawn in its place.
    assert_eq!(write(&small(1), &small(1), &small(1)), Ok(()));
    let loaded = Theme::load(&theme).unwrap().theme;
    let given = |loaded: &Theme| ["hero", "star", "badge"].map(|key| loaded.art_svg(key).is_some());
    assert_eq!(given(&loaded), [true, true, true]);
    let full = sized(256, &CIRCLE.repeat(64));
    for name in ["hero", "star"] {
        fs::write(folder.join(format!("{name}.svg")), &full).unwrap();
    }
    assert_eq!(given(&loaded), [true, false, false], "each alone is inside the counts; together they are not");
    fs::write(folder.join("hero.svg"), paths(1024)).unwrap();
    assert_eq!(given(&loaded), [true, false, true], "a file past the total is passed over, not counted");
    fs::write(folder.join("badge.svg"), paths(2)).unwrap();
    assert_eq!(given(&loaded), [true, false, false], "the numbers are summed as the paint is");
    let _ = fs::remove_dir_all(folder);
}

/// The same two counts for the kit's own art, which is built in code and does not come through the door (it blurs).
/// Counted here from its text: every shape, every number in a `d`, and the size the `<svg` states.
fn counted(svg: &str) -> ArtCost {
    let shapes: usize = ["<path", "<rect", "<circle", "<ellipse", "<line ", "<polyline", "<polygon"]
        .map(|s| svg.matches(s).count())
        .iter()
        .sum();
    let digits = |text: &str| text.split(|c: char| !c.is_ascii_digit() && c != '.').filter(|n| !n.is_empty()).count();
    let numbers = svg.split(" d=\"").skip(1).map(|rest| digits(rest.split('"').next().unwrap())).sum();
    let side = |name: &str| -> f32 {
        let rest = svg.split_once(&format!(" {name}=\"")).unwrap().1;
        rest.split('"').next().unwrap().parse().unwrap()
    };
    let fills = if svg.contains("Gradient") { 4 } else { 1 };
    ArtCost { numbers, paint: (shapes * fills) as u64 * side("width").ceil() as u64 * side("height").ceil() as u64 }
}

#[test]
fn the_kit_s_own_art_is_under_half_of_each_count() {
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
        assert_eq!(all, ArtCost { numbers: 368, paint: 1_728_164 }, "{}", theme.name);
        assert!(all.numbers * 2 < MAX_ART_NUMBERS && all.paint * 2 < MAX_ART_PAINT, "under half of each count");
    }
}
