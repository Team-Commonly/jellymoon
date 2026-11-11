//! What is read of a run's stream (`docs/development/run.md`, "What is read of the stream"). It is untrusted: a line
//! is read up to [`LINE_MAX`] bytes and the whole stream up to [`STREAM_MAX`]. Of all of it this is taken: whether
//! it starts as a run's stream does, how many result lines came, the first one's `subtype`, `is_error` and count of
//! `permission_denials`, and whether a line that could not be read came after it. No text of the agent's is kept.

use std::io::{BufReader, Read};
use std::process::ChildStdout;
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::thread;

use serde::Deserialize;
use serde::de::IgnoredAny;

use crate::session::next_line;

/// The longest line of the stream that is read, with its line break. A longer one is counted and skipped.
const LINE_MAX: usize = 1 << 20;
/// The most bytes of a whole stream that are read. A child that prints more is stopped, and its run has failed.
const STREAM_MAX: u64 = 64 << 20;
/// The tools a run is started with, in the order of their names, and its permission mode (ADR 0014, "Measured").
const TOOLS: [&str; 5] = ["Edit", "Glob", "Grep", "Read", "Write"];
const MODE: &str = "acceptEdits";

/// What the reader of the stream has, when it stops reading.
#[derive(Default, Clone, Copy)]
pub(super) struct Stream {
    /// How many lines came, the stream's or not.
    pub lines: usize,
    /// Whether the stream started as a run's does. None: the line that says so has not come.
    pub start: Option<bool>,
    /// How many result lines came. `fine` and `denied` are the first one's.
    pub results: usize,
    /// It says success and no error.
    pub fine: bool,
    pub denied: usize,
    /// The lines that were not the stream's.
    pub skipped: usize,
    /// One of them came after a result.
    pub unread: bool,
    /// The stream is past its cap.
    pub over: bool,
}

/// The parts of a line of the stream that are looked at. A line that is not an object with a `type` that is text,
/// or that has one of the other three in another shape, is not a line of the stream. A list is not one either,
/// though it could be read as these four in order: [`line`] takes only what starts as an object.
#[derive(Deserialize)]
struct Line {
    #[serde(rename = "type")]
    kind: String,
    subtype: Option<String>,
    is_error: Option<bool>,
    #[serde(default)]
    permission_denials: Vec<IgnoredAny>,
}

/// The three parts of the `system`/`init` line that are compared. Every other part of it is passed over.
#[derive(Deserialize)]
struct Start {
    tools: Vec<String>,
    mcp_servers: Vec<IgnoredAny>,
    #[serde(rename = "permissionMode")]
    permission_mode: String,
}

fn line(raw: &[u8]) -> Option<Line> {
    let object = raw.trim_ascii_start().first() == Some(&b'{');
    serde_json::from_slice(raw).ok().filter(|_| object)
}

/// Whether an init line says what a run is started with: exactly the five tools, in any order, no MCP server, and
/// the permission mode. A part that is missing, or in another shape, is a part that differs.
fn measured(raw: &[u8]) -> bool {
    let Ok(mut start) = serde_json::from_slice::<Start>(raw) else { return false };
    start.tools.sort();
    start.tools == TOOLS && start.mcp_servers.is_empty() && start.permission_mode == MODE
}

impl Stream {
    /// Takes the next line. False when nothing more is to be read: the stream did not start as a run's does, or
    /// said later that it is not one. Before the init line only other `system` lines are let by, which are the
    /// harness's own and show no work. Every init line is measured, wherever it comes.
    fn take(&mut self, raw: &[u8]) -> bool {
        self.lines += 1;
        let read = line(raw);
        let system = read.as_ref().is_some_and(|line| line.kind == "system");
        let init = system && read.as_ref().is_some_and(|line| line.subtype.as_deref() == Some("init"));
        if init || (self.start.is_none() && !system) {
            self.start = Some(init && measured(raw));
        }
        if self.start != Some(true) {
            return self.start.is_none();
        }
        match read {
            Some(line) if line.kind == "result" => {
                if self.results == 0 {
                    self.fine = line.subtype.as_deref() == Some("success") && line.is_error == Some(false);
                    self.denied = line.permission_denials.len();
                }
                self.results += 1;
            }
            Some(_) => {}
            None => {
                self.skipped += 1;
                self.unread |= self.results > 0;
            }
        }
        true
    }
}

/// Reads the stream on a thread of its own and sends what it found: at the stream's end, at the cap, or at a start
/// that is not a run's, after which nothing is read. What it has so far is kept in the second part after every
/// line, for a stream whose end never comes: something the child left may hold it open.
pub(super) fn reader(stdout: Option<ChildStdout>) -> (Receiver<Stream>, Arc<Mutex<Stream>>) {
    let (send, receive) = mpsc::channel();
    let so_far = Arc::new(Mutex::new(Stream::default()));
    let shared = Arc::clone(&so_far);
    thread::spawn(move || {
        let mut found = Stream::default();
        if let Some(stdout) = stdout {
            let mut pipe = BufReader::with_capacity(1 << 16, stdout.take(STREAM_MAX + 1));
            let mut raw = Vec::new();
            while let Ok(true) = next_line(&mut pipe, LINE_MAX, &mut raw) {
                let more = found.take(&raw);
                if let Ok(mut shared) = shared.lock() {
                    *shared = found;
                }
                if !more {
                    break;
                }
            }
            // One byte past the cap was asked for: when it came, the stream is over the cap, whatever was in it.
            found.over = pipe.get_ref().limit() == 0;
        }
        let _ = send.send(found);
    });
    (receive, so_far)
}
