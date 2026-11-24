//! What a finished run says of its own work (F85, ADR 0023; `docs/development/show.md`, "What the run says of its
//! work"): a `result`, a short list of pieces from a closed catalogue, read from the run's last word beside `done`
//! ([`said`]), kept as `runs/<id>/result.json` ([`keep`]) and checked against the work at every look
//! ([`checked`]), never when it is kept. Every word of it is a model's, and the file is anyone's to write. So
//! nothing here is printed as Jellymoon's own: a text passes the door a question passes, a quoted line is printed
//! only when it is a whole line the work took from its file or put in it, a file is named by its number and
//! core's own name for it, and a piece that fails is dropped whole and counted. No word of a result is logged,
//! put in a record or said in a refusal.

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::obeyed::{counted, plain};
use super::show::{self, FileChange, FileKind};
use crate::copy::{self, BEFORE, TEXT_MAX, WorkFile};
use crate::hook::put;
use crate::redact::holds_unseen;
use crate::still::{marks_only, still, still_text};
use crate::tap;

/// The most bytes of a result as it is kept, its line break with it. A longer one is no result.
pub(super) const SAID_MAX: usize = 8 * 1024;
/// The most pieces of a result that are kept.
const KEPT: usize = 12;
/// The file of a run's folder a result is kept in.
const FILE: &str = "result.json";
/// The most characters of what a run did, and of what it decided alone or is not sure of.
const DID_MAX: usize = 280;
const SAYS_MAX: usize = 160;
/// The most quoted lines on one side of a change, and the most characters of one.
const LINES: usize = 3;
const LINE_MAX: usize = 100;

/// The kinds of the catalogue, in the order they are printed, whatever order the run wrote.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Did,
    Change,
    Decided,
    Unsure,
}

/// How many pieces of each kind are looked at: the first ones. A later one is left out, and is not counted as
/// dropped, since it failed no check.
const ORDER: [(Kind, usize); 4] = [(Kind::Did, 1), (Kind::Change, BEFORE), (Kind::Decided, 3), (Kind::Unsure, 3)];

/// One piece as a run wrote it. Nothing of it is checked yet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum SaidPiece {
    Did {
        text: String,
    },
    Change {
        file: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        before: Vec<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        after: Vec<String>,
    },
    Decided {
        text: String,
    },
    Unsure {
        text: String,
    },
}

/// What a run said of its work, as it is kept: the shape, and nothing checked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Said {
    pieces: Vec<SaidPiece>,
}

/// One piece that passed its check, as it is printed. A text is the run's, through the door. Of a change, `n` is
/// the file's number in the list and `name` core's cleaned name of it, never the run's spelling; `made` says the
/// work made the file; and each line is a whole line of the file, without the spaces and tabs at its ends.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Piece {
    Did { text: String },
    Decided { text: String },
    Unsure { text: String },
    Change { n: usize, name: String, made: bool, before: Vec<String>, after: Vec<String> },
}

/// What a run says of its work, checked against the work: `result` in what `run show` and `run waiting` print.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Told {
    /// The pieces that passed, in the catalogue's order: `did`, `change`, `decided`, `unsure`.
    pub pieces: Vec<Piece>,
    /// How many pieces failed a check and are not there.
    pub dropped: usize,
}

/// The result a last word carries beside `done`, when it is the shape and within its size: its first [`KEPT`]
/// pieces. None of anything else, which is no result and fails no run. The size is that of the result as core
/// writes it, on one line with no space between its parts.
pub(super) fn said(result: &Value) -> Option<Said> {
    let mut said = Said::deserialize(result).ok()?;
    let fits = serde_json::to_string(&said).is_ok_and(|written| written.len() < SAID_MAX);
    said.pieces.truncate(KEPT);
    fits.then_some(said)
}

