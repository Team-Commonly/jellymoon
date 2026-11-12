//! A trial's two clocks and its pipes. The child ends itself at the limit whoever is or is not watching, so one
//! whose caller is gone lives at most the limit; the caller ends and waits for a child all the same. Neither side
//! reads more than it may, and a child that does not read or does not stop writing holds nobody.
//!
//! Every wait here has its own bound: a test of an end that does not come fails, and leaves no process.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::fs;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use sticker_kit::theme::MAX_ART_BYTES;
use support::{Folder, HONEST, OUTER, alone, trial, trial_as, wide_stroke};

/// The gate's slow picture at a size that draws for longer than every limit here, in this build: over ten
/// seconds. No run of it is left going past a few.
fn slow() -> String {
    wide_stroke(if cfg!(debug_assertions) { 40 } else { 250 })
}

/// How long after its limit a child may be seen to be gone, on a busy machine.
const MARGIN: Duration = Duration::from_secs(4);

#[test]
fn a_child_nobody_watches_ends_itself_at_its_limit() {
    let limit = Duration::from_millis(1500);
    let (status, said, lived) = alone(&["trial", "1500"], &slow(), limit + MARGIN, false);
    eprintln!("the slow picture, not watched: {status:?} after {lived:?}, saying {said:?}");
    let status = status.expect("the child was gone at its limit and a margin, with nobody ending it");
    // Its own clock ended it: it had no answer yet, and its code is the clock's.
    assert_eq!((status.code(), said.as_str()), (Some(124), ""), "after {lived:?}");
    assert!(lived >= limit, "and not before its limit: {lived:?}");

    // A limit it is not given, or one that is no number, is the kit's own; a longer one is no longer.
    let (status, said, _) = alone(&["trial"], HONEST, OUTER, false);
    assert!(status.is_some_and(|status| status.success()) && said.starts_with("accepted 26 356 400 "), "{said}");
    let (status, _, lived) = alone(&["trial", "0"], &slow(), MARGIN, false);
    assert_eq!(status.and_then(|status| status.code()), Some(124), "no time at all: {lived:?}");
}

// The gate's scene: the caller is ended a moment into a trial. The child holds a lock on a file for as long as it
// lives, and the lock is free within the limit.
#[test]
fn a_child_whose_caller_was_ended_mid_trial_is_gone_within_its_limit() {
    let folder = Folder::new("orphan");
    let _ = trial();
    fs::write(folder.0.join("star.svg"), slow()).unwrap();
    fs::write(folder.0.join("theme.toml"), "[art]\nstar = \"star.svg\"\n").unwrap();
    let (theme, lock) = (folder.0.join("theme.toml"), folder.0.join("held"));
    let limit = Duration::from_millis(1500);
    let mut caller = Command::new(env!("CARGO_BIN_EXE_trial-child"));
    caller.args(["parent", theme.to_str().unwrap(), lock.to_str().unwrap(), "1500"]);
    let mut caller = caller.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn().unwrap();
    let started = Instant::now();
    while !lock.with_extension("at").is_file() {
        assert!(started.elapsed() < OUTER, "the child never got as far as holding its lock");
        std::thread::sleep(Duration::from_millis(5));
    }
    let trying = Instant::now();
    caller.kill().unwrap();
    caller.wait().unwrap();
    let free = || fs::File::open(&lock).and_then(|held| Ok(held.try_lock()?)).is_ok();
    assert!(!free(), "the child lives on when its caller is ended: nobody ends it but itself");
    while !free() && trying.elapsed() < limit + MARGIN {
        std::thread::sleep(Duration::from_millis(20));
    }
    let gone = trying.elapsed();
    eprintln!("the orphan was gone {gone:?} after its caller");
    assert!(free(), "an orphan lives at most the limit: this one is not gone {gone:?} after its caller");
}

#[test]
fn a_child_that_writes_without_end_or_does_not_read_holds_its_caller_no_longer_than_the_limit() {
    let folder = Folder::new("pipes");
    assert_eq!(folder.star(HONEST, &trial()), Ok(HONEST.to_string()), "the helper has been started once");
    // The largest picture there may be. A pipe nobody reads holds 4 KiB of it on Windows, and all of it on macOS
    // and Linux: so it is on Windows that a caller writing it on its own thread is held here.
    let large = format!("{HONEST}{}", " ".repeat(MAX_ART_BYTES as usize - HONEST.len()));
    // All the wordy child says after the first kilobyte is never read: its pipe is shut on it, and it fails. A
    // caller that read all of it would have found eight megabytes that are no answer. The deaf child's picture
    // is written off the caller's thread, so the caller's clock runs.
    for (which, says) in
        [("wordy", "its trial ended badly ("), ("deaf", "its trial took longer than 1500 ms and was ended")]
    {
        let mut held = trial_as(&[which]);
        held.limit = Duration::from_millis(1500);
        let started = Instant::now();
        let said = folder.star(&large, &held).expect_err(which);
        let took = started.elapsed();
        assert!(said.starts_with(says), "{which}: {said}");
        assert!(took < held.limit + MARGIN, "{which}: the caller was held for {took:?}");
    }
    assert_eq!(folder.star(HONEST, &trial_as(&["twice"])), Err("its trial did not answer as a trial does".into()));
    // A child its own clock ended a moment before its caller's would have ran over, and did not end badly.
    let over = folder.star(HONEST, &trial_as(&["over"])).expect_err("over");
    assert!(over.starts_with("its trial took longer than ") && over.ends_with("is tried again at the next start"));
}

#[test]
fn a_child_reads_no_more_of_what_it_is_given_than_a_picture_may_be() {
    // Two bytes over, and the input left open: a child that read on would wait for its end.
    let long = format!("{HONEST}{}", " ".repeat(MAX_ART_BYTES as usize + 2 - HONEST.len()));
    let (status, said, lived) = alone(&["trial", "8000"], &long, Duration::from_secs(8) + MARGIN, true);
    assert_eq!(said, "refused: it is over 64 KiB\n", "{status:?} after {lived:?}");
    assert!(status.is_some_and(|status| status.success()) && lived < Duration::from_secs(8), "{status:?}, {lived:?}");
}

#[test]
fn the_trials_of_one_process_run_one_at_a_time() {
    let folder = Folder::new("alone");
    let lock = folder.0.join("lock");
    let one = trial_as(&["alone", lock.to_str().unwrap()]);
    let asked: Vec<_> = (0..3).map(|nth| (Folder::new(&format!("alone-{nth}")), one.clone())).collect();
    let tried: Vec<_> =
        asked.into_iter().map(|(of, trial)| std::thread::spawn(move || of.star(HONEST, &trial))).collect();
    for answer in tried {
        // A child that found the lock held by another would have failed.
        assert_eq!(answer.join().unwrap(), Ok(HONEST.to_string()));
    }
}
