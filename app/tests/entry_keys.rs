//! Typing goes to the entry: when the page's window has the keys and nothing else on the page holds them (a note's
//! field), what a person types is in the entry, with no click first. So it is when the window opens, on a page of
//! projects and on a first page with none (the field that takes a name), after a tap on a note's option or on a
//! character, and after Escape. A bare `t` is always a letter; the theme is switched with the command key and T.
//! Played on GPUI's test platform through the page's own window (`support/windowed.rs`), on the Mac only; no
//! window is opened on a screen.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

#[cfg(all(target_os = "macos", feature = "window"))]
mod window {
    use std::fs;

    use gpui::{Entity, Keystroke, Modifiers, TestAppContext, VisualTestContext, point, px};
    use jellymoon_app::showing::Showing;
    use jellymoon_app::starting::NEW;
    use jellymoon_app::wishing::ENTRY;
    use sticker_kit::draw::Page;
    use sticker_kit::{Note, PieceKind, Scene};

    use crate::support::windowed::{context, held, opened};
    use crate::support::wished::{World, quiet};

    /// A first run with no session logs; with `project`, one project, `alpha`, on offer, and the day's page read
    /// and kept as the window's reading keeps it.
    fn world(project: bool) -> World {
        let w = quiet();
        if project {
            let alpha = w.at("work").join("alpha");
            fs::create_dir_all(alpha.join(".git")).unwrap();
            w.jellymoon(&["project", "add", "--repo", alpha.to_str().unwrap()]);
            assert_eq!(w.placed().iter().filter(|piece| piece.kind() == "chip").count(), 1);
        } else {
            fs::remove_dir(w.at("logs")).unwrap();
        }
        w
    }

    fn keys(cx: &mut VisualTestContext, keystrokes: &str) {
        cx.simulate_keystrokes(keystrokes);
        cx.run_until_parked();
    }

    fn theme(page: &Entity<Page>, cx: &mut VisualTestContext) -> String {
        page.read_with(cx, |page, _| page.theme().name.clone())
    }

    /// The entry's id, its words, and whether the entry (or a note's field) has the keys.
    fn entry(page: &Entity<Page>, cx: &mut VisualTestContext) -> (String, String, bool) {
        let (id, _, _, text, keys) = held(page, cx);
        (id, text, keys)
    }

    fn click_at(cx: &mut VisualTestContext, (x, y): (f32, f32)) {
        cx.simulate_click(point(px(x), px(y)), Modifiers::default());
        cx.run_until_parked();
    }

    /// A tap on the first placed piece of `kind` whose words hold `words`.
    fn tap(page: &Entity<Page>, cx: &mut VisualTestContext, kind: PieceKind, words: &str) {
        let found = page.read_with(cx, |page, _| {
            let said = |piece: &&sticker_kit::Placed| piece.text.iter().any(|run| run.text.contains(words));
            page.placed().iter().filter(|piece| piece.piece == kind).find(said).map(|piece| piece.rect.center())
        });
        click_at(cx, found.unwrap_or_else(|| panic!("no {kind:?} saying {words:?}")));
    }

    #[test]
    fn a_page_of_projects_opens_with_the_keys_in_the_entry_and_a_t_is_a_letter() {
        let w = world(true);
        let mut cx: TestAppContext = context();
        let (page, cx) = opened(&mut cx, Showing::of(Some(w.on_show())));
        keys(cx, "t i d y space t h e space t a b l e");
        assert_eq!(entry(&page, cx), (ENTRY.into(), "tidy the table".into(), true), "typed with no click");
        assert_eq!(theme(&page, cx), "sticker", "a t typed switched the theme");
    }

    #[test]
    fn a_first_page_with_no_project_opens_with_the_keys_in_the_field_that_takes_a_name() {
        let w = world(false);
        let mut cx: TestAppContext = context();
        let (page, cx) = opened(&mut cx, Showing::of(Some(w.on_show())));
        keys(cx, "g a r d e n");
        assert_eq!(entry(&page, cx), (NEW.into(), "garden".into(), true));
    }

    #[test]
    fn after_a_tap_on_keep_it_or_on_a_character_the_next_words_typed_are_in_the_entry() {
        let w = world(true);
        let shown = Showing::of(Some(w.on_show()));
        let mut cx: TestAppContext = context();
        let (page, cx) = opened(&mut cx, shown.clone());
        let mut scene = shown.scene().unwrap_or_else(Scene::sample);
        let options = ["Show me", "Keep it", "Toss it"].map(String::from).to_vec();
        let (id, title, body, quoted) = ("made-up".into(), "Work waits".into(), vec![], String::new());
        scene.notes.insert(0, Note::Text { id, title, body, options, quoted, field: None });
        let characters = Scene::sample().characters;
        scene.characters = characters;
        page.update(cx, |page, cx| page.set_scene(scene, cx));
        cx.run_until_parked();
        // The character first: the tap on `Keep it` gives the page the day's own scene.
        let little = |piece: &&sticker_kit::Placed| matches!(piece.piece, PieceKind::Character | PieceKind::Hero);
        let character =
            page.read_with(cx, |page, _| page.placed().iter().find(little).map(|piece| piece.rect.center()));
        click_at(cx, character.expect("a character is placed"));
        keys(cx, "n e x t");
        assert_eq!(entry(&page, cx), (ENTRY.into(), "next".into(), true), "after a tap on a character");
        tap(&page, cx, PieceKind::Option, "Keep it");
        keys(cx, "space o n e");
        assert_eq!(entry(&page, cx), (ENTRY.into(), "next one".into(), true), "after Keep it");
    }

    #[test]
    fn after_escape_the_next_letter_typed_is_in_the_entry_and_is_not_lost() {
        let w = world(true);
        let mut cx: TestAppContext = context();
        let (page, cx) = opened(&mut cx, Showing::of(Some(w.on_show())));
        keys(cx, "a escape b");
        assert_eq!(entry(&page, cx), (ENTRY.into(), "ab".into(), true));
        // At once, before the window draws again: a key typed in the same frame has a holder too.
        let escape = Keystroke::parse("escape").expect("a key");
        let held_at_once = cx.update(|window, cx| {
            window.dispatch_keystroke(escape, cx);
            page.read(cx).entry_focused(window)
        });
        assert!(held_at_once, "Escape left the page with no holder until the next frame");
        keys(cx, "escape escape t");
        assert_eq!((entry(&page, cx).1, theme(&page, cx)), ("abt".into(), "sticker".into()));
    }

    #[test]
    fn the_command_key_and_t_switches_the_theme_and_types_nothing() {
        let w = world(true);
        let mut cx: TestAppContext = context();
        let (page, cx) = opened(&mut cx, Showing::of(Some(w.on_show())));
        keys(cx, "x cmd-t");
        assert_eq!((entry(&page, cx).1, theme(&page, cx)), ("x".into(), "plain".into()));
        keys(cx, "cmd-t");
        assert_eq!((entry(&page, cx).1, theme(&page, cx)), ("x".into(), "sticker".into()));
    }
}
