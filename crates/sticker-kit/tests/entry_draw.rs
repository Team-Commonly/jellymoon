//! The entry in the view, headless: no window is shown. Keys and clicks go through GPUI's own dispatch, as on a real
//! window. The test platform has no input method, so a composition is started the way a platform starts one: by
//! calling the page's `EntityInputHandler`. What a real input method does with the keys before that is not tested.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::cell::RefCell;
use std::rc::Rc;

use gpui::{
    ClipboardItem, Entity, EntityInputHandler, Modifiers, TestAppContext, TestDispatcher, VisualTestContext, point, px,
    size,
};
use sticker_kit::draw::headless::text_system;
use sticker_kit::draw::{EntryMade, Page, Tap, load_fonts};
use sticker_kit::{Entry, PieceKind, Placed, Scene, Theme};

type Seen<T> = Rc<RefCell<Vec<T>>>;

fn entry() -> Entry {
    let options = ["garden", "kitchen", "letters"].map(String::from).to_vec();
    Entry {
        id: "wish".into(),
        hint: "wish for something".into(),
        options,
        most: 12,
        action: String::new(),
        go: String::new(),
        loud: false,
    }
}

fn context() -> TestAppContext {
    TestAppContext::build_with_text_system(TestDispatcher::new(0), None, text_system())
}

fn open(cx: &mut TestAppContext) -> (Entity<Page>, &mut VisualTestContext, Seen<EntryMade>, Seen<Tap>) {
    cx.update(|cx| load_fonts(cx)).expect("bundled fonts load");
    let (made, taps) = (Seen::<EntryMade>::default(), Seen::<Tap>::default());
    let (entered, tapped) = (made.clone(), taps.clone());
    let (page, cx) = cx.add_window_view(move |_, _| {
        Page::new(Scene::sample(), Theme::sticker())
            .reduce_motion(true)
            .with_entry(entry())
            .on_entry(move |made, _, _| entered.borrow_mut().push(made.clone()))
            .on_tap(move |tap, _, _| tapped.borrow_mut().push(tap.clone()))
    });
    cx.simulate_resize(size(px(1440.0), px(900.0)));
    cx.run_until_parked();
    (page, cx, made, taps)
}

fn piece(page: &Entity<Page>, cx: &mut VisualTestContext, kind: PieceKind, index: usize) -> Placed {
    let shown = |page: &Page, piece: &Placed| Placed { rect: page.shown_at(piece), ..piece.clone() };
    let find = |p: &&Placed| p.piece == kind && p.index == index;
    let found = page.read_with(cx, |page, _| page.placed().iter().find(find).map(|piece| shown(page, piece)));
    found.expect("the piece is placed")
}

fn click(cx: &mut VisualTestContext, at: (f32, f32)) {
    cx.simulate_click(point(px(at.0), px(at.1)), Modifiers::default());
    cx.run_until_parked();
}

fn keys(cx: &mut VisualTestContext, keystrokes: &str) {
    cx.simulate_keystrokes(keystrokes);
    cx.run_until_parked();
}

/// The text, the caret, the chosen option and whether the entry has the keys.
fn state(page: &Entity<Page>, cx: &mut VisualTestContext) -> (String, usize, Option<usize>, bool) {
    cx.update(|window, cx| {
        let page = page.read(cx);
        let entry = page.entry();
        (entry.text().to_string(), entry.caret(), entry.chosen(), page.entry_focused(window))
    })
}

fn wish(text: &str, option: &str) -> EntryMade {
    EntryMade { id: "wish".into(), text: text.into(), option: Some(option.into()) }
}

/// How many of the entry's own pieces are placed: the field and its chips.
fn pieces(page: &Entity<Page>, cx: &mut VisualTestContext) -> usize {
    let placed = page.read_with(cx, |page, _| page.placed().to_vec());
    placed.iter().filter(|p| matches!(p.piece, PieceKind::Entry | PieceKind::Chip)).count()
}

