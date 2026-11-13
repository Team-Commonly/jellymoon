//! F32 at the binary: `--theme` takes a built-in name or a theme file. A theme changes where and how pieces are
//! drawn, never which pieces; a theme file the kit refuses stops the run with one line.

mod support;

use std::fs;

use support::{Piece, layout, page, pieces, wish};

#[test]
fn a_theme_file_changes_the_look_and_a_refused_one_stops_the_run_with_one_line() {
    let dir = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let file = |name: &str, text: &str| {
        fs::write(dir.path().join(name), text).unwrap();
        dir.path().join(name).to_str().unwrap().to_string()
    };
    let day =
        page(vec![wish("w1", "Login page", "working", &["form built"]), wish("w2", "Search", "done", &[])], vec![]);
    let which = |placed: &[Piece]| -> Vec<String> {
        placed.iter().map(|piece| format!("{} {} {}", piece.kind(), piece.id(), piece.0["index"])).collect()
    };
    let level = |placed: &[Piece]| placed.iter().all(|piece| piece.0["tilt"] == 0.0);

    let sticker = pieces(&day, &[]);
    let dusk =
        file("dusk.toml", "[palette]\ndesk = \"#2B2440\"\n[tilt]\nenabled = false\n[metrics]\ntitle_size = 34\n");
    let themed = pieces(&day, &["--theme", &dusk]);
    assert_eq!(which(&themed), which(&sticker), "the same pieces in the same order");
    assert_eq!(which(&pieces(&day, &["--theme", "plain"])), which(&sticker));
    assert!(level(&themed) && !level(&sticker), "the file's theme is the one laid out");

    // A theme file names no pictures: the one beside it, script and all, is not read, and it is said in one line.
    file("star.svg", r#"<svg xmlns="http://www.w3.org/2000/svg"><script>alert(1)</script></svg>"#);
    let newer = file("newer.toml", "sparkle = true\n[art]\nstar = \"star.svg\"\n");
    let out = layout(&day.to_string(), &["--theme", &newer]);
    assert!(out.status.success());
    assert_eq!(out.stdout, layout(&day.to_string(), &[]).stdout, "what is unknown or missing is the default");
    let said = String::from_utf8(out.stderr).unwrap();
    let pictures = "art: pictures in a theme are not read in this version; the kit's own art is drawn";
    let lines: Vec<_> = said.lines().map(|line| line.strip_prefix("jellymoon-app: theme: ")).collect();
    assert_eq!(lines, [Some(pictures), Some("sparkle: unknown key, ignored")]);

    let refused = [
        (file("out.toml", "[art]\nstar = \"../star.svg\"\n"), "art.star = \"../star.svg\" leaves the theme's folder"),
        (file("wide.toml", "[metrics]\nrim = 41\n"), "metrics.rim = 41 is not a number from 0 to 40"),
        (file("broken.toml", "[metrics\nrim = 4\n"), "cannot parse the theme: TOML parse error at line 1"),
        ("neon".to_string(), "cannot read the theme: "),
    ];
    for (theme, why) in refused {
        let out = layout(&day.to_string(), &["--theme", &theme]);
        let said = String::from_utf8(out.stderr).unwrap();
        assert_eq!(out.status.code(), Some(1), "{theme}: {said}");
        assert!(out.stdout.is_empty(), "{theme}: a refused theme printed something");
        let start =
            format!("jellymoon-app: --theme {theme:?} is not sticker, plain or a theme file the kit takes: {why}");
        assert!(said.starts_with(&start), "{said}");
        assert_eq!(said.lines().count(), 1, "one line, and no usage: {said}");
    }
}
