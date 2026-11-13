//! A start that is not this run's (the tool of a last word listed twice or not at all, another session than the
//! one core chose), and a harness too old for the options F51 adds to a run's command line.

use serde_json::json;

use super::{Made, failed, never, printed, world};

const NOT_A_RUN: &str = "it did not start with the tools, servers and permission mode of a run";
const OTHER: &str = "it did not start under the session id it was given";

/// A run of a script that never ends by itself: what `jellymoon run` printed, once the agent is known to be gone.
fn stopped_at_once(script: &str, number: usize) -> (Made, serde_json::Value) {
    let w = world();
    w.play(script, number);
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let from = std::time::Instant::now();
    let said = printed(&w.start(&wish, &repo));
    // The agent sleeps a minute and the run's limit is thirty: it was ended at its first line.
    assert!(from.elapsed() < std::time::Duration::from_secs(25), "{script} {number} was waited for");
    assert!(w.freed("held"), "{script} {number}: the agent was left going");
    let run = said["run"].as_str().unwrap().to_string();
    (Made { w, repo, wish, run }, said)
}

#[test]
fn a_start_that_lists_the_tool_of_a_last_word_twice_or_not_at_all_is_not_a_run_s() {
    for number in [0, 1] {
        let (made, said) = stopped_at_once("tools", number);
        assert_eq!(said, failed(&made, NOT_A_RUN), "tools {number}");
    }
}

#[test]
fn a_start_under_another_session_than_the_one_given_is_ended_at_once_and_said_in_its_own_words() {
    for number in 0..5 {
        let (made, said) = stopped_at_once("session", number);
        assert_eq!(said, failed(&made, OTHER), "session {number}");
        assert_eq!(made.record()["session"], json!(made.w.args().1), "the record names the id that was given");
        assert_eq!(made.keeps("0123abcd-4567"), None, "the other id is kept nowhere");
        assert_eq!(printed(&made.toss())["tossed"], true, "a toss goes by the record's end, which is core's");
    }
    // A second start, under another session, after the measured one: the work it did is not committed.
    let (made, said) = super::played("session", 5);
    assert_eq!(said, failed(&made, OTHER));
    assert_eq!(made.w.commit(&made.repo, &made.branch()), made.w.commit(&made.repo, "main"));
}

#[test]
fn a_harness_that_does_not_know_an_option_of_a_run_is_too_old_and_nothing_is_started() {
    for (number, option) in [(0, "--json-schema"), (1, "--session-id")] {
        let w = world();
        w.play("old-option", number);
        let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
        let (before, person) = (w.kept(&wish), w.persons(&repo));
        let why = format!(
            "nothing was started: this Claude Code is too old for runs, as it does not know {option}; update Claude \
             Code and run the wish again"
        );
        let want = json!({"run": null, "wish": wish, "outcome": "not_started", "star": null, "denied": 0, "why": why});
        assert_eq!(printed(&w.start(&wish, &repo)), want);
        assert_eq!((w.kept(&wish), w.copies(), w.branches(&repo)), (never(before, &why), vec![], vec![]));
        assert!(person == w.persons(&repo));
    }
    // The control: an option that is none of a run's own is no sign of age. That run was started, and failed.
    let (made, said) = super::played("old-option", 2);
    assert_eq!(said, failed(&made, "Claude Code ended without a result"));
}
