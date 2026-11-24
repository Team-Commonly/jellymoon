//! The window's own path for Enter in a note's field, with no window on a screen: GPUI's test platform opens the
//! page `window.rs` makes, a click and keys go through GPUI's own dispatch, and the work the window runs off its
//! thread is run by the test's executor. On the Mac only, as the kit's draw tests are. The day is typed here as
//! core keeps one: a made wish whose run asks, and no record of that run, so core refuses every answer and
//! nothing is ever started.

use std::fs;
use std::ops::Deref;
use std::path::Path;

use gpui::{ClipboardItem, Entity, Keystroke, Modifiers, TestAppContext, TestDispatcher, VisualTestContext, point, px};
use jellymoon_app::showing::Showing;
use jellymoon_app::taps::OnShow;
use jellymoon_app::today::Today;
use jellymoon_app::window::{page, page_window};
use serde_json::json;
use sticker_kit::draw::headless::text_system;
use sticker_kit::draw::{Page, load_fonts};
use sticker_kit::{PieceKind, Scene, Theme, Viewport};

use super::QUESTION;
use crate::support::wished::SAID;

const DAY: &str = "2026-03-10";
const NOTE: &str = "wish-m1";
const ANSWER: &str = "the newer one";

fn context() -> TestAppContext {
    TestAppContext::build_with_text_system(TestDispatcher::new(0), None, text_system())
}

/// A day whose one made wish, `m1`, asks, and the scene with its note open, as a tap on its character opens it.
fn asking(dir: &Path) -> (Showing, Scene) {
    let (home, logs) = (dir.join("jm"), dir.join("logs"));
    let wish = json!({"id": "m1", "said": SAID, "project": "alpha", "state": "needs_you", "stars": [],
        "run": "r1111111111111111", "question": QUESTION});
    let file = home.join("pages").join(format!("{DAY}.wishes.json"));
    fs::create_dir_all(file.parent().unwrap()).unwrap();
    fs::create_dir_all(&logs).unwrap();
    fs::write(&file, format!("{}\n", json!({"day": DAY, "wishes": [wish]}))).unwrap();
    // Core reads a kept file only when its owner alone can write to it, whatever this process's umask is.
    fs::set_permissions(&file, std::os::unix::fs::PermissionsExt::from_mode(0o644)).unwrap();
    let (date, model, harness) = (DAY.to_string(), "sonnet".to_string(), Some(dir.join("claude")));
    let today = Today { date, logs, skip: Vec::new(), model, home, harness, fresh: false, settings: None, bin: None };
    let (scene, page) = today.first();
    let shown = Showing::of(Some(OnShow { today, page, scene }));
    let look = shown.poked("m1").expect("a tap on the wish's character is a look");
    let scene = look().and_then(|seen| shown.seen(seen)).expect("the wish's note is opened");
    (shown, scene)
}

/// The body of the first plain note on show: core's reason, after an answer it refused.
fn reason(shown: &Showing) -> String {
    let scene = shown.scene().expect("a day is on show");
    super::plain(&scene).first().map(|(_, body)| body.to_string()).unwrap_or_default()
}

fn opened(cx: &mut TestAppContext, shown: Showing, scene: Scene) -> (Entity<Page>, &mut VisualTestContext) {
    cx.update(|cx| load_fonts(cx)).expect("bundled fonts load");
    let viewport = Viewport { w: 1440.0, h: 900.0 };
    let window = cx.update(|cx| page(scene, Theme::sticker(), viewport, shown, cx)).expect("a page");
    let view = window.root(cx).expect("the page is the window's root");
    let cx = VisualTestContext::from_window(*window.deref(), cx).into_mut();
    cx.simulate_resize(gpui::size(px(viewport.w), px(viewport.h)));
    cx.run_until_parked();
    (view, cx)
}

/// The words in the field of the wish's note and whether it waits. None while the note has no field.
fn field(page: &Entity<Page>, cx: &mut VisualTestContext) -> Option<(String, bool)> {
    page.read_with(cx, |page, _| page.field(NOTE).map(|field| (field.text().to_string(), field.waiting())))
}

/// A click on the field under the wish's note, and `words` pasted into it.
fn pastes(page: &Entity<Page>, cx: &mut VisualTestContext, words: &str) {
    let is = |placed: &&sticker_kit::Placed| placed.piece == PieceKind::Field && placed.id == NOTE;
    // Where it is in the window: the column is held at its end, where the note is.
    let field = page.read_with(cx, |page, _| page.placed().iter().find(is).map(|field| page.shown_at(field)));
    let (x, y) = field.expect("the field is placed").center();
    cx.simulate_click(point(px(x), px(y)), Modifiers::default());
    cx.write_to_clipboard(ClipboardItem::new_string(words.into()));
    cx.simulate_keystrokes("cmd-v");
}

/// Enter, and nothing after it: what the window left for its other thread has not begun.
fn enter(cx: &mut VisualTestContext) {
    cx.update(|window, cx| window.dispatch_keystroke(Keystroke::parse("enter").expect("a key"), cx));
}

#[test]
fn enter_in_a_note_s_field_is_carried_by_the_window_and_core_s_refusal_leaves_the_words() {
    let dir = crate::support::scratch();
    let (shown, scene) = asking(dir.path());
    let on_show = shown.clone();
    let mut cx = context();
    let (page, cx) = opened(&mut cx, shown, scene);
    pastes(&page, cx, ANSWER);
    assert_eq!(field(&page, cx), Some((ANSWER.to_string(), false)));
    enter(cx);
    // At once the note says the wish is being worked on, with no field: nothing waits on show.
    assert_eq!(field(&page, cx), None);
    cx.run_until_parked();
    // Core refused: the note has its field again, with the words, and it does not wait.
    assert_eq!(field(&page, cx), Some((ANSWER.to_string(), false)));
    // The review of F54's notes, note 4: core was given the words. It says of no words that an answer needs
    // some; of these it says what it found when it looked for the run.
    assert_eq!(reason(&on_show), "the record of run r1111111111111111 cannot be read, so nothing was started.");
    // The field takes keys again, and a second Enter is carried and answered as the first.
    cx.simulate_keystrokes("! enter");
    assert_eq!(field(&page, cx), Some((format!("{ANSWER}!"), false)));
}

#[test]
fn an_answer_that_comes_after_the_window_was_closed_is_told_to_the_field_all_the_same() {
    let dir = crate::support::scratch();
    let (shown, scene) = asking(dir.path());
    let mut cx = context();
    let (page, cx) = opened(&mut cx, shown, scene);
    pastes(&page, cx, ANSWER);
    enter(cx);
    assert_eq!(field(&page, cx), None);
    // The window is closed before core has answered. The page's view outlives it.
    cx.update(|window, _| window.remove_window());
    let app: &TestAppContext = cx;
    assert!(app.update(|cx| page_window(cx)).is_none(), "the page's window is open");
    cx.run_until_parked();
    assert_eq!(field(&page, cx), Some((ANSWER.to_string(), false)), "the field of a closed window waits for ever");
}
