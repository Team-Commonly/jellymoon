//! The door every kept text passes: a clash's statement and a made wish. One making is the reader's cleaning on one
//! line with each `"` made `'` ([`line`]). A text made once is not always the text made twice: a private key's first
//! line with a tab in it is one only once it is on one line, and a key straight after another is found only once the
//! first is the mark. So [`still`] makes a text again until a making changes nothing, and what is kept is unchanged
//! by one more making, which is all that a file read back is asked. A credential a line break splits is one once
//! the text is on one line with the break taken out, so the text is read that way too, first ([`mended`]). Only
//! these short texts come here; a session log's turn is cleaned once.

use crate::redact::{line, shown};

/// The most makings of one text that change it. One more follows, which must change nothing.
const CHANGES: usize = 8;
/// The most bytes of a text that is made at all: twenty times what is kept.
pub(crate) const LOOKED: usize = 4096;
/// The most characters a text may come to while it is made: far more than it can.
const ROOM: usize = 4 * LOOKED;
/// How many times its most characters a file's text is given room for while it is made ([`still_text`]).
const GROWN: usize = 20;
/// What is kept of a statement that cannot be made: the mark cleaning leaves where a credential was.
const MARK: &str = "[redacted]";

/// The characters that end a line. A making reads each as a space between two words, or takes it out.
const BREAKS: [char; 7] = ['\n', '\r', '\u{b}', '\u{c}', '\u{85}', '\u{2028}', '\u{2029}'];

/// `text` with every run of it that has a line break and no other space in it read twice: as a making reads
/// it, the break a space, and with the break taken out. A run the two readings clean alike is left as it is.
/// One they clean differently holds a credential's shape in one reading that the other does not find (a key
/// with a break in it; a key id that the next line's capitals would hide), and the whole run is the mark: no
/// part of either reading is kept. A text with no line break in it is returned as it is.
fn mended(text: &str) -> String {
    if !text.contains(BREAKS) {
        return text.to_string();
    }
    let near = |c: char| c.is_whitespace() && !BREAKS.contains(&c);
    let read = |run: &str| {
        let joined: String = run.chars().filter(|c| !BREAKS.contains(c)).collect();
        let alike = joined.len() == run.len() || line(&joined, ROOM) == line(run, ROOM).replace(' ', "");
        if alike { run.to_string() } else { MARK.to_string() }
    };
    text.split(near).map(read).collect::<Vec<_>>().join(" ")
}

/// Why a text is not kept in its own words.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Unkept {
    /// Over its length, or too long to be made.
    Long,
    /// Changed by a ninth making.
    Changing,
}

/// `text` made until a making changes nothing, of at most `most` characters. A text is cut only once it stands
/// still: cut sooner, half of a key not yet found could be what is left at the cut. `cut` says whether it is cut
/// then (a statement), and made again as cut, or refused as long (a made wish, which is never cut). The cut is no
/// making and is not counted: a text has its [`CHANGES`] whatever its length.
pub(crate) fn still(text: &str, most: usize, cut: bool) -> Result<String, Unkept> {
    if text.len() > LOOKED {
        return Err(Unkept::Long);
    }
    let make = |words: &str, room: usize| line(words, room).replace('"', "'");
    let (mut words, mut room, mut changes) = (mended(text), ROOM, 0);
    loop {
        let made = make(&words, room);
        let size = made.chars().count();
        if room == ROOM && size >= ROOM {
            return Err(Unkept::Long);
        }
        if made != words {
            if changes == CHANGES {
                return Err(Unkept::Changing);
            }
            (words, changes) = (made, changes + 1);
        } else if size <= most {
            return Ok(words);
        } else if cut {
            (words, room) = (make(&words, most), most);
        } else {
            return Err(Unkept::Long);
        }
    }
}

/// A file's text made until a making changes nothing, with its lines and its quotes kept: a making here is the
/// reader's cleaning of a turn (`redact::shown`). `most` is the most bytes the text has, since reading it stopped
/// there, and the most characters that are kept of it. A making can make a text longer (a token gains its
/// ellipsis, a password becomes the mark), so one can come to more than `most` characters: it is cut there, and
/// the answer says so. One a ninth making changes is refused.
pub(crate) fn still_text(text: &str, most: usize) -> Result<(String, bool), Unkept> {
    let (mut words, mut changes, mut shortened) = (text.to_string(), 0, false);
    loop {
        // Made first with room for whatever a making adds, which is under twenty characters for each one read.
        let mut made = shown(&words, most.saturating_mul(GROWN));
        if made.chars().count() > most {
            (made, shortened) = (shown(&words, most), true);
        }
        if made == words {
            return Ok((words, shortened));
        }
        if changes == CHANGES {
            return Err(Unkept::Changing);
        }
        (words, changes) = (made, changes + 1);
    }
}

