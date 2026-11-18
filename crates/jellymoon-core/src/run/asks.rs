//! A run that asks (F51, ADR 0018, decisions 2, 3 and 6; `docs/development/run.md`, "A run that asks"): the
//! session id core chooses before the harness is started, the schema the harness is given for its last word, how
//! that word is read from the stream's result, and the door a question passes before it is kept. A last word is a
//! model's: it is data, read in one shape only, and of one that fails nothing is kept.

use std::collections::hash_map::RandomState;
use std::fmt;
use std::fs::{self, File, OpenOptions, TryLockError};
use std::hash::{BuildHasher, Hasher};
use std::path::Path;

use serde::Deserialize;
use serde::de::{Deserializer, IgnoredAny, MapAccess, Visitor};
use serde_json::Value;

use super::RunOutcome;
use super::end::lock_file;
use super::record::{self, Record};
use crate::busy;
use crate::still::{LOOKED, Unkept, still};

/// The schema of the harness's last word, as its file holds it (F87, ADR 0023; `run.md`, "What the harness is
/// started with"): a state, with `asks` one question, and with `done` a result. It is a format of its own, outside
/// the code, versioned by its name.
const SCHEMA: &str = include_str!("../../formats/last-word.v1.json");

/// The schema as it is passed with `--json-schema`: the file without its last line break.
pub(super) fn schema() -> &'static str {
    SCHEMA.trim_end()
}
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
    /// Anything else: not an object, another state, `asks` with no question that is text, or any other part.
    Unfit,
}

/// A last word as the stream wrote it: an object with every part it wrote, or anything else.
#[derive(Deserialize)]
#[serde(untagged)]
pub(super) enum Word {
    Object(Parts),
    Other(IgnoredAny),
}

/// The parts of an object in the order they were written, a name written twice as two parts. A map would keep
/// the last of the two, and `{"state":"asks","state":"done"}` would read as done.
pub(super) struct Parts(Vec<(String, Value)>);

impl<'de> Deserialize<'de> for Parts {
    fn deserialize<D: Deserializer<'de>>(from: D) -> Result<Self, D::Error> {
        struct Written;
        impl<'de> Visitor<'de> for Written {
            type Value = Parts;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("an object")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Parts, A::Error> {
                let mut parts = Vec::new();
                while let Some(part) = map.next_entry()? {
                    parts.push(part);
                }
                Ok(Parts(parts))
            }
        }
        from.deserialize_map(Written)
    }
}

impl Word {
    /// What a run that is done says of its work (F85, `told.rs`): the one `result` beside `done` and any
    /// questions, in whatever shape it was written. None of any other word, and of a word that writes `result` twice.
    pub(super) fn result(&self) -> Option<&Value> {
        let Word::Object(Parts(word)) = self else { return None };
        let read = word.iter().filter(|part| part.0 != "question").count();
        let done = read == 2 && word.iter().any(|part| part.0 == "state" && part.1 == "done");
        word.iter().find(|part| done && part.0 == "result").map(|part| &part.1)
    }
}