fn take_away(page: &Entity<Page>, cx: &mut VisualTestContext) {
    page.update(cx, |page, cx| page.set_entry(None, cx));
    cx.run_until_parked();
}

/// The handler's answer to what Enter gave.
fn answer(page: &Entity<Page>, cx: &mut VisualTestContext, kept: bool) {
    cx.update(|window, cx| page.update(cx, |page, cx| page.answer_entry(kept, window, cx)));
    cx.run_until_parked();
}

#[cfg(debug_assertions)] // it asks `cx.debug_bounds`: see `tests/draw.rs`
#[test]
fn the_entry_has_the_keys_from_the_first_frame_and_enter_gives_what_was_typed() {
    let mut cx = context();
    let (page, cx, made, _) = open(&mut cx);
    assert!(cx.debug_bounds("Entry:wish:0:0").is_some(), "an empty entry shows its hint");
    keys(cx, "t");
    assert_eq!(state(&page, cx), ("t".into(), 1, Some(0), true), "a key is the entry's with no click");

    let field = piece(&page, cx, PieceKind::Entry, 0);
    click(cx, field.rect.center());
    keys(cx, "e a space a t space 4");
    assert_eq!(state(&page, cx), ("tea at 4".into(), 8, Some(0), true));
    assert!(cx.debug_bounds("Entry:wish:0:0").is_none(), "the hint gives way to what is typed");
    keys(cx, "backspace backspace backspace backspace backspace backspace backspace backspace space space");
    assert_eq!(state(&page, cx).0, "  ");
    assert!(cx.debug_bounds("Entry:wish:0:0").is_some(), "spaces are no words: the hint stays");
    keys(cx, "backspace backspace t e a space a t space 4");

    keys(cx, "tab");
    assert_eq!(state(&page, cx).2, Some(1), "Tab chooses the next option");
    keys(cx, "enter");
    assert_eq!(*made.borrow(), [wish("tea at 4", "kitchen")]);
    assert_eq!(state(&page, cx), (String::new(), 0, Some(1), true), "the words are kept aside until they are kept");
    answer(&page, cx, true);
    assert_eq!(state(&page, cx), (String::new(), 0, Some(1), true), "the entry is empty and keeps its option");

    keys(cx, "enter space enter");
    assert_eq!(made.borrow().len(), 1, "nothing typed is nothing entered");
    keys(cx, "escape");
    assert!(state(&page, cx).3, "Escape leaves the keys with the entry, where nothing else holds them");
    keys(cx, "x");
    assert_eq!(state(&page, cx).0, " x", "and a key after it is still text");
}

#[test]
fn the_editing_keys_and_a_click_move_the_caret_and_the_cap_holds() {
    let mut cx = context();
    let (page, cx, made, _) = open(&mut cx);
    let field = piece(&page, cx, PieceKind::Entry, 0);
    click(cx, field.rect.center());
    keys(cx, "a b c d left left backspace");
    assert_eq!(state(&page, cx), ("acd".into(), 1, Some(0), true));
    keys(cx, "delete end x home y");
    assert_eq!(state(&page, cx), ("yadx".into(), 1, Some(0), true));
    keys(cx, "secondary-right z secondary-left shift-tab");
    assert_eq!(state(&page, cx), ("yadxz".into(), 0, Some(2), true), "Shift-Tab chooses the option before");

    // A click far to the right in the field is the end of the line; one at its first letter is the start.
    click(cx, (field.rect.right() - 4.0, field.rect.center().1));
    assert_eq!(state(&page, cx).1, 5);
    click(cx, (field.rect.x + 15.0, field.rect.center().1));
    assert_eq!(state(&page, cx).1, 0);

    cx.write_to_clipboard(ClipboardItem::new_string("one\ntwo three four five".into()));
    keys(cx, "secondary-v");
    assert_eq!(state(&page, cx).0, "one twoyadxz", "pasted text is one line, cut to the entry's twelve characters");
    keys(cx, "q enter");
    assert_eq!(*made.borrow(), [wish("one twoyadxz", "letters")]);
}

