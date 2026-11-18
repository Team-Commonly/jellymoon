//! The entry of a day with no project on offer, in the page's window on GPUI's test platform: a name entered is
//! answered with core's words, and a folder picker that is cancelled changes nothing. A name core takes starts
//! git, which is left to the made-up window, where core has the test's variables (`f66.rs`); the name here is
//! one core refuses before git is looked for.

use gpui::TestAppContext;
use jellymoon_app::showing::Showing;
use jellymoon_app::starting::{NEW, START, UNMADE};
use sticker_kit::{Note, PieceKind};

use super::{first_run, listed};
use crate::support::windowed::{click, context, held, opened, types};

fn title(shown: &Showing) -> Option<String> {
    match shown.scene()?.notes.first()? {
        Note::Plain { title, .. } => Some(title.clone()),
        _ => None,
    }
}

#[test]
fn a_name_core_refuses_is_answered_in_the_window_and_its_words_stay() {
    let w = first_run(false);
    let shown = Showing::of(Some(w.on_show()));
    let mut cx: TestAppContext = context();
    let (page, cx) = opened(&mut cx, shown.clone());
    assert_eq!(held(&page, cx).0, NEW);
    types(&page, cx, ".hidden", true);
    let (id, options, _, text, _) = held(&page, cx);
    assert_eq!((id.as_str(), options, text.as_str()), (NEW, vec![START.to_string()], ".hidden"), "not kept");
    assert!(!page.read_with(cx, |page, _| page.entry().waiting()), "the entry was answered");
    assert_eq!(title(&shown).as_deref(), Some(UNMADE));
    assert!(listed(&w).is_empty());
}

#[test]
fn a_folder_picker_that_is_cancelled_changes_nothing_and_the_words_stay() {
    let w = first_run(false);
    let shown = Showing::of(Some(w.on_show()));
    let mut cx: TestAppContext = context();
    let (page, cx) = opened(&mut cx, shown.clone());
    types(&page, cx, "garden", false);
    let before = shown.scene();
    click(&page, cx, PieceKind::Action);
    assert!(cx.did_prompt_for_paths(), "the system's folder picker was asked for");
    cx.simulate_path_prompt_response(|asked| {
        assert!(asked.directories && !asked.files && !asked.multiple, "one folder");
        None
    });
    cx.run_until_parked();
    assert_eq!(held(&page, cx).3, "garden");
    assert_eq!(shown.scene(), before);
    assert!(listed(&w).is_empty());
}
