//! Where a log's reading begins and ends. It is read no further than where it ended when it was opened. The test
//! holds `jellymoon day` where the reader has asked the log's length and read nothing (`busy::pause`, a seam only a
//! test's build has), adds lines to the log, and lets the command go: the day holds what was there at the open and
//! nothing added after it. Nothing here is a race: the test waits for the file the held command leaves, and the
//! command for the test's. A log at rest is read to its last byte, and a byte-order mark at a log's start does not
//! cost it its first line.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::ffi::OsStr;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

use serde_json::Value;
use support::{A, LIMIT, ZONE, jellymoon, said, scratch, write_log};

/// What `jellymoon day` gives of the logs: each turn's words.
fn day_with(vars: &[(&str, &OsStr)], logs: &Path) -> Vec<String> {
    let vars = [&[("TZ", OsStr::new(ZONE))], vars].concat();
    let ran = jellymoon(&vars, &["day", "--date", "2026-03-10", "--logs", logs.to_str().unwrap()]);
    assert_eq!((ran.code, ran.stderr.as_str()), (0, ""));
    let day: Value = serde_json::from_str(&ran.stdout).expect("stdout is the day as JSON");
    day["turns"].as_array().unwrap().iter().map(|turn| turn["said"].as_str().unwrap().to_string()).collect()
}

fn day(logs: &Path) -> Vec<String> {
    day_with(&[], logs)
}

fn typed(nth: usize) -> String {
    said("2026-03-10T16:00:00.000Z", A, &format!("line {nth} of a log that grows"))
}

/// The day read from a log that holds `at_open` when the reader opens it and has `added` put after that before
/// the reader reads a byte. The command waits a minute at most for the test, so it ends whatever the test's end.
fn day_of_a_log_that_grows(at_open: &str, added: &str) -> Vec<String> {
    let dir = scratch();
    let (logs, go) = (dir.path().join("logs"), dir.path().join("go"));
    let log = logs.join("-work-alpha").join("s.jsonl");
    fs::create_dir_all(log.parent().unwrap()).unwrap();
    fs::write(&log, at_open).unwrap();
    let held = [("JELLYMOON_TEST_OPEN_WAIT_MS", OsStr::new("60000")), ("JELLYMOON_TEST_GO", go.as_os_str())];
    thread::scope(|scope| {
        let reading = scope.spawn(|| day_with(&held, &logs));
        // The command says where it is held: the log is open, its length asked.
        let (at, limit) = (go.with_extension("at"), Instant::now() + LIMIT);
        while !at.exists() && !reading.is_finished() && Instant::now() < limit {
            thread::sleep(Duration::from_millis(5));
        }
        let came = at.exists();
        if came {
            let mut log = OpenOptions::new().append(true).open(&log).unwrap();
            log.write_all(added.as_bytes()).unwrap();
            log.sync_all().unwrap();
        }
        // A file that is there whole from its first moment: written under another name, then renamed.
        fs::write(go.with_extension("tmp"), "").unwrap();
        fs::rename(go.with_extension("tmp"), &go).unwrap();
        let turns = reading.join().unwrap();
        assert!(came, "the command was never held with the log open, so nothing was added to it: {turns:?}");
        turns
    })
}

#[test]
fn a_day_holds_what_a_log_held_when_it_was_opened_and_nothing_typed_into_it_after() {
    let lines = |nths: std::ops::Range<usize>| nths.map(typed).collect::<Vec<_>>();
    let words =
        |nths: std::ops::Range<usize>| nths.map(|n| format!("line {n} of a log that grows")).collect::<Vec<_>>();
    // Whole lines, the last one with no line break yet: it is kept, and no line typed after the open is.
    let at_open = lines(0..5).join("\n");
    let added = format!("\n{}\n", lines(5..9).join("\n"));
    assert_eq!(day_of_a_log_that_grows(&at_open, &added), words(0..5));
    // A line half written at the open is not a line, though the session finishes it a moment later; and what is
    // typed after the open ends in a half-written line too.
    let sixth = typed(5);
    let (half, rest) = sixth.split_at(sixth.len() / 2);
    let at_open = format!("{}\n{half}", lines(0..5).join("\n"));
    let added = format!("{rest}\n{}\n{}", lines(6..9).join("\n"), &typed(9)[..40]);
    assert_eq!(day_of_a_log_that_grows(&at_open, &added), words(0..5));
    // Nothing at the open: nothing, whatever is typed after.
    assert_eq!(day_of_a_log_that_grows("", &added), Vec::<String>::new());
}

// No session and no seam: a small log that nothing writes to.
#[test]
fn a_log_at_rest_is_read_to_its_last_byte() {
    let dir = scratch();
    let log = dir.path().join("-work-alpha").join("s.jsonl");
    fs::create_dir_all(log.parent().unwrap()).unwrap();
    fs::write(&log, format!("{}\n{}", typed(0), said("2026-03-10T16:00:01.000Z", A, "the last line of the log")))
        .unwrap();
    assert_eq!(day(dir.path()), ["line 0 of a log that grows", "the last line of the log"], "no line break ends it");
}

#[test]
fn a_byte_order_mark_at_a_log_s_start_does_not_cost_it_its_first_line() {
    let at = |second: u8| format!("2026-03-10T16:00:0{second}.000Z");
    let marked = |second: u8, text: &str| format!("\u{feff}{}", said(&at(second), A, text));
    let dir = scratch();
    // A mark anywhere else is not the start of a log: that line is not valid JSON, as before.
    let lines = [marked(0, "the first line"), said(&at(1), A, "the second"), marked(2, "a line after a mark")];
    write_log(dir.path(), "-work-alpha/s.jsonl", &lines);
    assert_eq!(day(dir.path()), ["the first line", "the second"]);
    // Only the first line can begin with the mark, whatever that line is: after a blank one, a marked line is
    // not a log's start.
    let dir = scratch();
    write_log(
        dir.path(),
        "-work-alpha/s.jsonl",
        &[String::new(), marked(0, "marked, second"), said(&at(1), A, "third")],
    );
    assert_eq!(day(dir.path()), ["third"]);
    // One mark is taken off, once: a log that starts with two has a first line that is not valid JSON.
    let dir = scratch();
    let twice = format!("\u{feff}{}", marked(0, "marked twice"));
    write_log(dir.path(), "-work-alpha/s.jsonl", &[twice, said(&at(1), A, "the second")]);
    assert_eq!(day(dir.path()), ["the second"]);
}
