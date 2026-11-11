//! A clash's statement is cleaned until cleaning changes nothing (`still.rs` in core): the words a ruling carries are
//! the words the next tap compares, whether `jellymoon rule` or a tap made it. A statement that eight cleanings are
//! not done with is ruled on by neither.

use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

use super::{B, DAY, World, tree, world};
use crate::support::{key_header, key_id, map_key, restless, shows};

const UNMADE: &str = "jellymoon: a statement of over 4096 bytes, or one that cleaning still changes after 8 times, \
                      cannot be ruled on\n";

impl World {
    fn rule(&self, stands: &str, over: &str) -> super::Ran {
        self.fed(&["rule", "--stands", stands, "--over", over], "")
    }

    /// `rule` and a tap on either statement, each refused in the one line, with nothing written: of a clash whose
    /// first statement it is, and of one whose second it is.
    fn unmade(&self, typed: &str) {
        self.keep(&[(typed, B), (B, typed)]);
        let before = tree(self.dir.path());
        let taps =
            [("1", "1"), ("1", "2"), ("2", "1"), ("2", "2")].map(|(clash, option)| self.settle(DAY, clash, option));
        for ran in [self.rule(typed, B), self.rule(B, typed)].into_iter().chain(taps) {
            assert_eq!((ran.code, ran.stdout.as_str(), ran.stderr.as_str()), (1, "", UNMADE));
        }
        assert!(tree(self.dir.path()) == before && self.told("s-1").is_empty(), "no ruling is kept or told");
    }
}

#[test]
fn a_statement_one_cleaning_is_not_done_with_is_ruled_as_it_stands_still_by_a_tap_and_by_rule() {
    for (typed, still, planted) in restless() {
        if still.is_empty() {
            world().unmade(&typed);
            continue;
        }
        let w = world();
        w.keep(&[(&typed, B)]);
        let tapped = w.tap("1", "1");
        let stands = (tapped["stands"].as_str(), tapped["ruling"]["stands"].as_str());
        assert_eq!(stands, (Some(still.as_str()), Some(still.as_str())));
        assert_eq!(w.tap("1", "1")["ruling"], Value::Null, "the same tap again finds the ruling it made");
        let told = w.told("s-1");
        assert!(told.contains(&format!("- What stands: \"{still}\". It replaces: \"{B}\".")), "{told}");
        let mut seen = format!("{tapped}{told}{:?}{}", w.rulings(), fs::read_to_string(w.ruled()).unwrap());

        let w = world();
        let ran = w.rule(&typed, B);
        assert_eq!((ran.code, ran.stderr.as_str()), (0, ""), "rule {typed:?}");
        assert_eq!(serde_json::from_str::<Value>(&ran.stdout).unwrap()["stands"], still.as_str());
        w.keep(&[(&typed, B)]);
        assert_eq!(w.tap("1", "1")["ruling"], Value::Null, "a tap finds the ruling `rule` made of the same words");
        seen += &format!("{}{:?}{}", ran.stdout, w.rulings(), w.told("s-1"));
        for secret in planted {
            assert!(!shows(&seen, &secret), "no 8 characters in a row of what was planted are kept or told");
        }
    }
}

// The 225 characters before the keys are cut away once the text stands still, and the cut takes none of the eight
// cleanings: six keys and eight are kept, nine are not. A statement of 4097 bytes is not cleaned at all.
#[test]
fn a_long_statement_has_its_eight_cleanings_and_is_cut_after_them_and_what_the_cut_leaves_is_cleaned() {
    let (w, words, key) = (world(), "word ".repeat(45), map_key("kLmNo"));
    let ruled = |stands: &str| {
        let ran = w.rule(stands, B);
        assert_eq!((ran.code, ran.stderr.as_str()), (0, ""));
        assert!(!shows(&ran.stdout, &key) && !shows(&ran.stdout, &key_id()), "{}", ran.stdout);
        serde_json::from_str::<Value>(&ran.stdout).unwrap()["stands"].as_str().unwrap().to_string()
    };
    for keys in [6, 8] {
        assert_eq!(ruled(&format!("{words}{}", key.repeat(keys))), format!("{}…", &words[..199]), "{keys} keys");
    }
    // The cut ends a key id that had more capitals after it, so what the cut leaves is cleaned again: that is one of
    // the eight, and the cut itself is none. Seven keys cut away and the id are eight; eight keys and the id are nine.
    let cut = |keys: usize| format!("{} {}ZZ and {}", "w".repeat(178), key_id(), key.repeat(keys));
    assert_eq!(ruled(&cut(7)), format!("{} [redacted]…", "w".repeat(178)));
    world().unmade(&cut(8));
    assert_eq!(ruled(&format!("x{}", " ".repeat(4095))), "x", "4096 bytes are cleaned");
    world().unmade(&format!("{words}{}", key.repeat(9)));
    world().unmade(&format!("x{}", " ".repeat(4096)));
}

// A settlement a build before this rule wrote holds a statement cleaned once, on either side. It still says which
// side stands.
#[test]
fn a_settlement_kept_after_one_cleaning_still_says_what_stands() {
    let (typed, once) = (format!("use {} here", key_header("\t")), format!("use {} here", key_header(" ")));
    for (clash, kept, option) in
        [((typed.as_str(), B), (once.as_str(), B, "a"), "1"), ((B, typed.as_str()), (B, once.as_str(), "b"), "2")]
    {
        let w = world();
        w.keep(&[clash]);
        let one = json!([{"a": kept.0, "b": kept.1, "stands": kept.2}]);
        fs::write(w.ruled(), json!({"day": DAY, "clashes": one}).to_string()).unwrap();
        #[cfg(unix)]
        super::chmod(&w.ruled(), 0o644);
        assert_eq!(w.tap("1", option)["ruling"], Value::Null, "no second ruling is made");
        assert!(w.rulings().is_empty());
    }
}

// A ruling that waits from a build before this rule holds a statement cleaned once: the one that stands, or the one
// it replaces. A tap finds it standing, and a session is told it once, in the words it has once cleaned to the end.
#[test]
fn a_ruling_kept_after_one_cleaning_is_found_by_a_tap_and_told_once_as_it_stands_still() {
    let (once, still) = (format!("use {} here", key_header(" ")), "use [redacted]");
    for (kept, option, told_as) in [((once.as_str(), B), "1", (still, B)), ((B, once.as_str()), "2", (B, still))] {
        let w = world();
        w.keep(&[(&format!("use {} here", key_header("\t")), B)]);
        let id = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
        let once = json!({"id": id.to_string(), "stands": kept.0, "over": kept.1});
        fs::create_dir_all(w.home().join("rulings")).unwrap();
        fs::write(w.home().join("rulings").join(format!("{id}.json")), once.to_string()).unwrap();
        assert_eq!(w.tap("1", option)["ruling"], Value::Null, "the ruling that waits is this clash's");
        assert_eq!(w.rulings().len(), 1);
        let told = w.told("s-1");
        let said = format!("- What stands: \"{}\". It replaces: \"{}\".", told_as.0, told_as.1);
        assert!(told.contains(&said), "{told}");
        assert!(!told.contains("PRIVATE") && w.told("s-1").is_empty(), "once, and no word of the key's line");
    }
}
