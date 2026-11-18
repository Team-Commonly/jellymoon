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

/// The day the page was opened on, the day after it and the day before.
const DAY: &str = day::DAY;
const NEXT: &str = "2026-03-11";
const BEFORE: &str = "2026-03-09";
/// The run of the page's day, two runs of any other day, and the run of core's own example of a record, which
/// finished with one commit that waits.
const RUN: &str = "r1111111111111111";
const OTHER: &str = "r2222222222222222";
const THIRD: &str = "r3333333333333333";
const FINISHED: &str = "r0123456789abcdef";
/// Core's words for a run that asks, over the run's own question.
const ASKS: &str = "a run asks, and waits for your answer on the page";
/// What is seen of the jellyfish: its mood, the count on it (0 draws none) and the bubble's texts, none for no
/// bubble.
type Seen = (EdgeMood, usize, Vec<String>);

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

/// A file typed as core keeps one.
fn own(file: &Path, text: &str) {
    fs::create_dir_all(file.parent().unwrap()).unwrap();
    fs::write(file, text).unwrap();
    // Core reads a kept file only when its owner alone can write to it, whatever this process's umask is.
    fs::set_permissions(file, std::os::unix::fs::PermissionsExt::from_mode(0o644)).unwrap();
}

/// A made wish with a run, as core keeps it.
fn wish(at: usize, run: &str, state: &str, more: Value) -> Value {
    let mut wish =
        json!({"id": format!("m{at}"), "said": SAID, "project": "alpha", "state": state, "stars": [], "run": run});
    more.as_object().unwrap().iter().for_each(|(name, value)| wish[name] = value.clone());
    wish
}

/// The day's made wishes, typed as core keeps them.
fn typed(home: &Path, day: &str, wishes: &[Value]) {
    let file = home.join("pages").join(format!("{day}.wishes.json"));
    own(&file, &format!("{}\n", json!({"day": day, "wishes": wishes})));
}

