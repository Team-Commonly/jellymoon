//! A log is read no further than where it ended when it was opened (the second gate on F43, finding 3). A made-up
//! session (`support/fake_writer.rs`) goes on typing into a log while `jellymoon day` reads it: the day comes back
//! with the turns that were there. Before, the reader went on until one read gave nothing, so a log written as
//! fast as it was read had no end.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::ffi::OsStr;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};
use std::{fs, thread};

use serde_json::Value;
use support::{ZONE, jellymoon, scratch};

/// The lines typed before the day: each is decoded and dropped, so reading them takes the reader a while, and
/// the writer types many lines of the day in that while.
const EARLIER: usize = 40_000;

/// The made-up session. It is ended before its folder is removed, whatever the test's end.
struct Writer(Child);

impl Drop for Writer {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// How many lines of the day are in the log now. Every line is as long as the first.
fn typed(log: &Path) -> usize {
    let Ok(bytes) = fs::read(log) else { return 0 };
    let Some(width) = bytes.iter().position(|byte| *byte == b'\n').map(|end| end + 1) else { return 0 };
    (bytes.len() / width).saturating_sub(EARLIER)
}

#[test]
fn a_day_is_read_to_where_a_log_ended_when_it_was_opened_while_a_session_goes_on_typing() {
    let dir = scratch();
    let logs = dir.path().join("logs");
    let log = logs.join("-work-alpha").join("s.jsonl");
    fs::create_dir_all(log.parent().unwrap()).unwrap();
    let mut writer = Command::new(env!("CARGO_BIN_EXE_fake-writer"));
    writer.arg(&log).arg(EARLIER.to_string()).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    let _writer = Writer(writer.spawn().unwrap());
    let deadline = Instant::now() + Duration::from_secs(15);
    while typed(&log) < 5 {
        assert!(Instant::now() < deadline, "the made-up session typed nothing of the day");
        thread::sleep(Duration::from_millis(5));
    }

    let before = typed(&log);
    let args = ["day", "--date", "2026-03-10", "--logs", logs.to_str().unwrap()];
    let ran = jellymoon(&[("TZ", OsStr::new(ZONE))], &args);
    let after = typed(&log);
    assert_eq!((ran.code, ran.stderr.as_str()), (0, ""));
    let day: Value = serde_json::from_str(&ran.stdout).expect("stdout is the day as JSON");
    let said: Vec<&str> = day["turns"].as_array().unwrap().iter().map(|turn| turn["said"].as_str().unwrap()).collect();
    let there: Vec<String> = (0..said.len()).map(|nth| format!("line {nth:07} of a log that grows")).collect();
    assert_eq!(said, there, "the turns that were there, in their order");
    assert!(said.len() >= before, "{} of the {before} turns typed before the day was asked for", said.len());
    // The log was opened soon after the command started, and the command ended only after the lines before the
    // day were read. A reader that went on to the log's end as it grew gives nearly every turn typed by then.
    let middle = before + (after - before) / 2;
    assert!(
        said.len() <= middle,
        "{} turns: {before} were typed when the day was asked for and {after} when it came back",
        said.len()
    );
}
