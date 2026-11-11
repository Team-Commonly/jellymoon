//! The check of the installed `jellymoon` command before the hook may name it (F36, `docs/development/delivery.md`,
//! "Turning delivery on from the app"). Claude Code runs that file at every step of every session, so a file or a
//! folder that someone else could change is never named. What is looked at is looked at once, before the hook entry
//! is written: a change after the check is not seen.

use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};
use std::{env, fs};

use crate::hook::install_hook;
use crate::{Error, HookState};

/// The most links followed on the way to the command's folder.
#[cfg(unix)]
const LINKS_MAX: usize = 32;

/// The installed `jellymoon` command in `bin`, the folder `install.sh` puts it in, when the hook may name it. The
/// folder is an absolute path. The file is a plain file of that name, never a link (the installer copies a file),
/// and not empty. Where the system has modes it can also be run, only its owner can write to it, it is not setuid
/// or setgid, its owner is the owner of the person's home folder, and no folder on the way to it, from the root
/// down and with every link followed, can be written to by its group or by everyone unless it is sticky. Windows
/// has no such modes: there it is a plain file of that name in that folder, and who may write is the system's rule.
pub fn installed_command(bin: &Path) -> Result<PathBuf, Error> {
    let not = |why: &str| Error::Command(why.to_string());
    if !bin.is_absolute() {
        return Err(not("its folder is not an absolute path"));
    }
    let named = bin.join(format!("jellymoon{}", env::consts::EXE_SUFFIX));
    let meta = fs::symlink_metadata(&named).map_err(|_| not("it was not found"))?;
    if meta.file_type().is_symlink() {
        return Err(not("it is a link"));
    }
    if !meta.is_file() || meta.len() == 0 {
        return Err(not("it is not a program"));
    }
    #[cfg(unix)]
    sound(&meta, bin).map_err(not)?;
    Ok(named)
}

/// The modes and the owner of the command, then the folders on the way to it.
#[cfg(unix)]
fn sound(meta: &fs::Metadata, bin: &Path) -> Result<(), &'static str> {
    use std::os::unix::fs::MetadataExt;
    if meta.mode() & 0o111 == 0 {
        return Err("it cannot be run");
    }
    if meta.mode() & 0o022 != 0 {
        return Err("others can write to it");
    }
    if meta.mode() & 0o6000 != 0 {
        return Err("it is setuid or setgid");
    }
    // The person is taken to be the owner of their own home folder: the one id that is read with no call of the
    // system's own. With no home, or a home of someone else, the command is not used.
    let person = crate::user_home().and_then(|home| fs::metadata(home).ok()).map(|home| home.uid());
    if person != Some(meta.uid()) {
        return Err("it is not a file of your own");
    }
    folders(bin)
}

/// Every folder on the way to `bin`, from the root down, as it really is: a link is read and its own path walked in
/// turn, so a folder that only a link passes through is looked at too. None may be written to by its group or by
/// everyone, unless it is sticky (there, only a file's owner can replace it).
#[cfg(unix)]
fn folders(bin: &Path) -> Result<(), &'static str> {
    use std::os::unix::fs::MetadataExt;
    let parts = |path: &Path| path.components().rev().map(|part| part.as_os_str().to_os_string()).collect::<Vec<_>>();
    let (mut left, mut at, mut links) = (parts(bin), PathBuf::new(), 0);
    while let Some(part) = left.pop() {
        match part.to_str() {
            Some(".") => continue,
            Some("..") => {
                at.pop();
                continue;
            }
            _ => at.push(&part),
        }
        let meta = fs::symlink_metadata(&at).map_err(|_| "a folder on the way to it could not be read")?;
        if meta.file_type().is_symlink() {
            links += 1;
            let to = fs::read_link(&at).ok().filter(|_| links <= LINKS_MAX);
            let to = to.ok_or("the links on the way to it could not be followed")?;
            at.pop();
            left.extend(parts(&to));
        } else if !meta.is_dir() {
            return Err("its folder is not a folder");
        } else if meta.mode() & 0o022 != 0 && meta.mode() & 0o1000 == 0 {
            return Err("others can write to a folder on the way to it");
        }
    }
    Ok(())
}

/// The tap on "Turn on delivery", and `jellymoon hook install --installed`: [`install_hook`] for the checked
/// [`installed_command`]. The settings file is named by an absolute path, since where a relative one led would
/// depend on the folder the program was started in.
pub fn turn_on_delivery(settings: &Path, home: &Path, bin: &Path) -> Result<HookState, Error> {
    if !settings.is_absolute() {
        let why = "its folder is not an absolute path (CLAUDE_CONFIG_DIR)";
        return Err(Error::Settings(io::Error::new(ErrorKind::InvalidInput, why)));
    }
    install_hook(settings, home, &installed_command(bin)?)
}