#[test]
fn enter_during_a_composition_enters_nothing() {
    let mut cx = context();
    let (page, cx, made, _) = open(&mut cx);
    let field = piece(&page, cx, PieceKind::Entry, 0);
    click(cx, field.rect.center());
    keys(cx, "a");
    let compose = |cx: &mut VisualTestContext, text: &str| {
        cx.update(|window, cx| {
            page.update(cx, |page, cx| page.replace_and_mark_text_in_range(None, text, None, window, cx))
        });
        cx.run_until_parked();
    };
    compose(cx, "ni");
    compose(cx, "nihao");
    let marked = cx.update(|window, cx| page.update(cx, |page, cx| page.marked_text_range(window, cx)));
    assert_eq!(marked, Some(1..6), "the composition is marked, in UTF-16");
    keys(cx, "enter backspace left tab");
    assert_eq!(*made.borrow(), [], "Enter while composing is the input method's");
    assert_eq!(state(&page, cx), ("anihao".into(), 6, Some(1), true), "only the option moved");

    cx.update(|window, cx| page.update(cx, |page, cx| page.replace_text_in_range(None, "你好", window, cx)));
    let asked = cx.update(|window, cx| {
        page.update(cx, |page, cx| {
            let mut adjusted = None;
            let text = page.text_for_range(1..2, &mut adjusted, window, cx);
            (
                text,
                adjusted,
                page.marked_text_range(window, cx),
                page.selected_text_range(false, window, cx).map(|s| s.range),
            )
        })
    });
    assert_eq!(asked, (Some("你".into()), Some(1..2), None, Some(3..3)));
    keys(cx, "enter");
    assert_eq!(*made.borrow(), [wish("a你好", "kitchen")]);
}

#[test]
fn a_chip_chooses_its_option_and_the_page_under_the_entry_still_takes_taps() {
    let mut cx = context();
    let (page, cx, made, taps) = open(&mut cx);
    let chip = piece(&page, cx, PieceKind::Chip, 2);
    click(cx, chip.rect.center());
    assert_eq!(state(&page, cx), (String::new(), 0, Some(2), true), "a chip chooses and gives the entry the keys");
    keys(cx, "o k enter");
    assert_eq!(*made.borrow(), [wish("ok", "letters")]);
    answer(&page, cx, true);

    let option = piece(&page, cx, PieceKind::Option, 1);
    click(cx, option.rect.center());
    let words = "“the hill has the best view, let's eat up there”".to_string();
    assert_eq!(*taps.borrow(), [Tap::Option { note_id: "picnic-place".into(), index: 1, words }]);
    keys(cx, "x");
    assert_eq!(state(&page, cx), ("x".into(), 1, Some(2), true), "a press elsewhere gives the entry the keys");
    keys(cx, "backspace");

    let field = piece(&page, cx, PieceKind::Entry, 0);
    click(cx, field.rect.center());
    keys(cx, "l o n g");
    let fewer = Entry { options: vec!["garden".into()], most: 2, ..entry() };
    page.update(cx, |page, cx| page.set_entry(Some(fewer), cx));
    cx.run_until_parked();
    assert_eq!(state(&page, cx), ("lo".into(), 2, Some(0), true), "what was typed is held to the new entry");
    assert_eq!(pieces(&page, cx), 2);
    keys(cx, "backspace backspace");
    take_away(&page, cx);
    assert_eq!(
        (pieces(&page, cx), state(&page, cx)),
        (0, (String::new(), 0, None, false)),
        "a page with no entry takes no keys"
    );
}

