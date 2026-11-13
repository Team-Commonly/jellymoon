//! A run that asks (F51, ADR 0018, decisions 2, 3 and 6; `docs/development/run.md`, "A run that asks"): the
//! session id core chooses before the harness is started, the schema the harness is given for its last word, how
//! that word is read from the stream's result, and the door a question passes before it is kept. A last word is a
//! model's: it is data, read in one shape only, and of one that fails nothing is kept.

use std::collections::hash_map::RandomState;
use std::fs::{self, File, OpenOptions, TryLockError};
use std::hash::{BuildHasher, Hasher};
use std::path::{Path, PathBuf};

use serde_json::Value;

use super::RunOutcome;
use super::end::lock_file;
use super::record::{self, Record};
use crate::still::{LOOKED, Unkept, still};

/// The schema of the harness's last word, passed with `--json-schema`: a state, and with `asks` one question.
pub(super) const SCHEMA: &str = r#"{"type":"object","properties":{"state":{"type":"string","enum":["done","asks"]},"question":{"type":"string"}},"required":["state"],"additionalProperties":false}"#;
/// The tool the harness lists in its init line when it was given a schema, and only then (ADR 0018, "Measured").
pub(super) const WORD_TOOL: &str = "StructuredOutput";
/// The most characters of a question that are kept: it is cut there, once cleaning has changed all it will.
pub(crate) const QUESTION_MAX: usize = 300;

/// How the harness's turn ended, as the first result line's `structured_output` says. Nothing else of it is kept.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(super) enum Last {
    /// The result has no last word, or it is null.
    #[default]
    None,
    Done,
    /// The question as the agent wrote it, of no more bytes than the door looks at. It is not cleaned yet.
    Asks(String),
    /// `asks`, with a question of more bytes than the door looks at. No byte of it is kept.
    Long,
    /// Anything else: not an object, another state, `asks` with no question that is text, `done` with a
    /// question, or any other part.
    Unfit,
}

/// Reads a last word: exactly `{"state":"done"}`, or exactly `{"state":"asks","question":<text>}`.
pub(super) fn last(word: Option<Value>) -> Last {
    let Some(word) = word else { return Last::None };
    let Some(word) = word.as_object() else { return Last::Unfit };
    match (word.get("state").and_then(Value::as_str), word.get("question"), word.len()) {
        (Some("done"), None, 1) => Last::Done,
        (Some("asks"), Some(Value::String(question)), 2) if question.len() > LOOKED => Last::Long,
        (Some("asks"), Some(Value::String(question)), 2) => Last::Asks(question.clone()),
        _ => Last::Unfit,
    }
}

/// Why a run whose result was a success did not finish and does not ask, by its last word.
pub(super) const NO_WORD: &str = "its result did not say whether it is done or asks";
pub(super) const UNFIT: &str = "its last word was not `done` alone, or `asks` with one question";
pub(super) const LONG: &str = "its question was longer than Jellymoon reads";
pub(super) const EMPTY: &str = "nothing was left of its question once it was cleaned";
pub(super) const CHANGING: &str = "its question could not be cleaned";
/// And why one whose init line names another session than the one it was given, or none, did not.
pub(super) const OTHER: &str = "it did not start under the session id it was given";
/// Every one of them: a record's end is one core wrote only in these words (`end::recorded`).
pub(super) const KINDS: [&str; 6] = [NO_WORD, UNFIT, LONG, EMPTY, CHANGING, OTHER];

/// What the last word of a run that would else have finished makes of it: None, it is done; a question, as it
/// is kept, it asks. Err is why it has failed.
pub(super) fn word(last: &Last) -> Result<Option<String>, &'static str> {
    match last {
        Last::Done => Ok(None),
        Last::Asks(question) => cleaned(question).map(Some),
        Last::None => Err(NO_WORD),
        Last::Long => Err(LONG),
        Last::Unfit => Err(UNFIT),
    }
}

/// A question as it is kept: through the door every kept text passes (`still.rs`), on one line, cut at
/// [`QUESTION_MAX`] characters. Err is why the run has failed, in fixed words, with no word of the question.
pub(super) fn cleaned(question: &str) -> Result<String, &'static str> {
    match still(question, QUESTION_MAX, true) {
        Ok(kept) if kept.is_empty() => Err(EMPTY),
        Ok(kept) => Ok(kept),
        Err(Unkept::Long) => Err(LONG),
        Err(Unkept::Changing) => Err(CHANGING),
    }
}

