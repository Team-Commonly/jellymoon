//! Taking Jellymoon off a machine: the two things `install.sh` put there, and the delivery hook when one is installed.
//! What the person's journal holds (`JELLYMOON_HOME`) is theirs and stays.

use std::path::{Path, PathBuf};
use std::{fmt, fs, io};

use serde::Serialize;

use crate::Error;

/// Where the install put its two pieces.
pub struct UninstallOptions {
    /// The app bundle, a folder.
    pub app: PathBuf,
    /// The `jellymoon` binary.
    pub binary: PathBuf,
    /// Jellymoon's home, which holds the record of the delivery hook. None when it cannot be found: then no hook is
    /// looked for.
    pub home: Option<PathBuf>,
}

/// What an uninstall did: the paths it removed, and whether it took a delivery hook out of the harness's settings.
#[derive(Debug, Serialize)]
pub struct Uninstalled {
    pub removed: Vec<String>,
    pub hook: bool,
    /// The hook's entry was edited since it was installed, so it was left: the person removes it by hand.
    pub hook_by_hand: bool,
}

/// Prints as its JSON on one line, with no line break: what `jellymoon uninstall` prints.
impl fmt::Display for Uninstalled {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&serde_json::to_string(self).map_err(|_| fmt::Error)?)
    }
}

/// Removes the delivery hook, then the app, then the binary. The hook goes first: if it cannot be removed, the binary
/// is still there to try again with. A hook entry the person edited is left for them and everything else still goes.
/// A piece that is already gone is not an error and is not listed. A place that is not a full path is refused before
/// anything is touched: what it named would depend on the folder the command was started in.
pub fn uninstall(options: &UninstallOptions) -> Result<Uninstalled, Error> {
    if options.home.as_ref().is_some_and(|home| !home.is_absolute()) {
        return Err(Error::Home);
    }
    if !(options.app.is_absolute() && options.binary.is_absolute()) {
        return Err(Error::NotAbsolute);
    }
    let (hook, hook_by_hand) = match options.home.as_deref().map(crate::remove_hook) {
        None => (false, false),
        Some(Ok(state)) => (state.changed, false),
        Some(Err(Error::Changed)) => (false, true),
        Some(Err(failed)) => return Err(failed),
    };
    let mut removed = Vec::new();
    for path in [&options.app, &options.binary] {
        if remove(path).map_err(Error::Uninstall)? {
            removed.push(path.display().to_string());
        }
    }
    Ok(Uninstalled { removed, hook, hook_by_hand })
}

/// Removes a file, a link (never what it points at) or a folder with all it holds. False when nothing was there.
fn remove(path: &Path) -> io::Result<bool> {
    let kind = match fs::symlink_metadata(path) {
        Ok(found) => found.file_type(),
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(e),
    };
    if kind.is_dir() {
        fs::remove_dir_all(path)?
    } else {
        fs::remove_file(path)?
    }
    Ok(true)
}
