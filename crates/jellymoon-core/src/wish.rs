//! A made wish (F37, ADR 0014): one the person made themselves, with `jellymoon wish` or in the app, and not one the
//! reading call worked out. [`wish`] keeps it beside the day's page, in `pages/<day>.wishes.json`, because a reading
//! writes `pages/<day>.json` again; [`with_made`] puts the day's made wishes on its page when the page is drawn.
//!
//! A made wish is kept in the words a page draws: on one line, with nothing that cannot be seen and no credential,
//! as a clash's statement is. Its id is `m1` to `m8`, by its place in the file; a reading's are `w1` to `w8`. The
//! file is written behind the day's lock, the one a tap takes, and is used only when every part of it is what
//! [`wish`] or a run (`run.rs`) would have written. Its words are read through the door they were written through
//! ([`still`]). A run changes its wish through [`change`], behind the same lock and through the same door: what it
//! writes back is every wish of the day as the door made it.

use std::fmt;
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::hook::put;
use crate::reader::{self, parse};
use crate::reading::{LABEL_MAX, STAR_MAX, STARS_MAX};
use crate::redact::line;
use crate::still::{Unkept, statement, still};
use crate::tap::{Lock, locked, plain};
use crate::{DayOptions, Error, PageData, Wish, WishState, copy, run};

/// The most made wishes one day holds.
pub const MADE_MAX: usize = 8;
/// The most characters of a made wish, once cleaned: a statement's length. A longer one is refused, never cut.
pub const MADE_SAID_MAX: usize = 200;
const PROJECT_MAX: usize = 100;
/// The most bytes of a day's made wishes that are read. Eight wishes with their stars are far less.
const MADE_BYTES: usize = 64 * 1024;
/// The most characters of what a wish says of its run.
const WHY_MAX: usize = 200;
/// What refuses words that cleaning still changes after its last time: they are never kept half cleaned.
const CHANGING: &str = "a wish is kept as cleaning leaves it, and cleaning was not done with these words after 8 times";
/// What a made wish says while no run was started for it.
const WHY: &str = "nothing was started: `jellymoon wish` keeps a wish and starts no run";

/// A wish the person made, as it is kept and printed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MadeWish {
    /// `m1` to `m8`, in the order made that day.
    pub id: String,
    /// The person's own words, cleaned, at most 200 characters.
    pub said: String,
    /// The name of the one folder the wish is for.
    pub project: String,
    pub state: WishState,
    /// One fixed text for each run that finished for the wish, at most 4.
    pub stars: Vec<String>,
    /// The one run started for the wish (F39). Left out when none was.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run: Option<String>,
    /// How that run stands, in core's fixed words. Left out when no run was started.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub why: Option<String>,
    /// The question that run asks (F51): an agent's text as the door left it, on one line, at most 300
    /// characters. Only a wish that needs the person and names a run has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub question: Option<String>,
}

/// The made wishes of one day, `pages/<day>.wishes.json`.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Made {
    day: String,
    wishes: Vec<MadeWish>,
}

/// What `jellymoon wish` prints.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Wished {
    pub wish: MadeWish,
    /// The run started for the wish. None: no run was started.
    pub run: Option<String>,
    /// Why no run was started, in fixed words.
    pub why: String,
}

impl fmt::Display for Wished {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&serde_json::to_string(self).map_err(|_| fmt::Error)?)
    }
}

fn refused(why: impl Into<String>) -> Error {
    Error::Wish(why.into())
}

fn file(home: &Path, day: &str) -> PathBuf {
    home.join("pages").join(format!("{day}.wishes.json"))
}

/// The words of a wish as they are kept: cleaned until cleaning changes nothing, and never cut.
fn cleaned(typed: &str) -> Result<String, Unkept> {
    still(typed, MADE_SAID_MAX, false)
}

/// Whether a project is one folder's name: no path, and nothing that cleaning would change.
fn named(project: &str) -> bool {
    let path = project.contains(['/', '\\']) || project == "." || project == "..";
    !project.is_empty() && !path && project.chars().count() <= PROJECT_MAX && line(project, PROJECT_MAX) == project
}

/// The projects of `projects` a wish can be made for from a page, in their order: each is one folder's name as
/// [`wish`] takes it, and a folder of that name was typed in on `read`'s day and is not switched off. That is
/// asked of the reader, with the lookup a run finds its repo by (`reader::folders`), so the names switched off
/// (`read.skip`, and `skip.txt` in `read.home`) are matched against the folder's whole path and never against the
/// project's name alone. Every session log of the day is read. None at all when the day cannot be read: no logs,
/// or a `skip.txt` that cannot be used, never puts a folder on offer.
pub fn wishable(read: &DayOptions, projects: &[String]) -> Vec<String> {
    let Ok(typed) = reader::typed_in(read) else { return Vec::new() };
    let found = |project: &String| named(project) && typed.iter().any(|folder| reader::is_named(folder, project));
    projects.iter().filter(|project| found(project)).cloned().collect()
}

