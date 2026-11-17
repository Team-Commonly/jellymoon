//! The app's windows with no window on a screen: GPUI's test platform opens what `shell.rs` wires, a click goes
//! through GPUI's own dispatch, and the test's executor runs the looks and moves the clock. On the Mac only, as
//! the kit's draw tests are. The page has no day, so nothing is read and nothing is kept.

use std::ops::Deref;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use gpui::{Modifiers, PlatformDispatcher, TestAppContext, TestDispatcher, VisualTestContext, WindowHandle, point, px};
use jellymoon_app::jellyfish::Room;
use jellymoon_app::shell::{Asked, JellyLook, Opened, jelly_look, wire};
use jellymoon_app::window::page_window;
use jellymoon_core::{Jelly, JellyDrawn, JellyState};
use serde_json::{Value, json};
use sticker_kit::draw::headless::text_system;
use sticker_kit::draw::{EdgeView, load_fonts};
use sticker_kit::{Corner, EDGE_GAP, EDGE_MARGIN, EdgeKind, EdgeMood, Rect, Scene, Theme, Viewport};

use crate::SAID;
use crate::jelly::{asks, cannot, heard, listening, quiet, waits, working};

/// A display of 1440 by 900 whose menu bar takes 25 pixels at the top and whose Dock takes 70 at the bottom.
const ROOM: Room = Room {
    screen: Viewport { w: 1440.0, h: 900.0 },
    left: Rect { x: 0.0, y: 25.0, w: 1440.0, h: 805.0 },
    origin: (0.0, 0.0),
    display: None,
};
const JELLY_AT: [f32; 4] = [1328.0, 718.0, 96.0, 96.0];
const BUBBLE_AT: [f32; 4] = [1020.0, 596.0, 300.0, 218.0];
const SECOND: Duration = Duration::from_secs(1);

fn context() -> TestAppContext {
    TestAppContext::build_with_text_system(TestDispatcher::new(0), None, text_system())
}

fn opened(cx: &mut TestAppContext, jelly: Option<JellyLook>, quit_after: Option<Duration>) -> Opened {
    opened_in(cx, ROOM, jelly, quit_after)
}

fn opened_in(cx: &mut TestAppContext, room: Room, jelly: Option<JellyLook>, quit_after: Option<Duration>) -> Opened {
    cx.update(|cx| load_fonts(cx)).expect("bundled fonts load");
    let viewport = Viewport { w: 1440.0, h: 900.0 };
    let asked = Asked { scene: Scene::sample(), theme: Theme::sticker(), viewport, day: None, jelly, quit_after };
    let opened = cx.update(|cx| wire(asked, room, cx)).expect("the windows open");
    cx.run_until_parked();
    opened
}

/// What core says is drawn of one line of `jellymoon hear`.
fn drawn(line: &Value) -> JellyDrawn {
    Jelly::read(line.to_string().as_bytes()).expect("a line of a hearing").drawn()
}

fn draw(opened: &Opened, line: &Value, cx: &mut TestAppContext) {
    cx.update(|cx| opened.draw(&drawn(line), cx));
    cx.run_until_parked();
}

fn bounds(window: WindowHandle<EdgeView>, cx: &mut TestAppContext) -> [f32; 4] {
    let at = window.update(cx, |_, window, _| window.bounds()).expect("the window is open");
    [at.origin.x.into(), at.origin.y.into(), at.size.width.into(), at.size.height.into()]
}

fn jelly(opened: &Opened) -> WindowHandle<EdgeView> {
    opened.jelly().expect("the jellyfish is open").0
}

fn mood(opened: &Opened, cx: &mut TestAppContext) -> EdgeMood {
    jelly(opened).update(cx, |view, _, _| view.edge().mood).expect("the jellyfish is open")
}

fn close_page(cx: &mut TestAppContext) {
    let page = cx.update(|cx| page_window(cx)).expect("the page is open");
    page.update(cx, |_, window, _| window.remove_window()).expect("the page is open");
    cx.run_until_parked();
}

