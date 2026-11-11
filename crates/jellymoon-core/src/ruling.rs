//! Rulings and their delivery (ADR 0011, section 5). [`rule`] keeps the person's choice on a clash as one file under
//! `rulings/` in Jellymoon's home. [`deliver`] is what the hook in the harness runs: it hands each waiting ruling to
//! the session that called it, once. Once is kept by a mark file per ruling and session that only one caller can
//! create, so two hooks of one session that race hand a ruling over one time between them.

use std::fs::{self, OpenOptions};
use std::io::{self, ErrorKind, Read, Write};
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::Deserialize;
use serde_json::json;

use crate::still::statement;
use crate::{Error, Ruling};

/// The most characters of a statement kept: what a quote on a page may hold.
const WORDS_MAX: usize = 200;
/// How long a ruling waits for sessions. After that no session is told, and the next ruling made removes its files.
const WAIT: Duration = Duration::from_secs(24 * 60 * 60);
/// The most rulings handed over in one call, oldest first; the rest come at the session's next step. This keeps what
/// the hook prints far under the 10,000 characters Claude Code shows of it.
const TOLD_MAX: usize = 8;
/// The most rulings looked at to see which side of a clash stands, newest first, and the most bytes read of each.
const SEEN_MAX: usize = 256;
const RULING_MAX: u64 = 8 * 1024;
/// The most of the harness's input that is read.
const INPUT_MAX: u64 = 1 << 20;
/// The hook events whose output may carry context for the model in the shape [`deliver`] prints.
pub(crate) const EVENTS: [&str; 3] = ["PostToolUse", "PreToolUse", "UserPromptSubmit"];

/// What Claude Code gives a hook on its input, as far as it is used here.
#[derive(Deserialize)]
struct Step {
    session_id: String,
    hook_event_name: String,
    /// Set when the step is a subagent's. A ruling is for the session's main thread, which decides what is done.
    agent_id: Option<serde_json::Value>,
}

/// A statement as it is kept and handed over: cleaned and cut as the reader does it, on one line (a line break, a
/// tab and any run of spaces become one space), with each `"` made `'`. So a statement cannot end its own quote in
/// what the hook prints, or start a line that reads as another ruling. It is cleaned until cleaning changes nothing
/// ([`crate::still`]), so the words of a statement that was kept are the words of that statement read back.
pub(crate) fn words(text: &str) -> String {
    statement(text, WORDS_MAX)
}

/// Makes a folder of Jellymoon's own, and the folders above it that are missing, for the person alone (0700) where
/// the system has such modes. A folder that is there is left as it is.
pub(crate) fn private_dir(path: &Path) -> io::Result<()> {
    let mut make = fs::DirBuilder::new();
    make.recursive(true);
    #[cfg(unix)]
    std::os::unix::fs::DirBuilderExt::mode(&mut make, 0o700);
    make.create(path)
}

/// [`private_dir`] for a folder that holds what is put into sessions or onto the page: one that was there already,
/// open to others, is made the person's alone.
pub(crate) fn own_dir(path: &Path) -> io::Result<()> {
    private_dir(path)?;
    #[cfg(unix)]
    fs::set_permissions(path, std::os::unix::fs::PermissionsExt::from_mode(0o700))?;
    Ok(())
}

/// The rulings that still wait, oldest first, at most the newest [`SEEN_MAX`]. Each file is read as untrusted.
pub(crate) fn waiting(home: &Path) -> Vec<Ruling> {
    let (dir, now) = (home.join("rulings"), millis(SystemTime::now()));
    let ids = fs::read_dir(&dir).into_iter().flatten().flatten();
    let ids = ids.filter_map(|entry| made(entry.file_name().to_str()?.strip_suffix(".json")?));
    let mut ids: Vec<u64> = ids.filter(|made| waits(*made, now)).collect();
    ids.sort_unstable();
    let read = |id: &u64| {
        let mut text = String::new();
        fs::File::open(dir.join(format!("{id}.json"))).ok()?.take(RULING_MAX).read_to_string(&mut text).ok()?;
        let ruling: Ruling = serde_json::from_str(&text).ok()?;
        Some(Ruling { stands: words(&ruling.stands), over: words(&ruling.over), ..ruling })
    };
    ids[ids.len().saturating_sub(SEEN_MAX)..].iter().filter_map(read).collect()
}

fn millis(now: SystemTime) -> u64 {
    now.duration_since(UNIX_EPOCH).map_or(0, |since| u64::try_from(since.as_millis()).unwrap_or(u64::MAX))
}

/// Keeps a ruling: `stands` is the statement the person chose, `over` the one it replaces, both in their own words.
/// Each is cleaned and cut as the reader does it before it is stored. Returns the ruling as it will be handed over.
pub fn rule(home: &Path, stands: &str, over: &str) -> Result<Ruling, Error> {
    rule_at(home, stands, over, millis(SystemTime::now()))
}

fn rule_at(home: &Path, stands: &str, over: &str, now: u64) -> Result<Ruling, Error> {
    let (stands, over) = (words(stands), words(over));
    if stands.is_empty() || over.is_empty() {
        return Err(Error::Ruling);
    }
    let dir = home.join("rulings");
    own_dir(&dir).map_err(Error::Store)?;
    forget(&dir, now);
    // The id is the time it was made, moved on until no ruling has it. The file appears whole or not at all.
    let mut id = now;
    loop {
        let ruling = Ruling { id: id.to_string(), stands: stands.clone(), over: over.clone() };
        let mut file = tempfile::NamedTempFile::new_in(&dir).map_err(Error::Store)?;
        file.write_all(ruling.to_string().as_bytes()).map_err(Error::Store)?;
        match file.persist_noclobber(dir.join(format!("{id}.json"))) {
            Ok(_) => return Ok(ruling),
            Err(taken) if taken.error.kind() == ErrorKind::AlreadyExists => id += 1,
            Err(failed) => return Err(Error::Store(failed.error)),
        }
    }
}

