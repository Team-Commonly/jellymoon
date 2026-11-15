//! Events out of order, lines that are no event, what the ear refuses, and every bound of a hearing.

use serde_json::json;

use super::*;

const LONG: &str = "a wish is at most 200 characters, and this one is longer";
const MARKS: &str =
    "nothing is left of these words but the mark of what cleaning replaced, so nothing was made of them";
const EAR_SILENT: &str = "nothing was heard: the ear did not say what it heard";
/// What `jellymoon hear` says, once, of the lines it passed over.
fn passed_over(lines: usize) -> String {
    format!("jellymoon: lines of the input that were no event, and were passed over: {lines}\n")
}

#[test]
fn events_out_of_order_change_nothing() {
    let w = alpha_world();
    // A key let go that was never pressed, and the ear's words with no hold: nothing is shown but the first line.
    let stray = [UP.to_string(), fin(SAID), partial("make"), tick(0), fin(SAID), tick(60_000)];
    assert_eq!(w.hear(&[], &stray), [quiet()]);
    // A second press, a second letting go, a tick that goes back, and the final words of a wish already heard.
    // The second press starts no new hold: the words so far stay, and the hold is as long as it was.
    let mut events = vec![tick(0), DOWN.to_string(), partial("make"), tick(800), DOWN.to_string(), tick(1000)];
    events.extend([tick(200), UP.to_string(), UP.to_string(), tick(1100), fin(SAID)]);
    events.extend([fin("another thing"), partial("another"), tick(900)]);
    assert_eq!(w.hear(&[], &events), [quiet(), listening(""), listening("make"), heard(SAID, "alpha", 4000)]);
    assert!(!w.at("jm").exists());
}

