//! A made wish's words are cleaned until cleaning changes nothing (`still.rs` in core), so a wish that was kept is
//! never what makes the day's file refused, and words kept by a build before that rule are still drawn.

use std::fs;

use serde_json::{Value, json};

use super::{DAY, World, made, world};
use crate::support::{key_header, pay_key, restless, shows, tree};

const CHANGING: &str = "jellymoon: a wish is kept as cleaning leaves it, and cleaning was not done with these words \
                        after 8 times\n";

fn said(wishes: &[Value]) -> Vec<&str> {
    wishes.iter().map(|wish| wish["said"].as_str().unwrap()).collect()
}

impl World {
    /// Leaves these words as the day's made wishes, in a file as `jellymoon wish` leaves it.
    fn leave(&self, words: &[String]) -> String {
        let wishes: Vec<Value> = (1..).zip(words).map(|(n, said)| made(&format!("m{n}"), said, "alpha")).collect();
        let text = json!({"day": DAY, "wishes": wishes}).to_string();
        fs::write(self.wishes(), &text).unwrap();
        text
    }
}

#[test]
fn words_one_cleaning_is_not_done_with_are_kept_as_they_stand_still_and_the_day_s_wishes_stay() {
    let w = world();
    let (mut seen, mut kept) = (w.wish("the one before", "alpha").to_string(), vec!["the one before".to_string()]);
    for (typed, still, _) in restless() {
        if still.is_empty() {
            let (before, ran) = (tree(w.dir.path()), w.say(&typed, "alpha"));
            assert_eq!((ran.code, ran.stdout.as_str(), ran.stderr.as_str()), (1, "", CHANGING));
            assert!(tree(w.dir.path()) == before, "a wish that is refused writes nothing");
            continue;
        }
        let wished = w.wish(&typed, "alpha");
        assert_eq!(wished["wish"]["said"], still.as_str());
        seen += &wished.to_string();
        kept.push(still);
    }
    seen += &w.wish("the one after", "alpha").to_string();
    kept.push("the one after".to_string());
    let page = w.page();
    assert_eq!(said(&page), kept, "every made wish of the day is drawn");
    seen += &format!("{}{}", fs::read_to_string(w.wishes()).unwrap(), json!(page));
    for secret in restless().into_iter().flat_map(|(_, _, planted)| planted) {
        assert!(!shows(&seen, &secret), "no 8 characters in a row of what was planted are kept or printed");
    }
}

#[test]
fn ordinary_words_are_kept_exactly_as_typed() {
    let w = world();
    let typed = [
        "make the uploader retry twice, then stop: it's 50% done (see docs/notes.md, a/b [x])",
        "ship it \u{1f469}\u{200d}\u{1f4bb} today, as a \u{1f468}\u{200d}\u{1f469}\u{200d}\u{1f467} would",
        "\u{645}\u{6cc}\u{200c}\u{62e}\u{648}\u{627}\u{647}\u{645} \u{627}\u{6cc}\u{646} \u{6a9}\u{627}\u{631} \u{62a}\u{645}\u{627}\u{645} \u{634}\u{648}\u{62f}",
    ];
    for (n, words) in (1..).zip(typed) {
        assert_eq!(w.wish(words, "alpha")["wish"], made(&format!("m{n}"), words, "alpha"));
    }
    assert_eq!(said(&w.page()), typed);
}

// Made wishes a build before this rule kept, cleaned once: a key that stood straight after another is still in the
// file. They are drawn as they stand still, and the next wish made writes them so.
#[test]
fn made_wishes_kept_after_one_cleaning_are_drawn_as_they_stand_still_and_written_so_by_the_next_wish() {
    let w = world();
    w.wish("the one before", "alpha");
    let once = [
        "the one before".to_string(),
        format!("paste {} and go on", key_header(" ")),
        format!("use [redacted]{} here", pay_key()),
        "two\n\"lines\"".to_string(),
    ];
    let still = ["the one before", "paste [redacted]", "use [redacted][redacted] here", "two 'lines'"];
    let left = w.leave(&once);
    let page = w.page();
    assert_eq!(said(&page), still);
    assert!(!shows(&json!(page).to_string(), &pay_key()), "the key that was kept is not drawn");
    assert_eq!(fs::read_to_string(w.wishes()).unwrap(), left, "drawing a page writes nothing");
    assert_eq!(w.wish("one more", "alpha")["wish"]["id"], "m5");
    assert_eq!(said(w.kept()["wishes"].as_array().unwrap()), [&still[..], &["one more"]].concat());
    assert!(!shows(&fs::read_to_string(w.wishes()).unwrap(), &pay_key()), "and is in the file no more");
}