/// A statement as it is drawn and handed over. One that cannot be made is the mark and nothing else: it cannot be
/// refused here, since a page is drawn and a ruling file read whatever they hold. No ruling is made of one.
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

    // Each making finds one more of these keys, since a key is found only when the one before it is the mark:
    // eight keys are eight makings that change the text and a ninth that changes nothing. Words before them that
    // are cut away afterwards take none of the eight.
    #[test]
    fn eight_makings_may_change_a_text_and_a_ninth_refuses_it_whatever_its_length() {
        let (key, words) = (format!("AI{}{}", "za", "kLmNo".repeat(7)), "word ".repeat(45));
        for keys in [7, 8] {
            let text = key.repeat(keys);
            assert_eq!(still(&text, 200, false), Ok(MARK.repeat(keys)));
            assert_eq!(statement(&format!("{words}{text}"), 200), format!("{}…", &words[..199]));
        }
        let nine = key.repeat(9);
        assert_eq!((still(&nine, 200, false), statement(&nine, 200)), (Err(Unkept::Changing), MARK.to_string()));
        assert_eq!(statement(&format!("{words}{nine}"), 200), MARK);
        assert_eq!(statement(&key, 200), MARK, "the mark is what cleaning leaves");
    }

    #[test]
    fn a_text_of_4096_bytes_is_made_and_one_of_4097_is_not() {
        let fits = format!("x{}", " ".repeat(LOOKED - 1));
        assert_eq!((still(&fits, 200, false), still(&fits, 200, true)), (Ok("x".into()), Ok("x".into())));
        let over = format!("{fits} ");
        assert_eq!((still(&over, 200, false), still(&over, 200, true)), (Err(Unkept::Long), Err(Unkept::Long)));
        assert_eq!(statement(&over, 200), MARK);
    }

    // Each side of the break is no credential by itself; with the break out, the two are one.
    #[test]
    fn a_credential_split_by_a_line_break_is_the_mark_and_words_on_two_lines_stay_two_words() {
        let key = format!("AI{}{}", "za", "kLmNo".repeat(7));
        let id = format!("AK{}{}", "IA", "Q7".repeat(8));
        for split in ["\n", "\r\n", "\r", "\u{b}", "\u{c}", "\u{85}", "\u{2028}", "\u{2029}", "\n\n"] {
            for at in [1, 4, 17, 38] {
                let text = format!("use {}{split}{} here", &key[..at], &key[at..]);
                assert_eq!(still(&text, 200, false), Ok(format!("use {MARK} here")), "{split:?} at {at}");
            }
            // A break that a making takes out leaves one word, as before. The rest is of one read as a space.
            if line(&format!("a{split}b"), ROOM) != "a b" {
                assert_eq!(still(&format!("fix the upload{split}retry"), 200, false), Ok("fix the uploadretry".into()));
                continue;
            }
            // A key id that the next line would hide when joined to it is not shown either.
            assert_eq!(still(&format!("use {id}{split}ZZ9 here"), 200, false), Ok(format!("use {MARK} here")));
            // What is no credential either way is kept as it was, as two words.
            assert_eq!(still(&format!("fix the upload{split}retry"), 200, false), Ok("fix the upload retry".into()));
            // A key whole on its own line, with no space beside it, takes the words that touch its breaks with
            // it: the reading with the breaks out finds a longer key, and nothing of such a run is kept.
            assert_eq!(
                still(&format!("say use{split}{key}{split}here now"), 200, false),
                Ok(format!("say {MARK} now"))
            );
        }
        // A space of another kind beside the break keeps the two sides apart, as before: the door does not join them.
        let apart = format!("use {}\n {} here", &key[..17], &key[17..]);
        assert_eq!(still(&apart, 200, false), Ok(format!("use {} {} here", &key[..17], &key[17..])));
        assert_eq!(mended("no break at all"), "no break at all");
    }

    // The cut can end a key id that had more capitals after it, so the cut text is made again.
    #[test]
    fn a_text_that_was_cut_is_made_again() {
        let id = format!("AK{}{}", "IA", "Q7".repeat(8));
        let text = format!("{}{id}ZZ and {}", "w".repeat(178) + " ", "more ".repeat(9));
        assert_eq!(statement(&text, 200), format!("{} {MARK}…", "w".repeat(178)));
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
