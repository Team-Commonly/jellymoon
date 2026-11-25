//! A question takes the keys (ADR 0025, decision 10). A sharper wish that asks is answered by typing: its field
//! has the keys when it first stands, so the letters a person types on reading the question are the answer, with
//! no click, and are never a new wish in the pair's place. Words already typed in the entry keep the keys there.
//! Played through the page's own window on GPUI's test platform (`support/windowed.rs`), which is where the keys
//! are: the made-up window of `support/tapper.rs` has no view, and types into a field it is told by name. On the
//! Mac only; no window is opened on a screen. The made-up agent of F39's tests answers the shaping call, and no
//! run is started: the `jellymoon` command is not installed in this world.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

#[cfg(all(target_os = "macos", feature = "window"))]
mod window {
    use std::fs;

    use gpui::{Entity, Keystroke, TestAppContext, VisualTestContext};
    use jellymoon_app::shaping::SHAPE;
    use jellymoon_app::showing::Showing;
    use jellymoon_app::wishing::ENTRY;
    use serde_json::{Value, json};
    use sticker_kit::Note;
    use sticker_kit::draw::Page;

    use crate::support::day::DAY;
    use crate::support::windowed::{context, held, opened};
    use crate::support::wished::{World, quiet};

    /// A day with `alpha` on offer, on which the made-up agent answers a shaping call with a sharper wish that
    /// asks, and the window's own state of that day.
    fn asked() -> (World, Showing) {
        let w = quiet();
        w.jellymoon(&["project", "add", "--repo", w.at("work").join("alpha").to_str().unwrap()]);
        // The day's page is read and kept, as the window's reading keeps it: `alpha` has its chip.
        assert_eq!(w.placed().iter().filter(|piece| piece.kind() == "chip").count(), 1);
        let sharper = json!({"clear": false, "wish": "Make the uploader try a failed upload twice more.",
            "done_when": "A failed upload is tried three times in all.", "not_touching": "How a file is read.",
            "why": "The uploader gives up after one failure today.", "ask": "Should it wait between tries?"});
        let printed = json!({"type": "result", "is_error": false, "result": sharper.to_string()});
        fs::write(w.at("agent").join("shape"), format!("says\n{printed}")).unwrap();
        let shown = Showing::of(Some(w.on_show()));
        (w, shown)
    }

    fn keys(cx: &mut VisualTestContext, keystrokes: &str) {
        cx.simulate_keystrokes(keystrokes);
        cx.run_until_parked();
    }

    /// What is typed in the entry, and in the answer's field while the question is on the page.
    fn typed(page: &Entity<Page>, cx: &mut VisualTestContext) -> (String, Option<String>) {
        page.read_with(cx, |page, _| {
            (page.entry().text().to_string(), page.field(SHAPE).map(|field| field.text().to_string()))
        })
    }

    /// Whether the sharper wish and its question wait on the page.
    fn waits(shown: &Showing) -> bool {
        let asks = |note: &Note| matches!(note, Note::Text { id, field: Some(_), .. } if id == SHAPE);
        shown.scene().is_some_and(|scene| scene.notes.iter().any(asks))
    }

    /// The day's made wishes as they are kept.
    fn made(w: &World) -> Vec<Value> {
        let kept = fs::read_to_string(w.at("jm").join("pages").join(format!("{DAY}.wishes.json")));
        let kept: Value = kept.map(|kept| serde_json::from_str(&kept).unwrap()).unwrap_or_default();
        kept["wishes"].as_array().cloned().unwrap_or_default()
    }

    #[test]
    fn letters_typed_when_a_question_comes_are_its_answer_with_no_click_and_the_wish_goes_on_with_them() {
        let (w, shown) = asked();
        let mut cx: TestAppContext = context();
        let (page, cx) = opened(&mut cx, shown.clone());
        assert_eq!(held(&page, cx).0, ENTRY, "the entry a wish is typed in");
        keys(cx, "t i d y space i t enter");
        assert!(waits(&shown) && made(&w).is_empty(), "the question waits, and nothing is kept: {:?}", shown.scene());
        keys(cx, "t w o space s e c o n d s");
        assert_eq!(typed(&page, cx), (String::new(), Some("two seconds".into())), "the letters are the answer");
        keys(cx, "enter");
        let kept = made(&w);
        let wishes: Vec<_> = kept.iter().map(|wish| (wish["said"].as_str(), wish["answered"].as_str())).collect();
        assert_eq!(wishes, [(Some("tidy it"), Some("two seconds"))], "one wish, the first, with the answer");
        // The question is gone: the next letters are in the entry.
        keys(cx, "n e x t");
        assert_eq!((typed(&page, cx), waits(&shown)), (("next".into(), None), false));
    }

    #[test]
    fn words_already_typed_in_the_entry_when_the_question_comes_keep_the_keys_and_the_pair_still_waits() {
        let (w, shown) = asked();
        let mut cx: TestAppContext = context();
        let (page, cx) = opened(&mut cx, shown.clone());
        // Each key alone, and nothing after it: the next wish is begun before core has answered the first.
        for key in "t i d y space i t enter n e x t".split(' ') {
            cx.update(|window, cx| window.dispatch_keystroke(Keystroke::parse(key).expect("a key"), cx));
        }
        cx.run_until_parked();
        assert_eq!((typed(&page, cx), waits(&shown)), (("next".into(), Some(String::new())), true));
        keys(cx, "space o n e");
        assert_eq!(typed(&page, cx), ("next one".into(), Some(String::new())), "the letters join the entry's words");
        assert!(waits(&shown) && made(&w).is_empty(), "the pair still waits, and nothing is kept");
    }
}
