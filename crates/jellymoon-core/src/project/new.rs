//! A project Jellymoon makes (F65, ADR 0020, decision 3; `docs/development/project.md`, "A new project"):
//! [`new_project`] makes a folder of the name directly inside `projects` in Jellymoon's own folder, makes it a
//! repo with one empty first commit (`copy/fresh.rs`) and lists it. Nothing is written outside that one folder:
//! the name is one folder's name and no path, and what is at a name already, a link too, is never followed and
//! never replaced. One making goes at a time: all of it is behind the list's lock, so no other change to the
//! list meets one under way. A step that fails takes back the folder this command made, and only that one: it
//! is known by a file of this making's own in it, beside what the system says of the folder.

use std::fs;
use std::io::{self, ErrorKind};
use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use super::{Known, Line, hold, listed, put_on, refused, stands_as, write};
use crate::run::{plain as as_git_takes, theirs};
use crate::session::FOLDER_MAX;
use crate::wish::{day_of, named};
use crate::{Error, busy, copy};

const NAME: &str = "project new takes a name for the new project: one folder's name of at most 100 characters, \
                    with no dot, dash, tilde or space at its start, no dot or space at its end, no two spaces \
                    together, none of : * ? | < > \" in it, no name a system keeps for a device, nothing that \
                    cannot be seen and nothing shaped like a credential; nothing was made";
const LISTED: &str = "a project of that name is on the list already, so nothing was made; pick another name, or let \
                      the listed one go with `jellymoon project forget`";
const THERE: &str = "something is already there under that name in the folder Jellymoon makes projects in (projects \
                     in Jellymoon's folder), so nothing was made; pick another name, list what is there with \
                     `jellymoon project add --repo` and its folder, or, if a `project new` that was cut off left \
                     it, remove it and ask again";
const NO_FOLDER: &str = "the folder Jellymoon makes projects in (projects in Jellymoon's folder) is not a folder \
                         Jellymoon made: a link or a file is at its name; nothing was made";
const NO_GIT: &str = "no `git` program was found on PATH, and a new project is a git repo, so nothing was made";
const NOT_OWN: &str = "Jellymoon's folder is not a folder of your own, and a project is listed only in a folder of \
                       your own, so nothing was made";
const NOT_TEXT: &str = "the full path of Jellymoon's folder is not text, or a project's in it would be over 4096 \
                        bytes, so a new project could not be listed and nothing was made";
const SPELLED: &str = "this file system does not keep the folder under the name it was given, so what was made was \
                       taken back and nothing was listed; pick a name of plain letters";
const MOVED: &str = "the new project's folder was removed or replaced while it was being made, so nothing was listed, and \
                     what is there now was left as it is";

/// What follows a refusal that came after the folder was made and taken back, when its words do not say so.
const BACK: &str = "; the folder made for it meanwhile was taken back";

/// Whether a name is one a folder can be made under on every system: a project's name as the list takes one
/// ([`named`], which takes no space at either end and no two together), that no program reads as an option, a
/// hidden file, the person's home, a drive or a device, that Windows takes in a file's name, that no system cuts
/// a dot from, and that holds no joiner, which is not seen between two letters.
fn makeable(name: &str) -> bool {
    let stem = name.split('.').next().unwrap_or_default().trim_end_matches(' ').to_uppercase();
    let numbered = stem.chars().count() == 4 && stem.ends_with(|last| matches!(last, '0'..='9' | '¹' | '²' | '³'));
    let device = matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$")
        || (numbered && (stem.starts_with("COM") || stem.starts_with("LPT")));
    named(name)
        && !name.starts_with(['.', '-', '~'])
        && !name.ends_with('.')
        && !name.contains([':', '*', '?', '|', '<', '>', '"', '\u{200c}', '\u{200d}'])
        && !device
}

/// Two names that are one where a file system takes no note of the case of letters. They are one name here on
/// every system, so a list means the same wherever it is read.
fn same(one: &str, other: &str) -> bool {
    one.to_lowercase() == other.to_lowercase()
}

/// Lists a made project. A name that is listed, in whatever case, is taken, but by a line for this very name
/// and folder, which is the same line.
fn onto(lines: &mut Vec<Line>, name: &str, folder: &str, day: &str) -> Result<Known, Error> {
    if lines.iter().any(|line| same(&line.name, name) && (line.name != name || line.folder != folder)) {
        return Err(refused(LISTED));
    }
    put_on(lines, name, folder, day)
}

