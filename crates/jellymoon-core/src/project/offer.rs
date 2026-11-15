//! The one lookup from a project's name to its folder (ADR 0020, decision 2): what `wishable` offers and where a
//! run starts are both asked here, so what is offered is what starts. A name leads to the one folder of that
//! name typed in on the day, or else to the folder listed under it, when that folder passes, now, the checks a
//! run makes of a folder it did not get from the person's own command line: it is there, by the path that was
//! listed and through no link; it is the top of a repo; it is the person's own; it is not switched off. git
//! itself is asked only when a run copies the folder. A name both typed in and listed leads nowhere unless the
//! two are one folder.

use std::fs;
use std::path::{Path, PathBuf};

use super::{Line, kept};
use crate::run::{plain, theirs};
use crate::{DayOptions, Error, reader};

/// Where a project's name leads on a day.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Led {
    /// To the one folder of the name typed in that day, as the log has it.
    Typed(PathBuf),
    /// To the folder listed under the name, which stands.
    Listed(PathBuf),
    /// Nowhere: no folder of the name was typed in, and none is listed.
    Nowhere,
    /// To this many folders typed in that day.
    Many(usize),
    /// To a folder typed in and another one listed.
    Clash,
    /// To a listed folder a run cannot start in, and why, in fixed words.
    Fallen(&'static str),
}

/// A day's folders and the list, read once.
pub(crate) struct Offer {
    typed: Vec<PathBuf>,
    /// Why nothing typed is known, when no session logs are.
    pub unread: Option<Error>,
    /// The list's lines, or why the list is not used.
    pub lines: Result<Vec<Line>, String>,
    off: Vec<String>,
}

impl Offer {
    /// `typed` is the day's typed-in folders as the reader gave them. No session logs at all (the reader's word,
    /// or a run's that was given none) is a day nothing was typed on, and is remembered; any other error is the
    /// answer, and so is a `skip.txt` that cannot be used: a folder the person switched off is never offered.
    pub(crate) fn of(read: &DayOptions, typed: Result<Vec<PathBuf>, Error>) -> Result<Offer, Error> {
        let off = reader::switched_off_for(read)?;
        let (typed, unread) = match typed {
            Ok(typed) => (typed, None),
            Err(e @ (Error::NoLogs(_) | Error::Run(_))) => (Vec::new(), Some(e)),
            Err(e) => return Err(e),
        };
        let lines = read.home.as_deref().map_or(Ok(Vec::new()), kept);
        Ok(Offer { typed, unread, lines, off })
    }

    pub(crate) fn led(&self, name: &str) -> Led {
        let mut typed = self.typed.iter().filter(|folder| reader::is_named(folder, name));
        let listed = self.lines.as_ref().ok().and_then(|lines| lines.iter().find(|line| line.name == name));
        match (typed.next(), typed.count(), listed) {
            (Some(_), more @ 1.., _) => Led::Many(more + 1),
            (Some(folder), _, None) => Led::Typed(folder.clone()),
            (Some(folder), _, Some(line)) if real(folder).is_some_and(|real| real == Path::new(&line.folder)) => {
                Led::Typed(folder.clone())
            }
            (Some(_), _, Some(_)) => Led::Clash,
            (None, _, Some(line)) => match stands(line, &self.off) {
                Ok(folder) => Led::Listed(folder),
                Err(why) => Led::Fallen(why),
            },
            (None, _, None) => Led::Nowhere,
        }
    }

    /// Whether a wish can be made for the name: it leads to a folder, or to several typed in, which was on
    /// offer before there was a list and which a run still refuses.
    pub(crate) fn offers(&self, name: &str) -> bool {
        matches!(self.led(name), Led::Typed(_) | Led::Listed(_) | Led::Many(_))
    }

    /// Every name on offer, each once: the day's typed-in folders in their order, then the list's in its own.
    pub(crate) fn names(&self) -> Vec<String> {
        let typed = self.typed.iter().filter_map(|folder| folder.file_name()?.to_str());
        let listed = self.lines.iter().flatten().map(|line| line.name.as_str());
        let mut names: Vec<String> = Vec::new();
        for name in typed.chain(listed) {
            if crate::wish::named(name) && !names.iter().any(|known| known == name) && self.offers(name) {
                names.push(name.to_string());
            }
        }
        names
    }
}

fn real(folder: &Path) -> Option<PathBuf> {
    fs::canonicalize(folder).ok().map(plain)
}

/// The listed folder, when a run can start in it now. The path is the list's word: it is used only when it is
/// still, link for link, the real path of a folder, so a link put where a listed folder was leads nowhere.
fn stands(line: &Line, off: &[String]) -> Result<PathBuf, &'static str> {
    let folder = Path::new(&line.folder);
    if !real(folder).is_some_and(|real| real == folder && real.is_dir()) {
        return Err("it is not there, or is reached through a link now");
    }
    if !folder.join(".git").exists() {
        return Err("it is not the top of a git repo");
    }
    if theirs(folder) {
        return Err("it is not a folder of your own");
    }
    if reader::is_off(folder, off) {
        return Err("it is switched off");
    }
    Ok(folder.to_path_buf())
}
