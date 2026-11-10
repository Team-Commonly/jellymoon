//! F27, what code checks of the model's answer: whose words a quote is, what is not page data, and how many are kept.

use serde_json::{Value, json};

use super::{DAY, World, result, wish, world};
use crate::support::{self, A, said, write_log};

const NEXT_DAY: &[&str] = &["--date", "2026-03-11"];

/// Typed the day after: 306 characters whose 200th ends a word.
fn long() -> String {
    format!("start {}", "abcd ".repeat(60))
}

/// The world with a second day in it, for the rules on quotes.
fn world_with_a_second_day() -> World {
    let w = world();
    let lines = [
        said("2026-03-11T16:00:00.000Z", A, "please do not delete the prod database"),
        said("2026-03-11T17:00:00.000Z", A, "把重试次数改成三次然后再部署一遍"),
        said("2026-03-11T18:00:00.000Z", A, "ship it"),
        said("2026-03-11T19:00:00.000Z", A, "prefix the login, then fix the login"),
        said("2026-03-11T20:00:00.000Z", A, &long()),
    ];
    write_log(&w.dir.path().join("logs"), "-work-alpha/s2.jsonl", &lines);
    w
}

// Item 29, the limits of a quote: long enough to mean something, and whole words.
#[test]
fn a_quote_is_whole_words_and_at_least_eight_characters_or_everything_the_person_typed() {
    let w = world_with_a_second_day();
    let clash = |quote_b: &str| json!({"a": "not", "b": "zh", "quote_a": "do not delete", "quote_b": quote_b, "settled": false});
    let page = w.page(
        json!({
            "wishes": [
                wish("o", "o", "alpha"),                          // one letter of something typed
                wish("part", "ship", "alpha"),                    // short, and not the whole turn
                wish("mid", "lete the prod data", "alpha"),       // starts and ends inside a word
                wish("starts", "lete the prod database", "alpha"), // starts inside a word
                wish("ends", "delete the prod data", "alpha"),    // ends inside a word
                // Whole words the person typed, so it is kept: code cannot see that a "not" stood before them.
                wish("not", "delete the prod database", "alpha"),
                wish("zh", "重试次数改成三次", "alpha"), // Chinese puts no space between words: cut anywhere
                wish("zh4", "改成三次", "alpha"),        // and says more in a character: 4 are enough
                wish("zh3", "改成三", "alpha"),          // 3 are not
                wish("mix", "ship", "alpha"),            // 4 that are not all Chinese are not either
                wish("whole", "ship it", "alpha"),       // short, and everything typed in that turn
                wish("twice", "fix the login", "alpha"), // inside "prefix" the first time, whole words the second
            ],
            "clashes": [clash("o"), clash("试次数改成三次然后再"), clash("三次然")],
        }),
        NEXT_DAY,
    );
    let kept: Vec<&str> =
        page["wishes"].as_array().unwrap().iter().map(|wish| wish["said"].as_str().unwrap()).collect();
    assert_eq!(kept, ["delete the prod database", "重试次数改成三次", "改成三次", "ship it", "fix the login"]);
    // A clash's quotes are held to the same rules: the first is too short, and so is the last, at 3 Chinese characters.
    assert_eq!(
        page["clashes"],
        json!([{"a": "w1", "b": "w2", "quote_a": "do not delete", "quote_b": "试次数改成三次然后再", "settled": false}])
    );
}

// Item 32.
#[test]
fn a_quote_over_200_characters_is_cut_to_its_first_200_and_kept() {
    let w = world_with_a_second_day();
    let typed = long();
    let page = w.page(json!({"wishes": [wish("a", &typed[..230], "alpha")], "clashes": []}), NEXT_DAY);
    assert_eq!(page["wishes"][0]["said"].as_str(), Some(&typed[..200]), "{page}");
}

// Items 29 to 32.
#[test]
fn a_wish_or_a_clash_that_quotes_words_the_person_did_not_type_is_dropped_and_the_models_own_words_are_cleaned() {
    let w = world();
    let (hub, model) = (support::hub_key(), support::model_key());
    let label = format!("Fail\tfast\u{7}\u{202e} {hub} and a tail that runs well past forty characters");
    let stars = json!([
        "retry\nremoved",
        "s".repeat(80),
        format!("used {}\u{200b}{}", &model[..9], &model[9..]),
        " \t ",
        "three",
        "five is one too many"
    ]);
    let clash = |a: &str, b: &str, quote_b: &str, settled: bool| json!({"a": a, "b": b, "quote_a": "add a retry", "quote_b": quote_b, "settled": settled});
    let page = w.page(
        json!({
            "wishes": [
                wish("a", "add a retry to the uploader", "alpha"),
                wish("pony", "I want a pony", "alpha"),            // nobody typed it
                wish("reply", "retry added, tests pass", "alpha"), // the agent said it, not the person
                wish("gamma", "fix the login page", "gamma"),      // no turn ran in gamma
                wish("a", "fail fast", "alpha"),                   // an id already taken
                wish("nothing", " ", "alpha"),                     // no words at all
                {"id": "unnamed", "label": " \t\u{200b}", "said": "fail fast", "project": "alpha", "state": "working", "stars": []},
                {"id": "b", "label": label, "said": "never retry uploads", "project": "alpha", "state": "done", "stars": stars, "mood": "added by the model"},
                wish("c", "fix the login page", "beta"),
            ],
            "clashes": [
                clash("a", "b", "always retry uploads", false), // not typed
                clash("a", "pony", "fail fast", false),         // names a dropped wish
                clash("a", "a", "fail fast", false),            // one wish, not two
                clash("a", "b", "never retry uploads, fail fast", true),
            ],
        }),
        &[],
    );
    let wishes = page["wishes"].as_array().unwrap();
    assert_eq!(wishes.len(), 3, "{wishes:?}");
    assert_eq!(
        wishes[0],
        json!({"id": "w1", "label": "Wish a", "said": "add a retry to the uploader", "project": "alpha", "state": "working", "stars": []})
    );
    // The label has its tab and control character gone and the credential replaced, and is cut to 40 characters. A star's
    // line break is a space; a star is cut to 60; an empty star is no star; only the first 4 are kept.
    let stars = ["retry removed", &format!("{}…", "s".repeat(59)), "used [redacted]", "three"];
    assert_eq!(
        wishes[1],
        json!({"id": "w2", "label": "Fail fast [redacted] and a tail that ru…", "said": "never retry uploads", "project": "alpha", "state": "done", "stars": stars})
    );
    assert_eq!(wishes[1]["label"].as_str().unwrap().chars().count(), 40);
    assert_eq!(
        wishes[2],
        json!({"id": "w3", "label": "Wish c", "said": "fix the login page", "project": "beta", "state": "working", "stars": []})
    );
    assert!(
        !page.to_string().contains(|c: char| c.is_control()) && !page.to_string().contains("\\u00"),
        "a control character is left"
    );
    // A kept clash has its ids rewritten to the page's and keeps `settled`.
    assert_eq!(
        page["clashes"],
        json!([{"a": "w1", "b": "w2", "quote_a": "add a retry", "quote_b": "never retry uploads, fail fast", "settled": true}])
    );
}