/// Whether `projects` is there, as a real folder. A link or a file at its name is refused, and never followed.
fn folder_there(folder: &Path) -> Result<bool, Error> {
    match fs::symlink_metadata(folder) {
        Ok(meta) if meta.is_dir() => Ok(true),
        Ok(_) => Err(refused(NO_FOLDER)),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(false),
        Err(e) => Err(Error::Store(e)),
    }
}

/// Whether anything is at the name in `projects`: a folder, a file or a link, under the name in any case.
fn taken(folder: &Path, name: &str) -> bool {
    let names = fs::read_dir(folder).into_iter().flatten().flatten();
    names.filter_map(|entry| entry.file_name().into_string().ok()).any(|there| same(&there, name))
}

/// Whether a path of that many bytes, with `projects` and the name below it, is one the list holds.
fn fits(home: usize, name: &str) -> bool {
    home + "/projects/".len() + name.len() <= FOLDER_MAX
}

/// Whether a project made in `home` could be listed: the folder is the person's own, or is not there yet and
/// the person has a home of their own to be the owner of it; and its full path is text the list has room for.
fn listable(home: &Path, name: &str) -> Result<(), Error> {
    let own = match home.exists() {
        true => !theirs(home),
        false => cfg!(not(unix)) || crate::user_home().is_some_and(|person| person.is_dir()),
    };
    if !own {
        return Err(refused(NOT_OWN));
    }
    let real = fs::canonicalize(home).unwrap_or_else(|_| home.to_path_buf());
    match real.to_str().is_some_and(|text| fits(text.len(), name)) {
        true => Ok(()),
        false => Err(refused(NOT_TEXT)),
    }
}

/// Makes one folder for the person alone (0700). The folder above must be there, and the name must be free:
/// what is at it, a link too, is an error, so nothing is ever made through one.
fn make(folder: &Path) -> io::Result<()> {
    let mut make = fs::DirBuilder::new();
    make.recursive(false);
    #[cfg(unix)]
    std::os::unix::fs::DirBuilderExt::mode(&mut make, 0o700);
    make.create(folder)
}

/// What the system says of a folder: where it keeps it (Unix), and when it was made. None for what is not a
/// real folder. It does not tell a folder this command made from another put at its name on every system:
/// Windows has no number to read here, and hands a folder made again under a name its old time of making.
type Mark = (u64, u64, Option<SystemTime>);

fn mark(folder: &Path) -> Option<Mark> {
    let meta = fs::symlink_metadata(folder).ok().filter(fs::Metadata::is_dir)?;
    #[cfg(unix)]
    let (device, number) = (std::os::unix::fs::MetadataExt::dev(&meta), std::os::unix::fs::MetadataExt::ino(&meta));
    #[cfg(not(unix))]
    let (device, number) = (0, 0);
    Some((device, number, meta.created().ok()))
}

/// The folders this command made, each as it was made, and the name of the file this making puts in the
/// project's folder: no other folder holds one of that name, on any system.
struct Made {
    projects: Option<Mark>,
    project: Option<Mark>,
    token: String,
}

impl Made {
    /// Nothing made yet, and a file's name no other making has: this process, the clock and a count.
    fn new() -> Self {
        static COUNT: AtomicU32 = AtomicU32::new(0);
        let now = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |since| since.as_nanos());
        let (id, count) = (std::process::id(), COUNT.fetch_add(1, Ordering::Relaxed));
        Made { projects: None, project: None, token: format!(".jellymoon-making-{id}-{now}-{count}") }
    }

    /// Whether the folder at the project's name is the one this command made: what the system says of it is
    /// as it was, and this making's file is in it.
    fn ours(&self, project: &Path) -> bool {
        let file = fs::symlink_metadata(project.join(&self.token)).is_ok_and(|meta| meta.is_file());
        self.project.is_some() && mark(project) == self.project && file
    }

    /// Takes back what this command made, and nothing else: the project's folder when it is still the one that
    /// was made and no line of the list names it, and `projects` when this command made it and it is empty. A
    /// made folder the list names stays, without this making's file. Whether the project's folder was removed.
    fn take_back(&self, home: &Path, folder: &Path, project: &Path) -> bool {
        let on_the_list = || {
            let real = fs::canonicalize(project).map(as_git_takes).unwrap_or_default();
            listed(home).is_ok_and(|lines| lines.iter().any(|line| Path::new(&line.folder) == real))
        };
        let (ours, mut gone) = (self.ours(project), false);
        if ours && on_the_list() {
            let _ = fs::remove_file(project.join(&self.token));
        } else if ours {
            gone = fs::remove_dir_all(project).is_ok();
        }
        if self.projects.is_some() && mark(folder) == self.projects {
            let _ = fs::remove_dir(folder);
        }
        gone
    }
}

