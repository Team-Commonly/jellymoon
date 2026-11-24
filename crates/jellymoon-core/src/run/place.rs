//! The repo a wish runs in (`docs/development/run.md`, "The order"): the folder the person named, or the one
//! folder of the wish's project that was typed in that day, or else the folder listed under the project's name
//! (`project/offer.rs`, the lookup `wishable` asks too). A folder read from a session log is the log's word,
//! and a log can be written by whatever wrote to the harness's folder: it is looked up once ([`resolved`]), and
//! what is checked, and then copied, is the folder that lookup led to. A listed folder is the list's word, and
//! is used only as the real path it was listed by. Neither is ever put in a refusal.

use std::fs;
use std::path::{Path, PathBuf};

use super::{RunOptions, refused};
use crate::project::{Led, Offer, unused};
use crate::wish::MadeWish;
use crate::{DayOptions, Error, reader};

/// Where a run's folder came from: the person's command line, the day's session logs, or the list of projects.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum From {
    Named,
    Typed,
    Listed,
}

/// How a refusal of a listed folder, or of a name with two folders, ends: the two ways out.
const WAY_OUT: &str = "name one with --repo, or let it go with `jellymoon project forget`";

impl From {
    /// What is said when the folder is not one a copy can be made of. A folder that was not named on the command
    /// line is refused with copy's own reason, in fixed words: no folder above it is tried, and nothing git
    /// says of it is repeated.
    pub(super) fn uncopied(self, e: Error, day: &str) -> Error {
        let which = match (self, &e) {
            (From::Typed, Error::Copy(_)) => format!("the folder of that name typed in on {day}"),
            (From::Listed, Error::Copy(_)) => "the folder listed under the name the wish gives".to_string(),
            _ => return e,
        };
        refused(format!("{which} is not one a run can copy: {}", e.to_string().replacen("--repo", "it", 1)))
    }
}

/// The repo's folder. Its last name must be the wish's project.
pub(super) fn folder(home: &Path, day: &str, wish: &MadeWish, options: &RunOptions) -> Result<PathBuf, Error> {
    found(home, day, &wish.project, options).map(|(folder, _)| folder)
}

/// [`folder`] of the project `project`, and where it came from.
pub(super) fn found(home: &Path, day: &str, project: &str, options: &RunOptions) -> Result<(PathBuf, From), Error> {
    if let Some(repo) = &options.repo {
        if !repo.is_absolute() {
            return Err(refused("--repo must be a full path, from the root"));
        }
        let named = repo.file_name().is_some_and(|last| last == project);
        let why = "--repo is not the folder the wish is for: its last name is not the wish's project";
        return if named { Ok((repo.clone(), From::Named)) } else { Err(refused(why)) };
    }
    let (logs, skip) = (options.logs.clone().unwrap_or_default(), options.skip.clone());
    let read = DayOptions { logs, date: Some(day.to_string()), skip, home: Some(home.to_path_buf()) };
    let unknown = || refused("no session logs are known; name the folder with --repo");
    let typed = if options.logs.is_some() { reader::typed_in(&read) } else { Err(unknown()) };
    let offer = Offer::of(&read, typed)?;
    let none = format!("no folder of the name the wish gives was typed in on {day}");
    match offer.led(project) {
        Led::Typed(repo) => resolved(&repo, project, day).map(|folder| (folder, From::Typed)),
        Led::Listed(repo) => Ok((repo, From::Listed)),
        Led::Many(folders) => Err(refused(format!(
            "{folders} folders of the name the wish gives were typed in on {day}; name the one with --repo"
        ))),
        Led::Clash => Err(refused(format!(
            "a folder of the name the wish gives was typed in on {day} and another folder is listed under that \
             name; {WAY_OUT}"
        ))),
        Led::Fallen(why) => Err(refused(format!(
            "the folder listed under the name the wish gives is not one a run can start in: {why}; {WAY_OUT}"
        ))),
        Led::Nowhere => Err(match (offer.unread, &offer.lines) {
            (Some(unread), Ok(_)) => unread,
            (_, Ok(_)) => refused(format!("{none}; name the folder with --repo")),
            (_, Err(why)) => refused(format!("{none}, and {}; or name the folder with --repo", unused(why))),
        }),
    }
}

/// The folder a log's folder leads to, with every link and every `..` in it followed, once. The page names a
/// folder's project by the last name of its path (`session.rs`), and the caller found this folder by that name.
/// Where the last name is itself a link, the folder it leads to must carry the project's name too: a link named
/// after the project that leads to a folder of another name is refused. A last name that is no link is the
/// folder's own name, in whatever case or Unicode form it was typed: it is not compared with the form the file
/// system keeps, which is another spelling wherever the system takes the two for one name. A folder that is
/// someone else's is refused. The path returned is the one the copy is made of: nothing looks the log's own
/// path up a second time, so a link put in its place after this is not followed.
fn resolved(found: &Path, project: &str, day: &str) -> Result<PathBuf, Error> {
    let typed = format!("the folder of that name typed in on {day}");
    let real = fs::canonicalize(found)
        .map_err(|_| refused(format!("{typed} is not one a run can copy: it is not there, or cannot be reached")))?;
    let linked = fs::symlink_metadata(found).is_ok_and(|meta| meta.is_symlink());
    let last = if linked { real.file_name() } else { found.file_name() };
    if last.is_none_or(|last| last != project) {
        return Err(refused(format!("{typed} leads to a folder of another name; name the folder with --repo")));
    }
    if theirs(&real) {
        return Err(refused(format!("{typed} is not a folder of your own; name the folder with --repo")));
    }
    Ok(plain(real))
}

/// Whether a folder that is there is owned by someone other than the person, who is taken to be the owner of
/// their own home folder (`command.rs`). With no home every folder is someone else's. Windows has no such owner
/// to read.
#[cfg(unix)]
pub(crate) fn theirs(folder: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;
    let person = crate::user_home().and_then(|home| fs::metadata(home).ok()).map(|home| home.uid());
    fs::metadata(folder).is_ok_and(|folder| Some(folder.uid()) != person)
}

#[cfg(not(unix))]
pub(crate) fn theirs(_: &Path) -> bool {
    false
}

/// A resolved path as git takes one. Windows resolves a path on a drive to `\\?\C:\...`, which git does not open:
/// the same path is given without that start. Any other path is as it was resolved.
pub(crate) fn plain(real: PathBuf) -> PathBuf {
    let drive = |rest: &str| rest.as_bytes().get(1) == Some(&b':');
    match real.to_str().and_then(|text| text.strip_prefix(r"\\?\")).filter(|rest| cfg!(windows) && drive(rest)) {
        Some(rest) => PathBuf::from(rest),
        None => real,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_resolved_path_is_given_to_git_as_it_is_and_on_windows_without_the_long_path_start() {
        let long = PathBuf::from(r"\\?\C:\work\alpha");
        let want = if cfg!(windows) { PathBuf::from(r"C:\work\alpha") } else { long.clone() };
        assert_eq!(plain(long), want);
        for kept in [r"\\?\UNC\server\share\alpha", "/work/alpha", r"C:\work\alpha"] {
            assert_eq!(plain(PathBuf::from(kept)), PathBuf::from(kept));
        }
    }
}
