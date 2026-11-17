//! The projects Jellymoon knows (F62, ADR 0020; `docs/development/project.md`): one file in Jellymoon's own
//! folder, `projects.json`, with a line for each project: its name, its folder and the day it was last used. A
//! project gets on it when the person names its folder ([`add_project`]), when a run starts in it ([`used`])
//! and when Jellymoon makes it ([`new_project`], `new.rs`), and leaves by [`forget_project`]. A name has one
//! folder.
//!
//! The file is written whole, beside its place and moved there, behind a lock of its own, and read back as
//! untrusted: it is used only when it is a plain file nobody else can write to and every line of it is one
//! [`add_project`] would have written. A line that passes says only where to look. Whether a run may start
//! there is asked again each time it is used (`offer.rs`). No folder is ever printed: a project is named by its
//! name, which is what cleaning leaves of itself.

mod new;
mod offer;

use std::fmt;
use std::io::{self, ErrorKind};
use std::path::{Component, Path, PathBuf};

pub use new::new_project;
pub(crate) use offer::{Led, Offer};
use serde::{Deserialize, Serialize};

use crate::Error;
use crate::hook::put;
use crate::reader::parse;
use crate::ruling::private_dir;
use crate::run::{plain as as_git_takes, theirs};
use crate::session::FOLDER_MAX;
use crate::tap::{Lock, plain};
use crate::wish::{day_of, kept_name, named};

/// The most projects the list holds.
pub const PROJECTS_MAX: usize = 100;
/// The most bytes of the list that are read: a hundred of the longest lines are less.
const LIST_BYTES: usize = 512 * 1024;

/// One line of the list, as it is kept.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Line {
    pub name: String,
    /// The folder by its real path, every link in it followed when it was listed.
    pub folder: String,
    used: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct List {
    projects: Vec<Line>,
}

/// A project as it is printed: its name and the day it was last used, and never its folder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Known {
    pub name: String,
    pub used: String,
}

/// What `jellymoon project list` prints.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Projects {
    pub projects: Vec<Known>,
}

impl fmt::Display for Known {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&serde_json::to_string(self).map_err(|_| fmt::Error)?)
    }
}

impl fmt::Display for Projects {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&serde_json::to_string(self).map_err(|_| fmt::Error)?)
    }
}

impl Line {
    fn known(&self) -> Known {
        Known { name: self.name.clone(), used: self.used.clone() }
    }

    /// Whether this is a line [`add_project`] would have written: a name that is one folder's, a full path with
    /// no `.`, no `..` and nothing doubled in it whose last name is that name, and a day.
    fn own(&self) -> bool {
        let folder = Path::new(&self.folder);
        let parts = || folder.components();
        let straight = parts().all(|part| !matches!(part, Component::CurDir | Component::ParentDir))
            && parts().collect::<PathBuf>().as_os_str() == folder.as_os_str();
        kept_name(&self.name)
            && folder.is_absolute()
            && straight
            && self.folder.len() <= FOLDER_MAX
            && folder.file_name().is_some_and(|last| last == self.name.as_str())
            && parse(&self.used).is_ok()
    }
}

fn refused(why: impl Into<String>) -> Error {
    Error::Project(why.into())
}

fn file(home: &Path) -> PathBuf {
    home.join("projects.json")
}

/// The list's lines: none when there is no file. A file that is a link, that others can write to, that is too
/// large, that is not text or holds anything [`add_project`] would not have written is an error, which says why
/// in a few fixed words and names a line by its place, never by its text.
pub(crate) fn kept(home: &Path) -> Result<Vec<Line>, String> {
    let bytes = match plain(&file(home), LIST_BYTES) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e.to_string()),
    };
    let text = String::from_utf8(bytes).map_err(|_| "it is not text".to_string())?;
    let list = serde_json::from_str::<List>(&text).ok().filter(|list| list.projects.len() <= PROJECTS_MAX);
    let lines = list.ok_or_else(|| "it is not a list as Jellymoon writes one".to_string())?.projects;
    for (at, line) in lines.iter().enumerate() {
        if !line.own() {
            return Err(format!("project {} of it is not one Jellymoon wrote", at + 1));
        }
        if lines[..at].iter().any(|before| before.name == line.name) {
            return Err(format!("project {} of it has the name of one before it", at + 1));
        }
    }
    Ok(lines)
}

