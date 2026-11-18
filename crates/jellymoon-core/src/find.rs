//! Finding the harness (`docs/development/page.md`, "Where the harness is looked for"). `PATH` first, as a terminal
//! finds it. An app opened from Finder has a `PATH` of four system folders, so after it comes a short fixed list of the
//! folders Claude Code's installers use. When none of those holds it, on a Mac and on Linux the person's login
//! shell is asked for its `PATH`, and those folders are looked in (`find/shell.rs`; ADR 0021, decision 1). No
//! folder is listed and nothing is searched.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::{env, fmt, fs, io, path};

use crate::Error;

mod said;
mod shell;

pub use said::{HarnessFound, How, harness_found};
pub(crate) use shell::Shell;

/// The harness's file names, in the order tried in each folder. On Windows it is a program or a command script.
const PROGRAMS: &[&str] = if cfg!(windows) { &["claude.exe", "claude.cmd", "claude"] } else { &["claude"] };

/// Under the person's home: the native installer's folder, then the older local install's.
const USER: &[&[&str]] = &[&[".local", "bin"], &[".claude", "local"]];

/// Homebrew on Apple chips, then Homebrew on Intel chips and what an administrator installed by hand.
const SYSTEM: &[&str] = if cfg!(unix) { &["/opt/homebrew/bin", "/usr/local/bin"] } else { &[] };

/// What a harness in a folder the login shell named is called when it is passed over: no folder is named.
const BY_SHELL: &str = "a `claude` in a folder of the login shell's PATH";

/// The harness, the `PATH` it is started with when that is not this process's own, and how it was found.
#[derive(Clone)]
pub(crate) struct Found {
    pub program: PathBuf,
    pub path: Option<OsString>,
    pub how: How,
}

/// The harness a caller named, as it is, or the one found.
pub(crate) fn named_or_found(named: Option<&Path>) -> Result<Found, Error> {
    match named {
        Some(harness) => {
            Ok(Found { program: path::absolute(harness).map_err(Error::Call)?, path: None, how: How::Named })
        }
        None => harness(),
    }
}

/// A search that found no harness: where it looked, each program it passed over with why, whether there was a
/// home to look in, and how the ask of the login shell ended.
pub(crate) struct Missed {
    pub looked: Vec<String>,
    pub passed_over: Vec<String>,
    pub home: bool,
    pub shell: Shell,
}

impl fmt::Display for Missed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.looked.join(", "))?;
        if !self.home {
            f.write_str("; ~/.local/bin and ~/.claude/local were not looked in, as there is no home directory")?;
        }
        if self.passed_over.is_empty() { Ok(()) } else { write!(f, " (passed over {})", self.passed_over.join(", ")) }
    }
}

/// What the search for a harness missed, when none is named and none is found. It is one search: a caller with
/// words of its own for no harness takes them from this, and asks no second time.
pub(crate) fn missed(named: Option<&Path>) -> Option<Missed> {
    if named.is_some() {
        return None;
    }
    search().ok()?.err()
}

/// Finds `claude` as [`search`] does; the error says where it was looked for, and names what was passed over.
pub(crate) fn harness() -> Result<Found, Error> {
    search()?.map_err(|missed| Error::NoHarness(missed.to_string()))
}

/// Looks for `claude` in the folders of `PATH`, then in the well-known places, then in the folders the login shell
/// names. A program in a well-known place that someone other than its owner can write to, or whose folder everyone
/// can write to, is passed over, and is named in what was missed when no later place has one; one in a folder of
/// the login shell's is held to one check more ([`unfit`]). The login shell is asked only when nothing came before.
pub(crate) fn search() -> Result<Result<Found, Missed>, Error> {
    let own = env::var_os("PATH").unwrap_or_default();
    let on_path: Vec<PathBuf> = env::split_paths(&own).filter(|folder| !folder.as_os_str().is_empty()).collect();
    // The child starts in another folder, so a relative entry of PATH must not be read from there.
    let whole = |program: PathBuf| path::absolute(program).map_err(Error::Call);
    if let Some((program, _)) = on_path.iter().flat_map(|folder| programs(folder)).next() {
        return Ok(Ok(Found { program: whole(program)?, path: None, how: How::Path }));
    }
    let home = user_home();
    let places = places(home.as_deref());
    let found = |program: PathBuf, how: How| -> Result<Result<Found, Missed>, Error> {
        let path = Some(longer(&on_path, &program, &places)?);
        Ok(Ok(Found { program: whole(program)?, path, how }))
    };
    let mut looked = vec!["PATH".to_string()];
    let mut passed_over = Vec::new();
    for (folder, name) in &places {
        for (program, mode) in programs(folder) {
            match unfit(folder, &program, mode, false) {
                Some(why) => passed_over
                    .push(format!("{name}/{}: {why}", program.file_name().unwrap_or_default().to_string_lossy())),
                None => return found(program, How::FixedPlace),
            }
        }
        looked.push(name.clone());
    }
    let shell = match by_shell(&mut passed_over) {
        Ok(program) => return found(program, How::LoginShell),
        Err(shell) => shell,
    };
    looked.extend(shell.place());
    Ok(Err(Missed { looked, passed_over, home: home.is_some(), shell }))
}

/// The first harness in a folder the login shell named that passes the checks, or how the ask ended with none.
/// One that is passed over is said once for each reason, and its folder is not named: it is the person's own.
fn by_shell(passed_over: &mut Vec<String>) -> Result<PathBuf, Shell> {
    for folder in shell::folders()? {
        for (program, mode) in programs(&folder) {
            let Some(why) = unfit(&folder, &program, mode, true) else { return Ok(program) };
            let said = format!("{BY_SHELL}: {why}");
            if !passed_over.contains(&said) {
                passed_over.push(said);
            }
        }
    }
    Err(Shell::Answered)
}

/// Why a program found in `folder` is passed over, when it is: everyone can write to the folder, or someone other
/// than its owner to the file, whose mode was read after links were followed. With `resolved`, for a folder
/// Jellymoon did not choose, the folder the link-resolved file is in is held to the folder's check too.
fn unfit(folder: &Path, program: &Path, mode: u32, resolved: bool) -> Option<&'static str> {
    let real = || fs::canonicalize(program).ok().and_then(|real| Some(everyone_writes(real.parent()?)));
    if everyone_writes(folder) {
        Some("everyone can write to its folder")
    } else if mode & 0o022 != 0 {
        Some("others can write to it")
    } else if resolved && real() != Some(false) {
        Some("everyone can write to the folder its file is in")
    } else {
        None
    }
}

/// Whether everyone can write to a folder. One that is not there is one nobody can write to.
fn everyone_writes(folder: &Path) -> bool {
    fs::metadata(folder).is_ok_and(|meta| mode(&meta) & 0o002 != 0)
}

/// The `PATH` a harness found off `PATH` is started with: this process's, then the folder it was found in, then
/// the well-known places. A script that starts `env node` finds node beside itself or in the same places. A
/// folder whose name cannot be written in a PATH (it holds the mark that parts the folders) is left out of it.
fn longer(on_path: &[PathBuf], program: &Path, places: &[(PathBuf, String)]) -> Result<OsString, Error> {
    let after = program.parent().into_iter().chain(places.iter().map(|(folder, _)| folder.as_path()));
    let after = after.filter(|folder| env::join_paths([folder]).is_ok()).map(Path::to_path_buf);
    let longer = env::join_paths(on_path.iter().cloned().chain(after));
    longer.map_err(|e| Error::Call(io::Error::new(io::ErrorKind::InvalidInput, e)))
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
