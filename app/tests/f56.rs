//! F56: the jellyfish is drawn from core's state with the kit's edge scene. Black-box: each test types one line
//! of `jellymoon hear`, runs the built `jellymoon-app layout --jelly` on it and reads the lines it prints: where
//! the two windows stand on the screen, and the pieces of each. No window is opened, no key is held and nothing
//! is heard. The rules are core's (`crates/jellymoon-core/src/hear/drawn.rs`); the placing and the cut are the
//! kit's (`crates/sticker-kit/tests/edge.rs`).
#![allow(clippy::unwrap_used, clippy::expect_used)]

#[path = "f56/day.rs"]
mod day;
#[path = "f56/jelly.rs"]
mod jelly;
mod support;
#[path = "f56/words.rs"]
mod words;

use std::fs;

use jelly::{
    ASKS, NO_LINE, SCREEN, TAKE_BACK, app, asks, at, cannot, drawn, drawn_on, heard, inside, listening, placed, quiet,
    said, waits, working,
};
use serde_json::{Value, json};
use support::run;

const SAID: &str = "make the uploader retry twice";
const QUESTION: &str = "Which of the two uploaders did you mean?";
const REFUSED: &str = "nothing was heard: no microphone was found";

/// One line of each of the seven states, with its mood and whether it has a bubble.
fn seven() -> [(Value, &'static str, &'static str, bool); 7] {
    [
        (quiet(), "quiet", "floats", false),
        (listening("make the up"), "listening", "leans_in", true),
        (heard(SAID, "alpha", 2000), "heard", "holds_up", true),
        (working(1), "working", "swims", false),
        (asks(QUESTION), "asks", "waves", true),
        (waits(), "waits", "glows", false),
        (cannot(REFUSED), "cannot", "droops", true),
    ]
}

#[test]
fn each_of_the_seven_states_prints_its_own_jellyfish_and_quiet_working_and_waits_print_no_bubble() {
    let mut moods = Vec::new();
    for (line, state, mood, bubble) in seven() {
        let shown = drawn(&line);
        assert_eq!(shown.jellyfish(), (state, mood), "{line}");
        assert_eq!(shown.window("bubble").is_some(), bubble, "{state}: its bubble's window");
        assert_eq!(shown.of("bubble").len(), usize::from(bubble), "{state}: its bubble");
        let mut kinds = vec![("window", "jelly"), ("character", "jelly")];
        if bubble {
            kinds.extend([("window", "bubble"), ("bubble", "bubble")]);
        }
        if state == "heard" {
            kinds.push(("ring", "bubble"));
        }
        assert_eq!(shown.kinds(), kinds, "{state}");
        moods.push(mood);
    }
    moods.sort_unstable();
    moods.dedup();
    assert_eq!(moods.len(), 7, "each state has a mood of its own");
}

#[test]
fn the_bubble_holds_what_core_says_of_each_state_and_nothing_else() {
    let texts = |line: &Value| {
        let shown = drawn(line);
        let text = |role: &str| shown.text(role);
        (shown.roles().join(" "), text("row_main"), text("row_sub"), text("body"), text("quoted"))
    };
    let none = String::new;
    let own = |text: &str| ("row_main".to_string(), text.to_string(), none(), none(), none());
    assert_eq!(texts(&listening("make the up")), own("make the up"));
    assert_eq!(
        texts(&heard(SAID, "alpha", 4000)),
        ("row_main row_sub body".to_string(), SAID.to_string(), "alpha".to_string(), TAKE_BACK.to_string(), none())
    );
    assert_eq!(texts(&asks(QUESTION)), ("body quoted".to_string(), none(), none(), ASKS.to_string(), QUESTION.into()));
    assert_eq!(texts(&cannot(REFUSED)), ("body".to_string(), none(), none(), REFUSED.to_string(), none()));
    // No words yet, no bubble: the window of one is opened only for words.
    let silent = drawn(&listening(""));
    assert_eq!((silent.jellyfish(), silent.kinds().len()), (("listening", "leans_in"), 2));
    // Words that hold nothing a person sees are no words: no bubble is opened for them.
    let unseen = drawn(&listening("\u{200b} \u{7}\u{202e}"));
    assert_eq!((unseen.window("bubble"), unseen.kinds().len()), (None, 2));
    // The sentence core has for work that waits is the page's to say: the jellyfish only glows.
    assert_eq!(drawn(&waits()).runs().len(), 0);
    // A wish that was just made is named for the shell, and draws nothing.
    let mut made = quiet();
    made["made"] = json!("m1");
    assert_eq!(drawn(&made).bare(), drawn(&quiet()).bare());
}

