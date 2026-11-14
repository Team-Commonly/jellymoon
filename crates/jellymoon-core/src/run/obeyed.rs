//! Whether the person's tools obey a file of a run's work, by its name (ADR 0017, decision 6;
//! `docs/development/show.md`, "Files the person's tools obey"). The name is an agent's, and a disk takes more
//! spellings for one name than a comparison of letters does. So each part of a path is read as the disks read it
//! before it is compared: what a disk leaves out is left out, and what a disk takes for a letter is that letter.
//! The mark is a warning: a spelling marked that no disk folds costs the person one more yes, and one missed is
//! the fault. [`FOLDS`] and [`IGNORED`] were measured on a Mac's disks and are asked of the disk again by this
//! file's tests; what is there for Windows was reasoned and not run.

use std::ops::RangeInclusive;

use super::show::NAME_MAX;
use crate::still::statement;

/// A file's name as it is printed, and whether the person's tools obey the file. Asked of the name as git keeps
/// it and as it is printed: a character nobody sees must not hide one. The one rule of a look, a keep and a page.
pub(super) fn named(path: &[u8]) -> (String, bool) {
    let raw = String::from_utf8_lossy(path);
    let name = statement(&raw, NAME_MAX);
    let obeys = obeyed(&raw) || obeyed(&name);
    (name, obeys)
}

/// Names the person's tools obey, wherever they are in a path: the settings, rules, hooks and servers of coding
/// agents and editors, what a shell, git or cargo loads on entering the folder, and what a forge runs on a push.
/// A name with a `/` is obeyed only as those folders, one inside the other. A warning, not a wall: work that
/// touches none of these is still code in the person's project, and a tool this list does not know is obeyed all
/// the same.
const OBEYED: [&str; 19] = [
    ".claude",
    ".codex",
    ".cursor",
    ".cursorrules",
    ".vscode",
    ".idea",
    ".husky",
    ".pre-commit-config.yaml",
    ".devcontainer",
    ".cargo",
    "CLAUDE.md",
    "AGENTS.md",
    "GEMINI.md",
    ".mcp.json",
    ".envrc",
    ".gitmodules",
    ".gitattributes",
    ".github/workflows",
    ".github/copilot-instructions.md",
];

/// Each character a disk takes for ASCII letters, with the letters. APFS takes every one but `ı` (measured: every
/// character of the Basic Multilingual Plane, against every name of one, two and three ASCII letters, digits,
/// `.`, `-` and `_`). `ı` is NTFS's, whose table of capitals makes it `I`: reasoned, not run.
const FOLDS: [(char, &str); 12] = [
    ('\u{df}', "ss"),
    ('\u{131}', "i"),
    ('\u{17f}', "s"),
    ('\u{1e9e}', "ss"),
    ('\u{212a}', "k"),
    ('\u{fb00}', "ff"),
    ('\u{fb01}', "fi"),
    ('\u{fb02}', "fl"),
    ('\u{fb03}', "ffi"),
    ('\u{fb04}', "ffl"),
    ('\u{fb05}', "st"),
    ('\u{fb06}', "st"),
];

/// The characters HFS+ leaves out of a name wherever they stand in it (measured the same way: sixteen, and APFS
/// leaves out none).
const IGNORED: [RangeInclusive<char>; 4] =
    ['\u{200c}'..='\u{200f}', '\u{202a}'..='\u{202e}', '\u{206a}'..='\u{206f}', '\u{feff}'..='\u{feff}'];

/// One part of a path as the disks read it: cut at a `:` (after it Windows names a stream of the same file),
/// without what is [`IGNORED`], with each of the [`FOLDS`] as its letters, and without the dots and spaces at its
/// end, which Windows drops. The case of letters is left for the comparison.
fn folded(part: &str) -> String {
    let mut plain = String::new();
    let named = part.split(':').next().unwrap_or(part);
    for c in named.chars().filter(|c| !IGNORED.iter().any(|left_out| left_out.contains(c))) {
        match FOLDS.iter().find(|(taken, _)| *taken == c) {
            Some((_, letters)) => plain.push_str(letters),
            None => plain.push(c),
        }
    }
    plain.truncate(plain.trim_end_matches(['.', ' ']).len());
    plain
}

/// Whether a part has the shape of a short name, such as `CLAUDE~1`, which a Windows disk gives a file beside
/// its own name: a `~` and then a digit. Which name it is short for cannot be read from it, so every one is
/// marked.
fn short(part: &str) -> bool {
    part.as_bytes().windows(2).any(|two| two[0] == b'~' && two[1].is_ascii_digit())
}

/// Whether any folder or file of the path has one of the [`OBEYED`] names as a disk reads it, in any case of
/// ASCII letters. `/` and `\` both part a path.
pub(super) fn obeyed(path: &str) -> bool {
    let parts: Vec<String> = path.split(['/', '\\']).map(folded).collect();
    let named = |name: &&str| {
        let wanted: Vec<&str> = name.split('/').collect();
        let same = |have: &[String]| have.iter().zip(&wanted).all(|(have, wanted)| have.eq_ignore_ascii_case(wanted));
        parts.windows(wanted.len()).any(same)
    };
    parts.iter().any(|part| short(part)) || OBEYED.iter().any(named)
}

/// Whether any folder or file of the path is `.gitmodules` as a disk reads it: the file that names the repos
/// git fetches into a checkout. Work that touches one is never kept (`keep.rs`).
pub(super) fn modules(path: &str) -> bool {
    path.split(['/', '\\']).map(folded).any(|part| part.eq_ignore_ascii_case(".gitmodules"))
}

#[cfg(test)]
#[path = "obeyed/tests.rs"]
mod tests;
