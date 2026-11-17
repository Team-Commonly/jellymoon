//! The jellyfish's look at a day, through all of `shell.rs` on GPUI's test platform: the page is a day's, as
//! `main.rs` hands one over, and the clock is the test's own. The made wishes are typed here as core keeps
//! them, in a home of the test's own; no run is started.

use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use gpui::{PlatformDispatcher, TestAppContext};
use jellymoon_app::shell::{Clock, Opened};
use jellymoon_app::taps::OnShow;
use jellymoon_app::today::Today;
use serde_json::{Value, json};
use sticker_kit::EdgeMood;

use crate::SAID;
use crate::support::day;
use crate::window::{SECOND, close_page, context, mood, opened};

/// The day the page was opened on, and the day after it.
const DAY: &str = day::DAY;
const NEXT: &str = "2026-03-11";
const RUN: &str = "r1111111111111111";

/// A day's page with no session logs to read, in `home`.
fn page_of(home: &Path, date: &str) -> Today {
    Today {
        date: date.to_string(),
        logs: home.join("logs"),
        skip: Vec::new(),
        model: "sonnet".to_string(),
        home: home.to_path_buf(),
        harness: None,
        fresh: false,
        settings: None,
        bin: None,
    }
}

/// A day's one made wish with a run, typed as core keeps it.
fn made(home: &Path, day: &str, state: &str, more: Value) {
    let mut wish = json!({"id": "m1", "said": SAID, "project": "alpha", "state": state, "stars": [], "run": RUN});
    more.as_object().unwrap().iter().for_each(|(name, value)| wish[name] = value.clone());
    let file = home.join("pages").join(format!("{day}.wishes.json"));
    fs::create_dir_all(file.parent().unwrap()).unwrap();
    fs::write(&file, format!("{}\n", json!({"day": day, "wishes": [wish]}))).unwrap();
    // Core reads a kept file only when its owner alone can write to it, whatever this process's umask is.
    fs::set_permissions(&file, std::os::unix::fs::PermissionsExt::from_mode(0o644)).unwrap();
}

/// Holds the run's lock, as a `jellymoon run` that lives does. The page's own look writes a working wish of its
/// day down as cut off when nothing holds its run, and the jellyfish would then have no run to show.
fn held(home: &Path) -> fs::File {
    let lock = home.join("runs").join(RUN).join("run.lock");
    fs::create_dir_all(lock.parent().unwrap()).unwrap();
    let held = fs::File::create(&lock).unwrap();
    held.lock().unwrap();
    held
}

fn asking(home: &Path, day: &str) {
    made(home, day, "needs_you", json!({"question": "Which one?"}));
}

/// A clock a test moves, and the hand that moves it.
fn clock(at: &'static str) -> (Clock, Arc<Mutex<&'static str>>) {
    let day = Arc::new(Mutex::new(at));
    let read = day.clone();
    (Arc::new(move || Some(read.lock().unwrap().to_string())), day)
}

fn tick(cx: &mut TestAppContext) {
    cx.executor().advance_clock(SECOND);
    cx.run_until_parked();
}

/// The mood of the jellyfish, and the last words of its bubble when it is open.
fn shown(opened: &Opened, cx: &mut TestAppContext) -> (EdgeMood, Option<String>) {
    let last = |bubble: gpui::WindowHandle<sticker_kit::draw::EdgeView>| {
        bubble.update(cx, |view, _, _| view.edge().bubble.last().map(|said| said.text.clone())).unwrap()
    };
    let said = opened.bubble().and_then(last);
    (mood(opened, cx), said)
}

#[test]
fn once_a_second_the_jellyfish_is_looked_at_off_the_window_s_thread_and_drawn_again() {
    let dir = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let home = dir.path().join("jm");
    made(&home, DAY, "working", json!({}));
    let _run = held(&home);
    let mut cx = context();
    // The clock is asked once at each look: whether each ran on the window's thread.
    let looks = Arc::new(Mutex::new(Vec::new()));
    let (seen, dispatcher) = (looks.clone(), cx.dispatcher.clone());
    let clock: Clock = Arc::new(move || {
        seen.lock().unwrap().push(dispatcher.is_main_thread());
        Some(DAY.to_string())
    });
    let count = || looks.lock().unwrap().len();
    let opened = opened(&mut cx, Some((page_of(&home, DAY), false, clock)), None);
    // The first look is at once: the jellyfish is not quiet for a second while a run goes.
    assert_eq!((count(), mood(&opened, &mut cx), opened.bubble()), (1, EdgeMood::Swims, None));
    asking(&home, DAY);
    cx.executor().advance_clock(Duration::from_millis(999));
    cx.run_until_parked();
    assert_eq!((count(), mood(&opened, &mut cx)), (1, EdgeMood::Swims), "a look before a second had passed");
    cx.executor().advance_clock(Duration::from_millis(1));
    cx.run_until_parked();
    assert_eq!((count(), mood(&opened, &mut cx), opened.bubble().is_some()), (2, EdgeMood::Waves, true));
    made(&home, DAY, "let_go", json!({}));
    tick(&mut cx);
    assert_eq!((count(), mood(&opened, &mut cx), opened.bubble()), (3, EdgeMood::Floats, None));
    // It goes on with the page closed, and ends with the app.
    close_page(&mut cx);
    made(&home, DAY, "working", json!({}));
    tick(&mut cx);
    assert_eq!((count(), mood(&opened, &mut cx)), (4, EdgeMood::Swims));
    cx.update(|cx| opened.quit(cx));
    cx.executor().advance_clock(5 * SECOND);
    cx.run_until_parked();
    assert_eq!(count(), 4, "a look after Quit");
    assert_eq!(*looks.lock().unwrap(), [false; 4], "a look ran on the window's thread");
}

