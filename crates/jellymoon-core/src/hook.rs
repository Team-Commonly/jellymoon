//! Adds Jellymoon's hook to the harness settings and takes it out again (ADR 0011, section 5). The settings file is
//! the person's: it is never parsed and written back. [`install_hook`] puts one piece of text into it at the place a
//! parse found and keeps that text in `hook.json` in Jellymoon's home; [`remove_hook`] removes exactly that text.
//! Every other byte stays, whatever the person changed around it in between. The file is replaced whole ([`replace`]),
//! so a write that stops half way leaves the old file.

use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, ErrorKind, Write};
use std::ops::Range;
use std::path::{Path, PathBuf, absolute};
use std::process;

use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use serde_json::{Value, json};

use crate::busy::retried;
use crate::ruling::{EVENTS, private_dir};
use crate::{Error, HookState};

/// The hook event the entry is added under: it fires after each tool call that worked, in every session.
const EVENT: &str = "PostToolUse";

/// What was installed, so it can be found again and removed.
#[derive(Serialize, Deserialize)]
struct Record {
    settings: PathBuf,
    /// The program the entry runs.
    program: String,
    /// The text that was put into the settings file, byte for byte. Empty when the entry was already there.
    inserted: String,
    /// The settings file did not exist; when removing the text leaves it empty, the file is removed too.
    created: bool,
}

/// The members of a JSON object, each value as the piece of the text it was read from.
type Members<'a> = HashMap<String, &'a RawValue>;

fn record(at: &Path) -> Option<Record> {
    serde_json::from_str(&fs::read_to_string(at).ok()?).ok()
}

/// The entry as a value: one group with one handler, which runs `program hook` with no shell.
fn entry(program: &str) -> Value {
    json!({"hooks": [{"type": "command", "command": program, "args": ["hook"]}]})
}

