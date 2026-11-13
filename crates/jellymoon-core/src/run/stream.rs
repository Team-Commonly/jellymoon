//! What is read of a run's stream (`docs/development/run.md`, "What is read of the stream"). It is untrusted: a line
//! is read up to [`LINE_MAX`] bytes and the whole stream up to [`STREAM_MAX`]. Of all of it this is taken: whether
//! it starts as a run's stream does, how many result lines came, the first one's `subtype`, `is_error` and count of
//! `permission_denials`, and whether a line that could not be read came after it; and its last word, which is data
//! in one of two shapes (`asks.rs`). No text of the agent's is kept but a question, which then passes the door.

use std::io::{BufReader, Read};
use std::process::ChildStdout;
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::thread;

use serde::Deserialize;
use serde::de::IgnoredAny;
use serde_json::Value;

use super::asks::{self, Last, WORD_TOOL};
use crate::session::next_line;

/// The longest line of the stream that is read, with its line break. A longer one is counted and skipped.
const LINE_MAX: usize = 1 << 20;
/// The most bytes of a whole stream that are read. A child that prints more is stopped, and its run has failed.
const STREAM_MAX: u64 = 64 << 20;
/// The tools a run is started with, in the order of their names, and its permission mode (ADR 0014, "Measured").
const TOOLS: [&str; 5] = ["Edit", "Glob", "Grep", "Read", "Write"];
const MODE: &str = "acceptEdits";

/// What a run's stream must say of how the harness was started, by what core passed it.
pub(super) struct Expect {
    /// The session id passed with `--session-id`: the init line names it, or the stream is not this run's.
    pub session: String,
    /// Whether a schema for the last word was passed. Only then does the init line list the tool for one.
    pub schema: bool,
}

/// What the reader of the stream has, when it stops reading.
#[derive(Default, Clone)]
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
    /// An init line was a run's in its tools, servers and mode, and named another session than the one passed.
    pub other: bool,
    /// The first result's last word.
    pub last: Last,
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
    /// The last word. It is read whatever its shape, so a result with an odd one is still the stream's result.
    structured_output: Option<Value>,
}

/// The four parts of the `system`/`init` line that are compared. Every other part of it is passed over.
#[derive(Deserialize)]
struct Start {
    tools: Vec<String>,
    mcp_servers: Vec<IgnoredAny>,
    #[serde(rename = "permissionMode")]
    permission_mode: String,
    session_id: Option<Value>,
}

fn line(raw: &[u8]) -> Option<Line> {
    let object = raw.trim_ascii_start().first() == Some(&b'{');
    serde_json::from_slice(raw).ok().filter(|_| object)
}

/// Whether an init line says what a run is started with: exactly the five tools, in any order, and with them
/// the tool of a last word when a schema was passed and never when none was; no MCP server; and the permission
/// mode. A part that is missing, or in another shape, is a part that differs. The second answer is whether the
/// line names the session that was passed: one that names another, or none, or one that is not text, does not.
fn measured(raw: &[u8], expect: &Expect) -> (bool, bool) {
    let Ok(mut start) = serde_json::from_slice::<Start>(raw) else { return (false, false) };
    let mut tools: Vec<&str> = TOOLS.into_iter().chain(expect.schema.then_some(WORD_TOOL)).collect();
    start.tools.sort();
    tools.sort_unstable();
    let fits = start.tools == tools && start.mcp_servers.is_empty() && start.permission_mode == MODE;
    (fits, start.session_id.as_ref().and_then(Value::as_str) == Some(expect.session.as_str()))
}