/// Whether a question read from a file is one the door left: a file is anyone's to write, and a question that
/// the door would change, empty or cut is not core's.
pub(crate) fn kept(question: &str) -> bool {
    !question.is_empty() && still(question, QUESTION_MAX, true).as_deref() == Ok(question)
}

/// A new session id: a random UUID (version 4), in small letters, as Claude Code takes one for `--session-id`.
/// Its 122 random bits are two outputs of the standard library's keyed hasher, as a run's id is one: the keys are
/// read from the system's random source once for each thread (`getrandom` on Linux, `CCRandomGenerateBytes` on
/// macOS, `ProcessPrng` on Windows), and every new hasher has another key. The id is a name, not a secret.
pub(super) fn fresh() -> String {
    let half = || RandomState::new().build_hasher().finish();
    let (a, b) = ((half() & !0xf000) | 0x4000, (half() & !(0b11 << 62)) | (0b10 << 62));
    let (low, mid, version) = (a >> 32, (a >> 16) & 0xffff, a & 0xffff);
    format!("{low:08x}-{mid:04x}-{version:04x}-{:04x}-{:012x}", b >> 48, b & 0xffff_ffff_ffff)
}

/// Whether a session id read from a record is one [`fresh`] makes: nothing else is ever given to a harness.
pub(super) fn is_session(id: &str) -> bool {
    let shaped = |(at, byte): (usize, u8)| match at {
        8 | 13 | 18 | 23 => byte == b'-',
        14 => byte == b'4',
        19 => matches!(byte, b'8' | b'9' | b'a' | b'b'),
        _ => byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte),
    };
    id.len() == 36 && id.bytes().enumerate().all(shaped)
}

/// The lock of a run that asks, held by a toss of it for as long as the toss is at work, so that nothing takes
/// the run up meanwhile. Its file goes with it, as a run's own does when the run ends.
pub(super) struct Held(File, PathBuf);

impl Drop for Held {
    fn drop(&mut self) {
        let _ = self.0.unlock();
        let _ = fs::remove_file(&self.1);
    }
}