// Item 33.
#[test]
fn output_that_is_not_page_data_gives_an_error_and_no_page() {
    let w = world();
    let with = |change: fn(&mut Value)| {
        let mut wish = wish("a", "add a retry", "alpha");
        change(&mut wish);
        result(&json!({"wishes": [wish], "clashes": []}).to_string())
    };
    let good = with(|_| {});
    let cases = [
        ("the model's text is not JSON", result("Here is the page you asked for."), 0),
        ("an unknown state", with(|wish| wish["state"] = json!("finished")), 0),
        ("a missing field", with(|wish| drop(wish.as_object_mut().unwrap().remove("stars"))), 0),
        ("a wrong type", with(|wish| wish["said"] = json!(7)), 0),
        ("no clashes list", result(r#"{"wishes":[]}"#), 0),
        ("a list where the page should be", result("[]"), 0),
        ("the harness reports an error", good.replace(r#""is_error":false"#, r#""is_error":true"#), 0),
        ("the harness exits non-zero", good.clone(), 3),
        ("the harness prints no JSON", "Not logged in".to_string(), 0),
        ("the harness prints nothing", String::new(), 0),
    ];
    assert!(cases[6].1.contains(r#""is_error":true"#));
    for (name, reply, code) in cases {
        let ran = w.run(&reply, code, "page", DAY);
        assert!(
            ran.code == 1 && ran.stdout.is_empty() && ran.stderr.starts_with("jellymoon: "),
            "{name}: exit {}, stdout {:?}, stderr {:?}",
            ran.code,
            ran.stdout,
            ran.stderr
        );
    }
    // Neither error quotes what the model or the harness wrote as it came: no escape, no key, one line.
    let hostile = format!("\u{1b}[2J\u{202e}{}\nsecond line", support::hub_key());
    let state = json!({"wishes": [{"id": "a", "label": "x", "said": "add a retry", "project": "alpha", "state": hostile, "stars": []}], "clashes": []});
    let reported = json!({"type": "result", "is_error": true, "result": hostile});
    for (name, reply) in [("the model's text", result(&state.to_string())), ("the harness's", reported.to_string())] {
        let ran = w.run(&reply, 0, "page", DAY);
        let shown = ran.stderr.trim_end();
        let clean = !shown.contains(|c: char| c.is_control() || c == '\u{202e}') && !shown.contains("a1B2c3D4e5");
        assert!(
            ran.code == 1 && ran.stdout.is_empty() && clean && shown.starts_with("jellymoon: "),
            "{name}: {shown:?}"
        );
    }
    let ran = w.run(&result(&state.to_string()), 0, "page", DAY);
    // The state is the last thing in the wish, on the JSON's one line: the place is named and nothing is quoted.
    assert_eq!(ran.stderr, "jellymoon: the reading is not page data (line 1, column 175 of the model's JSON)\n");
    let ran = w.run(&good, 0, "page", DAY);
    let want = r#"{"day":"2026-03-10","wishes":[{"id":"w1","label":"Wish a","said":"add a retry","project":"alpha","state":"working","stars":[]}],"clashes":[],"read":{"turns":3,"projects":["alpha","beta"]}}"#;
    assert_eq!((ran.stdout, ran.code), (format!("{want}\n"), 0), "{}", ran.stderr);
}

// Item 34.
#[test]
fn nine_wishes_become_eight_and_three_clashes_two() {
    let w = world();
    let wishes: Vec<Value> = (1..=9).map(|n| wish(&format!("n{n}"), "add a retry to the uploader", "alpha")).collect();
    let clash =
        |a: &str, b: &str| json!({"a": a, "b": b, "quote_a": "add a retry", "quote_b": "fail fast", "settled": false});
    let page = w.page(json!({"wishes": wishes, "clashes": [clash("n1", "n9"), clash("n1", "n2"), clash("n2", "n3"), clash("n3", "n4")]}), &[]);
    let kept = page["wishes"].as_array().unwrap();
    assert_eq!(
        (kept.len(), &kept[7]["id"], &kept[7]["label"]),
        (8, &json!("w8"), &json!("Wish n8")),
        "want the first 8"
    );
    // The clash naming the ninth wish goes first, then the first 2 are kept.
    assert_eq!(page["clashes"], json!([clash("w1", "w2"), clash("w2", "w3")]));
}
