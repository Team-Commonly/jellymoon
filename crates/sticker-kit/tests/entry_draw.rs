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
    Entry { id: "wish".into(), hint: "wish for something".into(), options, most: 12 }
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
    let found =
        page.read_with(cx, |page, _| page.placed().iter().find(|p| p.piece == kind && p.index == index).cloned());
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

#[test]
fn a_click_gives_the_entry_the_keys_and_enter_gives_what_was_typed() {
    let mut cx = context();
    let (page, cx, made, _) = open(&mut cx);
    assert!(cx.debug_bounds("Entry:wish:0:0").is_some(), "an empty entry shows its hint");
    keys(cx, "t");
    assert_eq!(state(&page, cx), (String::new(), 0, Some(0), false), "a key is not the entry's until it is clicked");

    let field = piece(&page, cx, PieceKind::Entry, 0);
    click(cx, field.rect.center());
    keys(cx, "t e a space a t space 4");
    assert_eq!(state(&page, cx), ("tea at 4".into(), 8, Some(0), true));
    assert!(cx.debug_bounds("Entry:wish:0:0").is_none(), "the hint gives way to what is typed");

    keys(cx, "tab");
    assert_eq!(state(&page, cx).2, Some(1), "Tab chooses the next option");
    keys(cx, "enter");
    assert_eq!(*made.borrow(), [wish("tea at 4", "kitchen")]);
    assert_eq!(state(&page, cx), ("tea at 4".into(), 8, Some(1), true), "the words stay until they are kept");
    answer(&page, cx, true);
    assert_eq!(state(&page, cx), (String::new(), 0, Some(1), true), "the entry is empty and keeps its option");

    keys(cx, "enter space enter");
    assert_eq!(made.borrow().len(), 1, "nothing typed is nothing entered");
    keys(cx, "escape");
    assert!(!state(&page, cx).3, "Escape gives the keys back");
    keys(cx, "x");
    assert_eq!(state(&page, cx).0, " ", "and a key is no longer text");
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
    assert_eq!(*taps.borrow(), [Tap::Option { note_id: "picnic-place".into(), index: 1 }]);
    keys(cx, "x");
    assert_eq!(state(&page, cx), (String::new(), 0, Some(2), false), "a press elsewhere gives the keys back");

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

#[test]
fn an_entry_waits_with_its_words_for_its_answer_and_takes_no_key_or_composition_meanwhile() {
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
    let waits = ("tea".to_string(), 3, Some(1), true);
    assert_eq!((state(&page, cx), waiting(cx)), (waits.clone(), true), "the words stay while the entry waits");
    assert!(!accepts(cx), "an entry that waits takes no text input, so no composition begins");
    keys(cx, "x backspace left home delete tab shift-tab enter secondary-v");
    cx.update(|window, cx| {
        page.update(cx, |page, cx| page.replace_and_mark_text_in_range(None, "ni", None, window, cx));
        page.update(cx, |page, cx| page.replace_text_in_range(None, "你", window, cx));
    });
    let chip = piece(&page, cx, PieceKind::Chip, 2);
    click(cx, chip.rect.center());
    click(cx, (field.rect.x + 15.0, field.rect.center().1));
    assert_eq!((state(&page, cx), waiting(cx)), (waits.clone(), true), "a key was taken while the entry waited");
    assert_eq!(*made.borrow(), [wish("tea", "kitchen")], "a second Enter entered the words again");

    // Not kept: the words are still there, and the keys are the entry's again. Twice over.
    for nth in 1..3 {
        answer(&page, cx, false);
        assert_eq!((state(&page, cx), waiting(cx), accepts(cx)), (waits.clone(), false, true));
        keys(cx, "s backspace enter");
        assert_eq!(made.borrow().len(), nth + 1);
        assert_eq!((state(&page, cx), waiting(cx)), (waits.clone(), true), "refusal {nth} lost the words");
    }
    // The options move while it waits: the option is kept by its name. Then the words are kept.
    let moved = Entry { options: ["kitchen", "letters", "garden"].map(String::from).to_vec(), ..entry() };
    page.update(cx, |page, cx| page.set_entry(Some(moved), cx));
    cx.run_until_parked();
    assert_eq!((state(&page, cx).2, waiting(cx)), (Some(0), true), "kitchen is still the chosen option");
    answer(&page, cx, true);
    assert_eq!((state(&page, cx), waiting(cx)), ((String::new(), 0, Some(0), true), false));
    keys(cx, "o k enter");
    assert_eq!(made.borrow().last(), Some(&wish("ok", "kitchen")), "the option is given by its name");

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

    // Words that wait: the entry stays as it was, chips and all, and a refusal leaves them to be changed.
    page.update(cx, |page, cx| page.set_entry(Some(entry()), cx));
    cx.run_until_parked();
    let field = piece(&page, cx, PieceKind::Entry, 0);
    click(cx, field.rect.center());
    keys(cx, "d r a f t tab enter");
    take_away(&page, cx);
    let held = ("draft".to_string(), 5, Some(1), true);
    assert_eq!((pieces(&page, cx), state(&page, cx), waiting(cx)), (4, held.clone(), true));
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
}

#[test]
fn an_answer_is_always_for_the_words_on_show_since_an_entry_that_waits_cannot_go_and_come_back_empty() {
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
    assert_eq!((state(&page, cx).0.as_str(), waiting), ("first", true), "a second wish was typed under the first");
    assert_eq!(*made.borrow(), [wish("first", "garden")], "one Enter, so one answer to come");
    answer(&page, cx, true);
    assert_eq!((pieces(&page, cx), state(&page, cx).0.as_str()), (4, ""), "the entry given back is the page's");
}

#[test]
fn an_entry_that_waits_does_not_take_escape_so_the_keys_stay_its_own() {
    let mut cx = context();
    let (page, cx, _, _) = open(&mut cx);
    let field = piece(&page, cx, PieceKind::Entry, 0);
    click(cx, field.rect.center());
    keys(cx, "t e a enter escape");
    assert_eq!(state(&page, cx), ("tea".into(), 3, Some(0), true), "Escape was taken while the entry waited");
    answer(&page, cx, false);
    keys(cx, "escape");
    assert!(!state(&page, cx).3, "Escape gives the keys back once the answer has come");
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
