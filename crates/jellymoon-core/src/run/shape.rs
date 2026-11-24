//! The wish is shaped before a run (F91, ADR 0025). Between the words a person typed and the run of them, their own
//! Claude Code is asked once, headless and read-only, to say the same wish sharply: [`shape`]. It starts in a new
//! folder of Jellymoon's own, with the project's folder named by `--add-dir`, a fixed command line ([`LINE`]) and
//! fixed system words ([`PROMPT`]). Every word of the answer is a model's, so the answer is checked whole
//! ([`checked`]) and thrown away whole when any part fails: then the wish is as typed, and why is said in
//! Jellymoon's own fixed words, never the model's. A checked shape that is not `clear` waits in
//! `pages/<day>.shape.json`, one a day, with the words and the project it is of. A wish is kept from it only for
//! those same words ([`wish_shaped`], [`wish_of_shape`]): then the wish carries the run's prompt ([`prompt`]),
//! which core builds from fixed labels and ends with the person's own words.

use std::path::{Path, PathBuf};
use std::time::Duration;
use std::{fmt, fs, io};

use serde::{Deserialize, Serialize};

use super::{RunOptions, place, told};
use crate::busy::or_test;
use crate::call::{self, Reply};
use crate::hook::put;
use crate::reading::unfence;
use crate::still::still;
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
/// The most characters of a shape's wish, of each of its other texts, and of the run's prompt made of them.
const WISH_MAX: usize = 280;
const TEXT_MAX: usize = 160;
const SHAPED_MAX: usize = 1000;
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
/// and each other text does too, or is empty.
fn checked(answer: &str) -> Option<Shape> {
    let raw: Shape = serde_json::from_str(unfence(answer)).ok().filter(|_| answer.len() < told::SAID_MAX)?;
    if raw.clear {
        return Some(Shape { clear: true, ..Shape::default() });
    }
    let part = |text: &str| if text.trim().is_empty() { Some(String::new()) } else { told::clean(text, TEXT_MAX) };
    let (done_when, not_touching) = (part(&raw.done_when)?, part(&raw.not_touching)?);
    let (why, ask) = (part(&raw.why)?, part(&raw.ask)?);
    Some(Shape { clear: false, wish: told::clean(&raw.wish, WISH_MAX)?, done_when, not_touching, why, ask })
}

/// The shape that waits on `day`, when it is of the words `said`: a plain file of the person's own, within its
/// size, of that day, and every text of it what the check leaves. It is a file's word: checked again here.
fn waiting(home: &Path, day: &str, said: &str) -> Option<Kept> {
    let bytes = tap::plain(&file(home, day), KEPT_MAX).ok()?;
    let kept: Kept = serde_json::from_slice(&bytes).ok()?;
    let passes =
        serde_json::to_string(&kept.shape).ok().and_then(|shape| checked(&shape)).as_ref() == Some(&kept.shape);
    (kept.day == day && kept.said == said && !kept.shape.clear && passes).then_some(kept)
}

/// The run's prompt of a wish kept from a shape, on one line: Jellymoon's labels, each before the model's text,
/// and the person's own words, quoted, last. None when the door would change it: then the wish is as typed.
fn prompt(shape: &Shape, said: &str) -> Option<String> {
    let mut built = format!("The wish: {}", shape.wish);
    for (label, text) in [("Done when", &shape.done_when), ("Not touching", &shape.not_touching)] {
        if !text.is_empty() {
            built.push_str(&format!(" | {label}: {text}"));
        }
    }
    built.push_str(&format!(" | The person's own words: “{said}”"));
    Some(built).filter(|built| kept(built))
}

/// Whether a text is one a wish keeps as its run's prompt: what the door leaves of itself, within its length.
pub(crate) fn kept(shaped: &str) -> bool {
    !shaped.is_empty() && still(shaped, SHAPED_MAX, false).is_ok_and(|made| made == shaped)
}

/// A wish that was just kept, with its prompt from the shape `waits` when `go`. A shape a wish is kept from is
/// removed, used or not. Whether the wish carries the prompt is said too: it does not when the day was locked.
fn given(home: &Path, day: &str, mut made: Wished, waits: Option<Kept>, go: bool) -> (Wished, bool) {
    let Some(waits) = waits else { return (made, false) };
    let _ = fs::remove_file(file(home, day));
    let shaped = go.then(|| prompt(&waits.shape, &made.wish.said)).flatten();
    let carried = shaped.and_then(|shaped| {
        let carry = |wish: &mut wish::MadeWish| {
            wish.shaped = Some(shaped);
            Ok(())
        };
        wish::change(home, day, &made.wish.id, Duration::ZERO, carry).ok()
    });
    let carries = carried.is_some();
    made.wish = carried.unwrap_or(made.wish);
    (made, carries)
}

/// `jellymoon wish --shaped`: keeps the wish as [`wish::wish`] does, and with the shape that waits for these same
/// words and this project when there is one. Whether it was kept with it is said. No shape, or one of other
/// words or another project, changes nothing: the wish is kept as typed.
pub fn wish_shaped(home: &Path, day: Option<&str>, typed: &str, project: &str) -> Result<(Wished, bool), Error> {
    let (day, said) = (wish::day_of(day)?, wish::words(typed)?);
    let waits = waiting(home, &day, &said).filter(|kept| kept.project == project);
    let made = wish::wish(home, Some(&day), typed, project)?;
    Ok(given(home, &day, made, waits, true))
}

/// A tap under a shape on a page: keeps the wish of the words `typed`, for the project the shape that waits for
/// those words is of, as [`wish::wish_offered`] keeps one, with the shape (`go`) or as typed. Refused, with
/// nothing written, when no shape waits for those words.
pub fn wish_of_shape(read: &DayOptions, typed: &str, go: bool) -> Result<Wished, Error> {
    let (Some(home), Some(day)) = (&read.home, &read.date) else { return Err(Error::Home) };
    let waits = waiting(home, day, &wish::words(typed)?).ok_or_else(|| Error::Wish(GONE.to_string()))?;
    let made = wish::wish_offered(read, typed, &waits.project)?;
    Ok(given(home, day, made, Some(waits), go).0)
}

#[cfg(test)]
mod tests {
    use super::*;

    // The model's name is the one part of the line that is a name: it is where the line says it is.
    #[test]
    fn the_line_asks_for_the_shape_model_and_every_fixed_reason_is_what_cleaning_leaves() {
        assert!(crate::reading::is_model_name(SHAPE_MODEL) && LINE.contains(&format!(" --model {SHAPE_MODEL} ")));
        for why in [NO_WISH, NO_FOLDER, NO_HARNESS, UNASKED, TOO_LONG, UNANSWERED, UNCHECKED, UNKEPT, GONE] {
            assert_eq!(crate::redact::line(why, 200), why);
        }
    }
}
