//! F55: words heard are a heard wish, shown for four seconds and then made a wish through the door a typed wish
//! passes (ADR 0019, decisions 5 and 6; `docs/development/hear.md`). `jellymoon hear` is run as a black box: the
//! events go in on its input, a line each, and the lines it prints are compared whole. Time is the `tick` events
//! and nothing else, so no test here waits for a clock. The harness is the made-up `fake-agent` in F39's test
//! folder (`f39/world.rs`), and every word is typed here. This file holds a hold from its first event to its
//! wish, and what takes a heard wish back; `f55/started.rs` the run `--run` starts and the states a day's runs
//! give; `f55/project.rs` which project is picked and what core refuses; `f55/events.rs` events out of order
//! and lines that are none, with the bounds; `f55/voice.rs` the settings file.
#![allow(clippy::unwrap_used, clippy::expect_used)]

#[path = "f55/events.rs"]
mod events;
#[path = "f55/heard.rs"]
mod heard;
#[path = "f55/project.rs"]
mod project;
#[path = "f55/started.rs"]
mod started;
mod support;
#[path = "f55/voice.rs"]
mod voice;
#[path = "f39/world.rs"]
mod world;

pub use heard::*;
use serde_json::json;
use support::{hub_key, model_key, shows};
pub use world::*;

#[test]
fn a_hold_is_listened_to_and_its_final_words_are_shown_for_four_seconds_and_then_made_a_wish() {
    let w = alpha_world();
    let events = [
        tick(0),
        DOWN.to_string(),
        partial("make the up"),
        tick(900),
        UP.to_string(),
        fin(SAID),
        tick(1000),
        tick(1100),
        tick(4999),
        tick(5000),
        tick(9000),
        tick(60_000),
    ];
    let want = [
        quiet(),
        listening(""),
        listening("make the up"),
        heard(SAID, "alpha", 4000),
        heard(SAID, "alpha", 3900),
        heard(SAID, "alpha", 1),
        with(quiet(), "made", json!("m1")),
    ];
    assert_eq!(w.hear(&[], &events), want);
    // Made once, by the call the entry uses, in the working state; and with no `--run` no harness was started.
    assert_eq!(w.made_wishes(), Some(json!([made("m1", SAID, "alpha")])));
    assert_eq!((w.starts(), w.jm("runs").exists(), w.jm("copies").exists()), (0, false, false));
    assert_eq!(w.on_page("m1")["said"], SAID);
}

// The four seconds are the person's to take the wish back in, so they start when the words can be seen: at the
// first tick at or after the final words, never at the tick before them.
#[test]
fn the_four_seconds_start_at_the_first_tick_at_or_after_the_final_words() {
    let w = alpha_world();
    let said = [tick(0), DOWN.to_string(), tick(1000), UP.to_string(), fin(SAID)];
    let then = |ticks: &[u64]| [&said[..], &ticks.iter().map(|at| tick(*at)).collect::<Vec<_>>()].concat();
    let shown = [quiet(), listening(""), heard(SAID, "alpha", 4000)];
    // A sender that did not tick for four seconds: the words are shown, and nothing is made at that tick.
    assert_eq!(w.hear(&[], &then(&[5000])), shown);
    assert_eq!(w.hear(&[], &then(&[5000, 8999])), [&shown[..], &[heard(SAID, "alpha", 1)]].concat());
    // A tick at the time of the last one is one at the final words, and a tick that goes back is none.
    assert_eq!(w.hear(&[], &then(&[1000, 4999])), [&shown[..], &[heard(SAID, "alpha", 1)]].concat());
    assert_eq!(w.hear(&[], &then(&[900, 5000, 8999])), [&shown[..], &[heard(SAID, "alpha", 1)]].concat());
    assert!(!w.at("jm").exists(), "a heard wish was made before four seconds of ticks had passed");
    let made = [&shown[..], &[with(quiet(), "made", json!("m1"))]].concat();
    assert_eq!(w.hear(&[], &then(&[5000, 9000])), made);
}

// The same events but for the last tick: the input ends one millisecond inside the window.
#[test]
fn nothing_is_written_until_the_four_seconds_have_passed() {
    let w = alpha_world();
    let before = tree_of(&w);
    let mut events = hold(0, SAID);
    events.push(tick(4999));
    let lines = w.hear(&[], &events);
    assert_eq!(lines.last(), Some(&heard(SAID, "alpha", 1)));
    assert!(tree_of(&w) == before && !w.at("jm").exists(), "a heard wish was kept before its window ended");
    events.push(tick(5000));
    assert_eq!(w.hear(&[], &events).last(), Some(&with(quiet(), "made", json!("m1"))));
    assert_eq!(w.made_wishes().unwrap().as_array().unwrap().len(), 1);
}

// Core's own example, which the doc shows (`jellymoon-core/tests/fixtures`): these events print these lines.
#[test]
fn the_events_of_core_s_example_print_the_lines_of_core_s_example() {
    let fixtures = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../jellymoon-core/tests/fixtures");
    let fixture = |name: &str| std::fs::read_to_string(fixtures.join(name)).unwrap().replace("\r\n", "\n");
    let ran = alpha_world().hear_fed(&[], &fixture("hear-events.jsonl"));
    assert_eq!((ran.code, ran.stderr.as_str(), ran.stdout), (0, "", fixture("hear-lines.jsonl")));
}

