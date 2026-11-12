//! F32: a theme changes how a page looks and nothing else. The four steps, at the kit's door: `Theme::load`,
//! `Theme::from_toml` and `layout`. No GPUI.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::path::PathBuf;

use sticker_kit::motion::{bob, hop, pulse, twinkle};
use sticker_kit::{FixedAdvance, PieceKind, Scene, Theme, ThemeError, Viewport, layout};

const SIZES: [Viewport; 2] = [Viewport { w: 1440.0, h: 900.0 }, Viewport { w: 960.0, h: 640.0 }];

/// A theme folder of its own under the system's temporary folder, holding `files`.
fn folder(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let path = std::env::temp_dir().join(format!("sticker-kit-f32-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).unwrap();
    for (file, text) in files {
        fs::write(path.join(file), text).unwrap();
    }
    path
}

/// Which pieces a layout placed, in its order.
fn pieces(theme: &Theme, viewport: Viewport) -> Vec<(PieceKind, String, usize)> {
    let placed = layout(&Scene::sample(), theme, viewport, &FixedAdvance(0.56));
    placed.into_iter().map(|p| (p.piece, p.id, p.index)).collect()
}

#[test]
fn both_themes_place_the_same_pieces_in_the_same_order() {
    let dusk = Theme::from_toml("name = \"dusk\"\n[palette]\ndesk = \"#2B2440\"\n[tilt]\nenabled = false\n").unwrap();
    for viewport in SIZES {
        let sticker = pieces(&Theme::sticker(), viewport);
        assert!(sticker.len() > 40, "{} pieces", sticker.len());
        assert_eq!(pieces(&Theme::plain(), viewport), sticker);
        assert_eq!(pieces(&dusk.theme, viewport), sticker);
    }
    let rects = |theme: &Theme| -> Vec<_> {
        layout(&Scene::sample(), theme, SIZES[0], &FixedAdvance(0.56)).into_iter().map(|p| p.rect).collect()
    };
    assert_ne!(rects(&Theme::plain()), rects(&Theme::sticker()), "the look is what a theme changes");
}

#[test]
fn the_plain_theme_has_no_tilt_and_no_idle_motion() {
    let plain = Theme::plain();
    for viewport in SIZES {
        let placed = layout(&Scene::sample(), &plain, viewport, &FixedAdvance(0.56));
        assert!(placed.iter().all(|p| p.tilt_degrees == 0.0));
        let tilted = layout(&Scene::sample(), &Theme::sticker(), viewport, &FixedAdvance(0.56));
        assert!(tilted.iter().any(|p| p.tilt_degrees != 0.0), "the sticker theme tilts");
    }
    assert!(plain.motion.is_idle_still());
    assert_eq!((plain.motion.bob_amplitude, plain.motion.hop_height), (0.0, 0.0));
    assert_eq!((plain.motion.twinkle, plain.motion.pulse), (0.0, 0.0));
    for step in 0..400 {
        let t = step as f32 * 0.05;
        assert_eq!((bob(&plain.motion, t, 0.3), hop(&plain.motion, t, 0.3)), (0.0, 0.0));
        assert_eq!((twinkle(&plain.motion, t, 0.3), pulse(&plain.motion, t, 0.3)), (1.0, 1.0));
    }
    assert!(!Theme::sticker().motion.is_idle_still());
}

#[test]
fn an_unknown_key_or_a_missing_piece_falls_back_to_the_default() {
    let text = "sparkle = true\n[palette]\nink = \"#102030\"\nglitter = \"#FFFFFF\"\n\
[motion]\nwobble = 3\n[art]\nstar = \"not-there.svg\"\n";
    let dir = folder("fallback", &[("dusk.toml", text)]);
    let loaded = Theme::load(&dir.join("dusk.toml")).expect("the theme loads");
    let mut expected = Theme::sticker();
    expected.name = "dusk".into();
    expected.palette.ink = sticker_kit::Color::rgb(0x102030);
    assert_eq!(loaded.theme.to_toml().unwrap(), expected.to_toml().unwrap(), "every other piece is the default");
    // The star's picture is not looked for: a theme file names no pictures, and the kit's own star is drawn.
    for key in ["sparkle", "palette.glitter", "motion.wobble", "art"] {
        assert!(loaded.warnings.iter().any(|w| w.starts_with(&format!("{key}:"))), "{key}: {:?}", loaded.warnings);
    }
    assert_eq!(loaded.warnings.len(), 4);
    assert_eq!(pieces(&loaded.theme, SIZES[0]), pieces(&Theme::sticker(), SIZES[0]));
    let _ = fs::remove_dir_all(dir);
}

/// What the kit says of a theme file that names a picture.
const NO_PICTURES: &str = "art: pictures in a theme are not read in this version; the kit's own art is drawn";

// The step: "a theme carries no code: a theme file containing a script or a path outside its folder is refused".
// A path outside the folder is refused, by the text of the key, with no file opened. A script could only ever be
// in a picture the theme names, and a theme's pictures are never read: the theme is what its own text says,
// whatever lies beside it, and holds no folder to find a file in.
#[test]
fn a_theme_file_with_a_script_or_a_path_outside_its_folder_is_refused() {
    let text = "[art]\nstar = \"star.svg\"\n";
    let mut said = Theme::from_toml(text).expect("the text is a theme");
    said.theme.name = "theme".into();
    assert_eq!(said.warnings, [NO_PICTURES]);
    assert_eq!(said.theme.to_toml().unwrap(), Theme { name: "theme".into(), ..Theme::sticker() }.to_toml().unwrap());
    let scripts = [
        r#"<svg xmlns="http://www.w3.org/2000/svg"><script>alert(1)</script></svg>"#,
        r#"<svg xmlns="http://www.w3.org/2000/svg" onload="alert(1)"><circle r="4"/></svg>"#,
        r#"<svg xmlns="http://www.w3.org/2000/svg"><a href="javascript:x()"><circle r="4"/></a></svg>"#,
    ];
    for svg in scripts {
        let dir = folder("script", &[("star.svg", svg), ("theme.toml", text)]);
        assert_eq!(Theme::load(&dir.join("theme.toml")).as_ref(), Ok(&said), "{svg} is beside it, and is not read");
        let _ = fs::remove_dir_all(dir);
    }
    // Nor is the name looked for: a folder of that name, or nothing of it, is the same theme.
    let dir = folder("no-file", &[("theme.toml", text)]);
    assert_eq!(Theme::load(&dir.join("theme.toml")).as_ref(), Ok(&said));
    fs::create_dir(dir.join("star.svg")).unwrap();
    assert_eq!(Theme::load(&dir.join("theme.toml")).as_ref(), Ok(&said));
    let _ = fs::remove_dir_all(dir);

    let picture = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><circle r="4"/></svg>"#;
    let dir = folder("inside", &[("star.svg", picture)]);
    let outside = folder("outside", &[("star.svg", picture)]);
    let up = format!("../{}/star.svg", outside.file_name().unwrap().to_string_lossy());
    let absolute = outside.join("star.svg").to_string_lossy().replace('\\', "/");
    for (name, refused) in [("star.svg", false), (up.as_str(), true), (absolute.as_str(), true)] {
        fs::write(dir.join("theme.toml"), format!("[art]\nstar = \"{name}\"\n")).unwrap();
        let loaded = Theme::load(&dir.join("theme.toml"));
        assert_eq!(matches!(loaded, Err(ThemeError::PathEscapes { .. })), refused, "{name}: {loaded:?}");
        assert_eq!(loaded.is_ok(), !refused, "{name}");
    }
    let _ = fs::remove_dir_all(dir);
    let _ = fs::remove_dir_all(outside);
}