/// The entry as it is written, with its keys in the order a person would write them.
fn entry_text(program: &str) -> String {
    format!(r#"{{"hooks":[{{"type":"command","command":{},"args":["hook"]}}]}}"#, Value::from(program))
}

/// Whether a group of the event's list holds a handler that runs `program`; with `whole`, one that runs `program hook`.
fn runs(group: &Value, program: &str, whole: bool) -> bool {
    let ours = |handler: &Value| handler["command"] == program && (!whole || handler["args"] == json!(["hook"]));
    group["hooks"].as_array().is_some_and(|handlers| handlers.iter().any(ours))
}

/// The groups of the event's list in settings with no comments, each with its place in the text. None when the text
/// is not a JSON object or its hooks are not shaped as Claude Code shapes them.
fn groups(clean: &str) -> Option<Vec<(Range<usize>, Value)>> {
    let top: Members = serde_json::from_str(clean).ok()?;
    let Some(hooks) = top.get("hooks") else { return Some(Vec::new()) };
    let events: Members = serde_json::from_str(hooks.get()).ok()?;
    let Some(list) = events.get(EVENT) else { return Some(Vec::new()) };
    let list: Vec<&RawValue> = serde_json::from_str(list.get()).ok()?;
    let placed = |group: &&RawValue| {
        let start = group.get().as_ptr() as usize - clean.as_ptr() as usize;
        Some((start..start + group.get().len(), serde_json::from_str(group.get()).ok()?))
    };
    list.iter().map(placed).collect()
}

/// Whether settings with no comments name `program` in a handler under any event; with `whole`, as `program hook`
/// under an event the hook answers. None when the text is not a JSON object.
fn named(clean: &str, program: &str, whole: bool) -> Option<bool> {
    let top: HashMap<String, Value> = serde_json::from_str(clean).ok()?;
    let Some(events) = top.get("hooks").and_then(Value::as_object) else { return Some(false) };
    let answered = events.iter().filter(|(event, _)| !whole || EVENTS.contains(&event.as_str()));
    Some(answered.filter_map(|(_, groups)| groups.as_array()).flatten().any(|group| runs(group, program, whole)))
}

/// Whether `text` already holds an entry that runs `program hook` under an event the hook answers, however it is
/// laid out. An entry under another event, or in a comment, is not one.
fn holds(text: &str, program: &str) -> bool {
    named(&without_comments(text), program, true) == Some(true)
}

/// Whether `text` can be written as settings once the entry is out: it is empty or a JSON object, and no handler
/// names `program` any more.
fn sound(text: &str, program: &str) -> bool {
    text.trim().is_empty() || named(&without_comments(text), program, false) == Some(false)
}

/// Whether the settings text holds what the record says was installed: the recorded text, or an entry that runs the
/// recorded program however it is laid out.
fn has(record: &Record, text: &str) -> bool {
    let kept = !record.inserted.is_empty() && without_comments(text).contains(&record.inserted);
    kept || holds(text, &record.program)
}

/// Whether the hook is installed now: there is a record of it, and the settings file it names still holds the
/// entry. Nothing is written.
pub fn hook_installed(home: &Path) -> bool {
    let installed = |record: Record| fs::read_to_string(&record.settings).is_ok_and(|text| has(&record, &text));
    record(&home.join("hook.json")).is_some_and(installed)
}

/// Adds one entry to `settings` that runs `program hook` with no shell. A missing or empty file becomes one object
/// holding the entry. A file that is not a JSON object (comments are allowed) is refused and left as it was, and so is
/// one whose `hooks` are not shaped as Claude Code shapes them. A file that already holds the entry is left alone.
pub fn install_hook(settings: &Path, home: &Path, program: &Path) -> Result<HookState, Error> {
    let settings = absolute(settings).map_err(Error::Settings)?;
    let kept = home.join("hook.json");
    if let Some(record) = record(&kept)
        && fs::read_to_string(&record.settings).is_ok_and(|text| has(&record, &text))
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
    let program = program.to_str().ok_or_else(not_text)?.to_string();
    let keep = |record: &Record| {
        let line = serde_json::to_string(record).map_err(io::Error::from)?;
        private_dir(home).and_then(|()| fs::write(&kept, line))
    };
    if holds(&text, &program) {
        // Laid out by someone else, so there is no text to remember: it is found again by what it runs.
        let record = Record { settings, program, inserted: String::new(), created: false };
        keep(&record).map_err(Error::Store)?;
        return Ok(HookState { installed: true, changed: false });
    }
    let (at, mut inserted) = splice(&text, &entry_text(&program))?;
    if created {
        inserted.push('\n');
    }
    text.insert_str(at, &inserted);
    if serde_json::from_str::<Members>(&without_comments(&text)).is_err() {
        return Err(Error::NotSettings);
    }
    // The record goes first: an entry with no record could not be taken out again.
    let record = Record { settings, program, inserted, created };
    keep(&record).map_err(Error::Store)?;
    if let Err(e) = replace(&record.settings, &text) {
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

/// Puts `text` in the file at `path`, whole or not at all. It is written to a new file beside it (made new, never
/// over a file that is there, and for the person alone until it has the old file's mode), synced, and renamed over
/// the old one. A link is followed first, so the link stays and what it points at is replaced. When a step fails the
/// new file is removed and the old one is as it was. A run that is killed in between leaves the new file there.
fn replace(path: &Path, text: &str) -> io::Result<()> {
    put(&fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf()), text, true)
}

/// [`replace`] with no link followed: what is at `target` is replaced. With `as_old` the new file takes the old
/// file's mode; without it the file is the person's alone.
pub(crate) fn put(target: &Path, text: &str, as_old: bool) -> io::Result<()> {
    let mut name = target.file_name().ok_or(ErrorKind::InvalidInput)?.to_os_string();
    name.push(format!(".jellymoon-{}", process::id()));
    let new = target.with_file_name(name);
    let mut open = OpenOptions::new();
    open.write(true).create_new(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut open, 0o600);
    let file = open.open(&new)?;
    // Windows refuses a rename over a file another process is reading at that moment: tried again (`busy.rs`).
    let moved = || retried(|| fs::rename(&new, target));
    let filled = fill(file, text, as_old.then_some(target)).and_then(|()| moved());
    if filled.is_err() {
        let _ = fs::remove_file(&new);
    }
    filled
}

fn fill(mut file: File, text: &str, target: Option<&Path>) -> io::Result<()> {
    file.write_all(text.as_bytes())?;
    // Only where a mode says who may read: elsewhere it is the read-only mark, which would keep a new file that
    // could not be renamed from being removed.
    #[cfg(unix)]
    if let Some(old) = target.and_then(|target| fs::metadata(target).ok()) {
        file.set_permissions(old.permissions())?;
    }
    #[cfg(not(unix))]
    let _ = target;
    file.sync_all()
}

/// Takes out the entry [`install_hook`] put in, and nothing else. With nothing installed, or with the settings file
/// or the entry gone, it changes nothing. The recorded text is removed when that leaves sound settings; otherwise
/// (the file was laid out anew, or something was added beside the entry) the one group that is exactly the entry is
/// cut out. A file that would not be a JSON object afterwards, or would still name the program, is never written:
/// the file is left alone, the record is kept and the person is told to remove the entry.
pub fn remove_hook(home: &Path) -> Result<HookState, Error> {
    let kept = home.join("hook.json");
    let Some(record) = record(&kept) else {
        return Ok(HookState { installed: false, changed: false });
    };
    let exact = |text: &str| {
        let once = !record.inserted.is_empty() && text.matches(&record.inserted).count() == 1;
        Some(text.replacen(&record.inserted, "", 1)).filter(|rest| once && sound(rest, &record.program))
    };
    let rest = match fs::read_to_string(&record.settings) {
        Ok(text) => match exact(&text) {
            Some(rest) => Some(rest),
            None => cut(&text, &record.program)?,
        },
        Err(e) if e.kind() == ErrorKind::NotFound => None,
        Err(e) => return Err(Error::Settings(e)),
    };
    if let Some(rest) = &rest {
        if !sound(rest, &record.program) {
            return Err(Error::Changed);
        }
        let done = if record.created && rest.is_empty() {
            fs::remove_file(&record.settings)
        } else {
            replace(&record.settings, rest)
        };
        done.map_err(Error::Settings)?;
    }
    fs::remove_file(&kept).map_err(Error::Store)?;
    Ok(HookState { installed: false, changed: rest.is_some() })
}

/// `text` without the one group of the event's list that is exactly the entry. The group goes with one comma and the
/// blank space beside it: the comma after it, or the one before it when it is last. A comment is never removed, so a
/// comment between the group and its comma leaves text that is not JSON, which the caller refuses. None when no
/// handler under any event names the program: the entry is gone already.
fn cut(text: &str, program: &str) -> Result<Option<String>, Error> {
    if text.trim().is_empty() {
        return Ok(None);
    }
    let clean = without_comments(text);
    let groups = groups(&clean).unwrap_or_default();
    let exact: Vec<&Range<usize>> = groups.iter().filter(|(_, group)| *group == entry(program)).map(|g| &g.0).collect();
    let [place] = exact[..] else {
        return if named(&clean, program, false) == Some(false) { Ok(None) } else { Err(Error::Changed) };
    };
    let blank = |c: char| c.is_ascii_whitespace();
    let (before, after) = (text[..place.start].trim_end_matches(blank), text[place.end..].trim_start_matches(blank));
    let span = if let Some(next) = after.strip_prefix(',') {
        place.start..text.len() - next.trim_start_matches(blank).len()
    } else if before.ends_with(',') {
        before.len() - 1..place.end
    } else {
        place.clone()
    };
    let mut rest = text.to_string();
    rest.replace_range(span, "");
    Ok(Some(rest))
}

#[cfg(test)]
mod tests;