#[test]
fn runs_going_are_counted_on_the_jellyfish_when_more_than_one_goes() {
    assert_eq!(drawn(&working(1)).of("count").len(), 0, "one run is the jellyfish swimming, and no number");
    for (going, text) in [(2, "2"), (3, "3"), (99, "99"), (100, "99+")] {
        let shown = drawn(&working(going));
        let count = shown.of("count");
        assert_eq!((count.len(), count[0].said(), count[0].id()), (1, text.to_string(), "jelly"), "{going}");
        assert!(shown.window("bubble").is_none(), "a count opens no bubble");
        assert!(inside(count[0].rect(), [0.0, 0.0, 96.0, 96.0]), "{:?}", count[0]);
    }
}

#[test]
fn both_windows_stand_in_the_lower_right_corner_and_no_text_moves_or_resizes_one() {
    let lines = [listening("make"), heard(SAID, "alpha", 4000), asks(QUESTION), cannot(REFUSED)];
    let mut bubbles = Vec::new();
    for line in &lines {
        let shown = drawn(line);
        // 16 pixels from the right and the bottom edge of a screen of 1440 by 900.
        assert_eq!(shown.window("jelly"), Some([1328.0, 788.0, 96.0, 96.0]), "{line}");
        let (jelly, bubble) = ([1328.0, 788.0, 96.0, 96.0], shown.window("bubble").unwrap());
        assert!(inside(bubble, [0.0, 0.0, 1440.0, 900.0]), "{bubble:?}");
        assert_eq!(bubble[0] + bubble[2] + 8.0, jelly[0], "the bubble stands left of the jellyfish, 8 pixels off");
        assert_eq!(bubble[1] + bubble[3], jelly[1] + jelly[3], "their lower edges are level");
        bubbles.push(bubble);
        // A piece is placed in its own window, from that window's top left; a run in its piece.
        for piece in shown.0.iter().filter(|piece| piece.kind() != "window") {
            let window = shown.window(piece.id()).unwrap();
            assert!(inside(piece.rect(), [0.0, 0.0, window[2], window[3]]), "{piece:?} in {window:?}");
            piece.runs().iter().for_each(|run| assert!(inside(at(run), piece.rect()), "{run} in {piece:?}"));
        }
    }
    assert!(bubbles.iter().all(|bubble| *bubble == bubbles[0]), "{bubbles:?}");
    assert_eq!(drawn(&quiet()).window("jelly"), Some([1328.0, 788.0, 96.0, 96.0]));
    assert_eq!(drawn_on(&quiet(), "960x640").window("jelly"), Some([848.0, 528.0, 96.0, 96.0]));
}

#[test]
fn a_screen_smaller_than_a_window_keeps_the_window_whole_from_the_screen_s_top_left() {
    let shown = drawn_on(&heard(SAID, "alpha", 4000), "64x48");
    let (jelly, bubble) = (shown.window("jelly").unwrap(), shown.window("bubble").unwrap());
    assert_eq!(jelly, [0.0, 0.0, 96.0, 96.0]);
    assert_eq!((bubble[0], bubble[1]), (0.0, 0.0));
    assert_eq!((bubble[2], bubble[3]), {
        let whole = drawn(&heard(SAID, "alpha", 4000)).window("bubble").unwrap();
        (whole[2], whole[3])
    });
    // A screen with room for the jellyfish and none beside it: the bubble stands over it.
    let narrow = drawn_on(&cannot(REFUSED), "320x900");
    let (jelly, bubble) = (narrow.window("jelly").unwrap(), narrow.window("bubble").unwrap());
    assert!(inside(bubble, [0.0, 0.0, 320.0, 900.0]) && bubble[1] + bubble[3] + 8.0 == jelly[1], "{bubble:?}");
}