/// A day's one made wish with a run: [`RUN`] on the page's day, which [`held`] holds, and on any other day
/// [`OTHER`], which nothing holds.
fn made(home: &Path, day: &str, state: &str, more: Value) {
    typed(home, day, &[wish(1, if day == DAY { RUN } else { OTHER }, state, more)]);
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

/// What a test types for a day.
type Typed = fn(&Path, &str);

fn nothing(_: &Path, _: &str) {}

fn going(home: &Path, day: &str) {
    made(home, day, "working", json!({}));
}

/// A run that asks: `Which one?` on the page's day, `Which other?` on any other.
fn asking(home: &Path, day: &str) {
    made(home, day, "needs_you", json!({"question": if day == DAY { "Which one?" } else { "Which other?" }}));
}

/// A run that finished with work that waits: the wish is done, and the run's record, core's own example of one,
/// says it completed with one commit that was neither kept nor tossed.
fn waiting(home: &Path, day: &str) {
    made(home, day, "done", json!({"run": FINISHED, "stars": ["finished"]}));
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("crates").join("jellymoon-core");
    let record = fs::read_to_string(fixtures.join("tests").join("fixtures").join("run-record.json")).unwrap();
    own(&home.join("runs").join(FINISHED).join("run.json"), &record);
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

/// What is seen of the jellyfish now. The count is read off the character, the texts off the bubble's window.
fn shown(opened: &Opened, cx: &mut TestAppContext) -> Seen {
    let texts = |bubble: gpui::WindowHandle<sticker_kit::draw::EdgeView>| {
        let said = |said: &sticker_kit::EdgeText| said.text.clone();
        bubble.update(cx, |view, _, _| view.edge().bubble.iter().map(said).collect::<Vec<_>>()).unwrap()
    };
    let said = opened.bubble().map(texts).unwrap_or_default();
    let (character, _) = opened.jelly().expect("the jellyfish is open");
    let count = character.update(cx, |view, _, _| view.edge().count).unwrap();
    (mood(opened, cx), count, said)
}

fn floats() -> Seen {
    (EdgeMood::Floats, 0, Vec::new())
}

fn swims() -> Seen {
    (EdgeMood::Swims, 0, Vec::new())
}

fn glows() -> Seen {
    (EdgeMood::Glows, 0, Vec::new())
}

/// The question of the page's day, under core's words for it.
fn asks() -> Seen {
    (EdgeMood::Waves, 0, vec![ASKS.to_string(), "Which one?".to_string()])
}

/// A home of the test's own, and the folder that holds it.
fn home() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let home = dir.path().join("jm");
    (dir, home)
}

#[test]
fn once_a_second_the_jellyfish_is_looked_at_off_the_window_s_thread_and_drawn_again() {
    let (_dir, home) = home();
    going(&home, DAY);
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
    waiting(&home, DAY);
    tick(&mut cx);
    assert_eq!((count(), shown(&opened, &mut cx)), (4, glows()));
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
    let (_dir, home) = home();
    going(&home, NEXT);
    let (today, hand) = clock(DAY);
    let mut cx = context();
    let opened = opened(&mut cx, Some((page_of(&home, DAY), false, today)), None);
    assert_eq!(shown(&opened, &mut cx), floats());
    *hand.lock().unwrap() = NEXT;
    tick(&mut cx);
    assert_eq!(shown(&opened, &mut cx), swims(), "still yesterday's");
    // A day named with `--date` is that day, whatever the clock says.
    let (today, _hand) = clock(NEXT);
    let mut cx = context();
    let named = self::opened(&mut cx, Some((page_of(&home, DAY), true, today)), None);
    tick(&mut cx);
    assert_eq!(shown(&named, &mut cx), floats(), "a named day showed another day's run");
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
    assert_eq!(shown(&opened, &mut cx), floats());
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
    assert_eq!(shown(&opened, &mut cx), floats(), "a wish with no run is not the jellyfish's");
    // Its run, typed in as core keeps one that goes.
    let _run = held(&world.home);
    made["wishes"][0]["run"] = json!(RUN);
    fs::write(&file, format!("{made}\n")).unwrap();
    tick(&mut cx);
    assert_eq!(shown(&opened, &mut cx), swims(), "the page's wish is missing from the jellyfish");
}

#[test]
fn past_midnight_a_run_begun_on_the_page_s_day_is_still_shown_and_still_asks() {
    let (_dir, home) = home();
    asking(&home, DAY);
    let (today, hand) = clock(DAY);
    let mut cx = context();
    let opened = opened(&mut cx, Some((page_of(&home, DAY), false, today)), None);
    assert_eq!(shown(&opened, &mut cx), asks());
    let bubble = opened.bubble();
    *hand.lock().unwrap() = NEXT;
    tick(&mut cx);
    assert_eq!(shown(&opened, &mut cx), asks(), "the run's question was dropped at midnight");
    assert_eq!(opened.bubble(), bubble, "the bubble was opened anew");
}

// The review of the jellyfish notes, finding 1: the page shows the day it was opened on and no other, so nothing
// of a later day is said to ask, or to wait, "on the page". Of that day only a run at work is shown, and the
// page's day comes first in core's order (ADR 0019, decision 4: asks, working, waits, quiet). Nothing holds the
// later day's run: the look writes nothing, and takes the made wish's word that it goes.
#[test]
fn from_a_later_day_only_a_run_at_work_is_taken_and_nothing_of_it_asks_or_waits_on_the_page() {
    // The page's day, the later day, and what is seen.
    let table: [(Typed, Typed, Seen); 16] = [
        (nothing, nothing, floats()),
        (nothing, going, swims()),
        (nothing, asking, floats()),
        (nothing, waiting, floats()),
        (going, nothing, swims()),
        (going, going, swims()),
        (going, asking, swims()),
        (going, waiting, swims()),
        (asking, nothing, asks()),
        (asking, going, asks()),
        (asking, asking, asks()),
        (asking, waiting, asks()),
        (waiting, nothing, glows()),
        (waiting, going, swims()),
        (waiting, asking, glows()),
        (waiting, waiting, glows()),
    ];
    for (at, (on_the_page_s_day, later, seen)) in table.into_iter().enumerate() {
        let (_dir, home) = home();
        later(&home, NEXT);
        on_the_page_s_day(&home, DAY);
        let _run = held(&home);
        let mut cx = context();
        let opened = opened(&mut cx, Some((page_of(&home, DAY), false, clock(NEXT).0)), None);
        assert_eq!(shown(&opened, &mut cx), seen, "row {at}, at the first look");
        tick(&mut cx);
        assert_eq!(shown(&opened, &mut cx), seen, "row {at}, a second later");
    }
}

// Core gives one state for a day, a question before a run at work. So a run at work on the later day, beside a
// question of that day, is not seen until the app is opened again: the page's day alone is shown.
#[test]
fn a_later_day_that_asks_and_works_at_once_shows_the_page_s_day_alone() {
    for (on_the_page_s_day, seen) in [(nothing as Typed, floats()), (waiting, glows())] {
        let (_dir, home) = home();
        let question = json!({"question": "Which other?"});
        typed(&home, NEXT, &[wish(1, OTHER, "needs_you", question), wish(2, THIRD, "working", json!({}))]);
        on_the_page_s_day(&home, DAY);
        let mut cx = context();
        let opened = opened(&mut cx, Some((page_of(&home, DAY), false, clock(NEXT).0)), None);
        assert_eq!(shown(&opened, &mut cx), seen);
    }
}

// Runs at work on both days: the page's day's are shown, and its count. Only when the page's day has none at
// work, and no question, are the later day's.
#[test]
fn of_runs_at_work_on_both_days_the_page_s_day_s_are_shown_with_their_count() {
    let two = |home: &Path| {
        typed(home, NEXT, &[wish(1, OTHER, "working", json!({})), wish(2, THIRD, "working", json!({}))]);
    };
    let (_dir, home) = home();
    two(&home);
    going(&home, DAY);
    let _run = held(&home);
    let mut cx = context();
    let opened = opened(&mut cx, Some((page_of(&home, DAY), false, clock(NEXT).0)), None);
    assert_eq!(shown(&opened, &mut cx), swims(), "one run goes on the page's day");
    // Work that waits on the page's day comes after a run at work: the later day's two are shown.
    let (_dir, home) = self::home();
    two(&home);
    waiting(&home, DAY);
    let mut cx = context();
    let opened = self::opened(&mut cx, Some((page_of(&home, DAY), false, clock(NEXT).0)), None);
    assert_eq!(shown(&opened, &mut cx), (EdgeMood::Swims, 2, Vec::new()));
}

// The review of the jellyfish notes, finding 3: a clock set back, or a move to a zone further west, gives a day
// before the page's. It is not read: only a day after the page's is. The days are ISO dates, compared as text.
#[test]
fn a_day_before_the_page_s_is_not_read_and_a_day_after_it_is() {
    let (_dir, home) = home();
    going(&home, BEFORE);
    going(&home, "2026-04-01");
    let (today, hand) = clock(BEFORE);
    let mut cx = context();
    let opened = opened(&mut cx, Some((page_of(&home, DAY), false, today)), None);
    assert_eq!(shown(&opened, &mut cx), floats(), "an earlier day's run is shown");
    *hand.lock().unwrap() = DAY;
    tick(&mut cx);
    assert_eq!(shown(&opened, &mut cx), floats());
    // A later month, with an earlier day of the month.
    *hand.lock().unwrap() = "2026-04-01";
    tick(&mut cx);
    assert_eq!(shown(&opened, &mut cx), swims(), "a later day's run is missing");
}
