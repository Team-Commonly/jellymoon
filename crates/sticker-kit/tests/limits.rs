//! The theme door's numbers, each held at its edge: the last value inside loads and the first one outside is refused.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;

use sticker_kit::theme::{MAX_ART_BYTES, MAX_ART_PATH_BYTES, MAX_THEME_BYTES, svg_refusal};
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

#[test]
fn no_number_in_art_carries_a_unit_or_a_percent_sign() {
    let refused = [
        r#"<circle r="4in"/>"#,
        r#"<circle r="50%"/>"#,
        r#"<circle r="4" cx="1em"/>"#,
        r#"<rect width="4mm" height="4"/>"#,
        r#"<rect width="4px" height="4"/>"#,
        r#"<path d="M5 5h1" stroke-width="4096in"/>"#,
        r#"<path d="M5 5h1" stroke-width="2pc"/>"#,
        r#"<path d="M0 0L4in 4"/>"#,
        r#"<path d="M0 0L50% 4"/>"#,
        r#"<polyline points="0,0 1pt,1"/>"#,
        r#"<linearGradient id="g"><stop offset="50%"/></linearGradient>"#,
        r#"<circle r="4" fill-opacity="50%"/>"#,
        r#"<g transform="translate(4px 4)"/>"#,
        r#"<g transform="rotate(30deg)"/>"#,
    ];
    for inside in refused {
        assert!(svg_refusal(&svg(inside)).is_some(), "{inside} was accepted");
    }
    let said = svg_refusal(&svg(refused[0])).unwrap();
    assert_eq!(said, "`r` holds a unit or `%`: a number is plain digits");
    let commands =
        r#"<path d="M1 1L2 2H3V4C1 1 2 2 3 3S1 1 2 2Q1 1 2 2T3 3A4 4 0 0 1 5 5Zm1 1l1 1h1v1c1 1 1 1 1 1z"/>"#;
    assert_eq!(svg_refusal(&svg(commands)), None, "every path command is a letter a `d` may hold");
    let root = r#"<svg xmlns="http://www.w3.org/2000/svg" width="40px" height="40px"><circle r="4"/></svg>"#;
    assert_eq!(svg_refusal(root), None, "`px` may follow the picture's own width and height, and nothing else");
}

#[test]
fn a_path_or_a_list_of_points_is_held_to_a_length() {
    assert_eq!(MAX_ART_PATH_BYTES, 4096);
    for over in [0, 1] {
        let d = format!("M0 0{}  ", "h1000".repeat((MAX_ART_PATH_BYTES - 4) / 5)) + &" ".repeat(over);
        let points = "100,100 ".repeat(MAX_ART_PATH_BYTES / 8) + &" ".repeat(over);
        assert_eq!((d.len(), points.len()), (MAX_ART_PATH_BYTES + over, MAX_ART_PATH_BYTES + over));
        let shapes = [("d", format!(r#"<path d="{d}"/>"#)), ("points", format!(r#"<polygon points="{points}"/>"#))];
        for (name, shape) in shapes {
            let want = (over == 1).then(|| format!("the value of `{name}` is over 4096 bytes"));
            assert_eq!(svg_refusal(&svg(&shape)), want, "{name}, {over} over");
        }
    }
}