/// Every look that makes nothing, in the order its refusals are said: the list, `projects`, what is at the name,
/// git, and whether what is made could be listed. Whether `projects` is there.
fn look(home: &Path, name: &str, day: &str) -> Result<bool, Error> {
    onto(&mut listed(home)?, name, "", day)?;
    let folder = home.join("projects");
    let there = folder_there(&folder)?;
    if there && taken(&folder, name) {
        return Err(refused(THERE));
    }
    if !copy::git_on_path() {
        return Err(refused(NO_GIT));
    }
    listable(home, name).map(|()| there)
}

/// Makes a project of that name and lists it, used on `day` (today when none is named). Refused, with nothing
/// made and nothing changed: a name that is no folder's name, one that is listed, one something is at already,
/// a list that is full or is not used, no git on `PATH`, a home a project could not be listed in, and a lock
/// that is held, which is waited for. Behind the lock every look is taken again and the folder is made;
/// whatever fails then (git, a list written over meanwhile) takes it back.
pub fn new_project(home: &Path, name: &str, day: Option<&str>) -> Result<Known, Error> {
    let day = day_of(day)?;
    if !makeable(name) {
        return Err(refused(NAME));
    }
    // Something at the name may be a making under way: that is asked again behind the lock, which waits for one.
    let first = look(home, name, &day);
    if !matches!(&first, Err(Error::Project(why)) if why == THERE) {
        first?;
    }
    let fresh = !home.exists();
    let lock = hold(home)?;
    let (folder, mut made) = (home.join("projects"), Made::new());
    let project = folder.join(name);
    let done = build(home, &project, name, &day, &mut made)
        .map_err(|e| if made.take_back(home, &folder, &project) { after(e) } else { e });
    drop(lock);
    // Jellymoon's folder, made to hold the lock, goes too when the making left nothing in it.
    if done.is_err() && fresh {
        let _ = fs::remove_dir(home);
    }
    done
}

/// A refusal that came after the folder was made and taken back, saying so when its own words do not.
fn after(e: Error) -> Error {
    match e.to_string() {
        said if said.contains("taken back") => e,
        said => refused(format!("{said}{BACK}")),
    }
}

/// The real folder a made project is listed by, from what `project add`'s own look says of the folder
/// (`stands_as`): its refusal is passed on, and only a folder kept under another name than was asked is the
/// file system's doing.
fn kept_as(asked: Result<(String, String), Error>, name: &str) -> Result<String, Error> {
    let (kept, real) = asked?;
    if kept == name { Ok(real) } else { Err(refused(SPELLED)) }
}

/// The making, behind the lock: the looks again, the two folders, the repo, a look at the name the file system
/// kept and at whose the folder is, and the list.
fn build(home: &Path, project: &Path, name: &str, day: &str, made: &mut Made) -> Result<Known, Error> {
    if !look(home, name, day)?
        && let Some(folder) = project.parent()
    {
        make(folder).map_err(Error::Store)?;
        made.projects = mark(folder);
    }
    make(project).map_err(|e| match e.kind() {
        ErrorKind::AlreadyExists => refused(THERE),
        _ => Error::Store(e),
    })?;
    // This making's file goes in before anything else does: a folder without it is not the one made here.
    if let Err(e) = fs::File::create_new(project.join(&made.token)) {
        let _ = fs::remove_dir(project);
        return Err(Error::Store(e));
    }
    made.project = mark(project);
    copy::fresh_repo(home, project, &format!("Jellymoon project {name}"))
        .map_err(|said| refused(format!("{said}, so what was made was taken back and nothing was listed")))?;
    let real = kept_as(stands_as(project), name)?;
    busy::pause("JELLYMOON_TEST_NEW_WAIT_MS");
    if !made.ours(project) {
        return Err(refused(MOVED));
    }
    let mut lines = listed(home)?;
    let known = onto(&mut lines, name, &real, day)?;
    write(home, lines)?;
    // Listed, the folder is a made project, and keeps nothing of its making.
    let _ = fs::remove_file(project.join(&made.token));
    Ok(known)
}

#[cfg(test)]
mod tests;
