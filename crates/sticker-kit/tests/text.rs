//! What the kit removes from text before it measures or draws it.

use sticker_kit::text::{clean, fit, wrap};
use sticker_kit::{FixedAdvance, Role};

#[test]
fn every_control_and_format_character_is_removed_but_the_two_joiners() {
    let gone = "\u{0}\u{7}\u{1b}\u{7f}\u{85}\u{9b}\u{ad}\u{600}\u{61c}\u{6dd}\u{70f}\u{890}\u{8e2}\u{180e}\u{200b}\u{200e}\u{200f}\
\u{2028}\u{2029}\u{202a}\u{202e}\u{2060}\u{2064}\u{2066}\u{2069}\u{206f}\u{feff}\u{fff9}\u{fffb}\u{110bd}\u{13430}\u{1bca0}\
\u{1d173}\u{e0000}\u{e0001}\u{e0020}\u{e007f}";
    let count = gone.chars().count();
    let text: String = gone.chars().map(|c| format!("x{c}")).collect();
    assert_eq!(clean(&text, 400), "x".repeat(count));
    let joined = "\u{645}\u{200c}\u{6cc} and 👩\u{200d}🚀";
    assert_eq!(clean(joined, 400), joined, "the joiners are kept: emoji and several scripts need them");
    assert_eq!(clean("one\u{2028}two\u{2029}three\nfour", 400), "onetwothree four", "no line end inside a text");
}

#[test]
fn text_left_over_ends_the_last_line_after_the_last_word_that_fits() {
    let text = "one two three four five six seven eight nine ten eleven";
    assert_eq!(lines(text, 2), ["one two three four", "five six seven…"], "cut at a word, with room for the mark");
    assert_eq!(lines(text, 3), ["one two three four", "five six seven eight", "nine ten eleven"], "all of it fits");
    assert_eq!(lines("one two three four five six seven eight", 2), ["one two three four", "five six seven eight"]);
    assert_eq!(lines("abcdefghijklmnopqrstuvwxyz and more", 1), ["abcdefghijklmnopqr…"], "no word edge: cut inside");
    assert_eq!(lines("", 3), Vec::<String>::new());
}

/// Lines of 100 wide: a Latin letter or a mark is 5 wide, anything from U+1100 up is 10.
fn lines(text: &str, max_lines: usize) -> Vec<String> {
    let wrapped = wrap(text, 100.0, max_lines, Role::Body, 10.0, &FixedAdvance(0.5));
    wrapped.iter().for_each(|(line, width)| assert!(*width <= 100.0, "{line} is {width} wide"));
    wrapped.into_iter().map(|(line, _)| line).collect()
}

#[test]
fn a_word_too_long_for_a_line_carries_on_to_the_next_while_lines_remain() {
    let word = "abcdefghijklmnopqrstuvwxyz".repeat(3);
    let three = ["abcdefghijklmnopqrst", "uvwxyzabcdefghijklmn", "opqrstuvwxyzabcdef…"];
    assert_eq!(lines(&word[..62], 3), three, "an unbroken word fills its lines and ends in the mark");
    assert_eq!(lines(&word[..50], 3), ["abcdefghijklmnopqrst", "uvwxyzabcdefghijklmn", "opqrstuvwx"], "all of it");
    let cjk = "一二三四五六七八九十".repeat(3);
    let ten = "一二三四五六七八九十";
    assert_eq!(lines(&cjk, 2), [ten, "一二三四五六七八九…"], "text with no spaces fills its lines too");
    assert_eq!(lines(&cjk, 3), [ten, ten, ten]);
    assert_eq!(lines("see abcdefghijklmnopqrstuvwxyz", 2), ["see abcdefghijklmnop", "qrstuvwxyz"], "after a word");
    assert_eq!(lines("see abcdefghijklmnopqrstuvwxyz now", 1), ["see abcdefghijklmn…"]);
    assert_eq!(lines("one two three four five", 1), ["one two three four…"], "a word that fits a line is never cut");
    assert_eq!(lines("anything", 0), Vec::<String>::new());
}

#[test]
fn a_break_never_falls_inside_a_character() {
    // Each unit below is one character to a reader, and is 10, 30, 20 and 20 wide.
    let units = [("e\u{301}", 15), ("👩\u{200d}🚀", 7), ("🇯🇵", 9), ("☀\u{fe0f}", 9)];
    for (unit, count) in units {
        let text = unit.repeat(count);
        let wrapped = lines(&text, 2);
        assert_eq!(wrapped.len(), 2, "{unit}");
        for line in &wrapped {
            let whole = line.trim_end_matches('…');
            assert!(!whole.is_empty() && whole.replace(unit, "").is_empty(), "{unit} was split: {line:?}");
        }
        let (cut, _) = fit(&text, 65.0, Role::Body, 10.0, &FixedAdvance(0.5));
        let whole = cut.strip_suffix('…').expect("it is cut");
        assert!(whole.replace(unit, "").is_empty(), "{unit} was split by fit: {cut:?}");
        let cleaned = clean(&text, unit.chars().count() * 2 + 1);
        assert_eq!(cleaned, format!("{}…", unit.repeat(2)), "{unit} was split by the length cut");
    }
    assert_eq!(clean("ab cd", 3), "ab…", "a cut leaves no space before the mark");
}
