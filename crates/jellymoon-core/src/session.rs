//! One session log of Claude Code 2.1: which of its lines a person typed, and the agent's last text reply to each. A log
//! is untrusted and mostly not the person's words: only a line the log itself marks as typed by a person starts a turn.

use std::collections::HashSet;
use std::fs::{File, Metadata};
use std::io::{self, BufRead, BufReader, Read};
use std::path::Path;
use std::sync::LazyLock;

use jiff::Timestamp;
use regex::Regex;
use serde::Deserialize;

use crate::redact::{pattern, shown};
use crate::{Harness, ReaderTurn};

/// The lengths `said`, `reply` and `project` are cleaned and cut to, in characters (`redact::shown`).
const SAID_MAX: usize = 700;
const REPLY_MAX: usize = 500;
const PROJECT_MAX: usize = 255;
/// The most characters of a folder that are kept for a run to find its repo by. A longer one is no folder.
pub(crate) const FOLDER_MAX: usize = 4096;

/// What the harness adds inside a person's prompt: reminders, the view context of an open artifact and the paragraph
/// that explains it. Each is a pattern and a pass of its own: joined in one pattern, a block that never closes stays
/// open across every later match of another, and the pass rescans the prompt once for each.
static HARNESS_BLOCKS: LazyLock<[Regex; 4]> = LazyLock::new(|| {
    [
        r"(?s:<system-reminder>.*?</system-reminder>)",
        r"(?s:<artifact-view-context.*?</artifact-view-context>)",
        r"<artifact-content-authored-by-others[\t\n\f\r ]*/>",
        r"\(The JSON line above[^)]*\)",
    ]
    .map(pattern)
});

/// How a slash command and its output start. The person typed the command, not this text.
const WRAPPERS: [&str; 3] = ["<command-name>", "<command-message>", "<local-command"];

/// The part of a log line the reader looks at. Anything may be missing or null.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Line {
    #[serde(rename = "type")]
    kind: Option<String>,
    timestamp: Option<Timestamp>,
    cwd: Option<String>,
    is_meta: Option<bool>,
    is_sidechain: Option<bool>,
    is_compact_summary: Option<bool>,
    uuid: Option<String>,
    origin: Option<Origin>,
    message: Option<Message>,
}

#[derive(Deserialize)]
struct Origin {
    kind: Option<String>,
}

#[derive(Deserialize)]
struct Message {
    content: Option<Content>,
}

/// A prompt is a string, or a list when something was pasted with it; a reply is a list.
#[derive(Deserialize)]
#[serde(untagged)]
enum Content {
    Text(String),
    Parts(Vec<Part>),
}

#[derive(Deserialize)]
struct Part {
    #[serde(rename = "type")]
    kind: Option<String>,
    text: Option<String>,
}

impl Part {
    fn text(&self) -> Option<&str> {
        self.text.as_deref().filter(|_| self.kind.as_deref() == Some("text"))
    }
}

/// What a user line is to the day.
enum Prompt {
    /// A person typed it. The text is empty for a slash command and for a prompt that was only harness blocks: such a
    /// line makes no turn and still ends the one before it.
    Typed(String),
    /// Nobody typed it: a message from another session, or one another program sent. The agent's next reply answers
    /// that, not the person, so it ends the turn before it and starts none.
    Ends,
    /// Not a prompt at all: a tool result, a task notice, a line the harness wrote itself. The agent goes on with what
    /// the person asked, so the turn stays open.
    Neither,
}

/// A turn, when it was typed, and the folder its project is named after: the log's own text, which is never printed.
pub(crate) type Typed = (Option<Timestamp>, ReaderTurn, String);

