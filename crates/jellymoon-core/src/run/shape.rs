//! The wish is shaped before a run (F91, ADR 0025). Between the words a person typed and the run of them, their own
//! Claude Code is asked once, headless and read-only, to say the same wish sharply: [`shape`]. It starts in a new
//! folder of Jellymoon's own, with the project's folder named by `--add-dir`, a fixed command line ([`LINE`]) and
//! fixed system words ([`PROMPT`]). Every word of the answer is a model's, so the answer is checked whole
//! ([`checked`]) and thrown away whole when any part fails: then the wish is as typed, and why is said in
//! Jellymoon's own fixed words, never the model's. A checked shape that is not `clear` waits in
//! `pages/<day>.shape.json`, one a day, with the words and the project it is of. A wish is kept from it only for
//! those same words ([`wish_shaped`], [`wish_of_shape`]): then the wish carries the run's prompt ([`prompt`]),
//! which core builds from fixed labels and ends with the person's own words, after their answer to the shape's
//! question when they gave one ([`ShapeGo`]).

use std::path::{Path, PathBuf};
use std::time::Duration;
use std::{fmt, fs, io};

use serde::{Deserialize, Serialize};

use super::{RunOptions, place, told};
use crate::busy::or_test;
use crate::call::{self, Reply};
use crate::hook::put;
use crate::reading::unfence;
use crate::still::{MARKS, Unkept, marks_only, still};
use crate::wish::{self, Wished};
use crate::{DayOptions, Error, find, tap};

/// The model the shaping call asks the harness for: a small one, since the call stands between Enter and a run.
pub const SHAPE_MODEL: &str = "haiku";
/// The shaping call's whole command line but for its last three arguments: the system words, and `--add-dir`
/// with the project's folder. Read-only: no shell, no edit, no MCP server, no permission asked, no session kept.
const LINE: &str = "-p --restricted --strict-mcp-config --permission-prompts none --tools Read Glob Grep --model haiku \
                    --disable-slash-commands --no-session-persistence --output-format json --system-prompt";
/// The whole system prompt of the shaping call: fixed words, so nothing of a wish or a project can change them.
const PROMPT: &str = "A person typed a wish for the project in the folder named below. They may not program. You can \
                      read files in that folder and nothing else: you change nothing and you run nothing. Read only \
                      what you need, quickly. Then answer with one JSON object and no other word: {\"clear\": true or \
                      false, \"wish\": \"\", \"done_when\": \"\", \"not_touching\": \"\", \"why\": \"\", \"ask\": \
                      \"\"}. `clear` is true when the words as typed already say exactly what should be different; \
                      then leave every text empty. Otherwise `wish` says the same wish sharply, in one or two plain \
                      sentences; `done_when` says what the person will see when it is done; `not_touching` says what \
                      is left alone; `why` says in one sentence what you read that made you put it this way; and \
                      `ask` is one question that only the person can answer, or empty. Each text is under 120 \
                      characters. Plain words, and no file names unless the person used them.";
/// The two lines of the call's input, each before what core puts after it: the folder, then the typed words.
const IN_FOLDER: &str = "The project is in the folder: ";
const THE_WISH: &str = "The wish: ";

/// How long the shaping call may take, and how many bytes of what it prints are read.
const LIMIT: Duration = Duration::from_secs(20);
const READ: usize = 64 * 1024;
/// The most characters of a shape's wish, of each of its other texts, and of the run's prompt made of them: the
/// longest of everything, with the words and an answer of a wish's length each, is 1280.
const WISH_MAX: usize = 280;
const TEXT_MAX: usize = 160;
const SHAPED_MAX: usize = 1300;
/// The most bytes of a kept shape that are read.
const KEPT_MAX: usize = 16 * 1024;

