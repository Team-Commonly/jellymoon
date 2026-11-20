//! The steps read from a stream (live work): the five tools and no other, a file's name only inside the run's
//! place and only as core cleans it, no step of a line in another shape, the last few kept, and every other part
//! of what is read of the stream as it was.

use super::*;
use crate::run::steps::{KEPT, Step, Tool};

const ID: &str = "0123abcd-4567-4abc-8def-0123456789ab";
const PLACE: &str = "/w/copies/r0/alpha";
const SAID: &str = "zebra-quartz-said-by-the-agent";

fn expect() -> Expect {
    Expect { session: ID.to_string(), schema: true, resumed: false }
}

fn init() -> String {
    let tools = r#"["Edit","StructuredOutput","Glob","Grep","Read","Write"]"#;
    format!(
        r#"{{"type":"system","subtype":"init","tools":{tools},"mcp_servers":[],"permissionMode":"acceptEdits","session_id":"{ID}"}}"#
    )
}

/// An `assistant` line with these blocks in its `message.content`.
fn said(blocks: &str) -> String {
    format!(r#"{{"type":"assistant","message":{{"role":"assistant","content":[{blocks}]}}}}"#)
}

/// A `tool_use` block of `name` whose input is `input`.
fn tool(name: &str, input: &str) -> String {
    format!(r#"{{"type":"tool_use","id":"t1","name":"{name}","input":{input}}}"#)
}

fn at(path: &str) -> String {
    format!(r#"{{"file_path":"{path}"}}"#)
}

fn step(tool: Tool, file: Option<&str>) -> Step {
    Step { tool, file: file.map(str::to_string) }
}

/// What a stream is after these lines, read as the reader reads them.
fn read(lines: &[String]) -> Stream {
    let (mut stream, places) = (Stream::default(), [PathBuf::from(PLACE)]);
    for line in lines {
        stream.read(line.as_bytes(), &expect(), &places);
    }
    stream
}

#[test]
fn a_step_is_one_of_the_five_tools_and_names_a_file_only_for_edit_read_and_write() {
    let text = format!(r#"{{"type":"text","text":"{SAID} I will read src/a.rs"}}"#);
    let blocks = [text, tool("Read", &at(&format!("{PLACE}/src/a.rs")))].join(",");
    let lines = [
        init(),
        said(&blocks),
        said(&tool("Edit", &at(&format!("{PLACE}/src/a.rs")))),
        said(&tool("Write", &at(&format!("{PLACE}/notes.txt")))),
        said(&tool("Glob", &format!(r#"{{"pattern":"**/*.rs","path":"{PLACE}"}}"#))),
        said(&tool("Grep", &format!(r#"{{"pattern":"{SAID}","file_path":"{PLACE}/src/a.rs"}}"#))),
    ];
    let stream = read(&lines);
    assert_eq!(stream.steps.count, 5);
    let want = [
        step(Tool::Edit, Some("src/a.rs")),
        step(Tool::Write, Some("notes.txt")),
        step(Tool::Glob, None),
        step(Tool::Grep, None),
    ];
    assert_eq!(stream.steps.last, want, "the last four, oldest first");
    let kept = serde_json::to_string(&stream.steps).unwrap();
    assert!(!kept.contains(SAID) && !kept.contains(PLACE) && !kept.contains("**"), "{kept}");
    // Each name of the stream's own spelling of a tool is one of the five, and none else is a tool.
    assert!(TOOLS.iter().all(|name| Tool::of(name).is_some()));
    for other in ["Bash", "read", "READ", "StructuredOutput", "WebFetch", "Task", " Read", ""] {
        let stream = read(&[init(), said(&tool(other, &at(&format!("{PLACE}/a.txt"))))]);
        assert_eq!(stream.steps.count, 0, "{other:?}");
    }
}

#[test]
fn a_file_outside_the_place_or_whose_name_cleaning_changes_is_no_name_and_still_a_step() {
    let outside = [
        "/elsewhere/a.txt".to_string(),
        format!("{PLACE}/../beta/a.txt"),
        format!("{PLACE}bet/a.txt"),
        PLACE.to_string(),
        format!("{PLACE}/"),
        "a.txt".to_string(),
        "./a.txt".to_string(),
        "/w/copies/r0".to_string(),
    ];
    // A name shaped like a key is joined here, so no scanner reads one in this file.
    let key = format!("AI{}{}", "za", "kLmNo".repeat(7));
    let unplain = ["a\u{200b}b.txt", "2 more.txt", "x /y.txt", &key].map(|name| format!("{PLACE}/{name}"));
    for path in outside.iter().chain(&unplain) {
        let input = serde_json::json!({ "file_path": path }).to_string();
        let stream = read(&[init(), said(&tool("Read", &input))]);
        assert_eq!((stream.steps.count, &stream.steps.last[..]), (1, &[step(Tool::Read, None)][..]), "{path}");
    }
    for input in [r#"{"file_path":7}"#, r#"{"path":"/w/copies/r0/alpha/a.txt"}"#, "{}", "[]", "null"] {
        let stream = read(&[init(), said(&tool("Write", input))]);
        assert_eq!(stream.steps.last, [step(Tool::Write, None)], "{input}");
    }
    let inside = read(&[init(), said(&tool("Read", &at(&format!("{PLACE}/src/deep/b.rs"))))]);
    assert_eq!(inside.steps.last, [step(Tool::Read, Some("src/deep/b.rs"))]);
}

#[test]
fn a_tool_use_in_a_line_of_another_shape_or_before_the_start_is_no_step() {
    let use_ = tool("Read", &at(&format!("{PLACE}/a.txt")));
    let others = [
        format!(r#"{{"type":"user","message":{{"role":"user","content":[{use_}]}}}}"#),
        format!(r#"{{"type":"result","subtype":"success","is_error":false,"message":{{"content":[{use_}]}}}}"#),
        format!(r#"{{"type":"assistant","message":{{"content":{use_}}}}}"#),
        format!(r#"{{"type":"assistant","content":[{use_}]}}"#),
        format!(r#"{{"type":"assistant","message":[{use_}]}}"#),
        format!(r#"[{{"type":"assistant","message":{{"content":[{use_}]}}}}]"#),
        format!(r#"{{"type":"assistant","subtype":7,"message":{{"content":[{use_}]}}}}"#),
        said(&use_.replace(r#""type":"tool_use""#, r#""type":"tool_result""#)),
        said(&use_.replace(r#""name":"Read""#, r#""name":["Read"]"#)),
    ];
    for other in &others {
        assert_eq!(read(&[init(), other.clone()]).steps.count, 0, "{other}");
    }
    // Before the init line nothing is read as a step: such a line is a start that is not a run's.
    let mut early = Stream::default();
    assert_eq!(early.read(said(&use_).as_bytes(), &expect(), &[PathBuf::from(PLACE)]), (false, false));
    assert_eq!((early.start, early.steps.count), (Some(false), 0));
}

#[test]
fn every_step_is_counted_and_only_the_last_few_are_kept() {
    let mut lines = vec![init()];
    let many: Vec<String> = (0..10).map(|n| tool("Read", &at(&format!("{PLACE}/f{n}.txt")))).collect();
    lines.push(said(&many[..6].join(",")));
    lines.extend(many[6..].iter().map(|one| said(one)));
    let stream = read(&lines);
    let last: Vec<Step> = (6..10).map(|n| step(Tool::Read, Some(&format!("f{n}.txt")))).collect();
    assert_eq!((stream.steps.count, stream.steps.last, KEPT), (10, last, 4));
}

/// Every part of what is read of a stream but its steps.
type Parts = (usize, Option<bool>, usize, bool, usize, usize, bool, bool, bool, Last, Option<Said>, bool);

fn parts(stream: &Stream) -> Parts {
    let s = stream.clone();
    (s.lines, s.start, s.results, s.fine, s.denied, s.skipped, s.unread, s.over, s.other, s.last, s.said, s.lost)
}

#[test]
fn reading_steps_changes_nothing_else_that_is_read_of_a_stream() {
    let use_ = tool("Edit", &at(&format!("{PLACE}/a.txt")));
    let result = r#"{"type":"result","subtype":"success","is_error":false,"structured_output":{"state":"done"}}"#;
    let streams = [
        vec![init(), said(&use_), result.to_string()],
        vec![init(), said(&use_), result.to_string(), said(&use_)],
        vec![init(), said(&use_), result.to_string(), "not json".to_string()],
        vec![said(&use_), init(), result.to_string()],
        vec![init(), format!(r#"{{"type":"assistant","subtype":7,"message":{{"content":[{use_}]}}}}"#)],
    ];
    for lines in streams {
        let (mut taken, mut more) = (Stream::default(), Vec::new());
        for line in &lines {
            more.push(taken.take(line.as_bytes(), &expect()));
        }
        let (mut stepped, places) = (Stream::default(), [PathBuf::from(PLACE)]);
        let read: Vec<bool> = lines.iter().map(|line| stepped.read(line.as_bytes(), &expect(), &places).0).collect();
        assert_eq!((parts(&taken), more), (parts(&stepped), read), "{lines:?}");
    }
}
