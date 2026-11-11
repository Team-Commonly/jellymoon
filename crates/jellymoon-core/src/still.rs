//! The door every kept text passes: a clash's statement and a made wish. One making is the reader's cleaning on one
//! line with each `"` made `'` ([`line`]). A text made once is not always the text made twice: a private key's first
//! line with a tab in it is one only once it is on one line, and a key straight after another is found only once the
//! first is the mark. So [`still`] makes a text again until a making changes nothing, and what is kept is unchanged
//! by one more making, which is all that a file read back is asked. Only these short texts come here; a session
//! log's turn is cleaned once.

use crate::redact::line;

/// The most makings of one text.
const MAKINGS: usize = 8;
/// The most bytes of a text that is made at all: twenty times what is kept.
const LOOKED: usize = 4096;
/// The most characters a text may come to while it is made: far more than it can.
const ROOM: usize = 4 * LOOKED;
/// What is kept of a statement that cannot be made: the mark cleaning leaves where a credential was.
const MARK: &str = "[redacted]";

/// Why a text is not kept in its own words.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Unkept {
    /// Over its length, or too long to be made.
    Long,
    /// Still changing after the last making.
    Changing,
}

/// `text` made until a making changes nothing, of at most `most` characters. A text is cut only once it stands
/// still: cut sooner, half of a key not yet found could be what is left at the cut. `cut` says whether it is cut
/// then (a statement), and made again as cut, or refused as long (a made wish, which is never cut).
pub(crate) fn still(text: &str, most: usize, cut: bool) -> Result<String, Unkept> {
    if text.len() > LOOKED {
        return Err(Unkept::Long);
    }
    let (mut words, mut room) = (text.to_string(), ROOM);
    for _ in 0..MAKINGS {
        let made = line(&words, room).replace('"', "'");
        let size = made.chars().count();
        if room == ROOM && size >= ROOM {
            return Err(Unkept::Long);
        }
        if made != words {
            words = made;
        } else if size <= most {
            return Ok(words);
        } else if cut {
            room = most;
        } else {
            return Err(Unkept::Long);
        }
    }
    Err(Unkept::Changing)
}

/// A statement as it is kept and handed over. One that cannot be made is the mark and nothing else: it cannot be
/// refused, since a page is drawn and a ruling file read whatever they hold.
pub(crate) fn statement(text: &str, most: usize) -> String {
    still(text, most, true).unwrap_or_else(|_| MARK.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every shape cleaning knows, built here from pieces. The keys after the first are of letters only, so each
    /// is found by its own shape and not as a long mixed run.
    fn shapes() -> Vec<String> {
        let (dashes, fill) = ("-".repeat(5), "kLmNoPqRsT");
        let header = |gap: &str| format!("{dashes}BEGIN{gap}RSA PRIVATE KEY{dashes}");
        vec![
            format!("jm{}{}", "u_", "Q7".repeat(8)),
            format!("cm{}{}", "_", "Q7".repeat(8)),
            format!("sk{}{}", "-ant-", "Zx9Yw8Vu7T".repeat(3)),
            format!("AI{}{}", "za", "a1B2c".repeat(7)),
            format!("AI{}{}", "za", "kLmNo".repeat(7)),
            format!("gh{}{}", "p_", fill.repeat(2)),
            format!("sk{}{}", "_live_", &fill.repeat(2)[..16]),
            format!("xo{}{}", "xb-", fill),
            format!("AK{}{}", "IA", "Q7".repeat(8)),
            format!("ey{}{fill}.{fill}.{fill}", "J"),
            header(" "),
            header("\t"),
            format!("{}abc {dashes}END RSA PRIVATE KEY{dashes}", header("\t")),
            format!("https://me:{}@example.test/a", "pass"),
            "Ab3/dE+9".repeat(6),
            format!("/work/{}/notes", "Ab3dE9".repeat(8)),
        ]
    }

    #[test]
    fn a_text_built_of_any_two_or_three_shapes_is_unchanged_by_one_more_making() {
        let (shapes, joins) = (shapes(), ["", " ", "\t", "/", ".", MARK]);
        let mut texts = Vec::new();
        for (a, b, join) in pairs(&shapes, &joins) {
            texts.push(format!("{a}{join}{b}"));
            texts.extend(shapes.iter().map(|c| format!("{a}{join}{b}{join}{c}")));
        }
        assert_eq!(texts.len(), 16 * 16 * 6 * 17);
        let mut kept = 0;
        for text in &texts {
            for cut in [true, false] {
                let made = still(text, 200, cut);
                assert_ne!(made, Err(Unkept::Changing), "{text:?}");
                let Ok(made) = made else { continue };
                assert_eq!(line(&made, 200 + usize::from(!cut)).replace('"', "'"), made, "{text:?}");
                kept += 1;
            }
        }
        assert!(kept > texts.len(), "most are kept: {kept}");
    }

    fn pairs<'a>(shapes: &'a [String], joins: &'a [&str]) -> impl Iterator<Item = (&'a str, &'a str, &'a str)> {
        let two = shapes.iter().flat_map(move |a| shapes.iter().map(move |b| (a.as_str(), b.as_str())));
        two.flat_map(move |(a, b)| joins.iter().map(move |join| (a, b, *join)))
    }

    // Each making finds one more of these keys, since a key is found only when the one before it is the mark.
    #[test]
    fn a_text_still_changing_after_eight_makings_is_not_kept_and_a_statement_of_it_is_the_mark() {
        let key = format!("AI{}{}", "za", "kLmNo".repeat(7));
        for (keys, wish, said) in [(3, Ok(MARK.repeat(3)), MARK.repeat(3)), (12, Err(Unkept::Changing), MARK.into())] {
            assert_eq!((still(&key.repeat(keys), 200, false), statement(&key.repeat(keys), 200)), (wish, said));
        }
        let long = "x".repeat(LOOKED + 1);
        assert_eq!((still(&long, 200, true), statement(&long, 200)), (Err(Unkept::Long), MARK.to_string()));
        assert_eq!(statement(&key, 200), MARK, "the mark is what cleaning leaves");
    }

    // The second key is found by the second making. Cut at the first, 25 characters of it would be kept.
    #[test]
    fn a_statement_is_cut_only_once_it_stands_still() {
        let (words, key) = ("word ".repeat(33), format!("AI{}{}", "za", "kLmNo".repeat(7)));
        assert_eq!(statement(&format!("{words}{key}{key}"), 200), format!("{words}{MARK}{MARK}"));
        let cut = statement(&format!("{words}{key}{key} and {}", "more ".repeat(9)), 200);
        assert_eq!(cut, format!("{words}{MARK}{MARK} and more more…"));
        assert_eq!(statement(&cut, 200), cut);
    }
}
