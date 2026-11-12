//! A made-up session that goes on typing, for the test that a reading ends on a log that grows. It is given the
//! log's path and a folder for its signals. It types five lines and leaves the file `ready`. Then it types on, ten
//! megabytes a second, until the file `stop` is there. It ends with one last line that has no line break, and
//! leaves the file `done`, which says `stopped`. It caps itself: at 60 seconds or 200 MB it ends the same way
//! unasked, and `done` says `capped`. Every line is typed on the test's day.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use std::{env, thread};

const LONGEST: Duration = Duration::from_secs(60);
const MOST: usize = 200 << 20;
const A_SECOND: usize = 10 << 20;

fn line(said: &str) -> String {
    format!(
        r#"{{"type":"user","timestamp":"2026-03-10T16:00:00.000Z","cwd":"/work/alpha","origin":{{"kind":"human"}},"message":{{"content":"{said}"}}}}"#
    )
}

fn main() {
    let mut args = env::args_os().skip(1).map(PathBuf::from);
    let (path, signals) = (args.next().expect("the log's path"), args.next().expect("a folder for the signals"));
    let started = Instant::now();
    let mut log = OpenOptions::new().create(true).append(true).open(path).unwrap();
    let (mut typed, mut written) = (0, 0);
    let mut lines = |until: usize| {
        let mut lines = String::new();
        while typed < 5 || written + lines.len() < until {
            lines += &(line(&format!("line {typed:07} of a log that grows")) + "\n");
            typed += 1;
        }
        written += lines.len();
        lines
    };
    log.write_all(lines(0).as_bytes()).unwrap();
    fs::write(signals.join("ready"), "").unwrap();
    let why = loop {
        if signals.join("stop").exists() {
            break "stopped";
        }
        let allowed = (started.elapsed().as_millis() as usize).saturating_mul(A_SECOND / 1000);
        if started.elapsed() >= LONGEST || allowed >= MOST {
            break "capped";
        }
        log.write_all(lines(allowed).as_bytes()).unwrap();
        thread::sleep(Duration::from_millis(2));
    };
    log.write_all(line("the last line of the log").as_bytes()).unwrap();
    fs::write(signals.join("done"), why).unwrap();
}
