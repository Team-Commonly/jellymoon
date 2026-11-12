//! The order of a stop: the interrupt, then the stop signal, then the kill. An agent deaf to the one, to the other
//! or to both is gone all the same, and each is told from the others by how long it took: the made-up agent sleeps
//! a minute. Also here: what is asked while a stop goes on, and an agent that moved a program of its own away.

use std::path::Path;
use std::time::{Duration, Instant};
use std::{fs, thread};

use serde_json::json;

use super::{Going, World, ended, printed, refused, said_cancelled, wait_for, world};

/// A run whose agent has made itself deaf, by `number`: to the stop signal (0), the interrupt (1) or both (2). 3
/// is deaf to the interrupt and leaves `heard` when the stop signal reaches it.
fn deaf(w: &World, number: usize, grace: u64, wish: &str, repo: &Path) -> Option<(Going, String)> {
    w.minded(("deaf", number), grace, wish, repo)
}

#[test]
fn an_agent_deaf_to_the_stop_signal_is_gone_at_the_interrupt_which_comes_first() {
    let w = world();
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    // A minute after each asking: had the stop signal come first, the agent would still be there after it.
    let Some((going, run)) = deaf(&w, 0, 60_000, &wish, &repo) else { return };
    let from = Instant::now();
    assert_eq!(printed(&w.stop(&wish)), said_cancelled(&run, &wish, 0));
    assert!(from.elapsed() < Duration::from_secs(40), "{:?}", from.elapsed());
    assert_eq!(printed(&ended(going)), said_cancelled(&run, &wish, 0));
    assert!(w.agent_came_and_went());
}

#[test]
fn an_agent_deaf_to_the_interrupt_is_given_its_time_and_is_gone_at_the_stop_signal() {
    let w = world();
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let Some((going, run)) = deaf(&w, 3, 2000, &wish, &repo) else { return };
    let from = Instant::now();
    assert_eq!(printed(&w.stop(&wish)), said_cancelled(&run, &wish, 0));
    let took = from.elapsed();
    assert!(took >= Duration::from_secs(2), "it was not given its time after the interrupt: {took:?}");
    // It was the stop signal that ended it, and not the kill, which comes a second wait later: the agent says
    // that the signal reached it, which no kill lets it say. No clock is asked: a busy machine is slow to say.
    assert!(w.rec("heard").is_some(), "the stop signal never reached the agent: only the kill ended it");
    assert_eq!(printed(&ended(going)), said_cancelled(&run, &wish, 0));
    assert!(w.agent_came_and_went());
}

// While the stop goes on: a second stop is the same stop, and the wish is not run again.
#[test]
fn an_agent_deaf_to_the_interrupt_and_to_the_signal_is_killed_and_two_stops_are_one() {
    let w = world();
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let Some((going, run)) = deaf(&w, 2, 1500, &wish, &repo) else { return };
    let from = Instant::now();
    let (one, two, again) = thread::scope(|all| {
        let (one, two) = (all.spawn(|| w.stop(&wish)), all.spawn(|| w.stop(&wish)));
        thread::sleep(Duration::from_millis(500));
        let again = w.start(&wish, &repo);
        (one.join().unwrap(), two.join().unwrap(), again)
    });
    let took = from.elapsed();
    assert_eq!(refused(&again), format!("jellymoon: wish {wish} has had its run, {run}: a made wish gets one run\n"));
    for stopped in [one, two, ended(going)] {
        assert_eq!(printed(&stopped), said_cancelled(&run, &wish, 0));
    }
    // It was interrupted, given its time, signalled, given its time again, and then killed.
    assert!(took >= Duration::from_secs(3), "{took:?}");
    assert!(w.agent_came_and_went(), "the deaf agent is still going");
    w.is_cancelled(&wish, &run, &repo, 0);
    assert_eq!((w.copies(), w.starts()), (vec![run], 1));
}

// A program the agent started in another process group, with the agent's output open, is not reached by a stop
// (`docs/development/run.md`, "Not covered"). The stop does not wait for it: the run is cancelled all the same.
#[test]
fn a_stop_does_not_wait_for_a_program_the_agent_moved_away() {
    let w = world();
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    w.play("holds", 2);
    let going = w.start_going(&wish, &repo);
    wait_for(&w.at("rec").join("started"));
    let run = w.kept(&wish)["run"].as_str().unwrap().to_string();
    assert_eq!(printed(&w.stop(&wish)), said_cancelled(&run, &wish, 0));
    assert_eq!(printed(&ended(going)), said_cancelled(&run, &wish, 0));
    assert!(w.agent_came_and_went(), "the agent itself is still going");
    w.is_cancelled(&wish, &run, &repo, 0);
    // What it moved away goes on until it is told to end, which the test does here.
    assert_eq!(w.record(&run)["outcome"], json!("cancelled"));
    fs::write(w.at("rec").join("stop"), "").unwrap();
    assert!(w.freed("litter-held"), "what the agent left never ended");
}
