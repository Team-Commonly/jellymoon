//! What a tap does when there is nothing to choose, and when the day's lock is in the way: a lock left behind is
//! taken over, and a tap that is refused has changed nothing.

use std::fs;
use std::time::{Duration, Instant, SystemTime};

use serde_json::{Value, json};

use super::{A, B, DAY, World, tree, world};
use crate::support::{hub_key, key_header, model_key};

impl World {
    fn lock(&self) -> std::path::PathBuf {
        self.home().join("pages").join(format!("{DAY}.ruled.lock"))
    }

    /// Leaves a lock as a run that was killed would have, last written at `at`.
    fn leave_lock(&self, at: SystemTime) {
        fs::write(self.lock(), "").unwrap();
        fs::File::options().write(true).open(self.lock()).unwrap().set_modified(at).unwrap();
    }
}

// Two statements that are one once cleaned are no choice: either tap would be a ruling of a statement over itself,
// and the same tap again could never be told from the first.
#[test]
fn a_clash_whose_two_statements_are_the_same_once_cleaned_is_refused_and_writes_nothing() {
    let (first, second) = (format!("deploy with {}", hub_key()), format!("deploy with {}", model_key()));
    let cases: [(&str, &str, &str); 2] = [
        ("spacing and quote marks", "never  retry\n\"uploads\"", "never retry 'uploads'"),
        ("two different credentials", &first, &second),
    ];
    for (case, a, b) in cases {
        let w = world();
        w.keep(&[(A, B), (a, b)]);
        let before = tree(w.dir.path());
        for option in ["1", "2", "2"] {
            let ran = w.settle(DAY, "2", option);
            let line = "jellymoon: the two statements of clash 2 of 2026-03-10 are the same once cleaned, so there is \
                        nothing to choose\n";
            assert_eq!((ran.code, ran.stdout.as_str(), ran.stderr.as_str()), (1, "", line), "{case}, option {option}");
        }
        assert_eq!(tree(w.dir.path()), before, "{case}: no ruling, no settlement, no lock");
        assert_eq!(w.told("s-1"), "", "{case}: a session is told nothing");
        assert_eq!(w.tap("1", "2")["stands"], B, "{case}: the page's other clash can still be tapped");
    }
}

// The settlement holds the statements as the ruling carries them, so the next tap compares like with like.
// U+2065 is a character the page never draws, so the ruling does not carry it either.
#[test]
fn the_settlement_holds_the_cleaned_statements_and_never_a_credential() {
    let w = world();
    let raw = format!("deploy  with {}\nfr\u{2065}om \"now\" on", hub_key());
    w.keep(&[(&raw, B)]);
    let said = w.tap("1", "1");
    let clean = "deploy with [redacted] from 'now' on";
    assert_eq!(said["ruling"]["stands"], clean);
    let text = fs::read_to_string(w.ruled()).unwrap();
    assert!(!text.contains(&hub_key()), "the key is not kept");
    let kept: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(kept, json!({"day": DAY, "clashes": [{"a": clean, "b": B, "stands": "a"}]}));
    assert_eq!(w.tap("1", "1")["ruling"], Value::Null, "the same tap again makes no ruling");
    assert_eq!(w.rulings().len(), 1);
}

// A run that was killed leaves its lock. One older than 30 seconds, or dated ahead of now, is nobody's.
#[test]
fn a_lock_left_behind_long_ago_or_dated_ahead_is_taken_over() {
    let now = SystemTime::now();
    let day = Duration::from_secs(24 * 60 * 60);
    for (case, at) in [("a minute old", now - Duration::from_secs(60)), ("dated a day ahead", now + day)] {
        let w = world();
        w.keep(&[(A, B)]);
        w.leave_lock(at);
        let started = Instant::now();
        assert_eq!(w.tap("1", "2")["ruling"]["stands"], B, "{case}");
        assert!(started.elapsed() < Duration::from_secs(2), "{case}: with no wait for it");
        assert!(!w.lock().exists() && w.rulings().len() == 1, "{case}: and the lock is gone");
    }
}

// A statement is cleaned as it will be read, on one line: a private key's first line with a tab for one of its
// spaces is replaced before a session is told it, and the same tap again finds the ruling it made.
#[test]
fn a_statement_that_only_looks_like_a_key_once_on_one_line_is_replaced_and_ruled_once() {
    let w = world();
    w.keep(&[(&format!("use {} here", key_header("\t")), B)]);
    let said = w.tap("1", "1");
    assert_eq!(
        (said["stands"].as_str(), said["ruling"]["stands"].as_str()),
        (Some("use [redacted]"), Some("use [redacted]"))
    );
    assert_eq!(w.tap("1", "1")["ruling"], Value::Null, "the cleaned words are what is compared the next time");
    let told = w.told("s-1");
    assert!(w.rulings().len() == 1 && told.contains("What stands: \"use [redacted]\"") && !told.contains("PRIVATE"));
    let settlement: Value = serde_json::from_str(&fs::read_to_string(w.ruled()).unwrap()).unwrap();
    assert_eq!(settlement["clashes"], json!([{"a": "use [redacted]", "b": B, "stands": "a"}]), "as it is drawn");
}

// A lock that is fresh, and a folder at the lock's name, are waited for and then refused: 3 seconds by the clock.
#[test]
fn a_tap_behind_a_lock_waits_three_seconds_and_is_refused_having_changed_nothing() {
    type Held = fn(&World);
    let cases: [(&str, Held); 2] = [
        ("a fresh lock", |w| w.leave_lock(SystemTime::now())),
        ("a folder at the lock's name", |w| fs::create_dir(w.lock()).unwrap()),
    ];
    for (case, hold) in cases {
        let w = world();
        w.keep(&[(A, B)]);
        #[cfg(unix)]
        super::chmod(&w.home().join("pages"), 0o755);
        hold(&w);
        let (before, started) = (tree(w.dir.path()), Instant::now());
        let ran = w.settle(DAY, "1", "2");
        let waited = started.elapsed();
        let line = "jellymoon: jellymoon's own files: another tap on this day has not finished\n";
        assert_eq!((ran.code, ran.stdout.as_str(), ran.stderr.as_str()), (1, "", line), "{case}");
        assert!(waited >= Duration::from_secs(3) && waited < Duration::from_secs(8), "{case}: waited {waited:?}");
        assert_eq!(tree(w.dir.path()), before, "{case}: nothing was written, and the lock is not removed");
        #[cfg(unix)]
        assert_eq!(super::mode(&w.home().join("pages")), 0o755, "{case}: the folder's mode is as it was");
    }
}
