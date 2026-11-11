//! Finding the harness (`docs/development/page.md`, "Where the harness is looked for"). `PATH` first, as a terminal
//! finds it. An app opened from Finder has a `PATH` of four system folders, so after it comes a short fixed list of the
//! folders Claude Code's installers use. No folder is listed, no shell is asked, nothing is searched.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::{env, fs, path};

use crate::{Error, PageOptions};

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
/// its owner can write to is passed over, and the error says so.
pub(crate) fn harness(options: &PageOptions) -> Result<Found, Error> {
    let own = env::var_os("PATH").unwrap_or_default();
    let on_path: Vec<PathBuf> = env::split_paths(&own).filter(|folder| !folder.as_os_str().is_empty()).collect();
    // The child starts in another folder, so a relative entry of PATH must not be read from there.
    let whole = |program: PathBuf| path::absolute(program).map_err(Error::Call);
    if let Some((program, _)) = on_path.iter().flat_map(|folder| programs(folder)).next() {
        return Ok(Found { program: whole(program)?, path: None });
    }
    let places = places(options);
    let mut looked = vec!["PATH".to_string()];
    let mut skipped = Vec::new();
    for (folder, name) in &places {
        for (program, mode) in programs(folder) {
            if mode & 0o022 != 0 {
                let file = program.file_name().unwrap_or_default().to_string_lossy();
                skipped.push(format!("{name}/{file}"));
                continue;
            }
            // A script that starts `env node` finds node beside itself or in the same places.
            let mut folders = on_path;
            folders.extend(program.parent().map(Path::to_path_buf));
            folders.extend(places.iter().map(|(folder, _)| folder.clone()));
            return Ok(Found { program: whole(program)?, path: env::join_paths(folders).ok() });
        }
        looked.push(name.clone());
    }
    let mut said = looked.join(", ");
    if !skipped.is_empty() {
        said = format!("{said} (passed over {}: others can write to it)", skipped.join(", "));
    }
    Err(Error::NoHarness(said))
}

/// The well-known folders in the order tried, each with the name a person knows it by.
fn places(options: &PageOptions) -> Vec<(PathBuf, String)> {
    let user = options.home.iter().flat_map(|home| {
        USER.iter().map(move |parts| (parts.iter().fold(home.clone(), |at, part| at.join(part)), parts.join("/")))
    });
    let user = user.map(|(folder, under)| (folder, format!("~/{under}")));
    let named = |folder: PathBuf| (folder.clone(), folder.display().to_string());
    match &options.system {
        Some(system) => user.chain(system.iter().cloned().map(named)).collect(),
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