/// Reads a last word: `{"state":"done"}`, alone or with a `result` or a `question` beside it, or exactly
/// `{"state":"asks","question":<text>}`. A word that names a part twice has one part too many, whatever the two
/// say. A `result` is not counted: beside `done` or beside `asks` and its question it is of any shape and written
/// any number of times, since what a run says of its work never fails it, and a run that asks keeps none
/// (`Word::result`). Beside anything else it is a part too many. Nor is a `question` beside `done`, which is never
/// read: the state is the run's claim, and a run that needs an answer says `asks` (done with a question).
pub(super) fn last(word: Option<Word>) -> Last {
    let Some(word) = word else { return Last::None };
    let Word::Object(Parts(word)) = word else { return Last::Unfit };
    let part = |name: &str| word.iter().find(|part| part.0 == name).map(|part| &part.1);
    let state = part("state").and_then(Value::as_str);
    let unread = |name: &str| name == "result" || (state == Some("done") && name == "question");
    match (state, part("question"), word.iter().filter(|part| !unread(&part.0)).count()) {
        (Some("done"), _, 1) => Last::Done,
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
/// And why a run that was answered (F52, `answer.rs`) did not go on: the harness has the session no more.
pub(super) const LOST: &str = "Claude Code no longer keeps the session it asked in";
/// What Claude Code says of a session id it is told to go on with and does not have: on stderr, and in the
/// `errors` of a result that is an error. It is looked for there only, never in the agent's own words, and only
/// in a run that was answered, and it is never kept.
pub(super) const LOST_SIGN: &str = "No conversation found with session ID";
/// And why a follow-up's fork that said so was not started again under a new session (F53, `follow.rs`).
pub(super) const WORKED: &str = "Claude Code said it keeps no session to fork after it had worked in the copy";
/// Every one of them: a record's end is one core wrote only in these words (`end::recorded`).
pub(super) const KINDS: [&str; 8] = [NO_WORD, UNFIT, LONG, EMPTY, CHANGING, OTHER, LOST, WORKED];

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

/// The lock of a run that asks, held by a toss of it or an answer to it for as long as that is at work, so that
/// nothing else takes the run up meanwhile. Its file is never removed, as the keeps' lock's and the one runs are
/// counted behind are not: it goes with the run's folder. Removed when the lock is let go of, it could be held
/// under no name by a command that opened it just before, and the run would be held by two, or seem held by none.
pub(super) type Held = File;

/// Takes the lock of a run that asks. Err(true): a command holds the run. Err(false): there is no lock to be
/// had, as it cannot be made or asked about, or its name holds something other than a plain file, which is
/// not opened (`end::held`).
pub(super) fn take(home: &Path, run: &str) -> Result<Held, bool> {
    let lock = lock_file(home, run);
    if fs::symlink_metadata(&lock).is_ok_and(|meta| !meta.is_file()) {
        return Err(false);
    }
    let file = OpenOptions::new().write(true).create(true).truncate(false).open(&lock).map_err(|_| false)?;
    // A test's build can wait here, with the lock's file open and the lock not yet asked for.
    busy::pause("JELLYMOON_TEST_RUN_LOCK_WAIT_MS");
    match file.try_lock() {
        Ok(()) => Ok(file),
        Err(e) => Err(matches!(e, TryLockError::WouldBlock)),
    }
}

/// The run's record, read for a toss: that of a run that asks is read again behind the run's own lock, which
/// is then held. None: there is no record that can be used. Err: a command holds the run. Where there is no
/// lock to be had the record is read without one, as a look reads it.
pub(super) fn for_toss(home: &Path, run: &str) -> Result<Option<(Record, Option<Held>)>, ()> {
    let Some(record) = record::kept(home, run) else { return Ok(None) };
    if record.outcome != Some(RunOutcome::Asks) {
        return Ok(Some((record, None)));
    }
    // A test's build can wait here, with the record read and the run's lock not yet asked for.
    busy::pause("JELLYMOON_TEST_TOSS_LOCK_WAIT_MS");
    match take(home, run) {
        Ok(held) => Ok(record::kept(home, run).map(|record| (record, Some(held)))),
        Err(true) => Err(()),
        Err(false) => Ok(Some((record, None))),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    /// A last word as the stream's reader reads it from a result line.
    fn read(word: &str) -> Last {
        last(serde_json::from_str::<Option<Word>>(word).unwrap())
    }

    #[test]
    fn what_a_run_says_of_its_work_stands_beside_done_and_beside_nothing_else() {
        let last = |word: Value| read(&word.to_string());
        // What a run says of its work stands beside `done`, in any shape, and beside nothing else.
        for said in [json!(7), json!(null), json!("it is done"), json!({"pieces": []}), json!({"state": "asks"})] {
            assert_eq!(last(json!({"state": "done", "result": said})), Last::Done, "{said}");
            // Beside `asks` and its question it is left out: the run asks (F87's review).
            let asks = json!({"state": "asks", "question": "which?", "result": said});
            assert_eq!(last(asks), Last::Asks("which?".into()), "{said}");
            assert_eq!(last(json!({"state": "asks", "result": said})), Last::Unfit, "{said}");
            assert_eq!(last(json!({"state": "done", "question": "which?", "result": said})), Last::Done, "{said}");
            assert_eq!(last(json!({"result": said})), Last::Unfit, "{said}");
            assert_eq!(last(json!({"state": "done", "result": said, "more": 1})), Last::Unfit, "{said}");
        }
        assert_eq!(read(r#"{"state":"done","result":1,"result":2}"#), Last::Done);
        assert_eq!(read(r#"{"state":"done","state":"done","result":1}"#), Last::Unfit);
        let result = |word: &str| serde_json::from_str::<Word>(word).unwrap().result().cloned();
        assert_eq!(result(r#"{"result":[1],"state":"done"}"#), Some(json!([1])));
        assert_eq!(result(r#"{"state":"done","question":"which?","result":[1]}"#), Some(json!([1])));
        assert_eq!(result(r#"{"question":7,"state":"done","question":null,"result":[1]}"#), Some(json!([1])));
        for none in [
            r#"{"state":"done","question":"which?","result":1,"result":2}"#,
            r#"{"state":"done","question":"which?"}"#,
            r#"{"state":"done"}"#,
            r#"{"state":"done","result":1,"result":2}"#,
            r#"{"state":"asks","result":1}"#,
            r#"{"state":"asks","question":"which?","result":1}"#,
            r#"{"state":"done","result":1,"more":1}"#,
            r#"{"result":1,"result":2}"#,
            r#"["state","done","result",1]"#,
        ] {
            assert_eq!(result(none), None, "{none}");
        }
    }

    #[test]
    fn a_last_word_is_read_in_two_shapes_and_every_other_is_unfit() {
        let last = |word: Option<Value>| word.map_or(Last::None, |word| read(&word.to_string()));
        assert_eq!((last(None), read("null")), (Last::None, Last::None));
        // A name written twice is two parts: the word is read by neither of them.
        for twice in [
            r#"{"state":"asks","state":"done"}"#,
            r#"{"state":"done","state":"asks"}"#,
            r#"{"state":"done","state":"done"}"#,
            r#"{"state":"asks","question":"a?","question":"b?"}"#,
            r#"{"state":"asks","question":"a?","state":"asks"}"#,
            r#"{"question":"a?","state":"done","state":"asks"}"#,
        ] {
            assert_eq!(read(twice), Last::Unfit, "{twice}");
        }
        assert_eq!(last(Some(json!({"state": "done"}))), Last::Done);
        // A question beside `done`, of any shape and written any number of times, is not read: the run is done.
        for question in [json!("which?"), json!(""), json!(null), json!(["which?"]), json!({"q": 1})] {
            assert_eq!(last(Some(json!({"state": "done", "question": question}))), Last::Done, "{question}");
        }
        assert_eq!(read(r#"{"state":"done","question":"a?","question":"b?"}"#), Last::Done);
        assert_eq!(last(Some(json!({"state": "asks", "question": "which?"}))), Last::Asks("which?".into()));
        assert_eq!(last(Some(json!({"state": "asks", "question": ""}))), Last::Asks(String::new()));
        let (fits, over) = ("q".repeat(LOOKED), "q".repeat(LOOKED + 1));
        assert_eq!(last(Some(json!({"state": "asks", "question": fits}))), Last::Asks(fits));
        assert_eq!(last(Some(json!({"state": "asks", "question": over}))), Last::Long);
        for unfit in [
            json!("done"),
            json!(7),
            json!(true),
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
            json!({"state": "done", "question": "which?", "more": 1}),
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
        assert_eq!(kinds.len(), 8, "each way to fail has words of its own");
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
