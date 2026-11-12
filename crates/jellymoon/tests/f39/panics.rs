//! A test that panics leaves nothing going (`guard.rs`). The test that panics is run here as a test process of
//! its own: this program started again for that one test, with the folder its world is to be made in. Run with
//! the others it does nothing.

use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};
use std::{env, fs, thread};

use super::{DAY, alive, wait_for, world_in};
use crate::support::scratch;

/// Names the folder the panicking test makes its world in. Only the test that starts that one sets it.
const INSIDE: &str = "JELLYMOON_TEST_PANICS_IN";
const PANICS: &str = "panics::a_test_panics_with_an_agent_that_never_ends_and_a_command_that_waits";

/// Whether the made-up agent, the program it left, or a `jellymoon run` of a world's folder still lives. The
/// runs are asked here by their start locks, and not only through the guard's own `alive`.
fn going(world: &Path) -> bool {
    let free = |held: &Path| fs::File::open(held).is_ok_and(|held| held.try_lock().is_ok());
    let runs = fs::read_dir(world.join("jm").join("runs")).unwrap().map(|run| run.unwrap().path().join("start.lock"));
    let run = runs.filter(|lock| lock.is_file()).any(|lock| !free(&lock));
    alive(world) || run || !free(&world.join("rec").join("held")) || !free(&world.join("rec").join("litter-held"))
}

#[test]
fn a_test_panics_with_an_agent_that_never_ends_and_a_command_that_waits() {
    let Some(inside) = env::var_os(INSIDE) else { return };
    // Its folder is kept when the world is dropped, for the test that started this one to look in.
    let w = world_in(tempfile::Builder::new().disable_cleanup(true).tempdir_in(inside).unwrap());
    let (repo, wish, second) = (w.repo("alpha"), w.wish("alpha"), w.wish("alpha"));
    // The agent leaves a program of its own in another process group, and then neither ends by itself.
    w.play("holds", 2);
    // Its command is handed over, so nothing kills it when the test panics: it ends once its agent has, two
    // seconds after it (it is held that long before it writes its end), and only the world's wait covers that.
    let ends = [("JELLYMOON_TEST_NOTE_WAIT_MS", "2000".into())];
    drop(w.going(&ends, &["run", "--wish", &wish, "--date", DAY, "--repo", repo.to_str().unwrap()]).take());
    wait_for(&w.at("rec").join("started"));
    // A second command waits a minute with its copy made and no agent started: only a kill ends it sooner.
    let waits = [("JELLYMOON_TEST_CLAIM_WAIT_MS", "60000".into())];
    let _waiting = w.going(&waits, &["run", "--wish", &second, "--date", DAY, "--repo", repo.to_str().unwrap()]);
    let from = Instant::now();
    while w.copies().len() < 2 {
        assert!(from.elapsed() < Duration::from_secs(30), "the second command never made its copy");
        thread::sleep(Duration::from_millis(10));
    }
    assert!(going(w.dir.path()), "nothing was going when the test was to panic");
    panic!("as this test is for");
}

#[test]
fn a_test_that_panics_leaves_no_agent_and_no_command_going() {
    let dir = scratch();
    let mut test = Command::new(env::current_exe().unwrap());
    // Every wait of that test has its own end, and so has the wait of the world it drops.
    let ran = test.args(["--exact", PANICS, "--test-threads", "1"]).env(INSIDE, dir.path()).output().unwrap();
    let said = String::from_utf8_lossy(&ran.stdout);
    assert!(!ran.status.success() && said.contains("as this test is for"), "the test did not panic: {said}");
    let worlds: Vec<_> = fs::read_dir(dir.path()).unwrap().map(|entry| entry.unwrap().path()).collect();
    assert_eq!(worlds.len(), 1, "{worlds:?}");
    let rec = worlds[0].join("rec");
    assert!(rec.join("held").is_file() && rec.join("litter-held").is_file(), "no agent was started: {said}");
    assert!(!going(&worlds[0]), "a test that panicked left an agent or a jellymoon run going");
}
