//! Cleaning a statement twice is cleaning it once (`ruling::words_of`, which a made wish and a clash's statement are
//! kept through), and what it leaves holds no credential shape. The inputs are built here from pieces: every shape,
//! cut at every place, joined again with every kind of white space and of unseen character, alone and beside another.

use super::*;
use crate::ruling::words_of;

/// Every white-space character, and a run of them.
const SPACES: [&str; 28] = [
    " ", "\t", "\n", "\r", "\u{b}", "\u{c}", "\u{85}", "\u{a0}", "\u{1680}", "\u{2000}", "\u{2001}", "\u{2002}",
    "\u{2003}", "\u{2004}", "\u{2005}", "\u{2006}", "\u{2007}", "\u{2008}", "\u{2009}", "\u{200a}", "\u{2028}",
    "\u{2029}", "\u{202f}", "\u{205f}", "\u{3000}", " \t ", "\r\n", "\n\n",
];

/// What cleaning takes out or keeps unseen, the joiners it keeps, the quote it turns, and what ends a word or a path.
const UNSEENS: [&str; 20] = [
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
    "\"",
    "…",
    ",",
    "/",
    "@",
];

/// The credential shapes, each joined here so no scanner reads one in this file.
fn shapes() -> Vec<String> {
    let dashes = "-".repeat(5);
    let key = ["PRIVATE", "KEY"].join(" ");
    vec![
        format!("{dashes}BEGIN RSA {key}{dashes}"),
        format!("{dashes}BEGIN {key}{dashes} MIIEab {dashes}END {key}{dashes}"),
        format!("sk{}{}", "-proj-", "aB3x".repeat(5)),
        format!("gh{}{}", "p_", "aB3x".repeat(5)),
        format!("AK{}{}", "IA", "Q7".repeat(8)),
        format!("AK{}{}", "IA", "Q7".repeat(9)),
        format!("xo{}{}", "xb-", "1234-".repeat(3)),
        format!("ey{}{}.{}.sig", "J", "hbGci".repeat(2), "eyJzd".repeat(2)),
        format!("db://app:{}@host/x", "pass".repeat(2)),
        format!("jm{}{}", "u_", "Ab1".repeat(3)),
        "aB3+/x9Z".repeat(5),
        format!("/srv/{}/logs", "aB3x9Zq_".repeat(5)),
    ]
}

/// The statements a shape gives: whole, and cut in two at every character with `gap` put in between.
fn cut_with<'a>(shape: &'a str, gap: &'a str) -> impl Iterator<Item = String> + 'a {
    let seams = shape.char_indices().map(|(at, _)| at).chain([shape.len()]);
    seams.map(move |at| format!("{}{gap}{}", &shape[..at], &shape[at..]))
}

fn check(typed: &str, most: usize) {
    let once = words_of(typed, most);
    assert_eq!(words_of(&once, most), once, "cleaned twice is not cleaned once: {typed:?} at {most}");
    for (name, shape) in [("PEM", &PEM), ("JWT", &JWT), ("KEY", &KEY)] {
        assert!(!shape.is_match(&once), "a {name} shape is left in {once:?}, from {typed:?} at {most}");
    }
    let passwords = URL_PASSWORD.find_iter(&once).all(|found| found.as_str().ends_with(&format!(":{REPLACED}@")));
    assert!(passwords && TOKEN.replace_all(&once, "${1}…") == once, "a password or a token in {once:?}: {typed:?}");
}

#[test]
fn a_statement_cleaned_twice_is_cleaned_once_and_holds_no_credential_shape() {
    let (shapes, mut checked) = (shapes(), 0);
    for shape in &shapes {
        for gap in SPACES.iter().chain(&UNSEENS) {
            for typed in cut_with(shape, gap) {
                for most in [200, 4096] {
                    check(&format!("say {typed} then"), most);
                }
                checked += 1;
            }
        }
    }
    // Two shapes side by side, with every joint between them, and a statement cut short at every length.
    for (first, second) in shapes.iter().flat_map(|first| shapes.iter().map(move |second| (first, second))) {
        for gap in SPACES.iter().chain(&UNSEENS) {
            check(&format!("{first}{gap}{second}"), 4096);
            checked += 1;
        }
    }
    for shape in &shapes {
        for most in 1..=shape.chars().count() + 8 {
            check(&format!("a {shape} b {shape}"), most);
            checked += 1;
        }
    }
    assert!(checked > 20_000, "only {checked} statements were built");
}