/// [`wish`] for a project chosen on a page: kept only when [`wishable`] offers that project now, on `read`'s day
/// and in `read.home`. A refusal has written nothing.
pub fn wish_offered(read: &DayOptions, typed: &str, project: &str) -> Result<Wished, Error> {
    let (Some(home), Some(day)) = (&read.home, &read.date) else { return Err(Error::Home) };
    if wishable(read, &[project.to_string()]).is_empty() {
        return Err(refused(format!(
            "no folder of that project that is switched on was typed in on {day}, so this wish was not kept"
        )));
    }
    wish(home, Some(day), typed, project)
}

/// The start of the words typed for a wish, for a note to name the wish by: cleaned as a kept wish's words are,
/// then cut to `most` characters. Words that cannot be cleaned are the mark of a redaction and nothing else.
pub fn wish_start(typed: &str, most: usize) -> String {
    statement(typed, most)
}

/// A wish read from the file, when it is what [`wish`] would have written at that place. Its words are cleaned as
/// typed words are, so words kept before they were cleaned to their end are used as they stand still.
fn own(at: usize, wish: MadeWish) -> Option<MadeWish> {
    let star = |star: &String| !star.is_empty() && line(star, STAR_MAX) == *star;
    let said = cleaned(&wish.said).ok().filter(|said| !said.is_empty())?;
    let own = wish.id == format!("m{}", at + 1)
        && named(&wish.project)
        && wish.stars.len() <= STARS_MAX
        && wish.stars.iter().all(star)
        && wish.run.as_deref().is_none_or(copy::is_run_id)
        && wish.why.as_ref().is_none_or(|why| !why.is_empty() && line(why, WHY_MAX) == *why)
        && wish.question.as_deref().is_none_or(|question| {
            run::kept_question(question) && wish.state == WishState::NeedsYou && wish.run.is_some()
        });
    own.then_some(MadeWish { said, ..wish })
}

/// The day's made wishes: none when there is no file. A file that is a link, that others can write to, that is too
/// large, of another day or of another shape in any part is an error, which says why in a few fixed words.
fn kept(home: &Path, day: &str) -> Result<Vec<MadeWish>, String> {
    let bytes = match plain(&file(home, day), MADE_BYTES) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e.to_string()),
    };
    let made = serde_json::from_slice::<Made>(&bytes).ok();
    let made = made.filter(|made| made.day == day && made.wishes.len() <= MADE_MAX);
    let own = made.and_then(|made| made.wishes.into_iter().enumerate().map(|(at, w)| own(at, w)).collect());
    own.ok_or_else(|| "they are not the made wishes of that day".to_string())
}

/// Keeps a wish the person made for `project` on `day` (today when none is named), in the working state. Nothing
/// is started. A refusal has written nothing: words that are empty once cleaned or over 200 characters, a project
/// that is not one folder's name, a day that has its eight, made wishes that cannot be used, a day that is locked.
pub fn wish(home: &Path, day: Option<&str>, typed: &str, project: &str) -> Result<Wished, Error> {
    let day = day_of(day)?;
    let said = cleaned(typed).map_err(|why| match why {
        Unkept::Long => refused("a wish is at most 200 characters, and this one is longer"),
        Unkept::Changing => refused(CHANGING),
    })?;
    if said.is_empty() {
        return Err(refused("a wish needs words, and nothing is left of these once cleaned"));
    }
    if !named(project) {
        let no = "no path, nothing that cannot be seen and nothing shaped like a credential";
        return Err(refused(format!("--project takes one folder's name of at most {PROJECT_MAX} characters: {no}")));
    }
    // Looked at once before the lock, so a refusal makes no folder, and once behind it, where it cannot change.
    let room = || match kept(home, &day) {
        Ok(wishes) if wishes.len() >= MADE_MAX => {
            Err(refused(format!("the page of {day} has its {MADE_MAX} made wishes already, so this one was not kept")))
        }
        Ok(wishes) => Ok(wishes),
        Err(why) => Err(refused(format!("the made wishes of {day} are not used: {why}; this wish was not kept"))),
    };
    room()?;
    let _lock = behind(home, &day, Duration::ZERO, "this wish was not kept")?;
    let mut wishes = room()?;
    let (id, project) = (format!("m{}", wishes.len() + 1), project.to_string());
    let (state, stars) = (WishState::Working, Vec::new());
    let wish = MadeWish { id, said, project, state, stars, run: None, why: None, question: None };
    wishes.push(wish.clone());
    write(home, &day, wishes)?;
    Ok(Wished { wish, run: None, why: WHY.to_string() })
}

/// The day asked for, or today in the computer's time zone.
pub(crate) fn day_of(day: Option<&str>) -> Result<String, Error> {
    match day {
        Some(day) => Ok(parse(day)?.to_string()),
        None => Ok(jiff::Zoned::now().date().to_string()),
    }
}