#[test]
fn the_jellyfish_is_opened_where_the_kit_places_it_in_the_room_the_system_leaves() {
    let mut cx = context();
    let opened = opened(&mut cx, None, None);
    // 16 pixels from the display's right edge and from the top of the Dock, not from the display's lower edge.
    assert_eq!(bounds(jelly(&opened), &mut cx), JELLY_AT);
    let (character, bubble) = sticker_kit::edge_windows(&Theme::sticker());
    let placed =
        sticker_kit::place_pair(ROOM.screen, ROOM.left, character, bubble, Corner::BottomRight, EDGE_MARGIN, EDGE_GAP);
    assert_eq!(opened.jelly().map(|(_, stands)| stands), Some(placed));
    assert_eq!((mood(&opened, &mut cx), opened.bubble(), cx.windows().len()), (EdgeMood::Floats, None, 2));
}

// The review of F57, finding 3: where a display is among the displays is added to where the kit places a window
// on it. A display to the left of the first one, and lower.
#[test]
fn a_window_stands_where_its_display_is_among_the_displays() {
    let mut cx = context();
    let opened = opened_in(&mut cx, Room { origin: (-1440.0, 200.0), ..ROOM }, None, None);
    assert_eq!(bounds(jelly(&opened), &mut cx), [-112.0, 918.0, 96.0, 96.0]);
    draw(&opened, &asks("Which one?"), &mut cx);
    assert_eq!(bounds(opened.bubble().expect("words open the bubble"), &mut cx), [-420.0, 796.0, 300.0, 218.0]);
    // Where each stands on its own display is what it was.
    let stands = opened.jelly().map(|(_, (jelly, bubble))| ([jelly.x, jelly.y], [bubble.x, bubble.y]));
    assert_eq!(stands, Some(([1328.0, 718.0], [1020.0, 596.0])));
}

// The same finding: the room is the primary display's, as the system says it. The test platform has one display
// of 1920 by 1080 at the top left of the displays, with nothing taken from it.
#[test]
fn the_room_is_the_primary_display_s_and_the_jellyfish_is_opened_in_it() {
    let mut cx = context();
    let room = cx.update(|cx| Room::primary(cx));
    let whole = Rect { x: 0.0, y: 0.0, w: 1920.0, h: 1080.0 };
    assert_eq!((room.screen, room.left, room.origin), (Viewport { w: 1920.0, h: 1080.0 }, whole, (0.0, 0.0)));
    assert!(room.display.is_some(), "the windows are asked for on that display");
    let opened = opened_in(&mut cx, room, None, None);
    assert_eq!(bounds(jelly(&opened), &mut cx), [1808.0, 968.0, 96.0, 96.0]);
}

#[test]
fn each_of_the_seven_states_is_drawn_and_the_bubble_is_open_only_while_there_are_words() {
    let mut cx = context();
    let opened = opened(&mut cx, None, None);
    let states = [
        (quiet(), EdgeMood::Floats, false),
        (listening("make"), EdgeMood::LeansIn, true),
        (heard(SAID, "alpha", 2000), EdgeMood::HoldsUp, true),
        (working(3), EdgeMood::Swims, false),
        (asks("Which one?"), EdgeMood::Waves, true),
        (waits(), EdgeMood::Glows, false),
        (cannot("nothing was heard: no microphone was found"), EdgeMood::Droops, true),
    ];
    for (line, drawn_as, words) in states {
        draw(&opened, &line, &mut cx);
        let state = line["state"].as_str().unwrap();
        let kinds =
            jelly(&opened).update(&mut cx, |view, _, _| view.placed().iter().map(|p| p.kind).collect::<Vec<_>>());
        let count =
            if state == "working" { vec![EdgeKind::Character, EdgeKind::Count] } else { vec![EdgeKind::Character] };
        assert_eq!((mood(&opened, &mut cx), kinds.unwrap()), (drawn_as, count), "{state}");
        assert_eq!(opened.bubble().is_some(), words, "{state}");
        let Some(bubble) = opened.bubble() else { continue };
        let pieces = bubble.update(&mut cx, |view, _, _| view.placed().to_vec()).unwrap();
        assert!(pieces[0].kind == EdgeKind::Bubble && !pieces[0].text.is_empty(), "{state}: {pieces:?}");
        assert_eq!(pieces.iter().any(|piece| piece.kind == EdgeKind::Ring), state == "heard", "{state}");
        // A run's question is drawn apart from Jellymoon's own words, as text of its own role.
        let quoted = pieces[0].text.iter().filter(|run| run.role == sticker_kit::Role::Quoted).count();
        assert_eq!(quoted > 0, state == "asks", "{state}");
    }
    // Words nobody sees are no words: no window is opened for them.
    draw(&opened, &listening("\u{200b}\u{2800}"), &mut cx);
    assert_eq!((opened.bubble(), cx.windows().len()), (None, 2));
}

