//! Themes are data: defaults, ignored keys, the refusals, and no pictures.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::path::PathBuf;

use sticker_kit::theme::MAX_THEME_BYTES;
use sticker_kit::{Color, Scene, Theme, ThemeError};

const PICTURE: &str =
    r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><circle r="4" fill="#123456"/></svg>"##;

/// A fresh folder under the system's temporary folder, removed on drop.
struct Folder(PathBuf);

impl Folder {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("sticker-kit-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("temporary folder");
        Folder(path)
    }

    fn write(&self, name: &str, text: &str) -> PathBuf {
        let path = self.0.join(name);
        fs::write(&path, text).expect("file written");
        path
    }
}

impl Drop for Folder {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn missing_keys_fall_back_to_the_sticker_theme() {
    let loaded = Theme::from_toml("[palette]\nink = \"#102030\"\n\n[motion]\nbob_amplitude = 2.5\n")
        .expect("a partial theme loads");
    let sticker = Theme::sticker();
    assert!(loaded.warnings.is_empty());
    assert_eq!(loaded.theme.palette.ink, Color::rgb(0x102030));
    assert_eq!(loaded.theme.palette.paper, sticker.palette.paper);
    assert_eq!(loaded.theme.palette.hues, sticker.palette.hues);
    assert_eq!(loaded.theme.motion.bob_amplitude, 2.5);
    assert_eq!(loaded.theme.motion.bob_period, sticker.motion.bob_period);
    assert_eq!(loaded.theme.fonts, sticker.fonts);
    assert_eq!(loaded.theme.metrics, sticker.metrics);