/// Why a wish is as typed, in Jellymoon's own words. No word of a model's or a harness's is in any of them.
const NO_WISH: &str = "these words are not ones a wish is kept of, so nothing was read";
const NO_FOLDER: &str = "no one folder of that project was found, so nothing was read";
const NO_HARNESS: &str = "Claude Code was not found, so the wish was not read";
const UNASKED: &str = "Claude Code could not be started to read the wish";
const TOO_LONG: &str = "Claude Code took too long to read the wish, and was stopped";
const UNANSWERED: &str = "Claude Code did not answer as a reading does, so its answer was not used";
const UNCHECKED: &str = "the answer did not pass Jellymoon's check, so none of it was used";
const UNKEPT: &str = "the sharper wish could not be kept, so it was not used";
/// What refuses a tap on a shape that waits no more.
const GONE: &str = "the sharper wish on show waits no more, so nothing was kept";
/// What refuses an answer: the shape asks nothing; nothing is left of the answer once cleaned; it is longer than
/// a wish's words; no prompt can be built with it. Nothing is kept then, and the shape still waits.
const NO_ASK: &str = "the sharper wish on show asks nothing, so the answer was not used and nothing was kept";
const NO_ANSWER: &str = "an answer needs words, and nothing is left of these once cleaned, so nothing was kept";
const LONG_ANSWER: &str = "an answer is at most 200 characters, and this one is longer, so nothing was kept";
const UNBUILT: &str = "no run could be given the sharper wish with this answer, so nothing was kept";

/// How a shaping ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShapeOutcome {
    /// The words as typed are sharp enough to start: nothing is kept.
    Clear,
    /// A sharper wish passed the check and waits.
    Shaped,
    /// There is no sharper wish: the wish is the typed words.
    AsTyped,
}

/// The model's answer, and the sharper wish as it is kept and printed. Every text is a model's, as the door left
/// it: data to draw as a quote, and never a title, a label or a choice.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Shape {
    pub clear: bool,
    pub wish: String,
    pub done_when: String,
    pub not_touching: String,
    pub why: String,
    pub ask: String,
}

/// What `jellymoon shape` prints. `why` is Jellymoon's fixed words, of a wish that is as typed and of no other.
/// `said` is the typed words as a wish keeps them, beside a shape that waits for them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Shaped {
    pub outcome: ShapeOutcome,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub why: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub said: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shape: Option<Shape>,
}

impl fmt::Display for Shaped {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&serde_json::to_string(self).map_err(|_| fmt::Error)?)
    }
}

/// The shape that waits, `pages/<day>.shape.json`: the cleaned words and the project it is of.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Kept {
    day: String,
    said: String,
    project: String,
    shape: Shape,
}

fn file(home: &Path, day: &str) -> PathBuf {
    home.join("pages").join(format!("{day}.shape.json"))
}

fn as_typed(why: &str) -> Shaped {
    Shaped { outcome: ShapeOutcome::AsTyped, why: Some(why.to_string()), said: None, shape: None }
}

/// Shapes the words `typed` for `project`: the words are checked as a wish's are, the project's folder is found
/// as a run finds it, and only then is the harness asked. It never fails: whatever goes wrong, the wish is as
/// typed. The shape that waited goes first, so none of other words is left when this one ends.
pub fn shape(home: &Path, options: &RunOptions, typed: &str, project: &str) -> Shaped {
    let words = wish::day_of(options.day.as_deref()).and_then(|day| Ok((day, wish::words(typed)?)));
    let Ok((day, said)) = words else { return as_typed(NO_WISH) };
    let _ = fs::remove_file(file(home, &day));
    let Ok((folder, _)) = place::found(home, &day, project, options) else { return as_typed(NO_FOLDER) };
    let shape = match asked(home, options, &folder, &said) {
        Ok(answer) => checked(&answer),
        Err(why) => return as_typed(why),
    };
    let Some(shape) = shape else { return as_typed(UNCHECKED) };
    if shape.clear {
        return Shaped { outcome: ShapeOutcome::Clear, why: None, said: None, shape: None };
    }
    // A sharper wish no run could be given is not one to show: `Go with this` would go on with other words.
    if prompt(&shape, &said, None).is_none() {
        return as_typed(UNCHECKED);
    }
    let kept = Kept { day: day.clone(), said, project: project.to_string(), shape };
    let written = serde_json::to_string(&kept).map_err(io::Error::from).and_then(|text| {
        call::private_dir(&home.join("pages"))?;
        put(&file(home, &day), &format!("{text}\n"), false)
    });
    match written {
        Ok(()) => Shaped { outcome: ShapeOutcome::Shaped, why: None, said: Some(kept.said), shape: Some(kept.shape) },
        Err(_) => as_typed(UNKEPT),
    }
}

