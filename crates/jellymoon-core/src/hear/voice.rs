//! `voice.toml` in Jellymoon's home: the key that is held and the language that is heard (ADR 0019, decision 8).
//! The person writes it by hand, and nothing of Jellymoon's writes it. Two names are read, each a quoted text on a
//! line of its own, and anything else in the file refuses the whole file in one line: a setting half read would
//! register a key the person did not choose. The reader is this file; no TOML library is used for two names.

use std::fmt;
use std::io::ErrorKind;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::Error;
use crate::redact::line;
use crate::tap::plain;

/// The most bytes of `voice.toml` that are read. A larger file is refused.
pub const VOICE_BYTES: usize = 4096;
/// The most characters of a refused value that the refusal shows.
const SHOWN: usize = 40;
const KEY: &str = "Control+Shift+Space";
const LANGUAGE: &str = "default";
const MODIFIERS: [&str; 4] = ["Control", "Shift", "Option", "Command"];

/// The settings of voice, as `jellymoon hear --settings` prints them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Voice {
    /// The key that is held: one or more of Control, Shift, Option and Command, then one key, joined by `+`.
    pub key: String,
    /// The language that is heard: `default`, the system's own, or a tag such as `en-US`.
    pub language: String,
}

impl fmt::Display for Voice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&serde_json::to_string(self).map_err(|_| fmt::Error)?)
    }
}

fn refused(why: impl fmt::Display) -> Error {
    Error::Hear(format!("voice.toml is not used: {why}"))
}

/// The settings in `home`. With no file they are the defaults, `Control+Shift+Space` and `default`, and a name
/// the file leaves out has its default. Refused, in one line: a file that is a link, that others can write to,
/// that is over [`VOICE_BYTES`] or not text, a line that is neither of the two names, a name twice, and a key or
/// a language that is not one. The line names the value, cleaned and cut, or the number of the line.
pub fn voice(home: &Path) -> Result<Voice, Error> {
    let bytes = match plain(&home.join("voice.toml"), VOICE_BYTES) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == ErrorKind::NotFound => Vec::new(),
        Err(e) => return Err(refused(e)),
    };
    let text = String::from_utf8(bytes).map_err(|_| refused("it is not text"))?;
    let (mut key, mut language) = (None, None);
    for (at, written) in text.lines().enumerate() {
        let (at, written) = (at + 1, written.trim());
        if written.is_empty() || written.starts_with('#') {
            continue;
        }
        let odd = || refused(format!("line {at} is not `key = \"...\"` or `language = \"...\"`"));
        let (name, value) = written.split_once('=').ok_or_else(odd)?;
        let value = quoted(value.trim()).ok_or_else(odd)?;
        let named = match name.trim() {
            "key" => &mut key,
            "language" => &mut language,
            _ => return Err(odd()),
        };
        if named.replace(value.to_string()).is_some() {
            return Err(refused(format!("line {at} names {} a second time", name.trim())));
        }
    }
    let (key, language) = (key.unwrap_or_else(|| KEY.to_string()), language.unwrap_or_else(|| LANGUAGE.to_string()));
    if !is_key(&key) {
        let takes = "is not one or more of Control, Shift, Option and Command, each once, and then one of A to Z, \
                     0 to 9, F1 to F12 or Space, joined by +";
        return Err(refused(format!("the key \"{}\" {takes}", line(&key, SHOWN))));
    }
    if !is_language(&language) {
        let shown = line(&language, SHOWN);
        return Err(refused(format!("the language \"{shown}\" is not `default` or a tag such as en-US")));
    }
    Ok(Voice { key, language })
}

/// What is between a value's two quotes, when it has no quote and no backslash in it.
fn quoted(value: &str) -> Option<&str> {
    let inner = value.strip_prefix('"')?.strip_suffix('"')?;
    (!inner.contains(['"', '\\'])).then_some(inner)
}

fn is_key(key: &str) -> bool {
    let mut parts: Vec<&str> = key.split('+').collect();
    let Some(last) = parts.pop() else { return false };
    let function = |name: &str| name.strip_prefix('F').is_some_and(|n| (1..=12).any(|f| f.to_string() == n));
    let one = |name: &str| matches!(name.as_bytes(), [b'A'..=b'Z' | b'0'..=b'9']);
    let held = |at: usize, part: &&str| MODIFIERS.contains(part) && !parts[..at].contains(part);
    (one(last) || function(last) || last == "Space")
        && !parts.is_empty()
        && parts.iter().enumerate().all(|(at, p)| held(at, p))
}

/// `default`, or a language of two or three small letters, then a script and then a region if there is one:
/// `en`, `en-US`, `zh-Hans`, `zh-Hans-CN`, `es-419`.
fn is_language(language: &str) -> bool {
    let all = |part: &str, small: bool| {
        part.bytes().all(|b| if small { b.is_ascii_lowercase() } else { b.is_ascii_uppercase() })
    };
    let script = |part: &str| part.len() == 4 && part.is_ascii() && all(&part[..1], false) && all(&part[1..], true);
    let region = |part: &str| {
        (part.len() == 2 && all(part, false)) || (part.len() == 3 && part.bytes().all(|b| b.is_ascii_digit()))
    };
    let parts: Vec<&str> = language.split('-').collect();
    let base = |part: &str| (2..=3).contains(&part.len()) && all(part, true);
    language == LANGUAGE
        || match parts[..] {
            [first] => base(first),
            [first, second] => base(first) && (script(second) || region(second)),
            [first, second, third] => base(first) && script(second) && region(third),
            _ => false,
        }
}