    let empty = Theme::from_toml("").expect("an empty theme loads");
    assert_eq!(empty.theme, Theme::default());
}

#[test]
fn an_unknown_key_is_ignored_named_and_defaulted() {
    let text = "sparkle = true\n\n[palette]\nink = \"#102030\"\nglitter = \"#FFFFFF\"\n\n\
[[palette.hues]]\nbase = \"#000000\"\nshine = 3\n\n[future]\nanything = 1\n";
    let loaded = Theme::from_toml(text).expect("a theme from a newer kit still loads");
    let named = |key: &str| loaded.warnings.iter().any(|w| w.starts_with(&format!("{key}:")));
    assert!(named("sparkle"), "{:?}", loaded.warnings);
    assert!(named("palette.glitter"), "{:?}", loaded.warnings);
    assert!(named("palette.hues[0].shine"), "{:?}", loaded.warnings);
    assert!(named("future"), "{:?}", loaded.warnings);
    assert_eq!(loaded.warnings.len(), 4);
    assert_eq!(loaded.theme.palette.ink, Color::rgb(0x102030));
    assert_eq!(loaded.theme.palette.hues[0].base, Color::rgb(0x000000));
    assert_eq!(loaded.theme.palette.hues[0].dark, Theme::sticker().palette.hues[0].dark);
    assert_eq!(loaded.theme.paper, Theme::sticker().paper);
}

// A theme file names no pictures. The table an older one has is said in one line, and what it names is not
// read, found or missing, a folder or a link: the theme is the one its own text is.
#[test]
fn the_pictures_a_theme_file_names_are_not_read() {
    let text = "[art]\nstar = \"star.svg\"\nbadge = \"not-there.svg\"\nhero = \"hero.svg\"\nmore = 3\n";
    let said = Theme::from_toml(text).expect("loads");
    assert_eq!(said.warnings, ["art: pictures in a theme are not read in this version; the kit's own art is drawn"]);
    assert_eq!(said.theme, Theme::default());
    let folder = Folder::new("pictures");
    folder.write("star.svg", PICTURE);
    fs::create_dir_all(folder.0.join("hero.svg")).unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(folder.0.join("nowhere"), folder.0.join("not-there.svg")).expect("symlink made");
    let loaded = Theme::load(&folder.write("theme.toml", text)).expect("loads");
    assert_eq!(loaded.warnings, said.warnings);
    assert_eq!(loaded.theme, Theme { name: "theme".into(), ..Theme::default() });
    let kept = Theme::sticker().to_toml().unwrap();
    assert!(!kept.contains("art"), "and a theme the kit writes has no such table");
}

#[test]
fn a_path_out_of_the_theme_folder_is_refused() {
    for name in ["../outside.svg", "/outside.svg", "art/../../outside.svg"] {
        let text = format!("[art]\nstar = \"{name}\"\n");
        match Theme::from_toml(&text) {
            Err(ThemeError::PathEscapes { key, path }) => {
                assert_eq!((key.as_str(), path.as_str()), ("art.star", name));
            }
            other => panic!("{name} was not refused: {other:?}"),
        }
    }
}

#[test]
fn plain_is_level_and_still_when_idle() {
    let plain = Theme::plain();
    assert!(!plain.tilt.enabled);
    let t = plain.tilt;
    assert_eq!([t.date_tape, t.note_tape, t.list_note, t.choice_note, t.plain_note], [0.0; 5]);
    assert_eq!(plain.motion.bob_amplitude, 0.0);
    assert_eq!(plain.motion.twinkle, 0.0);
    assert_eq!(plain.motion.pulse, 0.0);
    assert!(!plain.paper.grid);
    assert!(Theme::sticker().tilt.enabled && Theme::sticker().motion.bob_amplitude > 0.0);
}

#[test]
fn built_in_themes_load_from_their_own_serialised_form() {
    for theme in [Theme::sticker(), Theme::plain()] {
        let text = theme.to_toml().expect("serialises");
        let loaded = Theme::from_toml(&text).expect("loads back");
        assert!(loaded.warnings.is_empty(), "{:?}", loaded.warnings);
        assert_eq!(loaded.theme, theme);
        assert_eq!(Theme::builtin(&theme.name), Some(theme));
    }
}

#[test]
fn a_scene_survives_serialisation() {
    let scene = Scene::sample();
    let text = toml::to_string(&scene).expect("serialises");
    let back: Scene = toml::from_str(&text).expect("loads back");
    assert_eq!(back, scene);
}

#[test]
fn a_number_that_is_not_finite_or_is_out_of_its_range_is_refused_by_key() {
    let wrong = [
        ("[metrics]\nheight = inf\n", "metrics.height"),
        ("[metrics]\nmargin_y = -inf\n", "metrics.margin_y"),
        ("[metrics]\nwidth = nan\n", "metrics.width"),
        ("[metrics]\nheight = 3e7\n", "metrics.height"),
        ("[metrics]\nheight = 30000000\n", "metrics.height"),
        ("[metrics]\nheight = 199.0\n", "metrics.height"),
        ("[metrics]\ncolumns = 0\n", "metrics.columns"),
        ("[metrics]\ncolumns = 13\n", "metrics.columns"),
        ("[metrics]\nmax_scale = 0.0\n", "metrics.max_scale"),
        ("[metrics]\nnote_width = 10001\n", "metrics.note_width"),
        ("[paper]\ngrid_step = -1.0\n", "paper.grid_step"),
        ("[tilt]\nnote_tape = 90.0\n", "tilt.note_tape"),
        ("[motion]\nbob_period = inf\n", "motion.bob_period"),
        ("[labels]\nmax_chars = 201\n", "labels.max_chars"),
        ("[labels]\nfont_size = -nan\n", "labels.font_size"),
    ];
    for (text, name) in wrong {
        match Theme::from_toml(text) {
            Err(ThemeError::OutOfRange { key, .. }) => assert_eq!(key, name),
            other => panic!("{text:?} was not refused: {other:?}"),
        }
    }
    let edge = "[metrics]\nheight = 10000\nwidth = 200.0\ncolumns = 12\n[tilt]\nnote_tape = -45.0\n";
    assert!(Theme::from_toml(edge).is_ok(), "the ends of a range are inside it");
    let said = Theme::from_toml("[metrics]\nheight = inf\n").unwrap_err().to_string();
    assert_eq!(said, "metrics.height = inf is not a number from 200 to 10000");
}

#[test]
fn a_theme_file_over_its_size_or_not_a_regular_file_is_refused() {
    let big = format!("# {}\n", "x".repeat(MAX_THEME_BYTES as usize));
    assert!(matches!(Theme::from_toml(&big), Err(ThemeError::Read(why)) if why == "it is over 256 KiB"));
    let folder = Folder::new("sizes");
    let path = folder.write("big.toml", &big);
    assert!(matches!(Theme::load(&path), Err(ThemeError::Read(why)) if why == "it is over 256 KiB"));
    assert!(matches!(Theme::load(&folder.0), Err(ThemeError::Read(why)) if why == "it is not a regular file"));
}

#[test]
fn a_bare_file_name_loads_from_the_folder_the_program_runs_in() {
    // Cargo runs a test in its package's folder, where this file always is. It is TOML with no key a theme has.
    let loaded = Theme::load(std::path::Path::new("Cargo.toml")).expect("loads");
    assert_eq!(loaded.theme.name, "Cargo");
}
