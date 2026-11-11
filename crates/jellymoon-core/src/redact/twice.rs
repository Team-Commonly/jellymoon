//! Cleaning a statement twice is cleaning it once (`ruling::words_of`, which a made wish and a clash's statement are
//! kept through), what it leaves holds no credential shape, and no eight characters of a secret planted whole are
//! left to read. The inputs are built here from pieces: every shape, cut at every place, joined again with every kind
//! of white space and of unseen character, alone and beside another. `planted.rs` is the cases two reviews found.

use super::*;
use crate::ruling::words_of;

/// Every white-space character, and a run of them.
const SPACES: [&str; 28] = [
    " ", "\t", "\n", "\r", "\u{b}", "\u{c}", "\u{85}", "\u{a0}", "\u{1680}", "\u{2000}", "\u{2001}", "\u{2002}",
    "\u{2003}", "\u{2004}", "\u{2005}", "\u{2006}", "\u{2007}", "\u{2008}", "\u{2009}", "\u{200a}", "\u{2028}",
    "\u{2029}", "\u{202f}", "\u{205f}", "\u{3000}", " \t ", "\r\n", "\n\n",
];

/// What cleaning takes out, and the joiners it looks through: a shape cut by one of these is still that shape.
const UNSEENS: [&str; 15] = [
    "",
    "\u{1b}[31m",
    "\u{1b}]0;t\u{7}",
    "\u{9b}2J",
    "\0",
    "\u{1b}",
    "\u{7f}",
    "\u{ad}",
    "\u{200b}",
    "\u{200c}",
    "\u{200d}",
    "\u{202e}",
    "\u{2065}",
    "\u{feff}",
    "\u{e0041}",
];

/// What ends a word, a key or a path, and so stands between two shapes: the mark itself is one.
const ENDS: [&str; 4] = [REPLACED, "\"", "…", ","];

/// What can be either, by where it falls.
const OTHERS: [&str; 2] = ["/", "@"];

/// A credential shape and the part of it nobody may read, each joined here so no scanner reads one in this file.
pub(super) fn shapes() -> Vec<(String, String)> {
    let dashes = "-".repeat(5);
    let key = ["PRIVATE", "KEY"].join(" ");
    let named = |prefix: [&str; 2], body: String| (format!("{}{}{body}", prefix[0], prefix[1]), body);
    vec![
        (format!("{dashes}BEGIN RSA {key}{dashes}"), key.clone()),
        (format!("{dashes}BEGIN {key}{dashes} MIIEab {dashes}END {key}{dashes}"), "MIIEab".to_string()),
        named(["sk", "-proj-"], "aB3x".repeat(5)),
        named(["gh", "p_"], "cD4y".repeat(5)),
        named(["AK", "IA"], "Q7".repeat(8)),
        named(["AK", "IA"], "Z9".repeat(9)),
        named(["xo", "xb-"], "1234-".repeat(3)),
        named(["AI", "za"], format!("{}eF5", "eF5z".repeat(8))),
        named(["gl", "pat-"], "gH6w".repeat(5)),
        named(["hf", "_"], "iJ7v".repeat(8)),
        named(["sk_", "live_"], "kL8u".repeat(4)),
        named(["github", "_pat_"], "mN9t".repeat(5)),
        (format!("ey{}{}.{}.sig", "J", "hbGci".repeat(2), "eyJzd".repeat(2)), "hbGcihbGci".to_string()),
        (format!("db://app:{}@host/x", "pass".repeat(2)), "pass".repeat(2)),
        named(["jm", "u_"], "Ab1".repeat(3)),
        named(["", ""], "oP1+/s9Z".repeat(5)),
        (format!("/srv/{}/logs", "qR2x9Zq_".repeat(5)), "qR2x9Zq_".repeat(5)),
    ]
}

/// The statements a shape gives: whole, and cut in two at every character with `gap` put in between.
fn cut_with<'a>(shape: &'a str, gap: &'a str) -> impl Iterator<Item = String> + 'a {
    let seams = shape.char_indices().map(|(at, _)| at).chain([shape.len()]);
    seams.map(move |at| format!("{}{gap}{}", &shape[..at], &shape[at..]))
}

/// Cleans `typed` and checks what must hold of every text: twice is once, and no shape is left, looked for as
/// cleaning looks, through the joiners.
pub(super) fn check(typed: &str, most: usize) -> String {
    let once = words_of(typed, most);
    assert_eq!(words_of(&once, most), once, "cleaned twice is not cleaned once: {typed:?} at {most}");
    let read = once.replace(JOINERS, "");
    for (name, shape) in [("PEM", &PEM), ("JWT", &JWT), ("KEY", &KEY)] {
        assert!(!shape.is_match(&read), "a {name} shape is left in {once:?}, from {typed:?} at {most}");
    }
    let passwords = URL_PASSWORD.find_iter(&read).all(|found| found.as_str().ends_with(&format!(":{REPLACED}@")));
    assert!(passwords && TOKEN.replace_all(&read, "${1}…") == read, "a password or a token in {once:?}: {typed:?}");
    once
}

/// No eight characters in a row of `secret` (all of it when it is shorter) can be read in `once`.
pub(super) fn hidden(once: &str, secret: &str, typed: &str) {
    let (read, secret): (String, Vec<char>) = (once.replace(JOINERS, ""), secret.chars().collect());
    for run in secret.windows(secret.len().min(8)) {
        let run: String = run.iter().collect();
        assert!(!read.contains(&run), "{run:?} of a secret can be read in {once:?}, from {typed:?}");
    }
}

#[test]
fn a_statement_cleaned_twice_is_cleaned_once_and_holds_no_credential_shape_and_no_planted_secret() {
    let (shapes, mut checked) = (shapes(), 0);
    for (shape, secret) in &shapes {
        for gap in SPACES.iter().chain(&UNSEENS).chain(&ENDS).chain(&OTHERS) {
            for typed in cut_with(shape, gap) {
                for most in [200, 4096] {
                    let once = check(&format!("say {typed} then"), most);
                    // Cut by what is not there once cleaned, the shape was whole.
                    if UNSEENS.contains(gap) {
                        hidden(&once, secret, &typed);
                    }
                }
                checked += 1;
            }
        }
    }
    // Two shapes side by side, in each order, with every joint between them. The first is whole, at the start. The
    // second is whole where the joint ends a word or a key; after nothing or a letter it may be the first one's tail.
    for ((first, secret), (second, other)) in
        shapes.iter().flat_map(|first| shapes.iter().map(move |second| (first, second)))
    {
        for gap in SPACES.iter().chain(&UNSEENS).chain(&ENDS).chain(&OTHERS) {
            let typed = format!("{first}{gap}{second}");
            let once = check(&typed, 4096);
            hidden(&once, secret, &typed);
            // A carriage return and the other control characters among the spaces are taken out, so they part nothing.
            if ENDS.contains(gap) || string(gap).contains(char::is_whitespace) {
                hidden(&once, other, &typed);
            }
            checked += 1;
        }
    }
    for (shape, _) in &shapes {
        for most in 1..=shape.chars().count() + 8 {
            check(&format!("a {shape} b {shape}"), most);
            checked += 1;
        }
    }
    assert!(checked > 40_000, "only {checked} statements were built");
}