impl Stream {
    /// Takes the next line. False when nothing more is to be read: the stream did not start as a run's does, or
    /// said later that it is not one. Before the init line only other `system` lines are let by, which are the
    /// harness's own and show no work. Every init line is measured, wherever it comes.
    fn take(&mut self, raw: &[u8], expect: &Expect) -> bool {
        self.lines += 1;
        let read = line(raw);
        let system = read.as_ref().is_some_and(|line| line.kind == "system");
        let init = system && read.as_ref().is_some_and(|line| line.subtype.as_deref() == Some("init"));
        if init || (self.start.is_none() && !system) {
            let (fits, same) = if init { measured(raw, expect) } else { (false, true) };
            self.other |= fits && !same;
            self.start = Some(fits && same);
        }
        if self.start != Some(true) {
            return self.start.is_none();
        }
        match read {
            Some(line) if line.kind == "result" => {
                if self.results == 0 {
                    self.fine = line.subtype.as_deref() == Some("success") && line.is_error == Some(false);
                    self.denied = line.permission_denials.len();
                    self.last = asks::last(line.structured_output);
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
pub(super) fn reader(stdout: Option<ChildStdout>, expect: Expect) -> (Receiver<Stream>, Arc<Mutex<Stream>>) {
    let (send, receive) = mpsc::channel();
    let so_far = Arc::new(Mutex::new(Stream::default()));
    let shared = Arc::clone(&so_far);
    thread::spawn(move || {
        let mut found = Stream::default();
        if let Some(stdout) = stdout {
            let mut pipe = BufReader::with_capacity(1 << 16, stdout.take(STREAM_MAX + 1));
            let mut raw = Vec::new();
            while let Ok(true) = next_line(&mut pipe, LINE_MAX, &mut raw) {
                let more = found.take(&raw, &expect);
                if let Ok(mut shared) = shared.lock() {
                    shared.clone_from(&found);
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

#[cfg(test)]
mod tests {
    use super::*;

    const ID: &str = "0123abcd-4567-4abc-8def-0123456789ab";
    const FIVE: &str = r#"["Edit","Glob","Grep","Read","Write"]"#;
    const SIX: &str = r#"["Edit","StructuredOutput","Glob","Grep","Read","Write"]"#;

    /// What a stream is after one init line with these tools and this `session` part, read with a schema or not.
    fn start(tools: &str, session: &str, schema: bool) -> (Option<bool>, bool, bool) {
        let parts = format!(r#""tools":{tools},"mcp_servers":[],"permissionMode":"acceptEdits"{session}"#);
        let mut stream = Stream::default();
        let expect = Expect { session: ID.to_string(), schema };
        let more = stream.take(format!(r#"{{"type":"system","subtype":"init",{parts}}}"#).as_bytes(), &expect);
        (stream.start, stream.other, more)
    }

    // Core always passes a schema, so only here is a harness seen that lists the tool with none asked for.
    #[test]
    fn the_tool_of_a_last_word_is_a_run_s_only_when_a_schema_was_passed() {
        let named = format!(r#","session_id":"{ID}""#);
        assert_eq!(start(SIX, &named, true), (Some(true), false, true));
        assert_eq!(start(FIVE, &named, false), (Some(true), false, true));
        assert_eq!(start(SIX, &named, false), (Some(false), false, false), "listed, and never asked for");
        assert_eq!(start(FIVE, &named, true), (Some(false), false, false), "asked for, and not listed");
    }

    #[test]
    fn a_start_under_another_session_is_not_this_run_s_and_is_known_from_one_with_other_tools() {
        let text = |id: &str| format!(r#","session_id":"{id}""#);
        let others = [String::new(), r#","session_id":7"#.into(), r#","session_id":null"#.into()];
        let others = others.into_iter().chain([text(&ID.to_uppercase()), text(&format!("{ID} ")), text("")]);
        for other in others {
            assert_eq!(start(SIX, &other, true), (Some(false), true, false), "{other}");
            // A start that is not a run's by its tools is said to be that, whatever session it names.
            assert_eq!(start(FIVE, &other, true), (Some(false), false, false), "{other}");
        }
    }

    #[test]
    fn the_last_word_is_the_first_result_s_and_no_other_line_s() {
        let expect = Expect { session: ID.to_string(), schema: true };
        let asks = r#""structured_output":{"state":"asks","question":"which?"}"#;
        let result = |word: &str| format!(r#"{{"type":"result","subtype":"success","is_error":false,{word}}}"#);
        let mut stream = Stream::default();
        let init = format!(
            r#"{{"type":"system","subtype":"init","tools":{SIX},"mcp_servers":[],"permissionMode":"acceptEdits","session_id":"{ID}"}}"#
        );
        assert!(stream.take(init.as_bytes(), &expect));
        for not_a_result in ["assistant", "system", "user"] {
            assert!(stream.take(format!(r#"{{"type":"{not_a_result}",{asks}}}"#).as_bytes(), &expect));
            assert_eq!((&stream.last, stream.results), (&Last::None, 0), "{not_a_result}");
        }
        assert!(stream.take(result(asks).as_bytes(), &expect));
        assert_eq!((&stream.last, stream.results, stream.fine), (&Last::Asks("which?".into()), 1, true));
        assert!(stream.take(result(r#""structured_output":{"state":"done"}"#).as_bytes(), &expect));
        assert_eq!((&stream.last, stream.results), (&Last::Asks("which?".into()), 2), "a second result changes none");
    }
}
