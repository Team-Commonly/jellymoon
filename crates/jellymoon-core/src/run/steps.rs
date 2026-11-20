//! What a run is doing now (live work; `docs/development/run.md`, "What is read of the stream"). Each tool use of
//! the five a run has (`stream.rs`, `TOOLS`) in an `assistant` line of its stream is a step: which tool, as a closed
//! list, and of `Edit`, `Read` and `Write` the file it names, as core's cleaned name of a file inside the run's
//! place, or no name. Nothing else of the line is kept: no text of the model's, no other input, no result. Every
//! step is counted and the last [`KEPT`] are kept, in `runs/<id>/steps.json`, written by the stream's reader as
//! steps come and left as it was at the run's end. A page reads it with [`doing`]: it is a file's word, read as
//! this shape and no other, with every name cleaned again.

use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::{Component, Path, PathBuf};
use std::process;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::obeyed::{named, plain};
use super::record;
use super::show::NAME_MAX;
use crate::{copy, tap, wish};

/// The most steps whose tool and name are kept: the last ones.
pub(super) const KEPT: usize = 4;
/// The file of a run's folder its steps are kept in, and the most bytes of it that are read.
const FILE: &str = "steps.json";
const FILE_MAX: usize = 4096;

/// A tool of a run's, by its name in the stream: one of five, and never the stream's text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tool {
    Edit,
    Glob,
    Grep,
    Read,
    Write,
}

/// One step: the tool, and of `Edit`, `Read` and `Write` the cleaned name of the file it names inside the run's
/// place, relative to it, when that name passed. `Glob` and `Grep` carry none.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Step {
    pub tool: Tool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
}

/// A run's steps so far: how many came, and the last [`KEPT`], oldest first.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Steps {
    pub count: usize,
    pub last: Vec<Step>,
}

/// Where the reader keeps a run's steps, the folders a step's file must be inside, and the steps it goes on from:
/// those kept already, when the run is answered and its harness started again.
pub(super) struct Live {
    pub file: PathBuf,
    pub places: Vec<PathBuf>,
    pub from: Steps,
}

impl Live {
    /// Of the run `run` working in `folder`. The folder is taken as given and as the system resolves it: a harness
    /// names files by the folder it was started in, and may name it either way.
    pub(super) fn of(home: &Path, run: &str, folder: &str) -> Live {
        let (places, file) = ([Some(PathBuf::from(folder)), fs::canonicalize(folder).ok()], file(home, run));
        Live { places: places.into_iter().flatten().collect(), from: kept(&file), file }
    }
}

/// A line's `type` alone, read first: every other part is passed over unparsed, and a line of another type is not
/// parsed again.
#[derive(Deserialize)]
struct Kind {
    #[serde(rename = "type")]
    kind: String,
}

/// The parts of an `assistant` line a step is read from. A line of another shape has no step.
#[derive(Deserialize)]
struct Said {
    message: Message,
}

#[derive(Deserialize)]
struct Message {
    content: Vec<Value>,
}

impl Tool {
    pub(super) fn of(name: &str) -> Option<Tool> {
        match name {
            "Edit" => Some(Tool::Edit),
            "Glob" => Some(Tool::Glob),
            "Grep" => Some(Tool::Grep),
            "Read" => Some(Tool::Read),
            "Write" => Some(Tool::Write),
            _ => None,
        }
    }

    /// Whether a step of this tool names a file.
    fn names(self) -> bool {
        matches!(self, Tool::Edit | Tool::Read | Tool::Write)
    }
}

impl Steps {
    /// Takes the steps of one line, and says whether it had any: each block of an `assistant` line's
    /// `message.content` that is a `tool_use` whose `name` is one of the five.
    pub(super) fn took(&mut self, raw: &[u8], places: &[PathBuf]) -> bool {
        self.took_as(raw, |path| inside(path, places))
    }

    /// [`Steps::took`] with the naming given. Only the last [`KEPT`] steps of a line can stay kept, so only they
    /// are named: a line of many steps costs [`KEPT`] names, and the others are counted alone.
    fn took_as(&mut self, raw: &[u8], mut name: impl FnMut(&str) -> Option<String>) -> bool {
        if !serde_json::from_slice::<Kind>(raw).is_ok_and(|line| line.kind == "assistant") {
            return false;
        }
        let Ok(said) = serde_json::from_slice::<Said>(raw) else { return false };
        let tool = |block: &Value| {
            text(block, "/name").filter(|_| text(block, "/type") == Some("tool_use")).and_then(Tool::of)
        };
        let steps: Vec<(&Value, Tool)> =
            said.message.content.iter().filter_map(|block| Some((block, tool(block)?))).collect();
        let named = steps.len().saturating_sub(KEPT);
        for (nth, (block, tool)) in steps.iter().enumerate() {
            let path = text(block, "/input/file_path").filter(|_| tool.names() && nth >= named);
            self.push(Step { tool: *tool, file: path.and_then(&mut name) });
        }
        !steps.is_empty()
    }

