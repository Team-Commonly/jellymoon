//! The entry between Enter and its answer, with no display: the words are kept aside, every key is taken, and no
//! answer costs a person anything they typed. It is here and not in `tests/entry.rs`, which is at the size a file
//! may be.

use sticker_kit::Entry;
use sticker_kit::entry::{EntryState, Made};

fn typed(text: &str) -> EntryState {
    let mut entry = EntryState::new(3, 80);
    entry.replace(None, text);
    entry
}

fn made(text: &str, option: Option<usize>) -> Option<Made> {
    Some(Made { text: text.into(), option })
}

/// The text, the caret, the chosen option and whether an answer is owed.
fn held(state: &EntryState) -> (&str, usize, Option<usize>, bool) {
    (state.text(), state.caret(), state.chosen(), state.waiting())
}

#[test]
fn every_key_is_taken_while_an_answer_is_owed_and_only_enter_gives_nothing() {
    let mut state = typed("water the basil");
    state.choose(1);
    assert!(!state.waiting());
    assert_eq!(state.enter(), made("water the basil", Some(1)));
    assert_eq!(held(&state), ("", 0, Some(1), true), "the words are kept aside, and the text is empty");

    // Typed before the answer: text, the caret, the editing keys and another option.
    state.replace(None, "fed the cat");
    state.home();
    state.right();
    state.replace(None, "e");
    assert_eq!(held(&state), ("feed the cat", 2, Some(1), true), "a key was dropped while an answer was owed");
    state.end();
    state.replace(None, "s!");
    state.backspace();
    state.left();
    state.delete();
    state.set_caret(4);
    state.next_option();
    state.next_option();
    state.previous_option();
    state.choose(0);
    assert_eq!(held(&state), ("feed the cat", 4, Some(0), true));
    assert_eq!(state.enter(), None, "a second Enter gave words while an answer was owed");
    assert_eq!(held(&state), ("feed the cat", 4, Some(0), true), "a second Enter lost what was typed since");

    // Kept: what was typed since is whole, with its caret, and is the next words Enter gives.
    state.answer(true);
    assert_eq!(held(&state), ("feed the cat", 4, Some(0), false));
    assert_eq!(state.enter(), made("feed the cat", Some(0)));
    state.answer(true);
    assert_eq!(held(&state), ("", 0, Some(0), false));
    // An answer to nothing changes nothing.
    state.replace(None, "tea");
    state.answer(true);
    state.answer(false);
    assert_eq!(held(&state), ("tea", 3, Some(0), false));
}

#[test]
fn words_not_kept_come_back_as_typed_and_in_front_of_what_was_typed_since() {
    // Nothing typed since: the words as they were typed, spaces and all, the caret at their end. Twice over.
    let mut state = typed(" water the basil ");
    state.choose(1);
    for _ in 0..2 {
        assert_eq!(state.enter(), made("water the basil", Some(1)));
        state.answer(false);
        assert_eq!(held(&state), (" water the basil ", 17, Some(1), false));
    }
    // Something typed since: both, in that order, one space between, the caret at the end. The option chosen
    // since stays.
    assert_eq!(state.enter(), made("water the basil", Some(1)));
    state.replace(None, "then tea");
    state.home();
    state.choose(2);
    state.answer(false);
    assert_eq!(held(&state), (" water the basil then tea", 25, Some(2), false));
    assert_eq!(state.enter(), made("water the basil then tea", Some(2)));
}

#[test]
fn a_composition_begun_while_an_answer_is_owed_is_marked_and_confirmed_as_any() {
    let mut state = typed("tea");
    assert_eq!(state.enter(), made("tea", Some(0)));
    state.replace(None, "买");
    state.mark(None, "niu", None);
    assert_eq!((state.text(), state.marked(), state.caret()), ("买niu", Some(3..6), 6));
    assert_eq!(state.enter(), None, "Enter while composing is the input method's");
    // Not kept while it composes: the words come back in front, and the composition is the same characters.
    state.answer(false);
    assert_eq!((state.text(), state.marked(), state.caret()), ("tea 买niu", Some(7..10), 10));
    state.replace(None, "牛");
    assert_eq!((state.text(), state.marked(), state.caret()), ("tea 买牛", None, 10));

    // Begun in the empty text, with the caret the input method asks for, confirmed, and then the words are kept.
    assert_eq!(state.enter(), made("tea 买牛", Some(0)));
    state.mark(None, "ka", Some(1..1));
    assert_eq!((state.text(), state.marked(), state.caret()), ("ka", Some(0..2), 1));
    state.replace(None, "か");
    state.answer(true);
    assert_eq!((held(&state), state.marked()), (("か", 3, Some(0), false), None));
}

