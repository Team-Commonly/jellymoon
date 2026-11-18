//! A folder dropped on the page's window, and one chosen with the system's folder picker, on GPUI's test
//! platform: GPUI plays an outside drop as `FileDropEvent`s, and answers the picker with
//! `simulate_path_prompt_response`. The folder is listed by core in this process, which starts no program for it.

use gpui::TestAppContext;
use jellymoon_app::showing::Showing;
use jellymoon_app::starting::{ADDED, NEW};
use jellymoon_app::wishing::ENTRY;
use sticker_kit::{Note, PieceKind};

use super::{draws, repo};
use crate::support::windowed::{click, context, drop_on, held, opened, types};
use crate::support::wished::quiet;

fn title(shown: &Showing) -> Option<String> {
    match shown.scene()?.notes.first()? {
        Note::Plain { title, .. } => Some(title.clone()),
        _ => None,
    }
}

#[test]
fn a_folder_dropped_while_the_day_is_read_after_typing_has_its_chip_chosen_and_the_typed_words_stay() {
    let w = quiet();
    std::fs::remove_dir(w.at("logs")).unwrap();
    let folder = repo(&w, "garden");
    let shown = Showing::of(Some(w.on_show()));
    let mut cx: TestAppContext = context();
    let (page, cx) = opened(&mut cx, shown.clone());
    assert_eq!(held(&page, cx).0, NEW);
    assert_eq!(title(&shown).as_deref(), Some("Reading your day…"), "the first page, before the reading answers");
    // Keys last: GPUI's own drop asks whether the page is hovered, which it is not after a key (F64).
    types(&page, cx, "half a na", false);
    drop_on(cx, folder.clone());
    assert_eq!(title(&shown).as_deref(), Some(ADDED), "{:?}", shown.scene().map(|scene| scene.notes));
    let page_now = shown.scene().map(|scene| scene.footer);
    let (id, options, chosen, text, keys) = held(&page, cx);
    assert_eq!((id.as_str(), options, chosen), (ENTRY, vec!["garden".to_string()], Some(0)), "{page_now:?}");
    assert_eq!((text.as_str(), keys), ("half a na", true), "the words stay, and the entry has the keys");
    assert!(!draws(&shown.scene().unwrap(), &folder, &w), "the path was drawn");
}

#[test]
fn a_folder_chosen_beside_other_projects_has_its_chip_chosen_and_the_typed_words_stay() {
    let w = quiet();
    let alpha = w.at("work").join("alpha");
    w.jellymoon(&["project", "add", "--repo", alpha.to_str().unwrap()]);
    let folder = repo(&w, "garden");
    // The day's page, read and kept as the window's reading keeps it: a wish waits for a page.
    assert_eq!(w.placed().iter().filter(|piece| piece.kind() == "chip").count(), 1);
    let shown = Showing::of(Some(w.on_show()));
    let mut cx: TestAppContext = context();
    let (page, cx) = opened(&mut cx, shown.clone());
    assert_eq!(held(&page, cx).1, ["alpha"]);
    types(&page, cx, "water the roses", false);
    click(&page, cx, PieceKind::Action);
    cx.simulate_path_prompt_response(|_| Some(vec![folder.clone()]));
    cx.run_until_parked();
    let (id, options, chosen, text, keys) = held(&page, cx);
    assert_eq!((id.as_str(), options, chosen), (ENTRY, vec!["alpha".to_string(), "garden".to_string()], Some(1)));
    assert_eq!((text.as_str(), keys), ("water the roses", true));
    assert_eq!(title(&shown).as_deref(), Some(ADDED));
    // A drop goes where the picker goes: a second folder, dropped.
    let more = repo(&w, "shed");
    drop_on(cx, more);
    assert_eq!(held(&page, cx).2, Some(2), "the dropped folder's chip is the chosen one");
}