/// Makes the shaping call and returns the model's text, or Jellymoon's words for why there is none. The harness
/// starts in a new folder under Jellymoon's home, removed afterwards, and never in the project: what a project
/// holds for a harness that starts in it (settings, hooks, instructions) is not what this call is started with.
fn asked(home: &Path, options: &RunOptions, folder: &Path, said: &str) -> Result<String, &'static str> {
    let harness = find::named_or_found(options.harness.as_deref()).map_err(|_| NO_HARNESS)?;
    let dir = home.join("shaping");
    call::private_dir(&dir).map_err(|_| UNASKED)?;
    let own = tempfile::Builder::new().prefix("call-").tempdir_in(&dir).map_err(|_| UNASKED)?;
    let mut command = call::harness_of(harness).map_err(|_| UNASKED)?;
    command.current_dir(own.path()).args(LINE.split(' ')).arg(PROMPT).arg("--add-dir").arg(folder);
    let input = format!("{IN_FOLDER}{}\n{THE_WISH}{said}\n", folder.display());
    let limit = or_test(LIMIT, "JELLYMOON_TEST_SHAPE_LIMIT_MS");
    let output = call::run_most(command, input.into_bytes(), limit, None, READ)
        .map_err(|e| if matches!(e, Error::TooLong(_)) { TOO_LONG } else { UNASKED })?;
    let reply = serde_json::from_slice::<Reply>(&output.stdout).ok();
    reply.filter(|reply| output.status.success() && !reply.is_error).map(|reply| reply.result).ok_or(UNANSWERED)
}

/// The model's answer as it is used, or None: it is thrown away whole. It is under 8 KiB, and one object of the
/// six fields and no other. A `clear` answer carries no text. Of any other, the wish passes the door a run's
/// result passes (`told::clean`: one line, no credential, nothing unseen, not only marks) within its length,
/// and each other text does too, or is empty. No text holds a mark the run's prompt is built with ([`prompt`]): a
/// model's text could else read, to the run, as a label of Jellymoon's or as the person's own words.
fn checked(answer: &str) -> Option<Shape> {
    let raw: Shape = serde_json::from_str(unfence(answer)).ok().filter(|_| answer.len() < told::SAID_MAX)?;
    if raw.clear {
        return Some(Shape { clear: true, ..Shape::default() });
    }
    let own = |text: &str, most| told::clean(text, most).filter(|text| !text.contains(['|', '“', '”']));
    let part = |text: &str| if text.trim().is_empty() { Some(String::new()) } else { own(text, TEXT_MAX) };
    let (done_when, not_touching) = (part(&raw.done_when)?, part(&raw.not_touching)?);
    let (why, ask) = (part(&raw.why)?, part(&raw.ask)?);
    Some(Shape { clear: false, wish: own(&raw.wish, WISH_MAX)?, done_when, not_touching, why, ask })
}

/// The shape that waits on `day`, when it is of the words `said`, with the run's prompt of it: a plain file of
/// the person's own, within its size, of that day, every text of it what the check leaves, and its prompt one
/// that can be built. It is a file's word: checked again here.
fn waiting(home: &Path, day: &str, said: &str) -> Option<(Kept, String)> {
    let bytes = tap::plain(&file(home, day), KEPT_MAX).ok()?;
    let kept: Kept = serde_json::from_slice(&bytes).ok()?;
    let passes =
        serde_json::to_string(&kept.shape).ok().and_then(|shape| checked(&shape)).as_ref() == Some(&kept.shape);
    let shaped = prompt(&kept.shape, said, None).filter(|_| kept.day == day && kept.said == said && passes)?;
    (!kept.shape.clear).then_some((kept, shaped))
}

/// The run's prompt of a wish kept from a shape, on one line: Jellymoon's labels, each before the model's text,
/// and the person's own words, quoted, last. With the person's `answer` to the shape's question, the question
/// stands before them under a label, bare as every text of the model's, and then the answer, quoted as the
/// person's words are. None when the door would change it: then no wish is kept with it.
fn prompt(shape: &Shape, said: &str, answer: Option<&str>) -> Option<String> {
    let mut built = format!("The wish: {}", shape.wish);
    for (label, text) in [("Done when", &shape.done_when), ("Not touching", &shape.not_touching)] {
        if !text.is_empty() {
            built.push_str(&format!(" | {label}: {text}"));
        }
    }
    if let Some(answer) = answer {
        built.push_str(&format!(" | Asked of the person: {} | The person's answer: “{answer}”", shape.ask));
    }
    built.push_str(&format!(" | The person's own words: “{said}”"));
    Some(built).filter(|built| kept(built))
}

