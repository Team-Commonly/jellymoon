//! The made-up agent's part in a run that asks (F51, `tests/f51.rs`): the session id and the schema it was
//! started with, which its stream's first line then echoes as Claude Code 2.1.323 did (ADR 0018, "Measured"), and
//! the scripts that end a turn with every last word a run can meet. Every line is typed here.

use std::path::Path;
use std::sync::OnceLock;
use std::{env, fs};

use super::{MARKER, MODE, TOOLS, edits, line, says, starts, waits, work};

/// The session id this program was started with, and whether it was given a schema.
static TOLD: OnceLock<(String, bool)> = OnceLock::new();

fn option(name: &str) -> Option<String> {
    let mut args = env::args();
    args.find(|arg| arg == name)?;
    args.next()
}

/// Reads what the command line says of a session and a schema. A program started a second time, below something
/// that minds a signal, has no command line of a run's, and reads what the first start left in `told`.
pub fn told(rec: &Path) {
    let told = match option("--session-id") {
        Some(id) => {
            let schema = option("--json-schema").is_some();
            fs::write(rec.join("told"), format!("{id}\n{schema}\n")).unwrap();
            (id, schema)
        }
        None => {
            let left = fs::read_to_string(rec.join("told")).unwrap_or_default();
            let mut left = left.lines();
            (left.next().unwrap_or_default().to_string(), left.next() == Some("true"))
        }
    };
    TOLD.set(told).unwrap();
}

/// The part of the first line that names the session, with its comma: nothing when none was given.
pub fn session() -> String {
    let id = TOLD.get().map_or("", |told| told.0.as_str());
    if id.is_empty() { String::new() } else { format!(r#""session_id":"{id}","#) }
}

/// A list of tools as the harness lists it: with the tool of a last word when a schema was given.
pub fn listed(tools: &str) -> String {
    let schema = TOLD.get().is_some_and(|told| told.1);
    if schema { tools.replacen('[', r#"["StructuredOutput","#, 1) } else { tools.to_string() }
}

/// What the agent asks when its question is a plain one. Jellymoon keeps it: it is not the agent's [`MARKER`].
const QUESTION: &str = "Which of the two uploaders did you mean?";

/// A result with `word` as its last word, written as JSON, and none at all when `word` is empty.
fn last(subtype: &str, is_error: bool, word: &str) -> String {
    let word = if word.is_empty() { String::new() } else { format!(r#","structured_output":{word}"#) };
    format!(
        r#"{{"type":"result","subtype":"{subtype}","is_error":{is_error},"num_turns":2,"result":"{MARKER} all done","permission_denials":[]{word}}}"#
    )
}

fn asks(question: &str) -> String {
    format!(r#"{{"state":"asks","question":"{question}"}}"#)
}

/// The question of `asks`, by its number, as it stands inside a JSON text.
fn question(number: usize) -> String {
    // A credential's shape, joined here so no scanner reads one in this file.
    let key = format!("AI{}{}", "za", "kLmNo".repeat(7));
    match number {
        2 => String::new(),
        3 => r" \n\t ".to_string(),
        4 => r"Which\u001b[31m uploader\u0007 did you\u200b mean?".to_string(),
        5 => format!("Which {}did you mean?", "uploader ".repeat(60)),
        6 => "q".repeat(4097),
        7 => "q".repeat(4096),
        8 => format!("Is {key} the key to use?"),
        9 => r"Which uploader?\nThe old one\r\nor the new one?".to_string(),
        10 => format!("use {} here", key.repeat(9)),
        11 => r#"Did you mean \"retry\" or \"resume\"?"#.to_string(),
        _ => QUESTION.to_string(),
    }
}

/// A last word that is neither of the two a run ends with, by its number. 0 is none at all, 1 is nothing.
const WORDS: [&str; 10] = [
    "",
    "null",
    r#"{"state":"paused"}"#,
    r#"{"state":"done","question":"Which of the two uploaders did you mean?"}"#,
    r#"{"state":"asks"}"#,
    r#""done""#,
    r#"{"state":"asks","question":"Which of the two uploaders did you mean?","more":1}"#,
    r#"{"state":"asks","question":7}"#,
    r#"{"state":"done","more":"Which of the two uploaders did you mean?"}"#,
    r#"[{"state":"asks","question":"Which of the two uploaders did you mean?"}]"#,
];

/// A first line with its tools and its session written out, for the starts that are not a run's.
fn init(tools: &str, session: &str) {
    let parts = format!(r#""tools":{tools},"mcp_servers":[],"permissionMode":{MODE}"#);
    line(&format!(r#"{{"type":"system","subtype":"init","cwd":"{MARKER}",{session}{parts}}}"#));
}

/// The scripts of F51. None: the script is not one of them.
pub fn play(script: &str, number: usize, rec: &Path) -> Option<i32> {
    let plain = asks(QUESTION);
    let code = match script {
        // A turn that ends with a question, by its number ([`question`]). 1 has done the work first.
        "asks" => {
            [starts as fn(), edits][usize::from(number == 1)]();
            line(&last("success", false, &asks(&question(number))));
            0
        }
        // The work, and a success whose last word is not one a run ends with ([`WORDS`]).
        "word" => {
            edits();
            line(&last("success", false, WORDS[number]));
            0
        }
        // A question, in a turn that did not end well: 0 a result that is no success, 1 one that is an error, 2 a
        // failing exit, 3 the result twice, 4 a line that is not the stream's after it.
        "asks-failed" => {
            edits();
            line(&last(if number == 0 { "error_during_execution" } else { "success" }, number == 1, &plain));
            match number {
                3 => line(&last("success", false, &plain)),
                4 => line(&format!("{MARKER} is not JSON")),
                _ => {}
            }
            if number == 2 { 3 } else { 0 }
        }
        // A start whose tools are not a run's with a schema: 0 lists no tool for a last word, 1 lists it twice.
        // It never ends by itself.
        "tools" => {
            let twice = TOOLS.replacen('[', r#"["StructuredOutput","StructuredOutput","#, 1);
            init(if number == 0 { TOOLS } else { &twice }, &session());
            says();
            waits(false, rec)
        }
        // A start under another session than the one given: 0 another id, 1 none, 2 a number, 3 the id in
        // capitals, 4 the id with a space after it. None ends by itself. 5 is the measured start, then a second
        // one under another id, the work and a success.
        "session" => {
            let id = TOLD.get().map_or("", |told| told.0.as_str());
            let other = match number {
                1 => String::new(),
                2 => r#""session_id":7,"#.to_string(),
                3 => format!(r#""session_id":"{}","#, id.to_uppercase()),
                4 => format!(r#""session_id":"{id} ","#),
                _ => r#""session_id":"0123abcd-4567-4abc-8def-0123456789ab","#.to_string(),
            };
            if number == 5 {
                starts();
            }
            init(&listed(TOOLS), &other);
            if number != 5 {
                says();
                return Some(waits(false, rec));
            }
            work();
            line(&last("success", false, r#"{"state":"done"}"#));
            0
        }
        // A harness too old for a run's command line, by the option it does not know. `old` is F39's.
        "old" | "old-option" => {
            let option = ["--json-schema", "--session-id", "--verbose"][number];
            eprintln!("error: unknown option '{}'", if script == "old" { "--restricted" } else { option });
            1
        }
        _ => return None,
    };
    Some(code)
}