// The review of F57, finding 9: the app lives for days, so the look asks which day it is at each look. The
// review of its notes, finding 2: the look is built in `wire`, from the page's day and whether it was named.
#[test]
fn past_midnight_the_jellyfish_shows_the_new_day_s_runs_and_a_day_that_was_named_stays() {
    let dir = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let home = dir.path().join("jm");
    made(&home, NEXT, "working", json!({}));
    let (today, hand) = clock(DAY);
    let mut cx = context();
    let opened = opened(&mut cx, Some((page_of(&home, DAY), false, today)), None);
    assert_eq!(shown(&opened, &mut cx), (EdgeMood::Floats, None));
    *hand.lock().unwrap() = NEXT;
    tick(&mut cx);
    assert_eq!(shown(&opened, &mut cx), (EdgeMood::Swims, None), "still yesterday's");
    // A day named with `--date` is that day, whatever the clock says.
    let (today, _hand) = clock(NEXT);
    let mut cx = context();
    let named = self::opened(&mut cx, Some((page_of(&home, DAY), true, today)), None);
    tick(&mut cx);
    assert_eq!(shown(&named, &mut cx), (EdgeMood::Floats, None), "a named day showed another day's run");
}

// The review of the F57 notes, finding 1: the page files what it writes under the day it was opened on, so past
// midnight the jellyfish looks at that day too. The wish is made by the call the page's entry makes.
#[test]
fn past_midnight_a_wish_made_on_the_page_is_shown_by_the_jellyfish_when_its_run_goes() {
    let world = day::world();
    let page = Today { logs: world.dir.path().join("logs"), ..page_of(&world.home, DAY) };
    let (today, hand) = clock(DAY);
    let mut cx = context();
    let opened = opened(&mut cx, Some((page.clone(), false, today)), None);
    *hand.lock().unwrap() = NEXT;
    tick(&mut cx);
    assert_eq!(shown(&opened, &mut cx), (EdgeMood::Floats, None));
    let (scene, kept) = page.first();
    let mut on = OnShow { today: page, page: kept, scene };
    // The asking for its run is left alone: the command core starts would inherit this process's variables.
    let (_, asking) = on.wished(SAID, Some("alpha"));
    assert!(asking.is_some(), "core kept the wish");
    let file = world.home.join("pages").join(format!("{DAY}.wishes.json"));
    let mut made: Value =
        serde_json::from_str(&fs::read_to_string(&file).expect("filed under the page's day")).unwrap();
    assert!(!world.home.join("pages").join(format!("{NEXT}.wishes.json")).exists());
    tick(&mut cx);
    assert_eq!(shown(&opened, &mut cx), (EdgeMood::Floats, None), "a wish with no run is not the jellyfish's");
    // Its run, typed in as core keeps one that goes.
    let _run = held(&world.home);
    made["wishes"][0]["run"] = json!(RUN);
    fs::write(&file, format!("{made}\n")).unwrap();
    tick(&mut cx);
    assert_eq!(shown(&opened, &mut cx), (EdgeMood::Swims, None), "the page's wish is missing from the jellyfish");
}

#[test]
fn past_midnight_a_run_begun_on_the_page_s_day_is_still_shown_and_still_asks() {
    let dir = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let home = dir.path().join("jm");
    asking(&home, DAY);
    let (today, hand) = clock(DAY);
    let mut cx = context();
    let opened = opened(&mut cx, Some((page_of(&home, DAY), false, today)), None);
    let asks = (EdgeMood::Waves, Some("Which one?".to_string()));
    assert_eq!(shown(&opened, &mut cx), asks);
    let bubble = opened.bubble();
    *hand.lock().unwrap() = NEXT;
    tick(&mut cx);
    assert_eq!(shown(&opened, &mut cx), asks, "the run's question was dropped at midnight");
    assert_eq!(opened.bubble(), bubble, "the bubble was opened anew");
}

// When the page's day and today both have something to show, the first in core's order is shown (ADR 0019,
// decision 4: asks, working, waits, quiet), whichever of the two days it is of.
#[test]
fn of_the_page_s_day_and_today_the_first_in_core_s_order_is_shown() {
    type Typed = fn(&Path, &str);
    let going: Typed = |home, day| made(home, day, "working", json!({}));
    let nothing: Typed = |_, _| {};
    let cases: [(Typed, Typed, EdgeMood); 5] = [
        (asking, going, EdgeMood::Waves),
        (going, asking, EdgeMood::Waves),
        (going, nothing, EdgeMood::Swims),
        (nothing, going, EdgeMood::Swims),
        (nothing, nothing, EdgeMood::Floats),
    ];
    for (at, (on_the_page_s_day, today, drawn)) in cases.into_iter().enumerate() {
        let dir = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
        let home = dir.path().join("jm");
        on_the_page_s_day(&home, DAY);
        today(&home, NEXT);
        let _run = held(&home);
        let mut cx = context();
        let opened = opened(&mut cx, Some((page_of(&home, DAY), false, clock(NEXT).0)), None);
        assert_eq!(mood(&opened, &mut cx), drawn, "case {at}");
        assert_eq!(opened.bubble().is_some(), drawn == EdgeMood::Waves, "case {at}");
    }
    // Of two days in one state the page's day's is shown: the page beside the jellyfish is where it is answered.
    let dir = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let home = dir.path().join("jm");
    asking(&home, DAY);
    made(&home, NEXT, "needs_you", json!({"question": "Which other?"}));
    let mut cx = context();
    let opened = opened(&mut cx, Some((page_of(&home, DAY), false, clock(NEXT).0)), None);
    assert_eq!(shown(&opened, &mut cx), (EdgeMood::Waves, Some("Which one?".to_string())));
}