#[test]
fn the_ring_is_as_full_as_the_heard_wish_s_four_seconds_are_left() {
    let ring = |left: u64| drawn(&heard(SAID, "alpha", left)).ring();
    assert_eq!([ring(4000), ring(3000), ring(2000), ring(1), ring(0)], [1.0, 0.75, 0.5, 0.0, 0.0].map(Some));
    // The time left changes the ring's number and nothing else: no piece moves.
    assert_eq!(drawn(&heard(SAID, "alpha", 4000)).bare(), drawn(&heard(SAID, "alpha", 250)).bare());
    // Words long enough to fill their three lines, a name to fill its one: no line reaches under the ring.
    let shown = drawn(&heard(&format!("{SAID} ").repeat(8), &"alpha-".repeat(20), 2000));
    assert!(shown.text("row_main").ends_with('…') && shown.text("row_sub").ends_with('…'));
    let (ring, bubble) = (shown.of("ring")[0].rect(), shown.of("bubble")[0].rect());
    assert!(inside(ring, bubble) && ring[2] == ring[3] && ring[2] > 0.0, "{ring:?} in {bubble:?}");
    // No run of text is under the ring.
    for run in shown.runs() {
        let [x, y, w, h] = at(run);
        let apart = x + w <= ring[0] || y + h <= ring[1] || y >= ring[1] + ring[3];
        assert!(apart, "{run} is under the ring {ring:?}");
    }
    for (line, state, ..) in seven() {
        assert_eq!(drawn(&line).ring().is_some(), state == "heard", "{state}");
    }
}

#[test]
fn a_line_is_read_from_a_file_as_from_stdin_and_a_theme_changes_no_piece() {
    let dir = support::scratch();
    let file = dir.path().join("line.json");
    let line = heard(SAID, "alpha", 2000);
    // With its line break, with none, and as Windows ends a line.
    for written in [format!("{line}\n"), line.to_string(), format!("{line}\r\n")] {
        fs::write(&file, written).unwrap();
        let from_file = placed(&run(&["layout", "--jelly", file.to_str().unwrap(), "--screen", SCREEN], None));
        assert_eq!(from_file.bare(), drawn(&line).bare());
        assert_eq!(from_file.text("row_main"), SAID);
    }
    let plain =
        placed(&run(&["layout", "--jelly", "-", "--screen", SCREEN, "--theme", "plain"], Some(&line.to_string())));
    assert_eq!(plain.kinds(), drawn(&line).kinds());
    assert_eq!((plain.text("row_main"), plain.text("row_sub")), (SAID.to_string(), "alpha".to_string()));
}