#[test]
fn the_bubble_is_opened_and_removed_and_no_window_is_ever_moved_or_resized() {
    let mut cx = context();
    let opened = opened(&mut cx, None, None);
    let first = jelly(&opened);
    let lines = [
        heard(SAID, "alpha", 4000),
        heard(SAID, "alpha", 1000),
        quiet(),
        asks("Which of the two uploaders did you mean, and should the other one be left as it is?"),
        listening(""),
        cannot("nothing was heard"),
        waits(),
    ];
    let mut seen = Vec::new();
    for line in lines {
        draw(&opened, &line, &mut cx);
        assert_eq!((jelly(&opened), bounds(first, &mut cx)), (first, JELLY_AT), "{line}");
        if let Some(bubble) = opened.bubble() {
            assert_eq!(bounds(bubble, &mut cx), BUBBLE_AT, "{line}");
        }
        assert_eq!(cx.windows().len(), 2 + usize::from(opened.bubble().is_some()), "{line}");
        seen.push(opened.bubble());
    }
    // Words that change are drawn in the window that is open; words that end remove it; new words open another.
    assert!(seen[0].is_some() && seen[1] == seen[0], "the ring ran down in a new window");
    assert!(seen[2].is_none() && seen[3].is_some() && seen[3] != seen[0]);
    assert!(seen[4].is_none() && seen[5].is_some() && seen[6].is_none());
}

// The review of F57, finding 8: a bubble that something else removed is not held as if it were open.
#[test]
fn a_bubble_removed_by_something_else_is_opened_again_at_the_next_words() {
    let mut cx = context();
    let opened = opened(&mut cx, None, None);
    draw(&opened, &asks("Which one?"), &mut cx);
    let first = opened.bubble().expect("words open the bubble");
    first.update(&mut cx, |_, window, _| window.remove_window()).expect("the bubble is open");
    cx.run_until_parked();
    assert_eq!(cx.windows().len(), 2);
    draw(&opened, &asks("Which of the two?"), &mut cx);
    let again = opened.bubble().expect("the next words open a bubble");
    assert!(again != first && cx.windows().len() == 3, "the bubble that was removed is still held");
    let said = again.update(&mut cx, |view, _, _| view.edge().bubble.last().map(|said| said.text.clone()));
    assert_eq!((said.unwrap().as_deref(), bounds(again, &mut cx)), (Some("Which of the two?"), BUBBLE_AT));
    // And when the words end it is removed as any bubble is.
    draw(&opened, &quiet(), &mut cx);
    assert_eq!((opened.bubble(), cx.windows().len()), (None, 2));
}

