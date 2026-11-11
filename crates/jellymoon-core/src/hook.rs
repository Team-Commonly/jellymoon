//! Adds Jellymoon's hook to the harness settings and takes it out again (ADR 0011, section 5). The settings file is
//! the person's: it is never parsed and written back. [`install_hook`] puts one piece of text into it at the place a parse
//! found and keeps that text in `hook.json` in Jellymoon's home; [`remove_hook`] removes exactly that text. Every other
//! byte stays, whatever the person changed around it in between.

use std::collections::HashMap;
use std::fs;
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf, absolute};

use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

use crate::{Error, HookState};

/// The hook event the entry is added under: it fires after each tool call that worked, in every session.
const EVENT: &str = "PostToolUse";

/// What was installed, so it can be found again and removed.
#[derive(Serialize, Deserialize)]
struct Record {
    settings: PathBuf,
    /// The text that was put into the settings file, byte for byte.
    inserted: String,
    /// The settings file did not exist; when removing the text leaves it empty, the file is removed too.
    created: bool,
}

/// The members of a JSON object, each value as the piece of the text it was read from.
type Members<'a> = HashMap<String, &'a RawValue>;

fn record(at: &Path) -> Option<Record> {
    serde_json::from_str(&fs::read_to_string(at).ok()?).ok()
}

/// Adds one entry to `settings` that runs `program hook` with no shell. A missing or empty file becomes one object
/// holding the entry. A file that is not a JSON object (comments are allowed) is refused and left as it was, and so is
/// one whose `hooks` are not shaped as Claude Code shapes them. Installing twice changes nothing the second time.
pub fn install_hook(settings: &Path, home: &Path, program: &Path) -> Result<HookState, Error> {
    let settings = absolute(settings).map_err(Error::Settings)?;
    let kept = home.join("hook.json");
    if let Some(record) = record(&kept)
        && fs::read_to_string(&record.settings).is_ok_and(|text| text.contains(&record.inserted))
    {
        let same = record.settings == settings;
        return if same { Ok(HookState { installed: true, changed: false }) } else { Err(Error::Elsewhere) };
    }
    let (mut text, created) = match fs::read_to_string(&settings) {
        Ok(text) => (text, false),
        Err(e) if e.kind() == ErrorKind::NotFound => (String::new(), true),
        Err(e) => return Err(Error::Settings(e)),
    };
    let not_text = || Error::Store(io::Error::new(ErrorKind::InvalidInput, "the program's path is not text"));
    let program = serde_json::to_string(program.to_str().ok_or_else(not_text)?).map_err(|e| Error::Store(e.into()))?;
    let entry = format!(r#"{{"hooks":[{{"type":"command","command":{program},"args":["hook"]}}]}}"#);
    let (at, mut inserted) = splice(&text, &entry)?;
    if created {
        inserted.push('\n');
    }
    text.insert_str(at, &inserted);
    if serde_json::from_str::<Members>(&without_comments(&text)).is_err() {
        return Err(Error::NotSettings);
    }
    // The record goes first: an entry with no record could not be taken out again.
    let record = Record { settings, inserted, created };
    let line = serde_json::to_string(&record).map_err(|e| Error::Store(e.into()))?;
    fs::create_dir_all(home).and_then(|()| fs::write(&kept, line)).map_err(Error::Store)?;
    if let Err(e) = fs::write(&record.settings, text) {
        let _ = fs::remove_file(&kept);
        return Err(Error::Settings(e));
    }
    Ok(HookState { installed: true, changed: true })
}

/// Where in `text` the entry goes, and what is written there: the entry alone at the head of the event's list, or
/// with the keys the file lacks around it. It goes right after an opening bracket, with a comma when something follows.
fn splice(text: &str, entry: &str) -> Result<(usize, String), Error> {
    let event = format!(r#""{EVENT}":[{entry}]"#);
    if text.trim().is_empty() {
        return Ok((text.len(), format!(r#"{{"hooks":{{{event}}}}}"#)));
    }
    let clean = without_comments(text);
    // A value's text is a piece of `clean`, so where it starts is where its opening bracket is in `text`.
    let inside = |value: &RawValue| value.get().as_ptr() as usize - clean.as_ptr() as usize + 1;
    let top: Members = serde_json::from_str(&clean).map_err(|_| Error::NotSettings)?;
    let (at, mut inserted, alone) = match top.get("hooks") {
        None => (clean.find('{').map_or(0, |open| open + 1), format!(r#""hooks":{{{event}}}"#), top.is_empty()),
        Some(hooks) => {
            let events: Members = serde_json::from_str(hooks.get()).map_err(|_| Error::NotSettings)?;
            match events.get(EVENT) {
                None => (inside(hooks), event, events.is_empty()),
                Some(groups) => {
                    let list: Vec<&RawValue> = serde_json::from_str(groups.get()).map_err(|_| Error::NotSettings)?;
                    (inside(groups), entry.to_string(), list.is_empty())
                }
            }
        }
    };
    if !alone {
        inserted.push(',');
    }
    Ok((at, inserted))
}

/// `text` with every `//` and `/* */` comment outside a string turned to spaces, so it is the same length and each
/// byte that is left is where it was.
fn without_comments(text: &str) -> String {
    let mut bytes = text.as_bytes().to_vec();
    let mut at = 0;
    while at < bytes.len() {
        match (bytes[at], bytes.get(at + 1)) {
            (b'"', _) => {
                at += 1;
                while at < bytes.len() && bytes[at] != b'"' {
                    at += if bytes[at] == b'\\' { 2 } else { 1 };
                }
                at += 1;
            }
            (b'/', Some(b'/' | b'*')) => {
                let (line, rest) = (bytes[at + 1] == b'/', &text[at..]);
                let end = if line { rest.find('\n') } else { rest[2..].find("*/").map(|close| close + 4) };
                let end = end.map_or(bytes.len(), |end| at + end);
                bytes[at..end].fill(b' ');
                at = end;
            }
            _ => at += 1,
        }
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

/// Takes out the text [`install_hook`] put in, and nothing else. With nothing installed it does nothing. When the text is
/// no longer in the file as it was written, the file is left alone and the person is told to remove the entry.
pub fn remove_hook(home: &Path) -> Result<HookState, Error> {
    let kept = home.join("hook.json");
    let Some(record) = record(&kept) else {
        return Ok(HookState { installed: false, changed: false });
    };
    let text = fs::read_to_string(&record.settings).map_err(Error::Settings)?;
    if text.matches(&record.inserted).count() != 1 {
        return Err(Error::Changed);
    }
    let rest = text.replacen(&record.inserted, "", 1);
    let done = if record.created && rest.is_empty() {
        fs::remove_file(&record.settings)
    } else {
        fs::write(&record.settings, rest)
    };
    done.map_err(Error::Settings)?;
    fs::remove_file(&kept).map_err(Error::Store)?;
    Ok(HookState { installed: false, changed: true })
}
