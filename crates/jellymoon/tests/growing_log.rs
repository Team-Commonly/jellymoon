//! Where a log's reading begins and ends. It is read no further than where it ended when it was opened: a made-up
//! session (`support/fake_writer.rs`) goes on typing into a log while `jellymoon day` reads it, and the day comes
//! back while it types. Before, the reader went on until one read gave nothing, so a log written as fast as it was
//! read had no end. No step here waits on the clock: each waits for a file the session leaves. And a byte-order
//! mark at a log's start does not cost it its first line.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::ffi::OsStr;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};
use std::{fs, thread};

use serde_json::Value;
use support::{A, ZONE, jellymoon, said, scratch, write_log};

/// The made-up session. It is ended before its folder is removed, whatever the test's end.
struct Writer(Child);

impl Drop for Writer {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Waits for a file the session leaves and gives what it holds. The limit is past the session's own, so it only
/// ends a test whose session is gone.
fn left(file: &Path) -> String {
    let limit = Instant::now() + Duration::from_secs(90);
    loop {
        match fs::read_to_string(file) {
            Ok(held) => return held,
            Err(_) if Instant::now() < limit => thread::sleep(Duration::from_millis(5)),
            Err(e) => panic!("the made-up session never left {}: {e}", file.display()),
        }
    }
}

/// What `jellymoon day` gives of the logs: each turn's words.
fn day(logs: &Path) -> Vec<String> {
    let args = ["day", "--date", "2026-03-10", "--logs", logs.to_str().unwrap()];
    let ran = jellymoon(&[("TZ", OsStr::new(ZONE))], &args);
    assert_eq!((ran.code, ran.stderr.as_str()), (0, ""));
    let day: Value = serde_json::from_str(&ran.stdout).expect("stdout is the day as JSON");
    day["turns"].as_array().unwrap().iter().map(|turn| turn["said"].as_str().unwrap().to_string()).collect()
}

fn typed(nth: usize) -> String {
    format!("line {nth:07} of a log that grows")
}

#[test]
fn a_day_comes_back_while_a_session_goes_on_typing_and_a_log_at_rest_is_read_to_its_last_byte() {
    let dir = scratch();
    let (logs, signals) = (dir.path().join("logs"), dir.path().join("signals"));
    let log = logs.join("-work-alpha").join("s.jsonl");
    fs::create_dir_all(log.parent().unwrap()).unwrap();
    fs::create_dir(&signals).unwrap();
    let mut writer = Command::new(env!("CARGO_BIN_EXE_fake-writer"));
    writer.arg(&log).arg(&signals).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    let _writer = Writer(writer.spawn().unwrap());
    left(&signals.join("ready"));

    // The session types faster than a log is read, so a reader that follows a log as it grows is still reading
    // when the session reaches its cap, or when the command is stopped at its own limit.
    let while_typing = day(&logs);
    fs::write(signals.join("stop"), "").unwrap();
    assert_eq!(left(&signals.join("done")), "stopped", "the day came back only when the session typed no more");
    let there: Vec<String> = (0..while_typing.len()).map(typed).collect();
    assert_eq!(while_typing, there, "the turns that were there, in their order");
    assert!(while_typing.len() >= 5, "{} of the five turns typed before the day was asked for", while_typing.len());

    // The session is gone and the log is at rest: every line is read, the last one too, which ends with no line
    // break at the log's last byte and was typed after the first day came back.
    let lines = fs::read(&log).unwrap().split(|byte| *byte == b'\n').count();
    let at_rest = day(&logs);
    assert_eq!((at_rest.len(), at_rest.last().map(String::as_str)), (lines, Some("the last line of the log")));
    assert!(while_typing.len() < lines);
}

#[test]
fn a_byte_order_mark_at_a_log_s_start_does_not_cost_it_its_first_line() {
    let dir = scratch();
    let first = format!("\u{feff}{}", said("2026-03-10T16:00:00.000Z", A, "the first line"));
    // A mark anywhere else is not the start of a log: that line is not valid JSON, as before.
    let marked = format!("\u{feff}{}", said("2026-03-10T16:00:02.000Z", A, "a line after a mark"));
    write_log(dir.path(), "-work-alpha/s.jsonl", &[first, said("2026-03-10T16:00:01.000Z", A, "the second"), marked]);
    assert_eq!(day(dir.path()), ["the first line", "the second"]);
}
