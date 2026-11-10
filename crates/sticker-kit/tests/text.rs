//! What the kit removes from text before it measures or draws it.

use sticker_kit::text::clean;

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
