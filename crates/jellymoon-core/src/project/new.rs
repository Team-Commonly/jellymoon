//! A project Jellymoon makes (F65, ADR 0020, decision 3; `docs/development/project.md`, "A new project"):
//! [`new_project`] makes a folder of the name directly inside `projects` in Jellymoon's own folder, makes it a
//! repo with one empty first commit (`copy/fresh.rs`) and lists it. Nothing is written outside that one folder:
//! the name is one folder's name and no path, and what is at a name already, a link too, is never followed and
//! never replaced. A step that fails takes the folder back, so a project is listed or is not there.

use std::fs;
use std::io::{self, ErrorKind};
use std::path::Path;

use super::{Known, Line, change, listed, put_on, refused, stands_as};
use crate::ruling::private_dir;
use crate::wish::{day_of, named};
use crate::{Error, busy, copy};

const NAME: &str = "project new takes a name for the new project: one folder's name of at most 100 characters, \
                    with no dot, dash or space at its start, no dot or space at its end, no colon, no name a \
                    system keeps for a device, nothing that cannot be seen and nothing shaped like a credential; \
                    nothing was made";
const LISTED: &str = "a project of that name is on the list already, so nothing was made; pick another name, or let \
                      the listed one go with `jellymoon project forget`";
const THERE: &str = "something is already there under that name in the folder Jellymoon makes projects in (projects \
                     in Jellymoon's folder), so nothing was made; pick another name, list what is there with \
                     `jellymoon project add --repo` and its folder, or, if a `project new` that was cut off left \
                     it, remove it and ask again";
const NO_FOLDER: &str = "the folder Jellymoon makes projects in (projects in Jellymoon's folder) is not a folder \
                         Jellymoon made: a link or a file is at its name; nothing was made";
const NO_GIT: &str = "no `git` program was found on PATH, and a new project is a git repo, so nothing was made";
const SPELLED: &str = "this file system does not keep the folder under the name it was given, so what was made was \
                       taken back and nothing was listed; pick a name of plain letters";

/// Whether a name is one a folder can be made under on every system: a project's name as the list takes one
/// ([`named`], which takes no space at either end), that no program reads as an option, a hidden file, a drive
/// or a device, and that no system cuts a dot from.
fn makeable(name: &str) -> bool {
    let stem = name.split('.').next().unwrap_or_default().to_ascii_uppercase();
    let numbered = stem.len() == 4 && stem.ends_with(|last: char| last.is_ascii_digit());
    let device = matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (numbered && (stem.starts_with("COM") || stem.starts_with("LPT")));
    named(name) && !name.starts_with(['.', '-']) && !name.ends_with('.') && !name.contains(':') && !device
}

/// Two names that are one where a file system takes no note of the case of letters. They are one name here on
/// every system, so a list means the same wherever it is read.
fn same(one: &str, other: &str) -> bool {
    one.to_lowercase() == other.to_lowercase()
}

/// Lists a made project. A name that is listed, in whatever case and for whatever folder, is taken.
fn onto(lines: &mut Vec<Line>, name: &str, folder: &str, day: &str) -> Result<Known, Error> {
    if lines.iter().any(|line| same(&line.name, name)) {
        return Err(refused(LISTED));
    }
    put_on(lines, name, folder, day)
}

/// Whether `projects` is there, as a real folder. A link or a file at its name is refused, and never followed.
fn stands(folder: &Path) -> Result<bool, Error> {
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

/// Makes one folder for the person alone (0700). The folder above must be there, and the name must be free:
/// what is at it, a link too, is an error, so nothing is ever made through one.
fn make(folder: &Path) -> io::Result<()> {
    let mut make = fs::DirBuilder::new();
    #[cfg(unix)]
    std::os::unix::fs::DirBuilderExt::mode(&mut make, 0o700);
    make.create(folder)
}

/// Makes a project of that name and lists it, used on `day` (today when none is named). Refused, with nothing
/// made and nothing changed: a name that is no folder's name, one that is listed, one something is at already,
/// a list that is full or is not used, and no git on `PATH`. After that the folder is made, and whatever fails
/// then (git, the list's lock, a list that changed meanwhile) takes it back.
pub fn new_project(home: &Path, name: &str, day: Option<&str>) -> Result<Known, Error> {
    let day = day_of(day)?;
    if !makeable(name) {
        return Err(refused(NAME));
    }
    onto(&mut listed(home)?, name, "", &day)?;
    let folder = home.join("projects");
    let there = stands(&folder)?;
    if there && taken(&folder, name) {
        return Err(refused(THERE));
    }
    if !copy::git_on_path() {
        return Err(refused(NO_GIT));
    }
    private_dir(home).map_err(Error::Store)?;
    let ours = !there && make(&folder).is_ok();
    let project = folder.join(name);
    let made = stands(&folder).and_then(|_| {
        make(&project).map_err(|e| match e.kind() {
            ErrorKind::AlreadyExists => refused(THERE),
            _ => Error::Store(e),
        })
    });
    let done = made.and_then(|()| {
        let done = finish(home, &project, name, &day);
        if done.is_err() {
            let _ = fs::remove_dir_all(&project);
        }
        done
    });
    if done.is_err() && ours {
        let _ = fs::remove_dir(&folder);
    }
    done
}

/// The steps after the folder is made: the repo, a look at the name the file system kept, and the list, behind
/// its lock, where a name listed meanwhile is found.
fn finish(home: &Path, project: &Path, name: &str, day: &str) -> Result<Known, Error> {
    copy::fresh_repo(home, project, &format!("Jellymoon project {name}"))
        .map_err(|said| refused(format!("{said}, so what was made was taken back and nothing was listed")))?;
    let kept = stands_as(project).ok().filter(|(kept, _)| kept == name);
    let (_, real) = kept.ok_or_else(|| refused(SPELLED))?;
    busy::pause("JELLYMOON_TEST_NEW_WAIT_MS");
    change(home, |lines| onto(lines, name, &real, day))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_is_makeable_when_no_system_reads_it_as_anything_but_one_folder() {
        for name in ["alpha", "my-app", "a.b", "v1.2", "caf\u{e9}", "COM", "COM10", "console", "nul0", "a b"] {
            assert!(makeable(name), "{name}");
        }
        let odd = [
            "",
            ".",
            "..",
            "a/b",
            "a\\b",
            ".a",
            "-a",
            " a",
            "a.",
            "a ",
            "a:b",
            "C:",
            "con",
            "NUL",
            "nul.txt",
            "PRN",
            "aux.tar.gz",
            "com1",
            "COM9.log",
            "lpt0",
            "LPT5",
        ];
        for name in odd {
            assert!(!makeable(name), "{name}");
        }
        assert!(same("Alpha", "aLPHA") && !same("alpha", "alpha1"));
    }
}
