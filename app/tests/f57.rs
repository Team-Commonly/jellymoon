//! F57: the jellyfish is a window. The app opens it as a pop-up of the kit's edge scene and its bubble as a
//! second, each asked for by options that are data. At the binary's boundary, here: `layout --jelly` prints what
//! each window is asked for and where it stands, and what F56's review found in what that command reads and
//! refuses. `f57/window.rs` opens both windows on GPUI's test platform, on the Mac only, with no window on a
//! screen: the states drawn, the bubble opened and removed, the page closed and shown again, Quit, and the look
//! once a second.
#![allow(clippy::unwrap_used, clippy::expect_used)]

#[allow(dead_code)]
#[path = "f56/jelly.rs"]
mod jelly;
mod support;
#[cfg(all(target_os = "macos", feature = "window"))]
#[path = "f57/window.rs"]
mod window;

use std::fs;

use jelly::{
    SCREEN, app, asks, cannot, drawn, drawn_on, heard, inside, listening, placed, quiet, said, waits, working,
};
use serde_json::{Value, json};
use support::run;

const SAID: &str = "make the uploader retry twice";

/// What a window line says the window is asked for.
fn opens(shown: &jelly::Drawn, id: &str) -> Value {
    let windows = shown.of("window");
    windows.iter().find(|window| window.id() == id).map(|window| window.0["opens"].clone()).unwrap_or(Value::Null)
}

fn apart(a: [f64; 4], b: [f64; 4]) -> bool {
    a[0] + a[2] <= b[0] || b[0] + b[2] <= a[0] || a[1] + a[3] <= b[1] || b[1] + b[3] <= a[1]
}

#[test]
fn each_window_is_asked_as_a_pop_up_one_sees_through_that_never_has_the_keys_and_only_the_jellyfish_is_tapped() {
    let asked = |tap: &str| json!({"kind": "pop_up", "focus": false, "see_through": true, "moves": false, "resizes": false, "tap": tap});
    let lines = [
        quiet(),
        listening("make"),
        heard(SAID, "alpha", 2000),
        working(3),
        asks("Which one?"),
        waits(),
        cannot("nothing was heard: no microphone was found"),
    ];
    for line in lines {
        let shown = drawn(&line);
        assert_eq!(opens(&shown, "jelly"), asked("page"), "{line}");
        let bubble = opens(&shown, "bubble");
        assert_eq!(bubble, if shown.window("bubble").is_some() { asked("none") } else { Value::Null }, "{line}");
        // Only a window says what it is asked for: a piece is in one.
        assert!(shown.0.iter().all(|piece| (piece.kind() == "window") == piece.0.get("opens").is_some()));
    }
}

// F56's review, note 1: these two screens hold both windows apart, and the bubble was laid over the jellyfish.
#[test]
fn on_a_screen_that_holds_both_windows_the_bubble_never_covers_the_jellyfish() {
    for screen in ["320x322", "419x240", "320x330", "420x240", "1440x900"] {
        let shown = drawn_on(&heard(SAID, "alpha", 4000), screen);
        let (jelly, bubble) = (shown.window("jelly").unwrap(), shown.window("bubble").unwrap());
        let (w, h) = screen.split_once('x').unwrap();
        let whole = [0.0, 0.0, w.parse().unwrap(), h.parse().unwrap()];
        assert!(apart(jelly, bubble), "{screen}: {jelly:?} under {bubble:?}");
        assert!(inside(jelly, whole) && inside(bubble, whole), "{screen}: {jelly:?} {bubble:?}");
        // The jellyfish stands where it stands with no bubble: a window is never moved for another.
        assert_eq!(drawn_on(&quiet(), screen).window("jelly"), Some(jelly), "{screen}");
    }
    // With room for the margin it is kept: 16 pixels from the corner's two edges.
    assert_eq!(drawn_on(&quiet(), "420x400").window("jelly"), Some([308.0, 288.0, 96.0, 96.0]));
}

// Note 2: a theme file's text sizes are held to the edge scene's own range.
#[test]
fn no_text_size_of_a_theme_file_puts_a_piece_out_of_its_window_or_makes_a_window_huge() {
    let dir = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let file = dir.path().join("sizes.toml");
    let name = "long-project-name-".repeat(5);
    for metric in ["small_size = 60", "small_size = 10000", "body_size = 0", "body_size = 300"] {
        fs::write(&file, format!("[metrics]\n{metric}\n")).unwrap();
        let theme = ["--theme", file.to_str().unwrap()];
        for line in [working(100), heard(SAID, &name, 2000), asks("Which of the two did you mean?")] {
            let shown = placed(&app(&line.to_string(), SCREEN, &theme, &[]));
            for piece in shown.0.iter().filter(|piece| piece.kind() != "window") {
                let window = shown.window(piece.id()).unwrap();
                assert!(window[2] <= 300.0 && window[3] <= 400.0, "{metric}: a window of {window:?}");
                assert!(inside(piece.rect(), [0.0, 0.0, window[2], window[3]]), "{metric}: {piece:?}");
                for run in piece.runs() {
                    let (at, size) = (jelly::at(run), run["size"].as_f64().unwrap());
                    assert!(inside(at, piece.rect()) && at[2] > 0.0 && at[3] > 0.0, "{metric}: {run}");
                    assert!((9.0..=22.0).contains(&size), "{metric}: {run}");
                }
            }
            if line["state"] == "heard" {
                // The words are drawn as words, and a name too long for its line ends in an ellipsis.
                assert!(shown.text("row_main").starts_with("make the"), "{metric}: {}", shown.text("row_main"));
                assert!(shown.text("row_sub").ends_with('…'), "{metric}: {}", shown.text("row_sub"));
            }
        }
    }
}

