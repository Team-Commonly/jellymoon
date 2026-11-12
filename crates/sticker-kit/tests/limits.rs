//! The theme door's numbers, each held at its edge: the last value inside loads and the first one outside is refused.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;

use sticker_kit::theme::{MAX_ART_BYTES, MAX_ART_PATH_BYTES, MAX_THEME_BYTES, art_cost, svg_refusal};
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
    assert_eq!(sized(r#"width="1in" height="2in""#), Ok(96 * 192), "an inch is 96 pixels to the renderer");
    assert_eq!(sized(r#"width="40px" height="1e1""#), Ok(400));
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