#[test]
fn a_line_that_is_no_line_of_hear_is_refused_in_one_sentence_and_nothing_is_drawn() {
    let long = format!(
        "{{\"state\":\"quiet\",\"says\":\"\",\"words\":\"{}\",\"project\":\"\",\"left\":0}}",
        "a".repeat(70_000)
    );
    let mut unknown = quiet();
    unknown["clock"] = json!(1);
    let no_lines = [
        String::new(),
        "\n".to_string(),
        "not an event".to_string(),
        "{}".to_string(),
        "[]".to_string(),
        r#"{"key":"down"}"#.to_string(),
        r#"{"day":"2026-03-10","wishes":[],"clashes":[],"read":{"turns":0,"projects":[]}}"#.to_string(),
        unknown.to_string(),
        quiet().to_string().replace("quiet", "asleep"),
        quiet().to_string().replace(",\"left\":0", ""),
        format!("{}\n{}\n", quiet(), quiet()),
        quiet().to_string().replace("\"says\"", "\n\"says\""),
        // Lines core's hearing never prints: words, a project, a time, a count or a question out of their state.
        listening("x").to_string().replace("listening", "quiet"),
        heard(SAID, "", 4000).to_string(),
        heard("", "alpha", 4000).to_string(),
        heard(SAID, "alpha", 4001).to_string(),
        heard(SAID, "alpha", 4000).to_string().replace(TAKE_BACK, "press return"),
        listening("x").to_string().replace("\"left\":0", "\"left\":1"),
        working(0).to_string(),
        working(1).to_string().replace(",\"going\":1", ""),
        working(2).to_string().replace("working", "waits"),
        asks(QUESTION).to_string().replace("asks\"", "quiet\""),
        cannot("").to_string(),
        long,
    ];
    for input in no_lines {
        let shown: String = input.chars().take(120).collect();
        assert_eq!(said(&app(&input, SCREEN, &[], &[])), (Some(1), String::new(), NO_LINE.to_string()), "{shown}");
    }
    // A file that is not there is said as a page's is, in fixed words (`f57.rs`), and is no line either.
    let (code, out, err) = said(&run(&["layout", "--jelly", "no-such-file.json", "--screen", SCREEN], None));
    assert_eq!((code, out.as_str()), (Some(1), ""), "{err}");
    assert_eq!(err, "jellymoon-app: the file cannot be read: no-such-file.json\n");
}

#[test]
fn a_screen_that_is_no_width_by_height_is_a_wrong_command_line_and_no_window_is_ever_opened() {
    let line = quiet().to_string();
    for screen in ["1440", "x", "1440x", "x900", "0x900", "1440x0", "axb", "1440x900x2", "-1x5", "16385x900", "1e3x900"]
    {
        let (code, out, err) = said(&app(&line, screen, &[], &[]));
        assert_eq!((code, out.as_str()), (Some(2), ""), "{screen}: {err}");
        let why =
            format!("jellymoon-app: --screen must be <W>x<H> in pixels, each side from 1 to 16384, not {screen:?}\n");
        assert!(err.starts_with(&why) && err.contains("usage:"), "{screen}: {err}");
    }
    assert_eq!(placed(&app(&line, "1x1", &[], &[])).window("jelly"), Some([0.0, 0.0, 96.0, 96.0]));
    assert!(placed(&app(&line, "16384x16384", &[], &[])).window("jelly").is_some());
    // `--jelly` is `layout`'s alone and needs a screen; a screen is of a jellyfish alone; a page is another command.
    let wrong: [&[&str]; 10] = [
        &["layout", "--jelly", "-", "--screen", SCREEN, "--fresh"],
        &["layout", "--jelly", "-", "--screen", SCREEN, "--skip", "alpha"],
        &["layout", "--jelly", "-", "--screen", SCREEN, "--model", "some"],
        &["--jelly", "-", "--screen", SCREEN],
        &["layout", "--jelly", "-"],
        &["layout", "--screen", SCREEN],
        &["layout", "--jelly", "-", "--screen", SCREEN, "--page", "-"],
        &["layout", "--jelly", "-", "--screen", SCREEN, "--waiting"],
        &["layout", "--jelly", "-", "--screen", SCREEN, "--size", "1440x900"],
        &["layout", "--jelly", "-", "--screen", SCREEN, "--logs", "logs"],
    ];
    for args in wrong {
        let (code, out, err) = said(&run(args, Some(&line)));
        assert_eq!((code, out.as_str()), (Some(2), ""), "{args:?}: {err}");
        assert!(err.starts_with("jellymoon-app: --jelly ") && err.contains("usage:"), "{args:?}: {err}");
    }
    let help = String::from_utf8(run(&["--help"], None).stdout).unwrap();
    assert!(help.contains("layout --jelly <file, or - for stdin> --screen <W>x<H>"), "{help}");
}