#[test]
fn the_first_tick_starts_the_clock_whatever_its_number_and_no_tick_at_all_is_no_time() {
    let w = alpha_world();
    let from = 1_741_600_000_000_u64;
    let mut events = vec![tick(from), DOWN.to_string(), tick(from + 300), UP.to_string(), fin(SAID)];
    events.extend([tick(from + 300), tick(from + 4299), tick(from + 4300)]);
    let want = [
        quiet(),
        listening(""),
        heard(SAID, "alpha", 4000),
        heard(SAID, "alpha", 1),
        with(quiet(), "made", json!("m1")),
    ];
    assert_eq!(w.hear(&[], &events), want);
    // With no tick no time passes: every hold is a slip, and a heard wish is never made.
    let timeless = [DOWN, UP, r#"{"said":"final","words":"make it"}"#];
    assert_eq!(w.hear(&[], &timeless), [quiet(), listening(""), quiet()]);
    assert_eq!(w.made_wishes().unwrap().as_array().unwrap().len(), 1);
}

#[test]
fn a_line_that_is_no_event_is_passed_over_and_counted_and_none_of_it_is_said() {
    let w = alpha_world();
    let odd = [
        "zebra-quartz is not JSON",
        "{\"key\":\"down\"",
        r#"{"key":"sideways"}"#,
        r#"{"key":"down","zebra-quartz":1}"#,
        r#"{"key":"down","tick":3}"#,
        r#"{"KEY":"down"}"#,
        r#"{"tick":-1}"#,
        r#"{"tick":1.5}"#,
        r#"{"tick":"0"}"#,
        r#"{"tick":18446744073709551616}"#,
        r#"{"said":"final"}"#,
        r#"{"said":"final","words":7}"#,
        r#"{"said":"final","words":"zebra-quartz","why":"failed"}"#,
        r#"{"said":"whispered","words":"zebra-quartz"}"#,
        r#"{"said":"cannot","why":"zebra-quartz"}"#,
        r#"{"said":"cannot"}"#,
        r#"{"words":"zebra-quartz"}"#,
        r#"[{"key":"down"}]"#,
        r#""down""#,
        "null",
        "{}",
        "7",
    ];
    let mut events = Vec::new();
    for event in hold(0, SAID) {
        events.extend(odd.iter().map(ToString::to_string));
        events.push(event);
    }
    let ran = w.hear_ran(&[], &events);
    assert_eq!((ran.code, ran.stderr.as_str()), (0, passed_over(odd.len() * 6).as_str()), "{}", ran.stdout);
    let printed: Vec<serde_json::Value> = ran.stdout.lines().map(|line| serde_json::from_str(line).unwrap()).collect();
    assert_eq!(printed, [quiet(), listening(""), heard(SAID, "alpha", 4000)]);
    assert!(!ran.stdout.contains("zebra") && !ran.stderr.contains("zebra"));
}

#[test]
fn an_empty_line_and_a_line_that_ends_in_a_carriage_return_are_not_counted() {
    let w = alpha_world();
    let input = format!("\n{}\r\n\r\n  \n{DOWN}\r\n \t{}  \r\n{UP}\n\n{}", tick(0), tick(300), fin(SAID));
    // The last line has no line break, and is an event all the same.
    assert_eq!(lines(&w.hear_fed(&[], &input)), [quiet(), listening(""), heard(SAID, "alpha", 4000)]);
}

#[test]
fn a_line_of_64_kib_is_an_event_and_a_longer_one_is_read_past() {
    let w = alpha_world();
    // 65536 bytes with its line break: the event, then spaces, which JSON allows after it.
    let most = format!("{DOWN}{}", " ".repeat(64 * 1024 - 1 - DOWN.len()));
    assert_eq!(most.len() + 1, 64 * 1024);
    assert_eq!(w.hear(&[], &[tick(0), most.clone()]), [quiet(), listening("")]);
    let over = [tick(0), format!("{most} "), tick(300), UP.to_string(), partial(&"x".repeat(70_000)), tick(400)];
    let ran = w.hear_ran(&[], &over);
    assert_eq!((ran.code, ran.stderr.as_str()), (0, passed_over(2).as_str()));
    let quiet = "{\"state\":\"quiet\",\"says\":\"\",\"words\":\"\",\"project\":\"\",\"left\":0}\n";
    assert_eq!(ran.stdout, quiet, "a line read past pressed no key");
    // The input's last line with no line break is held to the same length: it has one byte fewer to be an event in.
    let ran = w.hear_fed(&[], &format!("{}\n{most}", tick(0)));
    assert_eq!(lines(&ran), [super::quiet(), listening("")]);
    let ran = w.hear_fed(&[], &format!("{}\n{most} ", tick(0)));
    assert_eq!((ran.code, ran.stderr.as_str(), ran.stdout.as_str()), (0, passed_over(1).as_str(), quiet));
}

// What is refused by an event is said for six seconds of ticks from the first tick at or after it, and the ear's
// final words are waited for five seconds from the first tick at or after the key went up.
#[test]
fn a_refusal_s_six_seconds_and_the_ear_s_five_start_at_the_first_tick_at_or_after_them() {
    let w = alpha_world();
    let failed = cannot("nothing was heard: the recognizer failed");
    let refused = |ticks: &[u64]| {
        let events = [vec![tick(0), ear_cannot("failed")], ticks.iter().map(|at| tick(*at)).collect()].concat();
        w.hear(&[], &events)
    };
    assert_eq!(refused(&[7000, 12_999]), [quiet(), failed.clone()]);
    assert_eq!(refused(&[7000, 13_000]), [quiet(), failed.clone(), quiet()]);
    assert_eq!(refused(&[0, 5999]), [quiet(), failed.clone()]);
    assert_eq!(refused(&[0, 6000]), [quiet(), failed, quiet()]);
    // Words the door refuses are refused by an event too.
    let long = [tick(0), DOWN.to_string(), tick(1000), UP.to_string(), fin(&"word ".repeat(41)), tick(7000)];
    let then = |at: u64| w.hear(&[], &[&long[..], &[tick(at)]].concat());
    assert_eq!(then(12_999), [quiet(), listening(""), cannot(LONG)]);
    assert_eq!(then(13_000), [quiet(), listening(""), cannot(LONG), quiet()]);

    let waited = |ticks: &[u64]| {
        let events =
            [vec![tick(0), DOWN.to_string(), tick(1000), UP.to_string()], ticks.iter().map(|at| tick(*at)).collect()];
        w.hear(&[], &events.concat())
    };
    assert_eq!(waited(&[9000, 13_999]), [quiet(), listening("")]);
    assert_eq!(waited(&[9000, 14_000]), [quiet(), listening(""), cannot(EAR_SILENT)]);
    assert!(!w.at("jm").exists());
}

#[test]
fn the_words_so_far_are_cut_at_200_characters_and_change_4096_times_in_a_hold_at_most() {
    let w = alpha_world();
    let ran = w.hear(&[], &[tick(0), DOWN.to_string(), partial(&"wörd ".repeat(100))]);
    let shown = ran[2]["words"].as_str().unwrap();
    assert_eq!((shown.chars().count(), shown.starts_with("wörd wörd wörd")), (200, true), "{shown}");

    let mut events = vec![tick(0), DOWN.to_string()];
    events.extend((1..=4097).map(|n| partial(&format!("word {n}"))));
    events.extend([tick(1000), UP.to_string(), partial("after the hold")]);
    let printed = w.hear(&[], &events);
    assert_eq!(printed.len(), 2 + 4096, "{:?}", printed.last());
    assert_eq!(printed.last(), Some(&listening("word 4096")));
}

#[test]
fn what_is_refused_is_said_for_six_seconds() {
    let w = alpha_world();
    let long = format!("{}x", "word ".repeat(39) + "words");
    assert_eq!(long.chars().count(), 201);
    let mut events = hold(0, &long);
    events.push(tick(6999));
    assert_eq!(w.hear(&[], &events), [quiet(), listening(""), cannot(LONG)]);
    events.push(tick(7000));
    assert_eq!(w.hear(&[], &events), [quiet(), listening(""), cannot(LONG), quiet()]);
    // 200 characters are a wish.
    let fits = "word ".repeat(39) + "words";
    assert_eq!(w.hear(&[], &hold(0, &fits)).last(), Some(&heard(&fits, "alpha", 4000)));
    // Words too long to be looked at are refused the same way, and nothing of them is printed.
    let ran = w.hear_ran(&[], &hold(0, &format!("zebra{}", " ".repeat(4096))));
    assert_eq!(lines(&ran).last(), Some(&cannot(LONG)));
    assert!(!ran.stdout.contains("zebra") && !w.at("jm").exists());
    // Words of which cleaning leaves only the mark say nothing: no wish is made of them, a window later either.
    let mut events = hold(0, &crate::support::map_key("kLmNo"));
    events.push(tick(6999));
    assert_eq!(w.hear(&[], &events), [quiet(), listening(""), cannot(MARKS)]);
    assert!(!w.at("jm").exists());
}

#[test]
fn an_ear_that_does_not_answer_in_five_seconds_is_said_and_the_hold_is_over() {
    let w = alpha_world();
    let mut events =
        vec![tick(0), DOWN.to_string(), partial("make"), tick(1000), UP.to_string(), tick(1000), tick(5999)];
    assert_eq!(w.hear(&[], &events), [quiet(), listening(""), listening("make")]);
    events.extend([tick(6000), fin(SAID), tick(11_999)]);
    assert_eq!(w.hear(&[], &events), [quiet(), listening(""), listening("make"), cannot(EAR_SILENT)]);
    events.push(tick(12_000));
    assert_eq!(w.hear(&[], &events).last(), Some(&quiet()));
    assert!(!w.at("jm").exists());
}

#[test]
fn each_thing_the_ear_cannot_do_is_said_in_core_s_own_sentence_and_ends_the_hold() {
    let says = [
        ("no_ear", "voice does not work on this system yet: type the wish on the page"),
        ("no_microphone", "nothing was heard: no microphone was found"),
        (
            "microphone_refused",
            "nothing was heard: Jellymoon may not use the microphone; allow it in System Settings, Privacy & \
             Security, Microphone",
        ),
        (
            "speech_refused",
            "nothing was heard: Jellymoon may not recognize speech; allow it in System Settings, Privacy & Security, \
             Speech Recognition",
        ),
        (
            "not_on_device",
            "nothing was heard: this Mac cannot recognize that language on the device, and Jellymoon sends no sound \
             anywhere",
        ),
        ("no_such_language", "nothing was heard: the language in voice.toml is not one this Mac recognizes"),
        ("not_bundled", "nothing was heard: voice works only in Jellymoon.app, started as an app"),
        ("failed", "nothing was heard: the recognizer failed"),
    ];
    let w = alpha_world();
    for (why, sentence) in says {
        // While the key is held: the hold is over, and the words that follow are no one's.
        let events = [tick(0), DOWN.to_string(), tick(1000), ear_cannot(why), UP.to_string(), fin(SAID), tick(6999)];
        assert_eq!(w.hear(&[], &events), [quiet(), listening(""), cannot(sentence)], "{why}");
        // With no hold at all it is said too: the ear may refuse as the app starts.
        assert_eq!(w.hear(&[], &[ear_cannot(why)]), [quiet(), cannot(sentence)], "{why}");
    }
    assert!(!w.at("jm").exists());
}

#[test]
fn what_the_ear_refuses_while_a_heard_wish_is_shown_leaves_the_wish_and_is_said_after_it() {
    let w = alpha_world();
    let mut events = hold(0, SAID);
    events.extend([tick(2000), ear_cannot("failed"), tick(2000), tick(5000), tick(7999), tick(8000)]);
    let failed = "nothing was heard: the recognizer failed";
    let want = [
        quiet(),
        listening(""),
        heard(SAID, "alpha", 4000),
        heard(SAID, "alpha", 3000),
        with(cannot(failed), "made", json!("m1")),
        quiet(),
    ];
    assert_eq!(w.hear(&[], &events), want);
}
