//! What the kit removes from text before it measures or draws it.

use sticker_kit::text::{clean, fit, fit_middle, wrap};
use sticker_kit::{FixedAdvance, Measure, Role};

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

#[test]
fn the_end_of_a_name_cut_in_the_middle_never_begins_with_a_mark_or_a_joiner() {
    // Names in Thai, Devanagari, Hebrew with its points, Arabic with its vowels, Hangul in conjoining jamo, and
    // one of joined emoji; and every character of theirs that is drawn onto the one before it.
    let names = [
        "\u{0E04}\u{0E23}\u{0E31}\u{0E27}\u{0E0B}\u{0E34}\u{0E48}\u{0E07}\u{0E17}\u{0E35}\u{0E48}\u{0E22}\u{0E32}\u{0E27}\u{0E21}\u{0E32}\u{0E01}\u{0E19}\u{0E49}\u{0E33}",
        "\u{0930}\u{0938}\u{094B}\u{0908} \u{0915}\u{0940} \u{0915}\u{093F}\u{0924}\u{093E}\u{092C}\u{0947}\u{0902} \u{0915}\u{094D}\u{0937}",
        "\u{05E9}\u{05C1}\u{05B8}\u{05DC}\u{05D5}\u{05B9}\u{05DD} \u{05E2}\u{05D5}\u{05B9}\u{05DC}\u{05B8}\u{05DD} \u{05D1}\u{05BC}\u{05B0}\u{05E8}",
        "\u{0645}\u{064E}\u{0631}\u{0652}\u{062D}\u{064E}\u{0628}\u{064B}\u{0627} \u{0628}\u{0650}\u{0627}\u{0644}\u{0639}\u{064E}\u{0627}\u{0644}\u{064E}\u{0645}\u{0670}",
        "\u{1112}\u{1161}\u{11AB}\u{1100}\u{1173}\u{11AF}\u{1109}\u{1161}\u{1105}\u{1161}\u{11B7}",
        "a\u{0301}👩\u{200D}🚀b👨\u{200D}👩\u{200D}👧c\u{200C}d☀\u{FE0F}e\u{0301}",
    ];
    let leans = |c: char| {
        matches!(
            c as u32,
            0x0E31 | 0x0E33..=0x0E35 | 0x0E48 | 0x0E49 | 0x0902 | 0x093E..=0x0940 | 0x0947 | 0x094B | 0x094D
                | 0x05B0 | 0x05B8 | 0x05B9 | 0x05BC | 0x05C1 | 0x064B | 0x064E | 0x0650 | 0x0652 | 0x0670
                | 0x1161 | 0x1173 | 0x11AB | 0x11AF | 0x11B7 | 0x0301 | 0x200C | 0x200D | 0xFE0F
        )
    };
    for name in names {
        let (mut cuts, full) = (0, FixedAdvance(0.5).width(name, Role::Legend, 10.0));
        for tenth in 0..(full * 10.0) as usize {
            let (cut, width) = fit_middle(name, tenth as f32 / 10.0, Role::Legend, 10.0, &FixedAdvance(0.5));
            let Some((_, end)) = cut.split_once('…') else {
                assert!(cut.is_empty(), "{cut:?} is neither cut nor nothing");
                continue;
            };
            assert!(width <= tenth as f32 / 10.0 && name.ends_with(end), "{cut:?} of {name:?}");
            assert!(!end.starts_with(leans), "{cut:?} begins its end on a mark, of {name:?}");
            let before = name[..name.len() - end.len()].chars().next_back();
            assert_ne!(before, Some('\u{200D}'), "{cut:?} begins its end on what a joiner joined, of {name:?}");
            cuts += usize::from(!end.is_empty());
        }
        assert!(cuts > 5, "{name:?} was cut with an end {cuts} times");
    }
}

#[test]
fn a_cut_in_the_middle_keeps_the_longer_end_and_no_space_at_the_ellipsis() {
    let cut = |name: &str, width: f32| fit_middle(name, width, Role::Legend, 10.0, &FixedAdvance(0.5)).0;
    // Each letter is 5 wide and the ellipsis 10: 35 holds five letters and the ellipsis, the end the longer.
    assert_eq!((cut("abcdefghij", 35.0), cut("abcdefghij", 30.0)), ("ab…hij".to_string(), "ab…ij".to_string()));
    assert_eq!((cut("abcdefghij", 50.0), cut("abcdefghij", 4.0)), ("abcdefghij".to_string(), String::new()));
    for width in 5..65 {
        let cut = cut("a b c d e f g h", width as f32);
        assert!(!cut.contains(" …") && !cut.contains("… "), "{cut:?} has a space at its ellipsis");
    }
}