/// What is said of a list that cannot be used.
pub(crate) fn unused(why: &str) -> String {
    format!(
        "the list of projects (projects.json in Jellymoon's folder) is not used: {why}; move the file away and add \
         your projects again"
    )
}

fn listed(home: &Path) -> Result<Vec<Line>, Error> {
    kept(home).map_err(|why| refused(unused(&why)))
}

/// The projects on the list, in the order they were listed. Nothing is written, and no folder is looked at.
pub fn projects(home: &Path) -> Result<Projects, Error> {
    Ok(Projects { projects: listed(home)?.iter().map(Line::known).collect() })
}

/// What is said to a command that waited for the list's lock and did not get it. The holder is a change to the
/// list or a making (`new.rs`); a lock whose holder was cut off is taken over when it is 30 seconds old.
const BUSY: &str = "another change to the list of projects, or the making of a new project, has not finished, so \
                    nothing was changed; if it was cut off, ask again in half a minute";

/// Takes the list's lock, `projects.lock` in the home, which is made when it is not there. Every change to the
/// list is behind it, and the whole of a making.
fn hold(home: &Path) -> Result<Lock, Error> {
    private_dir(home).map_err(Error::Store)?;
    Lock::take(home.join("projects.lock")).map_err(|e| match e.kind() {
        ErrorKind::TimedOut => refused(BUSY),
        _ => Error::Store(e),
    })
}

/// Writes the list whole. Only behind its lock ([`hold`]).
fn write(home: &Path, lines: Vec<Line>) -> Result<(), Error> {
    let text = serde_json::to_string(&List { projects: lines }).map_err(io::Error::from);
    text.and_then(|text| put(&file(home), &format!("{text}\n"), false)).map_err(Error::Store)
}

/// Changes the list behind its lock. The change is tried once on the list as it is before the lock is asked
/// for, so a change that is refused makes no folder and no file; and once behind the lock, where the list
/// cannot change under it.
fn change<T>(home: &Path, change: impl Fn(&mut Vec<Line>) -> Result<T, Error>) -> Result<T, Error> {
    change(&mut listed(home)?)?;
    let _lock = hold(home)?;
    let mut lines = listed(home)?;
    let done = change(&mut lines)?;
    write(home, lines).map(|()| done)
}

/// The name and the real folder a project is listed by, when `repo` is a folder a project can be: there, the
/// top of a repo, the person's own, and named by one folder's name. Each refusal is fixed words: none names the
/// folder, or its last name.
fn stands_as(repo: &Path) -> Result<(String, String), Error> {
    if !repo.is_absolute() {
        return Err(refused("--repo must be a full path, from the root"));
    }
    let real = std::fs::canonicalize(repo).ok().filter(|real| real.is_dir()).map(as_git_takes);
    let real = real.ok_or_else(|| refused("--repo is not a folder that is there"))?;
    let name = real.file_name().and_then(|last| last.to_str()).filter(|last| named(last));
    let name = name.ok_or_else(|| {
        refused(
            "--repo is not a folder a project can be named after: its last name must be one folder's name of at \
             most 100 characters, with nothing that cannot be seen and nothing shaped like a credential",
        )
    })?;
    let folder = real.to_str().filter(|folder| folder.len() <= FOLDER_MAX);
    let folder = folder.ok_or_else(|| {
        refused("--repo is not a folder Jellymoon can list: its full path is not text, or is over 4096 bytes")
    })?;
    if !real.join(".git").exists() {
        return Err(refused("--repo is not the top of a git repo"));
    }
    if theirs(&real) {
        return Err(refused("--repo is not a folder of your own"));
    }
    Ok((name.to_string(), folder.to_string()))
}