// Typed ahead: a person presses Enter and types the next words at once, before the answer has come.
#[test]
fn an_entry_that_is_owed_an_answer_takes_every_key_and_only_its_enter_gives_nothing() {
    let mut cx = context();
    let (page, cx, made, _) = open(&mut cx);
    let accepts = |cx: &mut VisualTestContext| {
        cx.update(|window, cx| page.update(cx, |page, cx| page.accepts_text_input(window, cx)))
    };
    let waiting = |cx: &mut VisualTestContext| page.read_with(cx, |page, _| page.entry().waiting());
    assert!(accepts(cx), "a page with an entry takes text input");
    let field = piece(&page, cx, PieceKind::Entry, 0);
    click(cx, field.rect.center());
    keys(cx, "t e a tab enter");
    assert_eq!((state(&page, cx), waiting(cx)), ((String::new(), 0, Some(1), true), true), "the words are kept aside");
    assert!(accepts(cx), "an entry that is owed an answer takes text input, so a composition can begin");
    // Before the answer: letters, the editing keys, Escape, a paste, a second Enter, a composition, a chip, a click.
    cx.write_to_clipboard(ClipboardItem::new_string("k".into()));
    keys(cx, "m i x l k backspace backspace home delete escape end secondary-v enter");
    cx.update(|window, cx| {
        page.update(cx, |page, cx| page.replace_and_mark_text_in_range(None, "ni", None, window, cx));
        let marked = page.update(cx, |page, cx| page.marked_text_range(window, cx));
        assert_eq!(marked, Some(3..5), "the ranges are of the typed text, never of the words kept aside");
        page.update(cx, |page, cx| page.replace_text_in_range(None, "你", window, cx));
    });
    let chip = piece(&page, cx, PieceKind::Chip, 2);
    click(cx, chip.rect.center());
    click(cx, (field.rect.x + 15.0, field.rect.center().1));
    let since = ("ixk你".to_string(), 0, Some(2), true);
    assert_eq!((state(&page, cx), waiting(cx)), (since.clone(), true), "a key was dropped while an answer was owed");
    assert_eq!(*made.borrow(), [wish("tea", "kitchen")], "a second Enter gave words while an answer was owed");

    // Kept: what was typed since is whole, with its caret, and is what the next Enter gives.
    answer(&page, cx, true);
    assert_eq!((state(&page, cx), waiting(cx), accepts(cx)), (since, false, true));
    keys(cx, "enter t w o");
    // The options move while the answer is owed: the option is kept by its name. Then the words are not kept:
    // they come back in front of what was typed since.
    let moved = Entry { options: ["letters", "kitchen", "garden"].map(String::from).to_vec(), ..entry() };
    page.update(cx, |page, cx| page.set_entry(Some(moved), cx));
    cx.run_until_parked();
    assert_eq!((state(&page, cx).2, waiting(cx)), (Some(0), true), "letters is still the chosen option");
    answer(&page, cx, false);
    assert_eq!((state(&page, cx), waiting(cx)), (("ixk你 two".into(), 10, Some(0), true), false));
    keys(cx, "enter");
    assert_eq!(made.borrow().last(), Some(&wish("ixk你 two", "letters")), "the option is given by its name");

    answer(&page, cx, true);
    take_away(&page, cx);
    assert!(!accepts(cx), "a page with no entry takes no text input");
}

