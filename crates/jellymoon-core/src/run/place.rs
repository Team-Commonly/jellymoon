//! The repo a wish runs in (`docs/development/run.md`, "The order"): the folder the person named, or the one
//! folder of the wish's project that was typed in that day. A folder read from a session log is the log's word,
//! and a log can be written by whatever wrote to the harness's folder: it is looked up once ([`resolved`]), and
//! what is checked, and then copied, is the folder that lookup led to. It is never put in a refusal.

use std::fs;
use std::path::{Path, PathBuf};

use super::{RunOptions, refused};
use crate::wish::MadeWish;
use crate::{DayOptions, Error, reader};

/// The repo's folder. Its last name must be the wish's project.
pub(super) fn folder(home: &Path, day: &str, wish: &MadeWish, options: &RunOptions) -> Result<PathBuf, Error> {
    if let Some(repo) = &options.repo {
        if !repo.is_absolute() {
            return Err(refused("--repo must be a full path, from the root"));
        }
        let named = repo.file_name().is_some_and(|last| last == wish.project.as_str());
        let why = "--repo is not the folder the wish is for: its last name is not the wish's project";
        return if named { Ok(repo.clone()) } else { Err(refused(why)) };
    }
    let logs = options.logs.clone().ok_or_else(|| refused("no session logs are known; name the folder with --repo"))?;
    let skip = options.skip.clone();
    let read = DayOptions { logs, date: Some(day.to_string()), skip, home: Some(home.to_path_buf()) };
    let mut found = reader::folders(&read, &wish.project)?;
    match (found.pop(), found.len()) {
        (Some(repo), 0) => resolved(&repo, &wish.project, day),
        (None, _) => Err(refused(format!(
            "no folder of the name the wish gives was typed in on {day}; name the folder with --repo"
        ))),
        (Some(_), more) => Err(refused(format!(
            "{} folders of the name the wish gives were typed in on {day}; name the one with --repo",
            more + 1
        ))),
    }
}

/// The folder a log's folder leads to, with every link and every `..` in it followed, once. The page names a
/// folder's project by the last name of its path (`session.rs`), so the wish's project must be the last name of
/// the folder itself: a link named after the project that leads to a folder of another name is refused. So is a
/// folder that is someone else's. The path returned is the one the copy is made of: nothing looks the log's own
/// path up a second time, so a link put in its place after this is not followed.
fn resolved(found: &Path, project: &str, day: &str) -> Result<PathBuf, Error> {
    let typed = format!("the folder of that name typed in on {day}");
    let real = fs::canonicalize(found)
        .map_err(|_| refused(format!("{typed} is not one a run can copy: it is not there, or cannot be reached")))?;
    if real.file_name().is_none_or(|last| last != project) {
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
fn theirs(folder: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;
    let person = crate::user_home().and_then(|home| fs::metadata(home).ok()).map(|home| home.uid());
    fs::metadata(folder).is_ok_and(|folder| Some(folder.uid()) != person)
}

#[cfg(not(unix))]
fn theirs(_: &Path) -> bool {
    false
}

/// A resolved path as git takes one. Windows resolves a path on a drive to `\\?\C:\...`, which git does not open:
/// the same path is given without that start. Any other path is as it was resolved.
fn plain(real: PathBuf) -> PathBuf {
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