/// Every file of a world but the made-up agent's own, with its bytes.
fn tree_of(w: &World) -> Vec<(std::path::PathBuf, Vec<u8>)> {
    support::tree(w.dir.path())
}

#[test]
fn a_key_pressed_while_a_heard_wish_is_shown_takes_it_back_held_or_tapped() {
    let w = alpha_world();
    let before = tree_of(&w);
    // Tapped: a press of under 300 ms. The words are dropped, and no later tick makes them a wish.
    let mut tapped = hold(0, SAID);
    tapped.extend([tick(4999), DOWN.to_string(), tick(5298), UP.to_string(), tick(9000), tick(60_000)]);
    let lines = w.hear(&[], &tapped);
    assert_eq!(lines[lines.len() - 3..], [heard(SAID, "alpha", 1), listening(""), quiet()]);
    assert!(tree_of(&w) == before, "a wish that was taken back was kept");

    // Held: it listens anew, and what is said then is the heard wish.
    let again = "make the uploader retry three times";
    let mut held = hold(0, SAID);
    held.extend([tick(3000), DOWN.to_string(), partial("make the"), tick(4000), UP.to_string(), fin(again)]);
    held.extend([tick(4000), tick(7999), tick(8000), tick(20_000)]);
    let lines = w.hear(&[], &held);
    let end = [
        heard(SAID, "alpha", 2000),
        listening(""),
        listening("make the"),
        heard(again, "alpha", 4000),
        heard(again, "alpha", 1),
        with(quiet(), "made", json!("m1")),
    ];
    assert_eq!(lines[lines.len() - 6..], end);
    assert_eq!(w.made_wishes(), Some(json!([made("m1", again, "alpha")])), "only the words said again");
}

#[test]
fn a_hold_under_300_ms_hears_nothing_and_one_of_300_ms_is_heard() {
    let w = alpha_world();
    let slip = [tick(0), DOWN.to_string(), tick(299), UP.to_string(), fin(SAID), tick(10_000)];
    assert_eq!(w.hear(&[], &slip), [quiet(), listening(""), quiet()]);
    assert!(!w.at("jm").exists(), "a slip of the hand made something");
    let held = [tick(0), DOWN.to_string(), tick(300), UP.to_string(), fin(SAID)];
    assert_eq!(w.hear(&[], &held), [quiet(), listening(""), heard(SAID, "alpha", 4000)]);
}

#[test]
fn words_that_are_empty_once_cleaned_make_no_wish_and_no_noise() {
    let w = alpha_world();
    for empty in ["", " \t ", "\u{2065}\u{200b}\u{feff}"] {
        let mut events = hold(0, empty);
        events.push(tick(60_000));
        assert_eq!(w.hear(&[], &events), [quiet(), listening(""), quiet()], "{empty:?}");
    }
    assert!(!w.at("jm").exists());
}

#[test]
fn a_hold_ends_by_itself_at_60_seconds() {
    let w = alpha_world();
    // The key is never let go. At 60 seconds the hold is over, and the ear's last words are its heard wish.
    let events = [tick(0), DOWN.to_string(), tick(59_999), fin("too soon"), tick(60_000), fin(SAID), UP.to_string()];
    assert_eq!(w.hear(&[], &events), [quiet(), listening(""), heard(SAID, "alpha", 4000)]);
    // The tick that ended the hold is the one the ear is waited from.
    let silent = "nothing was heard: the ear did not say what it heard";
    let events = [tick(0), DOWN.to_string(), tick(60_000), tick(64_999)];
    assert_eq!(w.hear(&[], &events), [quiet(), listening("")]);
    assert_eq!(w.hear(&[], &[&events[..], &[tick(65_000)]].concat()), [quiet(), listening(""), cannot(silent)]);
}

// The door a typed wish passes (`still.rs`): one line, nothing unseen, credential shapes replaced, never cut.
#[test]
fn heard_words_and_the_words_so_far_pass_the_door_before_they_are_printed_or_kept() {
    let w = alpha_world();
    let so_far = format!("deploy  with {}\nfr\u{2065}om", model_key());
    let raw = format!("  deploy  with {}\nfr\u{2065}om \"now\" on ", hub_key());
    let clean = "deploy with [redacted] 'now' on";
    let events =
        [tick(0), DOWN.to_string(), partial(&so_far), tick(1000), UP.to_string(), fin(&raw), tick(1000), tick(5000)];
    let ran = w.hear_ran(&[], &events);
    let want = [
        quiet(),
        listening(""),
        listening("deploy with [redacted]"),
        heard(clean, "alpha", 4000),
        with(quiet(), "made", json!("m1")),
    ];
    assert_eq!(lines(&ran), want);
    assert_eq!(w.made_wishes(), Some(json!([made("m1", clean, "alpha")])));
    for secret in [hub_key(), model_key()] {
        assert!(!shows(&ran.stdout, &secret), "{}", ran.stdout);
        let kept = tree_of(&w).into_iter().filter(|(path, _)| path.starts_with(w.at("jm")));
        assert!(kept.into_iter().all(|(_, bytes)| !shows(&String::from_utf8_lossy(&bytes), &secret)));
    }
}
