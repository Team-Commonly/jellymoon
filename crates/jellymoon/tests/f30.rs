//! F30: holding a key and saying a wish starts an agent on it. Its three steps, for a wish that is entered through
//! `jellymoon hear` (F55, ADR 0019): the wish is on the day's page at once, in the person's words, in the working
//! state; with `--run` a run is started with the person's harness, and the wish's star appears when the run
//! reports it finished; and with no harness the wish stays on the page and the line says why nothing started.
//! No key is pressed and no sound is heard here: the events are typed, and the harness is the made-up
//! `fake-agent` in F39's test folder (`f39/world.rs`). What is not proved here is the key and the ear
//! themselves, which F56 to F60 add in the app: F30 is set to pass after them, and after a person held the key.
#![allow(clippy::unwrap_used, clippy::expect_used)]

#[path = "f55/heard.rs"]
mod heard;
mod support;
#[path = "f39/world.rs"]
mod world;

use std::fs;

pub use heard::*;
use serde_json::json;
pub use world::*;

/// A world whose project `alpha` is a repo something was typed in.
fn repo_world() -> World {
    let w = heard_world();
    let repo = w.repo("alpha");
    w.typed_at("-work-alpha", &repo);
    w
}

/// A hold in which the wish is said, and the ticks that end its four seconds.
fn said_and_waited() -> Vec<String> {
    let mut events = hold(0, SAID);
    events.push(tick(5000));
    events
}

#[test]
fn a_wish_said_aloud_is_on_the_day_s_page_at_once_in_the_person_s_words_and_working() {
    let w = repo_world();
    let printed = w.hear(&[], &said_and_waited());
    assert_eq!(printed.last(), Some(&with(quiet(), "made", json!("m1"))));
    let on_page = w.on_page("m1");
    assert_eq!(
        (&on_page["said"], &on_page["state"], &on_page["project"]),
        (&json!(SAID), &json!("working"), &json!("alpha"))
    );
    assert_eq!(w.starts(), 0, "with no --run nothing is started");
}

#[test]
fn its_run_is_started_with_the_person_s_harness_and_the_star_appears_when_the_run_reports_it_finished() {
    let w = repo_world();
    let printed = w.hear(&["--run"], &said_and_waited());
    assert_eq!(printed.last().unwrap()["made"], "m1");
    let run = w.run_ended("m1");
    assert_eq!((w.starts(), w.record(&run)["outcome"].as_str()), (1, Some("completed")));
    let on_page = w.on_page("m1");
    assert_eq!(
        (&on_page["said"], &on_page["state"], &on_page["stars"]),
        (&json!(SAID), &json!("done"), &json!([STAR]))
    );
}

#[test]
fn a_wish_said_aloud_while_no_harness_is_installed_stays_on_the_page_and_the_line_says_why_nothing_started() {
    let mut w = repo_world();
    fs::create_dir(w.at("empty")).unwrap();
    w.vars.push(("PATH", w.at("empty").into()));
    let why = "the wish is kept, but no run was started: no `claude` program was found (JELLYMOON_CLAUDE names \
               one)";
    let printed = w.hear(&["--run"], &said_and_waited());
    assert_eq!(printed.last(), Some(&with(cannot(why), "made", json!("m1"))));
    let on_page = w.on_page("m1");
    assert_eq!((&on_page["said"], &on_page["state"]), (&json!(SAID), &json!("working")));
    assert!(w.starts() == 0 && !w.jm("copies").exists());
}