/// Keeps what a run said, in its own folder, for the person alone. A result that cannot be written is no
/// result: the run has ended as it would have without one.
pub(super) fn keep(home: &Path, run: &str, said: &Said) {
    if let Ok(written) = serde_json::to_string(said) {
        let _ = put(&copy::kept_at(home, run).join(FILE), &format!("{written}\n"), false);
    }
}

/// What is kept as a run's result: a plain file of the shape, of at most [`SAID_MAX`] bytes, that only its owner
/// can write to. None of anything else. It is a file's word and no more than that: every piece is checked.
pub(super) fn kept(home: &Path, run: &str) -> Option<Said> {
    let bytes = tap::plain(&copy::kept_at(home, run).join(FILE), SAID_MAX).ok()?;
    let mut said: Said = serde_json::from_slice(&bytes).ok()?;
    said.pieces.truncate(KEPT);
    Some(said)
}

impl SaidPiece {
    fn kind(&self) -> Kind {
        match self {
            SaidPiece::Did { .. } => Kind::Did,
            SaidPiece::Change { .. } => Kind::Change,
            SaidPiece::Decided { .. } => Kind::Decided,
            SaidPiece::Unsure { .. } => Kind::Unsure,
        }
    }

    /// The piece as it is printed, or None: it failed its check.
    fn check(&self, files: &[WorkFile]) -> Option<Piece> {
        match self {
            SaidPiece::Did { text } => clean(text, DID_MAX).map(|text| Piece::Did { text }),
            SaidPiece::Decided { text } => clean(text, SAYS_MAX).map(|text| Piece::Decided { text }),
            SaidPiece::Unsure { text } => clean(text, SAYS_MAX).map(|text| Piece::Unsure { text }),
            SaidPiece::Change { file, before, after } => change(file, before, after, files),
        }
    }
}

/// The paths of the files a check will want as they were before the work: those the first changes name. They
/// are compared with the paths git printed and are never given to git (`copy/work.rs`).
pub(super) fn wanted(said: &Said) -> Vec<&[u8]> {
    fn named(piece: &SaidPiece) -> Option<&[u8]> {
        match piece {
            SaidPiece::Change { file, .. } => Some(file.as_bytes()),
            _ => None,
        }
    }
    said.pieces.iter().filter_map(named).take(BEFORE).collect()
}

/// What a run said, checked against the listed files of its work as they were read now. None when no piece
/// passed and none was dropped: there is nothing to print.
pub(super) fn checked(said: &Said, files: &[WorkFile]) -> Option<Told> {
    let mut told = Told { pieces: Vec::new(), dropped: 0 };
    for (kind, most) in ORDER {
        for piece in said.pieces.iter().filter(|piece| piece.kind() == kind).take(most) {
            match piece.check(files) {
                Some(passed) => told.pieces.push(passed),
                None => told.dropped += 1,
            }
        }
    }
    (!told.pieces.is_empty() || told.dropped > 0).then_some(told)
}

/// A text as it is printed, or None. It passes the door a question passes (`still.rs`) as the run wrote it, its
/// line breaks with it: the door makes one line of it, a credential's shape becomes the mark, one a line break
/// splits too, and a `"` a `'`. It is dropped, never cut, when it holds a character that draws as nothing (asked
/// of the text with every space read as a plain one, and again of what the door made), when nothing or only
/// marks are left of it, when it is over `most` characters, and when it reads like a note's count (`obeyed.rs`).
pub(super) fn clean(raw: &str, most: usize) -> Option<String> {
    let spaced = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    if holds_unseen(&spaced) {
        return None;
    }
    let made = still(raw, most, false).ok()?;
    let read = !made.is_empty() && !marks_only(&made) && !holds_unseen(&made) && !counted(&made);
    read.then_some(made)
}

/// A line's spaces and tabs at both ends are no part of it: in the quote, and in the file.
fn ends_off(line: &str) -> &str {
    line.trim_matches([' ', '\t'])
}

