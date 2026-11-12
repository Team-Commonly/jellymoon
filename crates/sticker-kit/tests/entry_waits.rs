//! The entry between Enter and its answer, with no display: the words stay, and no key is taken. It is here and not
//! in `tests/entry.rs`, which is at the size a file may be.

use sticker_kit::entry::{EntryState, Made};

fn typed(text: &str) -> EntryState {
    let mut entry = EntryState::new(3, 80);
    entry.replace(None, text);
    entry
}

fn made(text: &str, option: Option<usize>) -> Option<Made> {
    Some(Made { text: text.into(), option })
}

#[test]
fn an_entry_that_waits_keeps_its_words_and_takes_no_key_until_it_is_answered() {
    let mut state = typed("water the basil");
    state.choose(1);
    assert!(!state.waiting());
    assert_eq!(state.enter(), made("water the basil", Some(1)));
    let waits = state.clone();
    assert_eq!((state.text(), state.caret(), state.waiting()), ("water the basil", 15, true));
    let keys: [fn(&mut EntryState); 12] = [
        |entry| entry.replace(None, "x"),
        |entry| entry.replace(Some(0..5), ""),
        |entry| entry.mark(None, "ka", None),
        EntryState::backspace,
        EntryState::delete,
        EntryState::left,
        EntryState::home,
        |entry| entry.set_caret(0),
        EntryState::next_option,
        EntryState::previous_option,
        |entry| entry.choose(2),
        |entry| assert_eq!(entry.enter(), None, "a second Enter enters nothing"),
    ];
    for (nth, key) in keys.into_iter().enumerate() {
        key(&mut state);
        assert_eq!(state, waits, "key {nth} was taken while the entry waited");
    }

    // Not kept: the words stay, and the entry takes keys again. So do the words of the next Enter that is refused.
    for _ in 0..2 {
        state.answer(false);
        assert_eq!(
            (state.text(), state.caret(), state.chosen(), state.waiting()),
            ("water the basil", 15, Some(1), false)
        );
        state.replace(None, "!");
        state.backspace();
        assert_eq!(state.enter(), made("water the basil", Some(1)));
    }
    // Kept: the entry is empty, with the same option chosen.
    state.answer(true);
    assert_eq!((state.text(), state.caret(), state.chosen(), state.waiting()), ("", 0, Some(1), false));
    // An answer to nothing changes nothing.
    state.replace(None, "tea");
    state.answer(true);
    assert_eq!((state.text(), state.waiting()), ("tea", false));
}