/// The run's record, read for a toss: that of a run that asks is read again behind the run's own lock, which
/// is then held. None: there is no record that can be used. Err: a command holds the run. A lock that cannot be
/// made or asked about, or whose name holds something other than a plain file, is no lock, as it is for a look
/// (`end::held`): the record is read without one.
pub(super) fn for_toss(home: &Path, run: &str) -> Result<Option<(Record, Option<Held>)>, ()> {
    let Some(record) = record::kept(home, run) else { return Ok(None) };
    let lock = lock_file(home, run);
    if record.outcome != Some(RunOutcome::Asks) || fs::symlink_metadata(&lock).is_ok_and(|meta| !meta.is_file()) {
        return Ok(Some((record, None)));
    }
    let Ok(file) = OpenOptions::new().write(true).create(true).truncate(false).open(&lock) else {
        return Ok(Some((record, None)));
    };
    match file.try_lock() {
        Ok(()) => Ok(record::kept(home, run).map(|record| (record, Some(Held(file, lock))))),
        Err(TryLockError::WouldBlock) => Err(()),
        Err(TryLockError::Error(_)) => Ok(Some((record, None))),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn a_last_word_is_read_in_two_shapes_and_every_other_is_unfit() {
        assert_eq!(last(None), Last::None);
        assert_eq!(last(Some(json!({"state": "done"}))), Last::Done);
        assert_eq!(last(Some(json!({"state": "asks", "question": "which?"}))), Last::Asks("which?".into()));
        assert_eq!(last(Some(json!({"state": "asks", "question": ""}))), Last::Asks(String::new()));
        let (fits, over) = ("q".repeat(LOOKED), "q".repeat(LOOKED + 1));
        assert_eq!(last(Some(json!({"state": "asks", "question": fits}))), Last::Asks(fits));
        assert_eq!(last(Some(json!({"state": "asks", "question": over}))), Last::Long);
        for unfit in [
            json!("done"),
            json!(["done"]),
            json!({}),
            json!({"state": "paused"}),
            json!({"state": "Done"}),
            json!({"state": 1}),
            json!({"state": "asks"}),
            json!({"state": "asks", "question": null}),
            json!({"state": "asks", "question": ["which?"]}),
            json!({"state": "asks", "question": "which?", "more": 1}),
            json!({"state": "asks", "more": "which?"}),
            json!({"state": "done", "question": "which?"}),
            json!({"state": "done", "question": ""}),
            json!({"state": "done", "question": null}),
            json!({"state": "done", "more": 1}),
            json!({"question": "which?"}),
        ] {
            assert_eq!(last(Some(unfit.clone())), Last::Unfit, "{unfit}");
        }
    }

    #[test]
    fn a_question_is_cleaned_on_one_line_and_cut_and_one_that_is_empty_or_restless_is_refused() {
        assert_eq!(cleaned("Which file\nis \"the\" uploader?\u{1b}[31m"), Ok("Which file is 'the' uploader?".into()));
        let key = format!("AI{}{}", "za", "kLmNo".repeat(7));
        assert_eq!(cleaned(&format!("is {key} the key?")), Ok("is [redacted] the key?".into()));
        let long = cleaned(&"word ".repeat(100)).unwrap();
        assert_eq!((long.chars().count(), long.ends_with('…')), (QUESTION_MAX, true));
        assert_eq!(cleaned(&"q".repeat(QUESTION_MAX)).unwrap().chars().count(), QUESTION_MAX);
        for empty in ["", " \t\n", "\u{1b}\u{7}\u{200b}"] {
            assert_eq!(cleaned(empty), Err(EMPTY), "{empty:?}");
        }
        assert_eq!(cleaned(&key.repeat(9)), Err(CHANGING));
        assert_eq!(cleaned(&"q".repeat(LOOKED + 1)), Err(LONG));
        // What the door left is kept as it is, and nothing else read from a file is.
        for question in ["Which file is 'the' uploader?", "is [redacted] the key?", long.as_str()] {
            assert!(kept(question), "{question}");
        }
        let cut = format!("{}x", "q".repeat(QUESTION_MAX));
        for odd in ["", " which?", "which?\n", "a \"b\"", key.as_str(), cut.as_str(), &"q".repeat(LOOKED + 1)] {
            assert!(!kept(odd), "{odd:?}");
        }
    }

    #[test]
    fn a_last_word_ends_a_run_as_done_or_asking_or_failed_in_its_own_words() {
        assert_eq!(word(&Last::Done), Ok(None));
        assert_eq!(word(&Last::Asks("which\tone?".into())), Ok(Some("which one?".into())));
        assert_eq!(word(&Last::Asks(" \n".into())), Err(EMPTY));
        assert_eq!((word(&Last::None), word(&Last::Long), word(&Last::Unfit)), (Err(NO_WORD), Err(LONG), Err(UNFIT)));
        let mut kinds = KINDS.to_vec();
        kinds.dedup();
        assert_eq!(kinds.len(), 6, "each way to fail has words of its own");
    }

    #[test]
    fn a_session_id_is_a_random_uuid_and_no_two_are_one() {
        let ids: Vec<String> = (0..64).map(|_| fresh()).collect();
        for (at, id) in ids.iter().enumerate() {
            assert!(is_session(id), "{id}");
            assert!(!ids[..at].contains(id), "{id} was made twice");
        }
        let id = "0123abcd-4567-4abc-8def-0123456789ab";
        assert!(is_session(id));
        let changed = |at: usize, to: &str| format!("{}{to}{}", &id[..at], &id[at + 1..]);
        for odd in [
            changed(14, "1"),
            changed(19, "c"),
            changed(19, "7"),
            changed(8, "0"),
            changed(13, "0"),
            changed(18, "0"),
            changed(23, "0"),
            changed(0, "A"),
            changed(35, "g"),
            changed(35, ""),
            format!("{id}0"),
            format!("{id}\n"),
            "../../x".to_string(),
            String::new(),
        ] {
            assert!(!is_session(&odd), "{odd:?}");
        }
    }
}