#[test]
fn closing_the_page_leaves_the_app_and_the_jellyfish_and_a_tap_or_opening_again_shows_the_page() {
    let mut cx = context();
    let opened = opened(&mut cx, None, None);
    let page = cx.update(|cx| page_window(cx)).expect("the page is open");
    let view = page.root(&mut cx).expect("the page is its window's root");
    draw(&opened, &working(2), &mut cx);
    close_page(&mut cx);
    assert!(cx.update(|cx| page_window(cx)).is_none() && !opened.ended(), "closing the page ended the app");
    assert_eq!((cx.windows().len(), mood(&opened, &mut cx)), (1, EdgeMood::Swims));
    // The jellyfish is still drawn, and still opens its bubble.
    draw(&opened, &asks("Which one?"), &mut cx);
    assert!(opened.bubble().is_some() && cx.update(|cx| page_window(cx)).is_none());
    // A tap on the jellyfish shows the page, the same one, and does nothing else.
    let tapper = VisualTestContext::from_window(*jelly(&opened).deref(), &cx).into_mut();
    tapper.simulate_click(point(px(48.0), px(48.0)), Modifiers::default());
    tapper.run_until_parked();
    let again = cx.update(|cx| page_window(cx)).expect("a tap on the jellyfish shows the page");
    assert!(again != page && again.root(&mut cx).ok() == Some(view.clone()), "another page was shown");
    assert_eq!((mood(&opened, &mut cx), bounds(jelly(&opened), &mut cx)), (EdgeMood::Waves, JELLY_AT));
    assert!(opened.bubble().is_some() && cx.windows().len() == 3);
    // With the page open a tap opens no second one, and a tap on the bubble does nothing at all.
    tapper.simulate_click(point(px(48.0), px(48.0)), Modifiers::default());
    tapper.run_until_parked();
    assert_eq!((cx.update(|cx| page_window(cx)), cx.windows().len()), (Some(again), 3));
    close_page(&mut cx);
    let bubble = VisualTestContext::from_window(*opened.bubble().unwrap().deref(), &cx).into_mut();
    for (x, y) in [(150.0, 200.0), (20.0, 150.0), (280.0, 210.0)] {
        bubble.simulate_click(point(px(x), px(y)), Modifiers::default());
        bubble.run_until_parked();
    }
    assert!(cx.update(|cx| page_window(cx)).is_none(), "a tap on the bubble showed the page");
    // Opening the app again shows it too, once.
    cx.update(|cx| opened.show_page(cx));
    cx.update(|cx| opened.show_page(cx));
    cx.run_until_parked();
    let shown = cx.update(|cx| page_window(cx)).expect("opening the app again shows the page");
    assert!(shown.root(&mut cx).ok() == Some(view) && cx.windows().len() == 3 && !opened.ended());
}

#[test]
fn quit_ends_the_page_and_the_jellyfish_and_nothing_is_opened_after_it() {
    let mut cx = context();
    let opened = opened(&mut cx, None, None);
    draw(&opened, &heard(SAID, "alpha", 4000), &mut cx);
    assert_eq!(cx.windows().len(), 3);
    cx.update(|cx| opened.quit(cx));
    cx.run_until_parked();
    assert!(opened.ended() && opened.jelly().is_none() && opened.bubble().is_none());
    assert_eq!(cx.windows().len(), 0, "a window outlived Quit");
    cx.update(|cx| {
        opened.show_page(cx);
        opened.draw(&drawn(&asks("Which one?")), cx);
        opened.quit(cx);
    });
    cx.run_until_parked();
    assert_eq!(cx.windows().len(), 0, "something was opened after Quit");
    // With the page closed first, Quit ends the jellyfish all the same.
    let mut cx = context();
    let opened = self::opened(&mut cx, None, None);
    close_page(&mut cx);
    cx.update(|cx| opened.quit(cx));
    cx.run_until_parked();
    assert!(opened.ended() && cx.windows().is_empty());
}

#[test]
fn the_time_asked_for_ends_the_app_by_itself_and_not_before() {
    let mut cx = context();
    let opened = opened(&mut cx, None, Some(Duration::from_secs(3)));
    cx.executor().advance_clock(Duration::from_millis(2999));
    cx.run_until_parked();
    assert!(!opened.ended() && cx.windows().len() == 2);
    cx.executor().advance_clock(Duration::from_millis(1));
    cx.run_until_parked();
    assert!(opened.ended() && cx.windows().is_empty());
}