    fn push(&mut self, step: Step) {
        self.count = self.count.saturating_add(1);
        self.last.push(step);
        if self.last.len() > KEPT {
            self.last.remove(0);
        }
    }
}

/// The text at `at` of a block of a line.
fn text<'a>(block: &'a Value, at: &str) -> Option<&'a str> {
    block.pointer(at).and_then(Value::as_str)
}

/// A path as core names a file of the run's: relative to one of `places`, as [`own`] names it. None for a path
/// outside every place, for one [`own`] does not name, and for one with a part under the place, the last
/// included, that is a link on disk: it may lead out of the place. The name comes first: it bounds the path, so
/// the disk is asked of a short one only. A part is asked with `symlink_metadata`, which follows nothing and opens
/// no file; one that is not there is no link and has nothing behind it, so a file about to be made keeps its
/// name. A part that cannot be asked is taken as a link.
fn inside(path: &str, places: &[PathBuf]) -> Option<String> {
    let (place, within) = places.iter().find_map(|place| Some((place, Path::new(path).strip_prefix(place).ok()?)))?;
    let name = own(within)?;
    let mut at = place.clone();
    for part in within.components() {
        at.push(part);
        match fs::symlink_metadata(&at) {
            Ok(meta) if !meta.file_type().is_symlink() => {}
            Err(error) if error.kind() == ErrorKind::NotFound => break,
            _ => return None,
        }
    }
    Some(name)
}

/// A relative path as core names a file: its parts joined by `/`, printed so that a person reads it as it is
/// (`obeyed.rs`, `plain`). None for no part, a part that steps out or is a root, one that is not text, and a name
/// cleaning changes.
fn own(within: &Path) -> Option<String> {
    let parts = within.components().map(|part| match part {
        Component::Normal(part) => part.to_str(),
        _ => None,
    });
    let parts: Vec<&str> = parts.collect::<Option<_>>()?;
    let joined = parts.join("/");
    // What cleaning would cut or change is refused before it is cleaned: cleaning costs more than its length.
    if parts.is_empty() || joined.len() > 4 * NAME_MAX || joined.chars().any(char::is_control) {
        return None;
    }
    let (name, _) = named(joined.as_bytes());
    plain(joined.as_bytes(), &name).then_some(name)
}

fn file(home: &Path, run: &str) -> PathBuf {
    copy::kept_at(home, run).join(FILE)
}

/// Writes the steps so far over the kept ones, whole or not at all: a new file beside it, only its owner's, then
/// renamed over it. Not flushed to the disk: it is written at every step, on the reader's thread, and a step lost
/// to a crash is no harm. One that cannot be written is left as it was, with one try: the next step writes again.
pub(super) fn keep(file: &Path, steps: &Steps) {
    let Ok(written) = serde_json::to_string(steps) else { return };
    let mut name = file.as_os_str().to_os_string();
    name.push(format!(".jellymoon-{}", process::id()));
    let new = PathBuf::from(name);
    let mut open = OpenOptions::new();
    open.write(true).create_new(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut open, 0o600);
    let wrote = open.open(&new).and_then(|mut made| made.write_all(format!("{written}\n").as_bytes()));
    if wrote.and_then(|()| fs::rename(&new, file)).is_err() {
        let _ = fs::remove_file(&new);
    }
}

/// The steps kept in `file`: a plain file of the shape, within its size, that only its owner can write to, with
/// no more than [`KEPT`] steps and a count no smaller. A name core would not give is no name.
/// None of anything else is read: no steps.
fn kept(file: &Path) -> Steps {
    let read = tap::plain(file, FILE_MAX).ok().and_then(|bytes| serde_json::from_slice::<Steps>(&bytes).ok());
    let fits = |steps: &Steps| steps.last.len() <= KEPT && steps.count >= steps.last.len();
    let mut steps = read.filter(fits).unwrap_or_default();
    for step in &mut steps.last {
        let names = step.tool.names();
        step.file = step.file.take().filter(|name| names && own(Path::new(name)).as_ref() == Some(name));
    }
    steps
}

/// The steps of each made wish of `day` whose run kept any, by the wish's id: of a run that goes, the steps so
/// far; of one that ended, the steps it ended with. A run whose record is not of that wish and day has none here.
pub fn doing(home: &Path, day: &str) -> Vec<(String, Steps)> {
    let made = wish::kept_of(home, day).unwrap_or_default();
    let ran = |wish: &wish::MadeWish| {
        let run = wish.run.as_deref().filter(|run| copy::is_run_id(run))?;
        let own = record::kept(home, run).is_some_and(|record| record.wish == wish.id && record.day == day);
        let steps = kept(&file(home, run));
        (own && steps.count > 0).then(|| (wish.id.clone(), steps))
    };
    made.iter().filter_map(ran).collect()
}

#[cfg(test)]
#[path = "steps/tests.rs"]
mod tests;