// Note 3: no path a person typed, and no word of the system's, is said of a file that cannot be read.
#[test]
fn a_file_that_cannot_be_read_is_refused_in_fixed_words_with_its_last_name_on_one_line() {
    let dir = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let missing = dir.path().join("no-such-file.json");
    let turned = dir.path().join("a\u{1b}[31m\nb\u{202e}c.json");
    let cases = [
        (missing.to_str().unwrap().to_string(), "the file cannot be read: no-such-file.json"),
        ("no-such-file.json".to_string(), "the file cannot be read: no-such-file.json"),
        (turned.to_str().unwrap().to_string(), "the file cannot be read: bc.json"),
        (dir.path().to_str().unwrap().to_string() + "/", "the file cannot be read"),
    ];
    for (file, why) in cases {
        for flag in ["--jelly", "--page"] {
            let args: Vec<&str> = match flag {
                "--jelly" => vec!["layout", flag, &file, "--screen", SCREEN],
                _ => vec!["layout", flag, &file],
            };
            let (code, out, err) = said(&run(&args, None));
            assert_eq!((code, out.as_str(), err.as_str()), (Some(1), "", format!("jellymoon-app: {why}\n").as_str()));
        }
    }
}

// Note 4: the echo of a wrong `--screen` is cut at 40 characters, and a signed side is no side.
#[test]
fn a_wrong_screen_is_echoed_cut_short_and_a_signed_side_is_refused() {
    let line = quiet().to_string();
    let long = "9".repeat(5000);
    let (code, out, err) = said(&app(&line, &long, &[], &[]));
    let first = err.lines().next().unwrap();
    assert_eq!((code, out.as_str()), (Some(2), ""), "{first}");
    assert_eq!(
        first,
        format!(
            "jellymoon-app: --screen must be <W>x<H> in pixels, each side from 1 to 16384, not \"{}…\"",
            "9".repeat(40)
        )
    );
    for signed in ["+5x+5", "+1440x900", "1440x+900"] {
        let (code, _, err) = said(&app(&line, signed, &[], &[]));
        assert!(code == Some(2) && err.contains(&format!("not {signed:?}")), "{signed}: {err}");
    }
    // `--size` is read by the same rule.
    let (code, _, err) = said(&run(&["layout", "--waiting", "--size", "+1440x900"], None));
    assert!(code == Some(2) && err.starts_with("jellymoon-app: --size must be <W>x<H>"), "{err}");
    let (code, _, err) = said(&run(&["layout", "--waiting", "--size", &format!("{long}x900")], None));
    assert!(code == Some(2) && err.lines().next().unwrap().ends_with(&format!("not \"{}…\"", "9".repeat(40))), "{err}");
}

// Note 5: a heard wish whose words or project nobody sees, and a line with a second line's break in it.
#[test]
fn words_or_a_project_nobody_sees_and_a_break_inside_the_line_are_no_line_of_a_hearing() {
    let no_lines = [
        heard("\u{200b}", "alpha", 4000).to_string(),
        heard(" \u{2800}\u{200d} ", "alpha", 4000).to_string(),
        heard(SAID, "\u{200b}\u{feff}", 4000).to_string(),
        quiet().to_string().replace(",\"says\"", ",\r\"says\""),
        quiet().to_string().replace(",\"says\"", ",\u{c}\"says\""),
        format!("\u{c}{}", quiet()),
        // M4: words with what was refused, and nothing else that is not a refusal's.
        cannot("nothing was heard").to_string().replace("\"words\":\"\"", "\"words\":\"make it\""),
    ];
    for input in no_lines {
        assert_eq!(said(&app(&input, SCREEN, &[], &[])), (Some(1), String::new(), jelly::NO_LINE.to_string()));
    }
    // A line as Windows ends it is still one.
    assert!(app(&format!("{}\r\n", quiet()), SCREEN, &[], &[]).status.success());
}

// Note 7: four characters that end a line are a space between two words, as a line break is.
#[test]
fn every_character_that_ends_a_line_is_drawn_as_one_space() {
    let shown = drawn(&listening("one\u{2028}two\u{2029}three\u{85}four\u{b}five"));
    assert_eq!(shown.text("row_main"), "one two three four five");
}

// Note 8: marks piled on one letter are capped, so a text of one letter is still drawn as that letter.
#[test]
fn marks_piled_on_one_letter_are_capped_and_the_letter_is_drawn() {
    let piled = format!("a{} tail", "\u{301}".repeat(399));
    let shown = drawn(&listening(&piled));
    let text = shown.text("row_main");
    assert!(text.starts_with('a') && text.ends_with("tail"), "{}", text.escape_unicode());
    assert_eq!(text.chars().filter(|c| *c == '\u{301}').count(), 8);
}
