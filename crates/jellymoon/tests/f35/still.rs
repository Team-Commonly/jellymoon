//! A clash's statement is cleaned until cleaning changes nothing (`still.rs` in core): the words a ruling carries are
//! the words the next tap compares, whether `jellymoon rule` or a tap made it.

use std::fs;

use serde_json::{Value, json};

use super::{B, DAY, world};
use crate::support::{key_header, restless, shows};

#[test]
fn a_statement_one_cleaning_is_not_done_with_is_ruled_as_it_stands_still_by_a_tap_and_by_rule() {
    for (typed, still, planted) in restless() {
        // Twelve keys in a row are still changing after 8 cleanings: the statement is the mark and nothing else.
        let still = if still.is_empty() { "[redacted]".to_string() } else { still };
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
        let ran = w.fed(&["rule", "--stands", &typed, "--over", B], "");
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

// A settlement a build before this rule wrote holds a statement cleaned once. It still says which side stands.
#[test]
fn a_settlement_kept_after_one_cleaning_still_says_what_stands() {
    let w = world();
    w.keep(&[(&format!("use {} here", key_header("\t")), B)]);
    let once = json!([{"a": format!("use {} here", key_header(" ")), "b": B, "stands": "a"}]);
    fs::write(w.ruled(), json!({"day": DAY, "clashes": once}).to_string()).unwrap();
    #[cfg(unix)]
    super::chmod(&w.ruled(), 0o644);
    assert_eq!(w.tap("1", "1")["ruling"], Value::Null, "no second ruling is made");
    assert!(w.rulings().is_empty());
}