/// A quoted line as it is printed, without its ends, or None: it is empty or over [`LINE_MAX`] characters, holds
/// a line break, a tab or a character that draws as nothing, reads like a note's count, or is one the cleaning
/// of a file's text would change (`still_text`), as a credential's shape is. Spaces in a row inside it are the
/// line's own: code is typed so.
fn line(raw: &str) -> Option<&str> {
    let line = ends_off(raw);
    let spaced = line.split(' ').filter(|word| !word.is_empty()).collect::<Vec<_>>().join(" ");
    let fits = !line.is_empty() && line.chars().count() <= LINE_MAX;
    let read = fits && !holds_unseen(&spaced) && !counted(line);
    (read && still_text(line, LINE_MAX).is_ok_and(|(made, cut)| made == line && !cut)).then_some(line)
}

/// A file of the work as text, as it was before the work and as it is after: an empty text for the side a made
/// or a removed file does not have. None when a side is not text or was not read whole.
fn sides(work: &WorkFile, change: FileChange) -> Option<(&str, &str)> {
    let head = text(&work.head).filter(|_| work.size <= TEXT_MAX as u64)?;
    match change {
        FileChange::Made => Some(("", head)),
        FileChange::Removed => Some((head, "")),
        FileChange::Changed => Some((text(work.before.as_deref().filter(|was| was.len() <= TEXT_MAX)?)?, head)),
    }
}

/// Bytes as a text: every one of them is of a character, and none is a zero.
fn text(bytes: &[u8]) -> Option<&str> {
    std::str::from_utf8(bytes).ok().filter(|text| !text.contains('\0'))
}

/// The quoted lines of one side as they are printed, or None when one of them fails.
fn quoted(side: &[String]) -> Option<Vec<&str>> {
    side.iter().map(|raw| line(raw)).collect()
}

/// A change as it is printed, or None. `file` is, byte for byte, the path of a listed file of the work that is
/// text, whose printed name is the name git keeps. It quotes one line at least and [`LINES`] a side at most.
/// Each `before` line is a whole line the file has more often before the work than after, and each `after` line
/// one it has more often after: a line that is there as often as it was is neither. So a made file passes no
/// `before` line and a removed one no `after` line. Each side is also cleaned once, whole, as a look at the file
/// cleans it (`still_text`), and a quoted line must be a whole line of its side cleaned so: a line alone can be
/// no credential's shape and still be a part of one, as a body line of a private key block is. One line that
/// fails drops the piece.
fn change(file: &str, before: &[String], after: &[String], files: &[WorkFile]) -> Option<Piece> {
    let (at, work) = files.iter().enumerate().find(|(_, work)| work.path == file.as_bytes())?;
    let shown = show::entry(at + 1, work);
    let bound = before.len() + after.len() > 0 && before.len() <= LINES && after.len() <= LINES;
    if !bound || shown.kind != FileKind::Text || !plain(&work.path, &shown.name) {
        return None;
    }
    let (was, is) = sides(work, shown.change)?;
    let (before, after) = (quoted(before)?, quoted(after)?);
    let times = |text: &str, line: &str| text.lines().filter(|held| ends_off(held) == line).count();
    let cleaned = |side: &str, lines: &[&str]| {
        let whole = |(side, _): (String, bool)| lines.iter().all(|line| times(&side, line) > 0);
        lines.is_empty() || still_text(side, TEXT_MAX).is_ok_and(whole)
    };
    let took = cleaned(was, &before) && before.iter().all(|line| times(was, line) > times(is, line));
    let put = cleaned(is, &after) && after.iter().all(|line| times(is, line) > times(was, line));
    let own = |lines: Vec<&str>| lines.into_iter().map(str::to_string).collect();
    let made = shown.change == FileChange::Made;
    (took && put).then(|| Piece::Change { n: shown.n, name: shown.name, made, before: own(before), after: own(after) })
}

#[cfg(test)]
#[path = "told/tests.rs"]
mod tests;