#[test]
fn once_a_second_the_jellyfish_is_looked_at_off_the_window_s_thread_and_drawn_again() {
    let mut cx = context();
    let lines = [working(2), asks("Which one?"), quiet(), waits()];
    // Whether each look ran on the window's thread.
    let looks = Arc::new(Mutex::new(Vec::new()));
    let (seen, dispatcher) = (looks.clone(), cx.dispatcher.clone());
    let look: JellyLook = Arc::new(move || {
        let mut seen = seen.lock().unwrap();
        seen.push(dispatcher.is_main_thread());
        drawn(&lines[(seen.len() - 1).min(lines.len() - 1)])
    });
    let count = || looks.lock().unwrap().len();
    let opened = opened(&mut cx, Some(look), None);
    // The first look is at once: the jellyfish is not quiet for a second while a run goes.
    assert_eq!((count(), mood(&opened, &mut cx), opened.bubble()), (1, EdgeMood::Swims, None));
    cx.executor().advance_clock(Duration::from_millis(999));
    cx.run_until_parked();
    assert_eq!(count(), 1, "a look before a second had passed");
    cx.executor().advance_clock(Duration::from_millis(1));
    cx.run_until_parked();
    assert_eq!((count(), mood(&opened, &mut cx), opened.bubble().is_some()), (2, EdgeMood::Waves, true));
    cx.executor().advance_clock(SECOND);
    cx.run_until_parked();
    assert_eq!((count(), mood(&opened, &mut cx), opened.bubble()), (3, EdgeMood::Floats, None));
    // It goes on with the page closed, and ends with the app.
    close_page(&mut cx);
    cx.executor().advance_clock(SECOND);
    cx.run_until_parked();
    assert_eq!((count(), mood(&opened, &mut cx)), (4, EdgeMood::Glows));
    cx.update(|cx| opened.quit(cx));
    cx.executor().advance_clock(5 * SECOND);
    cx.run_until_parked();
    assert_eq!(count(), 4, "a look after Quit");
    assert_eq!(*looks.lock().unwrap(), [false; 4], "a look ran on the window's thread");
}

/// A day's made wishes in a home of the test's own, typed here as core keeps them.
fn made(home: &std::path::Path, day: &str, state: &str, more: Value) {
    let mut wish = json!({"id": "m1", "said": SAID, "project": "alpha", "state": state, "stars": []});
    wish["run"] = json!("r1111111111111111");
    more.as_object().unwrap().iter().for_each(|(name, value)| wish[name] = value.clone());
    let file = home.join("pages").join(format!("{day}.wishes.json"));
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(&file, format!("{}\n", json!({"day": day, "wishes": [wish]}))).unwrap();
    // Core reads a kept file only when its owner alone can write to it, whatever this process's umask is.
    std::fs::set_permissions(&file, std::os::unix::fs::PermissionsExt::from_mode(0o644)).unwrap();
}

// The review of F57, finding 9: the app lives for days, so the look asks which day it is at each look. The clock
// is the test's own.
#[test]
fn past_midnight_the_jellyfish_shows_the_new_day_s_runs_and_a_day_that_was_named_stays() {
    let dir = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let home = dir.path().join("jm");
    made(&home, "2026-03-10", "working", json!({}));
    made(&home, "2026-03-11", "needs_you", json!({"question": "Which one?"}));
    let today = Arc::new(Mutex::new("2026-03-10"));
    let clock = today.clone();
    let look = jelly_look(home.clone(), None, move || Some(clock.lock().unwrap().to_string()));
    let mut cx = context();
    let opened = opened(&mut cx, Some(look), None);
    assert_eq!((mood(&opened, &mut cx), opened.bubble()), (EdgeMood::Swims, None));
    *today.lock().unwrap() = "2026-03-11";
    cx.executor().advance_clock(SECOND);
    cx.run_until_parked();
    assert_eq!((mood(&opened, &mut cx), opened.bubble().is_some()), (EdgeMood::Waves, true), "still yesterday's");
    // A day named with `--date` is that day, whatever the clock says.
    let named = jelly_look(home, Some("2026-03-10".to_string()), || Some("2026-03-11".to_string()));
    assert_eq!(named().state, JellyState::Working);
}