/// Puts a project on the lines, used on `day`. One that is there with this folder keeps its place, and its day
/// moves on and never back. Refused: another folder under the name, and a list that is full.
fn put_on(lines: &mut Vec<Line>, name: &str, folder: &str, day: &str) -> Result<Known, Error> {
    if let Some(line) = lines.iter_mut().find(|line| line.name == name) {
        if line.folder != folder {
            return Err(refused(format!(
                "another folder is listed under the name {name}, and a name has one folder, so nothing was changed; \
                 let the listed one go with `jellymoon project forget {name}`, then add this one"
            )));
        }
        if line.used.as_str() < day {
            line.used = day.to_string();
        }
        return Ok(line.known());
    }
    if lines.len() >= PROJECTS_MAX {
        return Err(refused(format!(
            "the list has its {PROJECTS_MAX} projects already, so this one was not added; let one go with \
             `jellymoon project forget`"
        )));
    }
    lines.push(Line { name: name.to_string(), folder: folder.to_string(), used: day.to_string() });
    Ok(Known { name: name.to_string(), used: day.to_string() })
}

/// Lists the repo whose top folder is `repo` under that folder's last name, used on `day` (today when none is
/// named). Nothing of the folder is written and nothing is started. A refusal has written nothing.
pub fn add_project(home: &Path, repo: &Path, day: Option<&str>) -> Result<Known, Error> {
    let day = day_of(day)?;
    let (name, folder) = stands_as(repo)?;
    let known = change(home, |lines| put_on(lines, &name, &folder, &day))?;
    // A folder a `project new` that was cut off left holds that making's file: listed, it does not keep it.
    new::tidy(home, &folder);
    Ok(known)
}

/// Whether the list holds a line of exactly that name. False when the list cannot be used.
pub(crate) fn lists(home: &Path, name: &str) -> bool {
    listed(home).is_ok_and(|lines| lines.iter().any(|line| line.name == name))
}

/// Takes the line of that name off the list, and touches nothing else: not the folder, not a wish, not a run.
/// The name is one a line can have, so a line an earlier build wrote can be taken off.
pub fn forget_project(home: &Path, name: &str) -> Result<Known, Error> {
    if !kept_name(name) {
        return Err(refused("project forget takes a project's name, as `jellymoon project list` prints it"));
    }
    change(home, |lines| {
        let at = lines.iter().position(|line| line.name == name);
        let at = at.ok_or_else(|| refused("no project of that name is on the list, so nothing was changed"))?;
        Ok(lines.remove(at).known())
    })
}

/// Writes down the repo a run has started in, after every check the run made of it, as used on the wish's day.
/// A run is never failed for it: a folder that leads to another name than the wish's project, one that is not
/// the person's own, a name that has another folder, a list that is full or cannot be used leave the list as it
/// was.
pub(crate) fn used(home: &Path, repo: &Path, project: &str, day: &str) {
    if let Ok((name, folder)) = stands_as(repo)
        && name == project
    {
        let _ = change(home, |lines| put_on(lines, &name, &folder, day));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The file as it is kept: core's example, which the doc shows, written back byte for byte.
    #[test]
    fn the_kept_list_round_trips_byte_for_byte_and_a_field_nobody_knows_is_refused() {
        let example = include_str!("../tests/fixtures/projects-kept.json").replace("\r\n", "\n");
        let list: List = serde_json::from_str(&example).unwrap_or_else(|e| panic!("{e}"));
        let written = serde_json::to_string(&list).unwrap_or_default();
        assert_eq!(format!("{written}\n"), example);
        assert_eq!(list.projects[1].known(), Known { name: "beta".to_string(), used: "2026-03-11".to_string() });
        let more = example.replace("\"used\":\"2026-03-10\"", "\"used\":\"2026-03-10\",\"run\":true");
        assert!(serde_json::from_str::<List>(&more).is_err());
    }
}
