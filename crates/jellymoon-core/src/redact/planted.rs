//! The texts two reviews found, each with what may not be read in it once cleaned. No build of an earlier commit
//! can run inside a test, so this table is what holds the line: everything that commit hid in a URL, a path or a run
//! is listed by its kind, and a kind that is shown again fails here. Every secret is built from pieces.

use super::twice::{check, hidden, shapes};
use super::*;
use crate::ruling::words_of;

/// A secret with slashes in it: 45 characters, no piece between two slashes near 40.
fn slashed(unit: &str) -> String {
    format!("{unit}/").repeat(9)
}

fn aws(body: &str) -> String {
    format!("AK{}{}", "IA", body.repeat(8))
}

/// A URL with a password, and a secret in its path, its query, its fragment, and each two of them.
fn urls() -> Vec<(String, Vec<String>)> {
    let (password, one, two) = ("hunter".repeat(2), slashed("oP1s"), slashed("qR2t"));
    let places = [
        (format!("/{one}"), 1),
        (format!("/x?sig={one}"), 1),
        (format!("/x#{one}"), 1),
        (format!("/{one}?sig={two}"), 2),
        (format!("/{one}#{two}"), 2),
        (format!("/x?sig={one}#{two}"), 2),
        (format!("/x?a=1&sig={one}"), 1),
    ];
    let mut urls = Vec::new();
    for (scheme, (place, secrets)) in ["https", "db"].iter().flat_map(|scheme| places.iter().map(move |p| (scheme, p)))
    {
        let mut secret = vec![password.clone(), one.clone(), two.clone()];
        secret.truncate(1 + secrets);
        urls.push((format!("open {scheme}://app:{password}@host.example{place} now"), secret));
    }
    urls
}

/// An AWS key id with a tail, and what stands after it in the same run: a second id nothing sets apart.
fn aws_runs() -> Vec<(String, Vec<String>)> {
    let (first, second) = (aws("Q7"), aws("Z9"));
    let between = ["/wxyz", "/", "+wxyz", "=wxyz", "/wx/yz"];
    let run = |between: &str| (format!("use {first}tail{between}{second}"), vec!["Q7".repeat(8), "Z9".repeat(8)]);
    between.iter().map(|between| run(between)).collect()
}

/// A shape with a joiner inside it, at every place: cleaning looks through the two joiners it keeps.
fn joined() -> Vec<(String, Vec<String>)> {
    let cut = |(shape, secret): (String, String), joiner: char| {
        let seams = shape.char_indices().map(|(at, _)| at).skip(1).step_by(3);
        let texts = seams.map(|at| format!("say {}{joiner}{} then", &shape[..at], &shape[at..]));
        texts.map(|text| (text, vec![secret.clone()])).collect::<Vec<_>>()
    };
    shapes().into_iter().flat_map(|shape| JOINERS.map(|joiner| cut(shape.clone(), joiner))).flatten().collect()
}

#[test]
fn what_the_reviews_found_shown_is_hidden_and_stays_as_it_is_when_cleaned_again() {
    let all = shapes();
    let named = |prefix: &str| all.iter().find(|(shape, _)| shape.starts_with(prefix)).cloned().unwrap_or_default();
    let (google, gitlab, header) = (named("AIza"), named("glpat"), named("-----BEGIN RSA"));
    // Two keys with nothing between them: the second has no word boundary until the first is a mark.
    let mut cases = vec![(format!("{}{}", google.0, gitlab.0), vec![google.1.clone(), gitlab.1.clone()])];
    cases.push((format!("{}{}{}", google.0, google.0, google.0), vec![google.1.clone()]));
    cases.push((format!("{}{}", google.0, named("eyJ").0), vec![google.1, "eyJzdeyJzd".to_string()]));
    // A private key's first line with a tab for one of its spaces.
    cases.push((format!("paste {} here", header.0.replacen(' ', "\t", 1)), vec![header.1]));
    // A path whose replaced segment is followed by a secret with slashes.
    cases.push((format!("/srv/{}/{}", "qR2x9Zq_".repeat(5), slashed("uV3w")), vec![slashed("uV3w")]));
    cases.extend(urls().into_iter().chain(aws_runs()).chain(joined()));
    assert!(cases.len() > 200, "{} cases", cases.len());
    for (typed, secrets) in cases {
        for (once, most) in [(check(&typed, 4096), 4096), (shown(&typed, 4096), 4096)] {
            assert_eq!(shown(&once, most), shown(&shown(&once, most), most), "{typed:?}");
            for secret in &secrets {
                hidden(&once, secret, &typed);
            }
        }
    }
}

// Words with no shape in them are the person's, and come back as they were typed: the joiners of an emoji and of
// a script written with them too.
#[test]
fn a_sentence_with_no_shape_in_it_is_returned_as_it_was_typed() {
    let family = "\u{1f468}\u{200d}\u{1f469}\u{200d}\u{1f467}";
    let sentences = [
        "fix the login page before friday".to_string(),
        format!("the {family} photo goes on the first page"),
        format!("{family}{family} and a flag \u{1f3f3}\u{fe0f}\u{200d}\u{1f308}"),
        "\u{645}\u{6cc}\u{200c}\u{62e}\u{648}\u{627}\u{647}\u{645} a note in two scripts".to_string(),
        "see https://example.com/docs/a-long-page/about-nothing and /usr/local/bin/tool".to_string(),
        "commit 3f2a9c1d8e7b6a5f4e3d2c1b0a9f8e7d6c5b4a39 is the one, not the 40 after it".to_string(),
        "words that start like a key are words: skate, ghost, AKIAK, eyJ, hf_x and sk-short".to_string(),
        "a [redacted] someone typed, and a/path/with/[redacted]/in/it".to_string(),
    ];
    for typed in sentences {
        assert_eq!(words_of(&typed, 200), typed);
        assert_eq!(shown(&typed, 700), typed);
    }
}