#[test]
fn the_cap_holds_when_the_words_not_kept_and_the_words_typed_since_are_joined() {
    let joined = |first: &str, since: &str| {
        let mut state = EntryState::new(0, 12);
        state.replace(None, first);
        assert!(state.enter().is_some(), "{first:?}");
        state.replace(None, since);
        state.answer(false);
        (state.text().to_string(), state.caret())
    };
    // What was typed since is whole; the words that come back are as many of their first as there is room for.
    assert_eq!(joined("water basil", "x"), ("water basi x".to_string(), 12));
    assert_eq!(joined("one two four", "abc"), ("one two abc".to_string(), 11), "one space between, never two");
    assert_eq!(joined("tea", "abcdefghij"), ("t abcdefghij".to_string(), 12));
    assert_eq!(joined("tea", "abcdefghijk"), ("abcdefghijk".to_string(), 11), "no room for a space and a letter");
    assert_eq!(joined("tea", "abcdefghijkl"), ("abcdefghijkl".to_string(), 12));
    // A character a reader sees as one comes back whole or not at all.
    assert_eq!(joined("e\u{301}e\u{301}", "0123456789"), ("0123456789".to_string(), 10));
    assert_eq!(joined("e\u{301}e\u{301}", "01234567"), ("e\u{301} 01234567".to_string(), 12));

    // The cap got smaller while the answer was owed: the words that come back are held to it.
    let mut state = typed("water the basil");
    assert!(state.enter().is_some());
    state.fit(&Entry { most: 5, ..Entry::default() });
    state.answer(false);
    assert_eq!(held(&state), ("water", 5, None, false));
}

#[test]
fn one_enter_while_an_answer_is_owed_is_remembered_and_given_once_when_the_words_are_kept() {
    // No words typed since when Enter was pressed: nothing is remembered, whatever is typed after it.
    let mut state = typed("water the basil");
    assert_eq!(state.enter(), made("water the basil", Some(0)));
    assert_eq!(state.enter(), None);
    state.replace(None, "tea");
    assert_eq!(state.answer_and_enter(true), None, "an Enter with no words typed since was remembered");
    // Words, Enter twice, more words and another option: one Enter, given with the words as they stand at the
    // answer, and it is an Enter like any: its words are owed an answer.
    assert_eq!(state.enter(), made("tea", Some(0)));
    state.replace(None, "feed");
    assert_eq!((state.enter(), state.enter()), (None, None), "an Enter was given before the answer");
    assert_eq!(held(&state), ("feed", 4, Some(0), true));
    state.replace(None, " the cat ");
    state.choose(2);
    assert_eq!(state.answer_and_enter(true), made("feed the cat", Some(2)));
    assert_eq!((held(&state), state.owed()), (("", 0, Some(2), true), Some("feed the cat ")));
    // Given once: the next answer finds no Enter, and an Enter with no answer owed is as it ever was.
    state.replace(None, "again");
    assert_eq!(state.answer_and_enter(true), None, "two Enters were two, or one was given twice");
    assert_eq!(held(&state), ("again", 5, Some(2), false));
    assert_eq!(state.enter(), made("again", Some(2)));
}

#[test]
fn a_remembered_enter_is_not_given_for_words_not_kept_nor_once_forgotten_nor_for_no_words() {
    // Not kept: the owed words and the later ones both stand, and the Enter is gone for good.
    let mut state = typed("water");
    assert_eq!(state.enter(), made("water", Some(0)));
    state.replace(None, "tea");
    assert_eq!(state.enter(), None);
    assert_eq!(state.answer_and_enter(false), None, "an Enter was given over words that were not kept");
    assert_eq!(held(&state), ("water tea", 9, Some(0), false));
    assert_eq!(state.enter(), made("water tea", Some(0)));
    state.replace(None, "x");
    assert_eq!(state.answer_and_enter(true), None, "the Enter of an answer before this one was given");
    // Forgotten by whoever answers, and the plain answer (a field's, a closed window's): the words stay.
    for forget in [true, false] {
        assert_eq!((state.enter(), state.text()), (made("x", Some(0)), ""));
        state.replace(None, "x");
        assert_eq!(state.enter(), None);
        if forget {
            state.forget_enter();
        } else {
            state.answer(true);
        }
        assert_eq!(state.answer_and_enter(true), None, "forgotten: {forget}");
        assert_eq!(held(&state), ("x", 1, Some(0), false));
    }
    // The words went before the answer, or a composition has the keys: the Enter given then gives nothing.
    for composing in [false, true] {
        assert_eq!(state.enter(), made("x", Some(0)));
        state.replace(None, "y");
        assert_eq!(state.enter(), None);
        state.backspace();
        if composing {
            state.mark(None, "ni", None);
        } else {
            state.replace(None, " ");
        }
        assert_eq!(state.answer_and_enter(true), None, "composing: {composing}");
        assert!(!state.waiting());
        state.replace(None, "x");
    }
}