/// Returns one log's turns, on any day, each with when it was typed. A line that is not valid JSON is skipped; a file
/// that cannot be read gives what was read before the error. A log is read no further than its length when it was
/// opened: one that is written to as fast as it is read would have no end, and a line cut off there is not valid
/// JSON. A byte-order mark before the first line is not part of it. `seen` holds the uuid of every typed line so far: a resumed
/// or forked session repeats earlier lines in a new file, and the first one read is the one kept.
pub(crate) fn turns(file: &Path, skip: &[String], seen: &mut HashSet<String>, line_max: usize) -> Vec<Typed> {
    let Ok(log) = File::open(file) else {
        return Vec::new();
    };
    let folder =
        file.parent().and_then(Path::file_name).map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();
    let len = length(log.metadata());
    let (mut log, mut raw) = (BufReader::with_capacity(1 << 16, log.take(len)), Vec::new());
    let mut turns: Vec<Typed> = Vec::new();
    let mut open = false; // whether the last turn can still take a reply
    let mut mark = Some(b"\xef\xbb\xbf".as_slice()); // a byte-order mark, which only a log's first line can begin with
    while let Ok(true) = next_line(&mut log, line_max, &mut raw) {
        let Some(line) = decode(mark.take().and_then(|mark| raw.strip_prefix(mark)).unwrap_or(&raw), open) else {
            continue;
        };
        let uuid = line.uuid.as_deref().unwrap_or_default();
        match prompt(&line) {
            Prompt::Typed(_) if !uuid.is_empty() && seen.contains(uuid) => open = false,
            Prompt::Ends => open = false,
            Prompt::Typed(said) => {
                seen.insert(uuid.to_string());
                let cwd = line.cwd.as_deref().unwrap_or_default();
                // A line with no folder is skipped by its session's folder name.
                open = !said.is_empty() && !skipped(if cwd.is_empty() { &folder } else { cwd }, skip);
                if open {
                    // A folder's name is the log's text too: it is printed, and the model reads it.
                    let (project, folder) = place(cwd);
                    let project = shown(project, PROJECT_MAX);
                    let turn = ReaderTurn {
                        at: String::new(),
                        tool: Harness::ClaudeCode,
                        project,
                        said,
                        reply: String::new(),
                    };
                    turns.push((line.timestamp, turn, folder.chars().take(FOLDER_MAX + 1).collect()));
                }
            }
            Prompt::Neither => {
                if let (true, Some(text), Some((_, turn, _))) = (open, reply(&line), turns.last_mut()) {
                    turn.reply = shown(&text, REPLY_MAX);
                }
            }
        }
    }
    turns
}

/// How far a log is read: its length when it was opened. One whose length cannot be asked is read to its end, as
/// every log was before there was a bound: it is never taken for an empty one.
fn length(held: io::Result<Metadata>) -> u64 {
    held.map_or(u64::MAX, |held| held.len())
}

/// Reads the next line into `line` and reports whether there was one. A line longer than `max` bytes is read past and
/// comes back empty: no prompt a person typed is that long.
pub(crate) fn next_line(log: &mut impl BufRead, max: usize, line: &mut Vec<u8>) -> io::Result<bool> {
    line.clear();
    let (mut any, mut over) = (false, false);
    loop {
        let read = log.by_ref().take(max as u64 + 1).read_until(b'\n', line)?;
        any |= read > 0;
        let ended = read == 0 || line.ends_with(b"\n");
        over |= line.len() > max;
        if over {
            line.clear();
        }
        if ended {
            return Ok(any);
        }
    }
}

/// Decodes a line that can matter. Most of a log is tool results, megabytes each, so those are never decoded: a line that
/// names no origin is not a typed prompt, and a reply, or a prompt nobody typed, matters only while a turn is open. The
/// words are looked for in quotes, which cannot occur inside a JSON string, only as a key or a value of their own. A
/// sidechain is a subagent's own conversation: none of it is the person's.
fn decode(raw: &[u8], open: bool) -> Option<Line> {
    let text = String::from_utf8_lossy(raw);
    let has = |word: &str| text.contains(word);
    if !(has("\"origin\"") || open && (has("\"assistant\"") || has("\"user\"") && !has("\"tool_result\""))) {
        return None;
    }
    serde_json::from_str::<Line>(&text).ok().filter(|line| line.is_sidechain != Some(true))
}

/// Reports whether the log marks the line as typed by a person, and what they typed once the harness's blocks are cut out
/// and the start of what is left is cleaned and cut to its length. A prompt typed with a pasted image is a list: what was typed is its text parts, joined. A
/// list that holds a tool result is never typed, whatever its origin says.
fn prompt(line: &Line) -> Prompt {
    if line.kind.as_deref() != Some("user") || line.is_meta == Some(true) || line.is_compact_summary == Some(true) {
        return Prompt::Neither;
    }
    let said = match line.message.as_ref().and_then(|message| message.content.as_ref()) {
        None => return Prompt::Neither,
        Some(Content::Text(text)) => text.clone(),
        Some(Content::Parts(parts)) if parts.iter().any(|part| part.kind.as_deref() == Some("tool_result")) => {
            return Prompt::Neither;
        }
        Some(Content::Parts(parts)) => parts.iter().filter_map(Part::text).collect(),
    };
    match line.origin.as_ref().and_then(|origin| origin.kind.as_deref()) {
        Some("human") => {}
        Some("task-notification") => return Prompt::Neither,
        _ => return Prompt::Ends,
    }
    let said = without_blocks(&said);
    let said = said.trim();
    if WRAPPERS.iter().any(|wrapper| said.starts_with(wrapper)) {
        return Prompt::Typed(String::new());
    }
    Prompt::Typed(shown(said, SAID_MAX))
}

/// Cuts the harness's blocks out of a prompt.
fn without_blocks(said: &str) -> String {
    HARNESS_BLOCKS.iter().fold(said.to_string(), |said, block| block.replace_all(&said, "").into_owned())
}

