//! A made wish (F37, ADR 0014): one the person made themselves, with `jellymoon wish` or in the app, and not one the
//! reading call worked out. [`wish`] keeps it beside the day's page, in `pages/<day>.wishes.json`, because a reading
//! writes `pages/<day>.json` again; [`with_made`] puts the day's made wishes on its page when the page is drawn.
//!
//! A made wish is kept in the words a page draws: on one line, with nothing that cannot be seen and no credential,
//! as a clash's statement is. Its id is `m1` to `m8`, by its place in the file; a reading's are `w1` to `w8`. The
//! file is written behind the day's lock, the one a tap takes, and is used only when every part of it is what
//! [`wish`] would have written. Its words are read through the door they were written through ([`still`]).

use std::fmt;
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::hook::put;
use crate::reader::parse;
use crate::reading::{LABEL_MAX, STAR_MAX, STARS_MAX};
use crate::redact::line;
use crate::still::{Unkept, still};
use crate::tap::{locked, plain};
use crate::{Error, PageData, Wish, WishState};

/// The most made wishes one day holds.
pub const MADE_MAX: usize = 8;
/// The most characters of a made wish, once cleaned: a statement's length. A longer one is refused, never cut.
const SAID_MAX: usize = 200;
const PROJECT_MAX: usize = 100;
/// The most bytes of a day's made wishes that are read. Eight wishes with their stars are far less.
const MADE_BYTES: usize = 64 * 1024;
/// What refuses words that cleaning still changes after its last time: they are never kept half cleaned.
const CHANGING: &str = "a wish is kept as cleaning leaves it, and cleaning was not done with these words after 8 times";
/// What a made wish says while no run can be started for it.
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
    still(typed, SAID_MAX, false)
}

/// Whether a project is one folder's name: no path, and nothing that cleaning would change.
fn named(project: &str) -> bool {
    let path = project.contains(['/', '\\']) || project == "." || project == "..";
    !project.is_empty() && !path && project.chars().count() <= PROJECT_MAX && line(project, PROJECT_MAX) == project
}

/// A wish read from the file, when it is what [`wish`] would have written at that place. Its words are cleaned as
/// typed words are, so words kept before they were cleaned to their end are used as they stand still.
fn own(at: usize, wish: MadeWish) -> Option<MadeWish> {
    let star = |star: &String| !star.is_empty() && line(star, STAR_MAX) == *star;
    let said = cleaned(&wish.said).ok().filter(|said| !said.is_empty())?;
    let own = wish.id == format!("m{}", at + 1)
        && named(&wish.project)
        && wish.stars.len() <= STARS_MAX
        && wish.stars.iter().all(star);
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
    let day = match day {
        Some(day) => parse(day)?.to_string(),
        None => jiff::Zoned::now().date().to_string(),
    };
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
    let _lock = locked(home, &day).map_err(|e| match e.kind() {
        ErrorKind::TimedOut => {
            refused(format!("another change to the page of {day} has not finished, so this wish was not kept"))
        }
        _ => Error::Store(e),
    })?;
    let mut wishes = room()?;
    let id = format!("m{}", wishes.len() + 1);
    let wish = MadeWish { id, said, project: project.to_string(), state: WishState::Working, stars: Vec::new() };
    wishes.push(wish.clone());
    let text = serde_json::to_string(&Made { day: day.clone(), wishes }).map_err(io::Error::from);
    text.and_then(|text| put(&file(home, &day), &format!("{text}\n"), false)).map_err(Error::Store)?;
    Ok(Wished { wish, run: None, why: WHY.to_string() })
}

/// The day's made wishes, as a page draws them. None when the day has none, or when what is kept cannot be used.
/// Nothing is written.
pub fn made(home: &Path, day: &str) -> Vec<Wish> {
    let kept = if parse(day).is_ok() { kept(home, day).unwrap_or_default() } else { Vec::new() };
    let drawn = |wish: MadeWish| Wish {
        label: line(&wish.said, LABEL_MAX),
        id: wish.id,
        said: wish.said,
        project: wish.project,
        state: wish.state,
        stars: wish.stars,
    };
    kept.into_iter().map(drawn).collect()
}

/// The page with the day's made wishes after the reading's. A wish whose id is on the page already is not added,
/// so a page that was merged once is the same merged twice.
pub fn with_made(home: &Path, mut page: PageData) -> PageData {
    for wish in made(home, &page.day) {
        if page.wishes.iter().all(|on| on.id != wish.id) {
            page.wishes.push(wish);
        }
    }
    page
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PageRead;

    // `jellymoon page` prints a merged page. One that is merged again must not hold a made wish twice.
    #[test]
    fn a_page_merged_twice_holds_each_made_wish_once() -> Result<(), Box<dyn std::error::Error>> {
        let home = tempfile::tempdir()?;
        wish(home.path(), Some("2026-03-10"), "ship the changelog", "alpha")?;
        let read = PageRead { turns: 0, projects: Vec::new() };
        let page = PageData { day: "2026-03-10".to_string(), wishes: Vec::new(), clashes: Vec::new(), read };
        let once = with_made(home.path(), page);
        assert_eq!(once.wishes.iter().map(|wish| wish.id.as_str()).collect::<Vec<_>>(), ["m1"]);
        assert_eq!(with_made(home.path(), once.clone()), once);
        Ok(())
    }
}
