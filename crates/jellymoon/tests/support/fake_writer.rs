//! A made-up session that goes on typing, for the test that a reading ends on a log that grows (F43's second gate).
//! It is given the log's path and a number of lines. It writes that many lines typed two days before the test's
//! day, then one line typed on the day every millisecond. Every line is as long as every other, so a test counts
//! them by the file's length. It caps itself: it ends after 20 seconds or at 20 MB, whichever is first, and the
//! test ends it sooner.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use std::{env, thread};

const LONGEST: Duration = Duration::from_secs(20);
const MOST: usize = 20 << 20;

fn line(day: u8, nth: usize) -> String {
    format!(
        r#"{{"type":"user","timestamp":"2026-03-{day:02}T16:00:00.000Z","cwd":"/work/alpha","origin":{{"kind":"human"}},"message":{{"content":"line {nth:07} of a log that grows"}}}}"#
    ) + "\n"
}

fn main() {
    let mut args = env::args_os().skip(1);
    let path = PathBuf::from(args.next().expect("the log's path"));
    let before: usize = args.next().and_then(|count| count.to_str()?.parse().ok()).expect("a number of lines");
    let started = Instant::now();
    let mut log = OpenOptions::new().create(true).append(true).open(path).unwrap();
    let earlier: String = (0..before).map(|nth| line(8, nth)).collect();
    assert!(earlier.len() < MOST / 2, "the lines before the day are over half the cap");
    log.write_all(earlier.as_bytes()).unwrap();
    let mut written = earlier.len();
    for nth in 0.. {
        let typed = line(10, nth);
        if started.elapsed() >= LONGEST || written + typed.len() > MOST {
            return;
        }
        log.write_all(typed.as_bytes()).unwrap();
        written += typed.len();
        thread::sleep(Duration::from_millis(1));
    }
}
