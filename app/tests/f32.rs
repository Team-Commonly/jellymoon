//! F32 at the binary: `--theme` takes a built-in name or a theme file. A theme changes where and how pieces are
//! drawn, never which pieces; a theme file the kit refuses stops the run with one line. A picture of a theme's is
//! tried in another start of this very program before it is trusted, and one that is refused is said in one line,
//! with the page laid out all the same (`docs/adr/0016-a-picture-is-tried-before-it-is-trusted.md`).
#![allow(clippy::unwrap_used)]

mod support;

use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};

use support::{Piece, layout, page, pieces, wish};

const HONEST: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><circle cx="5" cy="5" r="4" fill="#123456"/></svg>"##;

/// `jellymoon-app layout` of an empty day under a theme file, with Jellymoon's own folder at `home` and what else
/// is set: whether it ended well, and what it said on stderr.
fn tried(theme: &Path, home: &Path, set: &[(&str, &str)]) -> (bool, String) {
    let day = home.join("page.json");
    fs::write(&day, page(vec![], vec![]).to_string()).unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_jellymoon-app"));
    command.args(["layout", "--page", day.to_str().unwrap(), "--theme", theme.to_str().unwrap()]);
    command.env_clear().env("JELLYMOON_HOME", home).envs(set.iter().copied()).stdin(Stdio::null());
    if let Some(root) = std::env::var_os("SystemRoot").filter(|_| cfg!(windows)) {
        command.env("SystemRoot", root); // a Windows program needs it to start
    }
    let out = command.output().unwrap();
    assert!(out.status.success() == !out.stdout.is_empty(), "a page is laid out, or the run failed");
    (out.status.success(), String::from_utf8(out.stderr).unwrap())
}

#[test]
fn a_theme_s_picture_is_tried_by_this_program_started_again_and_its_answer_is_kept() {
    let dir = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let (home, theme) = (dir.path().join("home"), dir.path().join("theme.toml"));
    fs::create_dir(&home).unwrap();
    fs::write(&theme, "[art]\nstar = \"star.svg\"\n").unwrap();
    let kept = || fs::read_dir(home.join("art-trials")).map_or(0, Iterator::count);

    // A trial that is given no time is ended, the page is laid out with the kit's own art, and nothing is kept.
    fs::write(dir.path().join("star.svg"), HONEST).unwrap();
    let ended = "jellymoon-app: theme: the SVG for art.star is refused: its trial took longer than 0 ms and was \
                 ended; the built-in art is drawn\n";
    assert_eq!(tried(&theme, &home, &[("STICKER_KIT_TEST_TRIAL_MS", "0")]), (true, ended.to_string()));
    assert_eq!(kept(), 0);

    assert_eq!(tried(&theme, &home, &[]), (true, String::new()), "an honest picture is accepted and nothing is said");
    assert_eq!(kept(), 1, "its answer is kept in Jellymoon's own folder");
    // With the answer kept no child is started: a limit of no time at all would have ended one.
    assert_eq!(tried(&theme, &home, &[("STICKER_KIT_TEST_TRIAL_MS", "0")]), (true, String::new()));

    // Another picture is another trial, and what the rules say inside it comes back as the line.
    fs::write(dir.path().join("star.svg"), HONEST.replace("<circle", "<script/><circle")).unwrap();
    let scripted = "jellymoon-app: theme: the SVG for art.star is refused: it contains the element `<script`; the \
                    built-in art is drawn\n";
    assert_eq!(tried(&theme, &home, &[]), (true, scripted.to_string()));
    assert_eq!(tried(&theme, &home, &[("STICKER_KIT_TEST_TRIAL_MS", "0")]), (true, scripted.to_string()));
    assert_eq!(kept(), 2);

    // This program is a trial only when that is its first argument, and then it is nothing else.
    let out = support::run(&["--try-art", "layout"], None);
    assert!(out.status.success() && out.stderr.is_empty());
    let said = String::from_utf8(out.stdout).unwrap();
    assert_eq!(said, "refused: it is not XML the renderer reads: the document does not have a root node\n");
    // It answers for the pixels it drew, at twice the picture's size, and reads no more than a picture may be.
    let said = |given: &str| String::from_utf8(support::run(&["--try-art"], Some(given)).stdout).unwrap();
    assert_eq!(said(HONEST), "accepted 26 356 400\n");
    assert_eq!(said(&format!("{HONEST}{}", " ".repeat(64 * 1024))), "refused: it is over 64 KiB\n");
}

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

    let newer = file("newer.toml", "sparkle = true\n[art]\nstar = \"not-there.svg\"\n");
    let out = layout(&day.to_string(), &["--theme", &newer]);
    assert!(out.status.success());
    assert_eq!(out.stdout, layout(&day.to_string(), &[]).stdout, "what is unknown or missing is the default");
    let said = String::from_utf8(out.stderr).unwrap();
    assert!(said.starts_with("jellymoon-app: theme: sparkle: unknown key, ignored\n"), "{said}");

    file("star.svg", r#"<svg xmlns="http://www.w3.org/2000/svg"><script>alert(1)</script></svg>"#);
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
