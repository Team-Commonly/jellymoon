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

fn wish(text: &str, option: usize) -> EntryMade {
    EntryMade { id: "wish".into(), text: text.into(), option: Some(option) }
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
    assert_eq!(*made.borrow(), [wish("tea at 4", 1)]);
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
    assert_eq!(*made.borrow(), [wish("one twoyadxz", 2)]);
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
    assert_eq!(*made.borrow(), [wish("a你好", 1)]);
}

#[test]
fn a_chip_chooses_its_option_and_the_page_under_the_entry_still_takes_taps() {
    let mut cx = context();
    let (page, cx, made, taps) = open(&mut cx);
    let chip = piece(&page, cx, PieceKind::Chip, 2);
    click(cx, chip.rect.center());
    assert_eq!(state(&page, cx), (String::new(), 0, Some(2), true), "a chip chooses and gives the entry the keys");
    keys(cx, "o k enter");
    assert_eq!(*made.borrow(), [wish("ok", 2)]);

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
    let count = |cx: &mut VisualTestContext| {
        let placed = page.read_with(cx, |page, _| page.placed().to_vec());
        placed.iter().filter(|p| matches!(p.piece, PieceKind::Entry | PieceKind::Chip)).count()
    };
    assert_eq!(count(cx), 2);
    page.update(cx, |page, cx| page.set_entry(None, cx));
    cx.run_until_parked();
    assert_eq!(
        (count(cx), state(&page, cx)),
        (0, (String::new(), 0, None, false)),
        "a page with no entry takes no keys"
    );
}

#[test]
fn refused_words_come_back_the_option_stays_by_name_and_no_entry_takes_no_text_input() {
    let mut cx = context();
    let (page, cx, made, _) = open(&mut cx);
    let accepts = |cx: &mut VisualTestContext| {
        cx.update(|window, cx| page.update(cx, |page, cx| page.accepts_text_input(window, cx)))
    };
    assert!(accepts(cx), "a page with an entry takes text input");
    let field = piece(&page, cx, PieceKind::Entry, 0);
    click(cx, field.rect.center());
    keys(cx, "t e a tab enter");
    assert_eq!(state(&page, cx), (String::new(), 0, Some(1), true));
    let words = made.borrow()[0].text.clone();
    cx.update(|window, cx| page.update(cx, |page, cx| page.put_back_entry(&words, window, cx)));
    cx.run_until_parked();
    assert_eq!(state(&page, cx), ("tea".into(), 3, Some(1), true), "the words the handler did not take are back");

    let moved = Entry { options: ["kitchen", "letters", "garden"].map(String::from).to_vec(), ..entry() };
    page.update(cx, |page, cx| page.set_entry(Some(moved), cx));
    cx.run_until_parked();
    assert_eq!(state(&page, cx).2, Some(0), "kitchen is still the chosen option");
    page.update(cx, |page, cx| page.set_entry(None, cx));
    cx.run_until_parked();
    assert!(!accepts(cx), "a page with no entry takes no text input");
}