/// The text parts of an assistant line, without its thinking and its tool calls. None when it has no text.
fn reply(line: &Line) -> Option<String> {
    let Some(Content::Parts(parts)) = line.message.as_ref().and_then(|message| message.content.as_ref()) else {
        return None;
    };
    if line.kind.as_deref() != Some("assistant") {
        return None;
    }
    let texts: Vec<&str> = parts.iter().filter_map(Part::text).map(str::trim).filter(|text| !text.is_empty()).collect();
    (!texts.is_empty()).then(|| texts.join(" "))
}

/// Names the folder a turn ran in: the repo's folder inside a harness worktree of it, the folder's last segment
/// otherwise. A folder is written with `\` on Windows, so both separators count on every system. One walk that holds
/// three segments at a time: a log's folder can be any length, and nothing here grows with it. With the name comes
/// the folder that carries it: the start of `cwd` up to the end of that segment.
fn place(cwd: &str) -> (&str, &str) {
    let (mut before, mut last, mut at) = ([("", 0); 3], ("", 0), 0);
    for segment in cwd.split(['/', '\\']) {
        if !before[0].0.is_empty() && before[1].0 == ".claude" && before[2].0 == "worktrees" {
            return (before[0].0, &cwd[..before[0].1]);
        }
        let end = at + segment.len();
        before = [before[1], before[2], (segment, end)];
        if !segment.is_empty() {
            last = (segment, end);
        }
        at = end + 1; // both separators are one byte
    }
    (last.0, &cwd[..last.1])
}

/// Whether a folder is switched off: it contains one of the names, character for character. White space around a
/// name is dropped first, then separators at its end, unless that leaves nothing: a name of separators alone stays as
/// written and matches every folder written with it. Only a name of white space alone is no name. Dropping can only
/// make a name match more, so nothing that switched a folder off before leaves it on.
fn skipped(place: &str, skip: &[String]) -> bool {
    let names = skip.iter().map(|name| name.trim()).map(|name| match name.trim_end_matches(['/', '\\']) {
        "" => name,
        cut => cut,
    });
    names.into_iter().any(|name| !name.is_empty() && place.contains(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    // The blocks are cut out of the whole prompt, before anything bounds it, so this step has to grow with its length.
    #[test]
    fn a_megabyte_of_block_starts_with_whole_blocks_between_them_is_cut_in_seconds() {
        let shapes = [
            "<system-reminder><artifact-content-authored-by-others/>",
            "<artifact-view-context(The JSON line above)",
            "(The JSON line above<system-reminder>x</system-reminder>",
            "<system-reminder></artifact-view-context><artifact-view-context</system-reminder>",
        ];
        for shape in shapes {
            let text = shape.repeat((1 << 20) / shape.len());
            let start = std::time::Instant::now();
            let _ = without_blocks(&text);
            assert!(start.elapsed() < std::time::Duration::from_secs(5), "{shape}: {:?}", start.elapsed());
        }
        let said = "a <system-reminder>LEAK\n</system-reminder>b <artifact-view-context x>LEAK</artifact-view-context>c \
                    <artifact-content-authored-by-others />d (The JSON line above is LEAK)e";
        assert_eq!(without_blocks(said), "a b c d e");
    }

    #[test]
    fn a_log_whose_length_cannot_be_asked_is_read_to_its_end_and_never_taken_for_an_empty_one() {
        assert_eq!(length(Err(io::Error::other("no length"))), u64::MAX);
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("log"), "abc").unwrap();
        assert_eq!(length(std::fs::metadata(dir.path().join("log"))), 3);
    }

    // A folder of a megabyte of slashes is walked once, three segments in hand, with no list of them made.
    #[test]
    fn a_project_is_named_from_a_folder_of_any_length_in_one_walk() {
        let names = [
            ("/work/alpha", "alpha"),
            ("/work/alpha/", "alpha"),
            ("/work/beta/.claude/worktrees/fix-login/web", "beta"),
            (r"C:\work\beta\.claude\worktrees\fix", "beta"),
            ("/a/.claude/worktrees/x/b/.claude/worktrees/y", "a"),
            ("/.claude/worktrees/x", "x"),
            ("/work/beta/.claude/worktrees", "worktrees"),
            ("", ""),
        ];
        fn project(cwd: &str) -> &str {
            place(cwd).0
        }
        for (cwd, want) in names {
            assert_eq!(project(cwd), want, "{cwd}");
        }
        assert_eq!(place("/work/beta/.claude/worktrees/fix-login/web").1, "/work/beta");
        assert_eq!(
            (place("/work/alpha/").1, place(r"C:\work\beta").1, place("").1),
            ("/work/alpha", r"C:\work\beta", "")
        );
        let slashes = "/".repeat(1 << 20);
        let start = std::time::Instant::now();
        assert_eq!(project(&slashes), "");
        assert_eq!(project(&format!("{slashes}gamma{slashes}")), "gamma");
        assert_eq!(project(&format!("{slashes}beta/.claude/worktrees/x{slashes}")), "beta");
        assert!(start.elapsed() < std::time::Duration::from_secs(20), "{:?}", start.elapsed());
    }
}