/// The time in a store file's name: a ruling is `<time>.json`, a mark `<time>.<session>.told`.
fn made(name: &str) -> Option<u64> {
    name.split('.').next()?.parse().ok()
}

fn waits(made: u64, now: u64) -> bool {
    now.saturating_sub(made) < u64::try_from(WAIT.as_millis()).unwrap_or(u64::MAX)
}

/// Removes the rulings that no longer wait, and their marks. A file that cannot be removed is left for next time.
fn forget(dir: &Path, now: u64) {
    for entry in fs::read_dir(dir).into_iter().flatten().flatten() {
        if entry.file_name().to_str().and_then(made).is_some_and(|made| !waits(made, now)) {
            let _ = fs::remove_file(entry.path());
        }
    }
}

/// What the hook prints for one step of a session: nothing when no ruling waits for it, otherwise one JSON object that
/// adds the waiting rulings to the model's context. `input` is what the harness gave the hook; it is not read at all
/// when no ruling waits. Nothing here fails: a hook that cannot deliver says nothing, so it can never stop a session.
pub fn deliver(home: &Path, input: impl Read) -> Option<String> {
    deliver_at(home, input, millis(SystemTime::now()))
}

fn deliver_at(home: &Path, input: impl Read, now: u64) -> Option<String> {
    let dir = home.join("rulings");
    let mut waiting: Vec<u64> = fs::read_dir(&dir)
        .ok()?
        .flatten()
        .filter_map(|entry| made(entry.file_name().to_str()?.strip_suffix(".json")?))
        .filter(|made| waits(*made, now))
        .collect();
    if waiting.is_empty() {
        return None;
    }
    waiting.sort_unstable();
    let mut text = String::new();
    input.take(INPUT_MAX).read_to_string(&mut text).ok()?;
    let step: Step = serde_json::from_str(&text).ok()?;
    // The session's id becomes part of a file name, so it must be a plain name; Claude Code's is a UUID.
    let plain = |c: char| c.is_ascii_alphanumeric() || c == '-' || c == '_';
    if step.session_id.is_empty() || step.session_id.len() > 64 || !step.session_id.chars().all(plain) {
        return None;
    }
    if step.agent_id.is_some() {
        return None;
    }
    let event = EVENTS.iter().find(|event| **event == step.hook_event_name)?;
    let mut told = Vec::new();
    for id in waiting {
        if told.len() == TOLD_MAX {
            break;
        }
        // Only one caller can create the mark, and only that one hands the ruling over.
        let mark = dir.join(format!("{id}.{}.told", step.session_id));
        if OpenOptions::new().write(true).create_new(true).open(mark).is_err() {
            continue;
        }
        let kept = fs::read_to_string(dir.join(format!("{id}.json"))).ok();
        if let Some(ruling) = kept.and_then(|kept| serde_json::from_str::<Ruling>(&kept).ok()) {
            // The file is read as untrusted: its words are cleaned again on the way out.
            told.push(format!(
                "- What stands: \"{}\". It replaces: \"{}\".",
                words(&ruling.stands),
                words(&ruling.over)
            ));
        }
    }
    if told.is_empty() {
        return None;
    }
    let context = format!(
        "Jellymoon: the person has made a newer decision about two things they said that cannot both stand. \
         It is their own choice and replaces what they said before. Follow it from here on.\n{}",
        told.join("\n")
    );
    Some(json!({"hookSpecificOutput": {"hookEventName": event, "additionalContext": context}}).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const STEP: &[u8] = br#"{"session_id":"s-1","hook_event_name":"PostToolUse"}"#;

    #[test]
    fn a_ruling_waits_a_day_and_the_next_ruling_removes_what_stopped_waiting() {
        let home = tempfile::tempdir().unwrap();
        let day = u64::try_from(WAIT.as_millis()).unwrap();
        rule_at(home.path(), "fail fast", "add a retry", 1_000).unwrap();
        assert!(deliver_at(home.path(), STEP, 1_000 + day).is_none(), "a ruling a day old is not handed over");
        assert!(deliver_at(home.path(), STEP, 999 + day).unwrap().contains("fail fast"));
        rule_at(home.path(), "keep the old name", "rename it", 2_000 + day).unwrap();
        let mut left: Vec<_> =
            fs::read_dir(home.path().join("rulings")).unwrap().map(|e| e.unwrap().file_name()).collect();
        left.sort();
        assert_eq!(left, [format!("{}.json", 2_000 + day).as_str()]);
    }

    #[test]
    fn two_rulings_made_in_one_millisecond_are_both_kept_and_at_most_eight_are_told_at_a_step() {
        let home = tempfile::tempdir().unwrap();
        for n in 0..10 {
            let ruling = rule_at(home.path(), &format!("choice number {n}"), "the other", 5).unwrap();
            assert_eq!(ruling.id, (5 + n).to_string());
        }
        let first = deliver_at(home.path(), STEP, 20).unwrap();
        assert_eq!((first.matches("What stands").count(), first.contains("choice number 8")), (TOLD_MAX, false));
        let second = deliver_at(home.path(), STEP, 20).unwrap();
        assert!(second.contains("choice number 8") && second.contains("choice number 9"), "{second}");
        assert!(deliver_at(home.path(), STEP, 20).is_none());
    }
}