/// An answer as the run's prompt holds it: cleaned as a wish's words are, and of their length at most.
fn answered(typed: &str) -> Result<String, &'static str> {
    let answer = wish::cleaned(typed).map_err(|why| if why == Unkept::Long { LONG_ANSWER } else { wish::CHANGING })?;
    match answer.as_str() {
        "" => Err(NO_ANSWER),
        marks if marks_only(marks) => Err(MARKS),
        _ => Ok(answer),
    }
}

/// Whether a text is one a wish keeps as its run's prompt: what the door leaves of itself, within its length.
pub(crate) fn kept(shaped: &str) -> bool {
    !shaped.is_empty() && still(shaped, SHAPED_MAX, false).is_ok_and(|made| made == shaped)
}

/// `jellymoon wish --shaped`: keeps the wish as [`wish::wish`] does, and with the shape that waits for these same
/// words and this project when there is one. Whether it was kept with it is said. No shape, or one of other
/// words or another project, changes nothing: the wish is kept as typed. The prompt is kept in the same write as
/// the wish, so a wish is never kept without the shape it was to go with; a shape a wish is kept from waits no
/// more, and one whose wish was refused still waits.
pub fn wish_shaped(home: &Path, day: Option<&str>, typed: &str, project: &str) -> Result<(Wished, bool), Error> {
    let (day, said) = (wish::day_of(day)?, wish::words(typed)?);
    let shaped = waiting(home, &day, &said).filter(|kept| kept.0.project == project).map(|kept| kept.1);
    let with_shape = shaped.is_some();
    let made = wish::wish_with(home, &day, typed, project, shaped)?;
    if with_shape {
        let _ = fs::remove_file(file(home, &day));
    }
    Ok((made, with_shape))
}

/// What a wish is kept with, from the shape that waits for its words.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShapeGo<'a> {
    /// As typed: the shape is not used.
    Typed,
    /// With the sharper wish.
    Shape,
    /// With the sharper wish and the person's answer to its question, as they typed it.
    Answer(&'a str),
}

/// The sharper wish a page has on show: the words it was made of as core keeps them, the project it was read
/// for, and its texts. A wish is kept from the shape that waits only while that shape is this one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShapeSeen {
    pub said: String,
    pub project: String,
    pub shape: Shape,
}

/// A tap under a shape on a page, or Enter in the field under it: keeps the wish of the words the shape `seen`
/// on that page was made of, for its project, as [`wish::wish_offered`] keeps one, as `go` says, in one write.
/// Refused, with nothing written, when the shape that waits is not the one seen (other words, another project,
/// or other texts: one made since for the same words), whatever `go` says, and with the shape still waiting
/// when an answer is one the shape did not ask for, is not one once cleaned, or leaves no prompt to build.
pub fn wish_of_shape(read: &DayOptions, seen: &ShapeSeen, go: ShapeGo) -> Result<Wished, Error> {
    let (Some(home), Some(day)) = (&read.home, &read.date) else { return Err(Error::Home) };
    let refused = |why: &str| Error::Wish(why.to_string());
    let same = |waits: &(Kept, String)| waits.0.project == seen.project && waits.0.shape == seen.shape;
    let (waits, shaped) = waiting(home, day, &wish::words(&seen.said)?).filter(same).ok_or_else(|| refused(GONE))?;
    let shaped = match go {
        ShapeGo::Typed => None,
        ShapeGo::Shape => Some(shaped),
        ShapeGo::Answer(_) if waits.shape.ask.is_empty() => return Err(refused(NO_ASK)),
        ShapeGo::Answer(answer) => {
            let answer = answered(answer).map_err(refused)?;
            Some(prompt(&waits.shape, &waits.said, Some(&answer)).ok_or_else(|| refused(UNBUILT))?)
        }
    };
    let made = wish::offered_with(read, &seen.said, &waits.project, shaped)?;
    let _ = fs::remove_file(file(home, day));
    Ok(made)
}

#[cfg(test)]
mod tests;