/// The day's lock, taken. A day that stays locked is a refusal that says what was not done. The lock's own wait
/// is three seconds; it is asked for again until `wait` has passed.
fn behind(home: &Path, day: &str, wait: Duration, undone: &str) -> Result<Lock, Error> {
    let started = Instant::now();
    loop {
        match locked(home, day) {
            Ok(lock) => return Ok(lock),
            Err(e) if e.kind() != ErrorKind::TimedOut => return Err(Error::Store(e)),
            Err(_) if started.elapsed() < wait => {}
            Err(_) => return Err(refused(format!("{LOCKED}{day} has not finished, so {undone}"))),
        }
    }
}

/// How the refusal for a day that stayed locked starts.
const LOCKED: &str = "another change to the page of ";

/// Whether a change was refused for the day's lock, and for nothing in the made wishes.
pub(crate) fn was_locked(refusal: &Error) -> bool {
    matches!(refusal, Error::Wish(why) if why.starts_with(LOCKED))
}

fn write(home: &Path, day: &str, wishes: Vec<MadeWish>) -> Result<(), Error> {
    let text = serde_json::to_string(&Made { day: day.to_string(), wishes }).map_err(io::Error::from);
    text.and_then(|text| put(&file(home, day), &format!("{text}\n"), false)).map_err(Error::Store)
}

/// The day's made wishes as they are kept, for a run to find its wish in. Nothing is written.
pub(crate) fn kept_of(home: &Path, day: &str) -> Result<Vec<MadeWish>, Error> {
    kept(home, day).map_err(|why| refused(format!("the made wishes of {day} are not used: {why}")))
}

/// Changes one made wish of the day behind the day's lock, waited for `wait` at least, and returns it as it is
/// kept then. A change that is refused, a wish that is not there and made wishes that cannot be used write nothing.
/// The wishes are read through the door ([`kept`]) and all written back, so one kept by an earlier build is written
/// as it stands still, and none is dropped.
pub(crate) fn change(
    home: &Path,
    day: &str,
    id: &str,
    wait: Duration,
    change: impl FnOnce(&mut MadeWish) -> Result<(), Error>,
) -> Result<MadeWish, Error> {
    let _lock = behind(home, day, wait, "the wish was not changed")?;
    let mut wishes = kept_of(home, day)?;
    let wish = wishes.iter_mut().find(|wish| wish.id == id);
    let wish = wish.ok_or_else(|| refused(format!("the page of {day} has no made wish of that id")))?;
    change(wish)?;
    let changed = wish.clone();
    write(home, day, wishes)?;
    Ok(changed)
}

/// The day's made wishes, as a page draws them. None when the day has none, or when what is kept cannot be used.
/// A wish whose run no longer goes, and which nothing told how the run ended, is never drawn as working for ever:
/// it is drawn, and written down, as its run's own record says it ended, or else as cut off, with the agent it
/// left stopped (`run::bury`). Nothing else is written.
pub fn made(home: &Path, day: &str) -> Vec<Wish> {
    made_said(home, day).0
}

fn made_said(home: &Path, day: &str) -> (Vec<Wish>, Vec<String>) {
    let kept = if parse(day).is_ok() { kept(home, day).unwrap_or_default() } else { Vec::new() };
    // What a `jellymoon run` that was killed left with no wish to name it is taken back (`run/left.rs`).
    let said = run::tidy(home, &kept);
    let drawn = |wish: MadeWish| {
        let wish = run::bury(home, day, wish);
        let (label, question) = (line(&wish.said, LABEL_MAX), wish.question);
        Wish {
            label,
            state: wish.state,
            id: wish.id,
            said: wish.said,
            project: wish.project,
            stars: wish.stars,
            question,
        }
    };
    (kept.into_iter().map(drawn).collect(), said)
}

/// The ids of the day's made wishes whose run is going: working, and naming a run, once a run that nothing holds
/// has been given its end as [`made`] gives it. These are the wishes `stop_run` has a run to stop for.
pub fn going(home: &Path, day: &str) -> Vec<String> {
    let kept = if parse(day).is_ok() { kept(home, day).unwrap_or_default() } else { Vec::new() };
    let going = |wish: MadeWish| {
        let wish = run::bury(home, day, wish);
        (wish.state == WishState::Working && wish.run.is_some()).then_some(wish.id)
    };
    kept.into_iter().filter_map(going).collect()
}

/// The page with the day's made wishes after the reading's. A wish whose id is on the page already is not added,
/// so a page that was merged once is the same merged twice.
pub fn with_made(home: &Path, page: PageData) -> PageData {
    with_made_said(home, page).0
}

/// [`with_made`], and what this look has to say that is no part of a page: a line for each copy a killed command
/// left that was given up, and may have left a branch in its repo (`copy/back.rs`). A question the page was
/// given with a wish stays only when it is what the door leaves of one: no reading writes one.
pub fn with_made_said(home: &Path, mut page: PageData) -> (PageData, Vec<String>) {
    let (made, said) = made_said(home, &page.day);
    for wish in &mut page.wishes {
        wish.question.take_if(|question| !run::kept_question(question));
    }
    for wish in made {
        if page.wishes.iter().all(|on| on.id != wish.id) {
            page.wishes.push(wish);
        }
    }
    (page, said)
}

#[cfg(test)]
mod tests;