#[test]
fn an_entry_taken_away_stays_with_its_words_and_goes_once_it_is_empty_and_answered() {
    let mut cx = context();
    let (page, cx, made, _) = open(&mut cx);
    let waiting = |cx: &mut VisualTestContext| page.read_with(cx, |page, _| page.entry().waiting());
    take_away(&page, cx);
    assert_eq!(pieces(&page, cx), 0, "an empty entry that does not wait goes at once");

    // Words an answer is owed for: the entry stays, chips and all, and a refusal gives them back to be changed.
    page.update(cx, |page, cx| page.set_entry(Some(entry()), cx));
    cx.run_until_parked();
    let field = piece(&page, cx, PieceKind::Entry, 0);
    click(cx, field.rect.center());
    keys(cx, "d r a f t tab enter");
    take_away(&page, cx);
    let held = ("draft".to_string(), 5, Some(1), true);
    assert_eq!((pieces(&page, cx), state(&page, cx), waiting(cx)), (4, (String::new(), 0, Some(1), true), true));
    answer(&page, cx, false);
    assert_eq!((pieces(&page, cx), state(&page, cx), waiting(cx)), (4, held, false), "the refusal found no words");
    keys(cx, "backspace s enter");
    assert_eq!(made.borrow().last(), Some(&wish("drafs", "kitchen")), "the words can be changed and entered");
    // Kept: it empties, and only then goes.
    answer(&page, cx, true);
    assert_eq!((pieces(&page, cx), state(&page, cx)), (0, (String::new(), 0, None, false)));

    // Words typed and not entered stay too, until the person takes them out.
    page.update(cx, |page, cx| page.set_entry(Some(entry()), cx));
    cx.run_until_parked();
    click(cx, field.rect.center());
    keys(cx, "t e a");
    take_away(&page, cx);
    assert_eq!((pieces(&page, cx), state(&page, cx).0.as_str()), (4, "tea"));
    keys(cx, "backspace backspace");
    assert_eq!((pieces(&page, cx), state(&page, cx).0.as_str()), (4, "t"));
    keys(cx, "backspace");
    assert_eq!(pieces(&page, cx), 0, "the entry that was taken away goes with its last word");

    // Spaces alone are no words: Enter gives nothing for them, so an entry that holds only them goes as an empty one.
    for (typed, left) in [("space space", 0), ("space t e a space", 4)] {
        page.update(cx, |page, cx| page.set_entry(Some(entry()), cx));
        cx.run_until_parked();
        click(cx, field.rect.center());
        keys(cx, typed);
        take_away(&page, cx);
        assert_eq!(pieces(&page, cx), left, "after {typed:?}");
    }
    keys(cx, "backspace backspace backspace");
    assert_eq!((pieces(&page, cx), state(&page, cx).0.as_str()), (4, " t"));
    keys(cx, "backspace");
    assert_eq!(pieces(&page, cx), 0, "it goes with its last word, whatever spaces are left");
}

#[test]
fn an_entry_that_is_owed_an_answer_cannot_go_and_come_back_empty_and_keeps_what_is_typed_meanwhile() {
    let mut cx = context();
    let (page, cx, made, _) = open(&mut cx);
    let field = piece(&page, cx, PieceKind::Entry, 0);
    click(cx, field.rect.center());
    keys(cx, "f i r s t enter");
    take_away(&page, cx);
    page.update(cx, |page, cx| page.set_entry(Some(entry()), cx));
    cx.run_until_parked();
    keys(cx, "s e c o n d enter");
    let waiting = page.read_with(cx, |page, _| page.entry().waiting());
    assert_eq!((state(&page, cx).0.as_str(), waiting), ("second", true), "what was typed since was dropped");
    assert_eq!(*made.borrow(), [wish("first", "garden")], "one Enter, so one answer to come");
    answer(&page, cx, false);
    assert_eq!((pieces(&page, cx), state(&page, cx).0.as_str()), (4, "first second"), "the answer found its words");
}

#[test]
fn a_page_with_no_handler_answers_itself_so_its_entry_never_stays_waiting() {
    let mut cx = context();
    cx.update(|cx| load_fonts(cx)).expect("bundled fonts load");
    let (page, cx) =
        cx.add_window_view(|_, _| Page::new(Scene::sample(), Theme::sticker()).reduce_motion(true).with_entry(entry()));
    cx.simulate_resize(size(px(1440.0), px(900.0)));
    cx.run_until_parked();
    let field = piece(&page, cx, PieceKind::Entry, 0);
    click(cx, field.rect.center());
    keys(cx, "t e a enter");
    let waiting = page.read_with(cx, |page, _| page.entry().waiting());
    assert_eq!((state(&page, cx), waiting), ((String::new(), 0, Some(0), true), false));
    keys(cx, "o k");
    assert_eq!(state(&page, cx).0, "ok", "the entry takes keys again");
}
