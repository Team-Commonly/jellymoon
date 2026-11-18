//! `Start` in the page's window on GPUI's test platform. With nothing typed it enters nothing, gives the field the
//! keys, and the note says what to type first; with words it does what Enter does. The name is one core refuses
//! before git is looked for, as in `f66/window.rs`: a name core takes would start git with this test's variables.

use gpui::TestAppContext;
use jellymoon_app::showing::Showing;
use jellymoon_app::starting::{NEW, START, UNMADE, UNREADY};
use sticker_kit::{Note, PieceKind};

use super::first_run;
use crate::support::windowed::{click, context, held, opened, types};

fn title(shown: &Showing) -> Option<String> {
    match shown.scene()?.notes.first()? {
        Note::Plain { title, .. } => Some(title.clone()),
        _ => None,
    }
}

#[test]
fn start_does_what_enter_does_and_with_nothing_typed_says_what_to_type_first() {
    let w = first_run(false);
    let shown = Showing::of(Some(w.on_show()));
    let mut cx: TestAppContext = context();
    let (page, cx) = opened(&mut cx, shown.clone());
    click(&page, cx, PieceKind::Go);
    let (id, _, _, text, keys) = held(&page, cx);
    assert_eq!((id.as_str(), text.as_str(), keys), (NEW, "", true), "nothing entered, and the field has the keys");
    assert_eq!(title(&shown).as_deref(), Some(UNREADY));
    types(&page, cx, ".hidden", false);
    click(&page, cx, PieceKind::Go);
    let (id, options, _, text, _) = held(&page, cx);
    assert_eq!((id.as_str(), options, text.as_str()), (NEW, vec![START.to_string()], ".hidden"), "not kept");
    assert!(!page.read_with(cx, |page, _| page.entry().waiting()), "the entry was answered");
    assert_eq!(title(&shown).as_deref(), Some(UNMADE), "core was given the name, as Enter gives it");
    let list = w.jellymoon(&["project", "list"]);
    assert!(list["projects"].as_array().unwrap().is_empty());
}
