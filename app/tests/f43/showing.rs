//! The window's acts with no window: `Showing` is what `window.rs` calls, and here the same calls are made in the
//! same order, the work between them run on this thread. No run is asked for here: an asking is never run.

use jellymoon_app::showing::{Showing, entry_line, other_theme, switches_theme, tap_line};
use jellymoon_app::stopping::{STOPPING, UNSTOPPED};
use jellymoon_app::today::{Next, waiting};
use jellymoon_app::wishing::{UNKEPT, UNSTARTED};
use sticker_kit::{Scene, State};

use super::{drawn, first, planted, wished_alone};
use crate::support::day::DAY;
use crate::support::wished::SAID;

fn title(scene: &Scene) -> Option<&str> {
    first(scene).map(|(title, _)| title)
}

#[test]
fn a_page_with_no_day_takes_every_act_as_nothing_and_still_answers_its_entry() {
    let none = Showing::of(None);
    assert!(none.offering().is_none() && none.look().is_none() && none.poked("m1").is_none());
    assert!(none.reading(Box::new(|| None)).is_none() && none.wishing(SAID, Some("alpha")).is_none());
    assert!(none.chose("stop-m1", 0).is_none() && none.chose("clash-1", 0).is_none());
    assert!(none.asked(Some((UNSTARTED, String::new()))).is_none());
    let entered = none.answered(None);
    assert!(!entered.kept && entered.scene.is_none() && entered.asking.is_none(), "the words stay, nothing is shown");
}

#[test]
fn enter_is_answered_kept_only_when_core_kept_the_wish_and_only_then_is_its_run_asked_for() {
    let (_world, on) = wished_alone();
    let showing = Showing::of(Some(on));
    // Words core refuses: the entry keeps them, the page says why, and no run is asked for.
    let refused = showing.answered(Some(showing.wishing(&"w".repeat(201), Some("alpha")).unwrap()()));
    assert!(!refused.kept && refused.asking.is_none());
    assert_eq!(refused.scene.as_ref().and_then(title), Some(UNKEPT));
    // Core was never asked: the entry is answered all the same, and its words stay.
    let silent = showing.answered(None);
    assert!(!silent.kept && silent.scene.is_none() && silent.asking.is_none());

    let kept = showing.answered(Some(showing.wishing(SAID, Some("alpha")).unwrap()()));
    assert!(kept.kept && kept.asking.is_some(), "the entry empties, and the wish's run is asked for next");
    let scene = kept.scene.unwrap();
    assert_eq!((drawn(&scene, "m2").map(|drawn| drawn.0), title(&scene)), (Some(State::Busy), None));
    // What the asking has to say is shown; a run that started says nothing.
    let said = showing.asked(Some((UNSTARTED, "why".to_string()))).unwrap();
    assert_eq!(first(&said), Some((UNSTARTED, "why")));
    assert!(showing.asked(None).is_none());
}

#[test]
fn a_tap_on_an_option_shows_its_scene_at_once_and_only_a_choice_to_stop_leaves_work() {
    let (_world, mut on) = wished_alone();
    on.scene.notes.insert(0, planted("m1"));
    let showing = Showing::of(Some(on));
    assert!(showing.chose("stop-m2", 0).is_none(), "a choice to stop that is not on show is nothing");
    let (at_once, stop) = showing.chose("stop-m1", 0).unwrap();
    assert_eq!(title(&at_once), Some(STOPPING));
    // The wish has no run, so core refuses the stop: the page says so.
    let after = showing.stopped(stop.expect("the stop is work for another thread")()).unwrap();
    assert_eq!(title(&after), Some(UNSTOPPED));
    assert!(showing.chose("stop-m1", 0).is_none(), "a second tap on the choice is nothing");
    // Any other option is a tap for core: its scene, and no work after it.
    let (tapped, stop) = showing.chose("clash-1", 0).unwrap();
    assert!(stop.is_none() && !tapped.notes.is_empty());
}

#[test]
fn a_look_and_a_step_of_the_reading_show_what_they_give_and_a_look_of_a_scene_that_is_gone_is_dropped() {
    let (world, on) = wished_alone();
    let showing = Showing::of(Some(on));
    let chips = showing.offering().unwrap()().expect("alpha was typed in today");
    assert!(chips.options.contains(&"alpha".to_string()), "{chips:?}");
    assert!(showing.look().unwrap()().is_none(), "what is shown is what is kept");
    assert!(showing.poked("b").unwrap()().is_none(), "a wish of the reading has no run to stop");

    // A wish made in a terminal: a look shows it, and a look made before the scene changed is dropped.
    jellymoon_core::wish(&world.home, Some(DAY), "another wish", "alpha").unwrap();
    let (stale, fresh) = (showing.look().unwrap()().unwrap(), showing.look().unwrap()().unwrap());
    let scene = showing.seen(fresh).unwrap();
    assert_eq!(drawn(&scene, "m2").map(|drawn| drawn.0), Some(State::Busy));
    assert!(showing.seen(stale).is_none());

    // A step of the reading that gives nothing ends it; one that gives a scene shows that scene.
    assert!(showing.reading(Box::new(|| None)).unwrap()().is_none());
    let step = Box::new(|| Some(Next { scene: waiting(DAY), page: None, later: None }));
    let (next, entry) = showing.reading(step).unwrap()().unwrap();
    assert_eq!((showing.read(&next), entry), (Some(next.scene.clone()), None), "a day with no page has no entry");
}

#[test]
fn t_switches_the_theme_unless_the_entry_has_the_keys_and_the_window_s_lines_say_no_word_of_the_page() {
    assert!(switches_theme("t", false));
    assert!(!switches_theme("t", true), "in the entry it is a letter of a wish");
    assert!(!switches_theme("x", false) && !switches_theme("T", false) && !switches_theme("", false));
    let others = ["sticker", "plain", "mine"].map(|name| other_theme(name).name);
    assert_eq!(others, ["plain", "sticker", "plain"]);
    assert_eq!(tap_line("m1", None), "tap id=m1");
    assert_eq!(tap_line("stop-m1", Some(0)), "tap id=stop-m1 option=0");
    assert_eq!(entry_line("wish", "tea at 4 ☕"), "entry id=wish characters=10");
}
