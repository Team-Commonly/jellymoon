//! What `jellymoon harness` prints: whether a harness was found and how, in one line with no folder in it. It is
//! the same look a page or a run makes, so it asks the login shell when they would.

use std::fmt;
use std::path::Path;

use serde::Serialize;

use super::{Missed, Shell, search};
use crate::Error;

/// How the harness was found.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum How {
    /// `JELLYMOON_CLAUDE` names it: nothing was looked for.
    Named,
    Path,
    FixedPlace,
    LoginShell,
    None,
}

/// The line of `jellymoon harness`. `says` is one fixed sentence for each way the look can end.
#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct HarnessFound {
    pub found: bool,
    pub how: How,
    pub says: String,
}

impl fmt::Display for HarnessFound {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&serde_json::to_string(self).map_err(|_| fmt::Error)?)
    }
}

/// Looks for the harness as a page or a run does, and says how it ended. No harness is started.
pub fn harness_found(named: Option<&Path>) -> Result<HarnessFound, Error> {
    if named.is_some() {
        return Ok(found(How::Named));
    }
    Ok(match search()? {
        Ok(harness) => found(harness.how),
        Err(missed) => HarnessFound { found: false, how: How::None, says: none(&missed) },
    })
}

fn found(how: How) -> HarnessFound {
    let says = match how {
        How::Named => "Claude Code is the program JELLYMOON_CLAUDE names, and was not looked for",
        How::Path => "Claude Code was found on PATH",
        How::FixedPlace => "Claude Code was found in a fixed place, as PATH has none",
        How::LoginShell => {
            "Claude Code was found in a folder of the login shell's PATH, as PATH and the fixed places have none"
        }
        How::None => "Claude Code was not found",
    };
    HarnessFound { found: how != How::None, how, says: says.to_string() }
}

/// The sentence of a look that found no harness: where it looked, and what became of the login shell.
fn none(missed: &Missed) -> String {
    const NOT: &str = "Claude Code was not found on PATH or in a fixed place";
    let mut says = match (missed.shell, missed.shell.why()) {
        (_, Some(why)) => format!("{NOT}, and none came of the login shell: {why}"),
        (Shell::Off, _) => format!("{NOT}, and the login shell was not asked, as JELLYMOON_SHELL is set to nothing"),
        (Shell::NotHere, _) => format!("{NOT}, and on Windows the login shell is not asked"),
        _ => "Claude Code was not found on PATH, in a fixed place or in a folder of the login shell's PATH".to_string(),
    };
    if !missed.passed_over.is_empty() {
        says.push_str("; a `claude` program was passed over, as others can write to it or to a folder it is in");
    }
    says
}

#[cfg(test)]
mod tests {
    use super::*;

    fn missed(shell: Shell, passed_over: &[&str]) -> Missed {
        let passed_over = passed_over.iter().map(ToString::to_string).collect();
        Missed { looked: vec!["PATH".to_string()], passed_over, home: true, shell }
    }

    // On every system, Windows too: each end has a sentence, and none holds a folder.
    #[test]
    fn each_end_of_a_look_that_found_none_has_its_sentence_and_names_no_folder() {
        let not = "Claude Code was not found on PATH or in a fixed place";
        assert_eq!(none(&missed(Shell::NotHere, &[])), format!("{not}, and on Windows the login shell is not asked"));
        assert_eq!(
            none(&missed(Shell::Off, &[])),
            format!("{not}, and the login shell was not asked, as JELLYMOON_SHELL is set to nothing")
        );
        assert_eq!(
            none(&missed(Shell::TooLong, &["~/.local/bin/claude: others can write to it"])),
            format!(
                "{not}, and none came of the login shell: it did not answer in 5 seconds; a `claude` program was \
                 passed over, as others can write to it or to a folder it is in"
            )
        );
        let line = HarnessFound { found: false, how: How::None, says: none(&missed(Shell::Answered, &[])) };
        assert_eq!(
            line.to_string(),
            "{\"found\":false,\"how\":\"none\",\"says\":\"Claude Code was not found on PATH, in a fixed place or in \
             a folder of the login shell's PATH\"}"
        );
        for how in [How::Named, How::Path, How::FixedPlace, How::LoginShell] {
            let line = found(how).to_string();
            assert!(line.starts_with("{\"found\":true,\"how\":\"") && !line.contains('/'), "{line}");
        }
    }
}
