//! Finding the harness (`docs/development/page.md`, "Where the harness is looked for"). `PATH` first, as a terminal
//! finds it. An app opened from Finder has a `PATH` of four system folders, so after it comes a short fixed list of the
//! folders Claude Code's installers use. No folder is listed, no shell is asked, nothing is searched.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::{env, fs, io, path};

use crate::Error;

/// The harness's file names, in the order tried in each folder. On Windows it is a program or a command script.
const PROGRAMS: &[&str] = if cfg!(windows) { &["claude.exe", "claude.cmd", "claude"] } else { &["claude"] };

/// Under the person's home: the native installer's folder, then the older local install's.
const USER: &[&[&str]] = &[&[".local", "bin"], &[".claude", "local"]];

/// Homebrew on Apple chips, then Homebrew on Intel chips and what an administrator installed by hand.
const SYSTEM: &[&str] = if cfg!(unix) { &["/opt/homebrew/bin", "/usr/local/bin"] } else { &[] };

/// The harness, and the `PATH` it is started with when that is not this process's own.
pub(crate) struct Found {
    pub program: PathBuf,
    pub path: Option<OsString>,
}

/// Finds `claude` in the folders of `PATH`, then in the well-known places. A program there that someone other than
/// its owner can write to, or whose folder everyone can write to, is passed over; the error names it when no later
/// place has one.
pub(crate) fn harness() -> Result<Found, Error> {
    let own = env::var_os("PATH").unwrap_or_default();
    let on_path: Vec<PathBuf> = env::split_paths(&own).filter(|folder| !folder.as_os_str().is_empty()).collect();
    // The child starts in another folder, so a relative entry of PATH must not be read from there.
    let whole = |program: PathBuf| path::absolute(program).map_err(Error::Call);
    if let Some((program, _)) = on_path.iter().flat_map(|folder| programs(folder)).next() {
        return Ok(Found { program: whole(program)?, path: None });
    }
    let home = user_home();
    let places = places(home.as_deref());
    let mut looked = vec!["PATH".to_string()];
    let mut skipped = Vec::new();
    for (folder, name) in &places {
        let open = fs::metadata(folder).is_ok_and(|meta| mode(&meta) & 0o002 != 0);
        for (program, mode) in programs(folder) {
            if open || mode & 0o022 != 0 {
                let why = if open { "everyone can write to its folder" } else { "others can write to it" };
                skipped.push(format!("{name}/{}: {why}", program.file_name().unwrap_or_default().to_string_lossy()));
                continue;
            }
            // A script that starts `env node` finds node beside itself or in the same places. A folder whose name
            // cannot be written in a PATH (it holds the mark that parts the folders) is left out of it.
            let after = program.parent().into_iter().chain(places.iter().map(|(folder, _)| folder.as_path()));
            let after = after.filter(|folder| env::join_paths([folder]).is_ok()).map(Path::to_path_buf);
            let longer = env::join_paths(on_path.into_iter().chain(after));
            let longer = longer.map_err(|e| Error::Call(io::Error::new(io::ErrorKind::InvalidInput, e)))?;
            return Ok(Found { program: whole(program)?, path: Some(longer) });
        }
        looked.push(name.clone());
    }
    let mut said = looked.join(", ");
    if home.is_none() {
        said += "; ~/.local/bin and ~/.claude/local were not looked in, as there is no home directory";
    }
    if !skipped.is_empty() {
        said = format!("{said} (passed over {})", skipped.join(", "));
    }
    Err(Error::NoHarness(said))
}

/// The person's home: `HOME` (`USERPROFILE` on Windows) and nothing else, and only when it names a full path, since
/// where a relative one led would depend on the folder the program was started in. The system's own record of the
/// account is not asked, so a run given no home, or a scratch one, can never reach what is the person's at the machine.
pub fn user_home() -> Option<PathBuf> {
    env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).map(PathBuf::from).filter(|home| home.is_absolute())
}

/// Where Jellymoon keeps its own files: `JELLYMOON_HOME`, or `.jellymoon` in the person's home; none with neither. A
/// `JELLYMOON_HOME` that is set and not a full path is refused, for every command and the app alike: the list of
/// folders switched off, the rulings and the hook's record would be other files in every folder a program starts in.
pub fn own_folder() -> Result<Option<PathBuf>, Error> {
    match env::var_os("JELLYMOON_HOME").filter(|home| !home.is_empty()).map(PathBuf::from) {
        Some(home) if home.is_absolute() => Ok(Some(home)),
        Some(_) => Err(Error::Home),
        None => Ok(user_home().map(|home| home.join(".jellymoon"))),
    }
}

/// The well-known folders in the order tried, each with the name a person knows it by.
/// `JELLYMOON_SYSTEM_BIN`, folders written as `PATH` is, takes the place of the system's: what the tests set.
fn places(home: Option<&Path>) -> Vec<(PathBuf, String)> {
    let user = home.into_iter().flat_map(|home| {
        USER.iter().map(move |parts| (parts.iter().fold(home.to_path_buf(), |at, part| at.join(part)), parts.join("/")))
    });
    let user = user.map(|(folder, under)| (folder, format!("~/{under}")));
    let named = |folder: PathBuf| (folder.clone(), folder.display().to_string());
    match env::var_os("JELLYMOON_SYSTEM_BIN") {
        Some(system) => {
            let system = env::split_paths(&system).filter(|folder| !folder.as_os_str().is_empty());
            user.chain(system.map(named)).collect()
        }
        None => user.chain(SYSTEM.iter().map(PathBuf::from).map(named)).collect(),
    }
}

/// The harness's names in `folder` that are a file, or a link to one, that can be run, each with its mode.
fn programs(folder: &Path) -> impl Iterator<Item = (PathBuf, u32)> + '_ {
    PROGRAMS.iter().map(move |name| folder.join(name)).filter_map(|program| {
        let meta = fs::metadata(&program).ok().filter(fs::Metadata::is_file)?;
        let mode = mode(&meta);
        (mode & 0o111 != 0).then_some((program, mode))
    })
}

#[cfg(unix)]
fn mode(meta: &fs::Metadata) -> u32 {
    std::os::unix::fs::PermissionsExt::mode(&meta.permissions())
}

/// Windows has no such modes: a file with one of the harness's names can be run, and who may write to it is the
/// system's own rule for the folder.
#[cfg(not(unix))]
fn mode(_: &fs::Metadata) -> u32 {
    0o755
}
