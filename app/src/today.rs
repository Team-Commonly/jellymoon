//! A day's page for the app that was opened with no page file. The page is kept per day in the user's own folder,
//! shown from there at once, and read again through core when the day has turns the kept page did not see. No GPUI:
//! the window only shows the scenes this hands it.
//!
//! Only core's `day` and `page` are called, with the folders and the harness the `jellymoon` command would use, so
//! core's rules hold unchanged. What is kept is the checked page data, as `jellymoon page` prints it, and nothing
//! else. Nothing of a day is printed.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;
use std::{env, fs, io};

use jellymoon_core::{DayOptions, Error, PageData, PageOptions};
use sticker_kit::{Note, Scene};

use crate::read::{self, LeftOut, Page};
use crate::scene::{bare, scene};

/// How long the reading call may take: the `jellymoon` command's own limit.
const READING_LIMIT: Duration = Duration::from_secs(180);

/// One day to show, and where everything it needs is.
#[derive(Debug, Clone)]
pub struct Today {
    /// The local calendar day, as YYYY-MM-DD.
    pub date: String,
    /// The Claude Code projects directory.
    pub logs: PathBuf,
    /// Names of the folders the person switched off, besides those in `skip.txt`.
    pub skip: Vec<String>,
    pub model: String,
    /// Where jellymoon keeps its own files.
    pub home: PathBuf,
    /// The harness program. None is `claude`, found on `PATH`.
    pub harness: Option<PathBuf>,
    /// Read again even when the kept page saw every turn.
    pub fresh: bool,
}

/// What was asked for on the command line; what is left out comes from the environment, as for `jellymoon`.
#[derive(Debug, Default)]
pub struct Asked {
    pub date: Option<String>,
    pub logs: Option<PathBuf>,
    pub skip: String,
    pub model: Option<String>,
    pub fresh: bool,
}

/// The day written exactly YYYY-MM-DD, or today in the computer's time zone, which `TZ` overrides. It names a file,
/// so nothing but a real day passes.
pub fn date(asked: Option<&str>) -> Option<String> {
    let Some(asked) = asked else {
        return Some(jiff::Zoned::now().date().to_string());
    };
    let day = asked.parse::<jiff::civil::Date>().ok()?.to_string();
    (day == asked).then_some(day)
}

/// The scene shown while the day is being read for the first time.
pub fn waiting(date: &str) -> Scene {
    let body = "This takes a few seconds: your own Claude Code is reading what you typed today.";
    bare(date, vec![Note::Plain { title: "Reading your day…".to_string(), body: body.to_string() }])
}

impl Today {
    /// The folders `jellymoon` uses: the logs under `CLAUDE_CONFIG_DIR` or `.claude` in the user's home, jellymoon's
    /// own files under `JELLYMOON_HOME` or `.jellymoon` there, the harness `JELLYMOON_CLAUDE` names or `claude`. The
    /// user's home is `HOME` (`USERPROFILE` on Windows).
    pub fn from_env(asked: Asked) -> Result<Today, String> {
        let named = |variable: &str| env::var_os(variable).filter(|path| !path.is_empty()).map(PathBuf::from);
        // The home is the variable and nothing else. The system's own record of it is not asked, so a run with an
        // empty environment (a test's) is refused here and can never reach a real person's logs.
        let user = || named(if cfg!(windows) { "USERPROFILE" } else { "HOME" });
        let config = || named("CLAUDE_CONFIG_DIR").or_else(|| Some(user()?.join(".claude")));
        let logs = asked.logs.or_else(|| Some(config()?.join("projects")));
        let home = named("JELLYMOON_HOME").or_else(|| Some(user()?.join(".jellymoon")));
        Ok(Today {
            date: date(asked.date.as_deref()).ok_or("--date must be a day as YYYY-MM-DD")?,
            logs: logs.ok_or("finding the session logs: no home directory; name them with --logs")?,
            skip: asked.skip.split(',').map(str::to_string).collect(),
            model: asked.model.unwrap_or_else(|| "sonnet".to_string()),
            home: home.ok_or("no home directory: set JELLYMOON_HOME")?,
            harness: named("JELLYMOON_CLAUDE"),
            fresh: asked.fresh,
        })
    }

    /// Where the day's page is kept.
    fn file(&self) -> PathBuf {
        self.home.join("pages").join(format!("{}.json", self.date))
    }

    /// The kept page of the day, when there is one that reads as page data of that day. It comes through the same
    /// door as any page file.
    pub fn kept(&self) -> Option<Page> {
        let mut bytes = Vec::new();
        fs::File::open(self.file()).ok()?.take(read::MAX_PAGE_BYTES as u64 + 1).read_to_end(&mut bytes).ok()?;
        read::bytes(bytes).ok().filter(|page| page.data.day == self.date)
    }

    /// What the window shows at once: the kept page, or the waiting scene. The kept page goes on to [`Today::read`].
    pub fn first(&self) -> (Scene, Option<Page>) {
        match self.kept() {
            Some(page) => (scene(&page), Some(page)),
            None => (waiting(&self.date), None),
        }
    }

    /// What the window shows in the end, for `layout`: the kept page when it is current, else what reading gave.
    pub fn last(&self) -> Scene {
        let (first, kept) = self.first();
        self.read(kept.as_ref()).unwrap_or(first)
    }

    /// Reads the day, which can take as long as a reading call. None when the kept page saw every turn the day has
    /// and nobody asked for a new reading: what is shown stays. Otherwise the new page, which is kept, or why there
    /// is none.
    pub fn read(&self, kept: Option<&Page>) -> Option<Scene> {
        let mut skip = self.skip.clone();
        skip.extend(fs::read_to_string(self.home.join("skip.txt")).unwrap_or_default().lines().map(str::to_string));
        let day = DayOptions { logs: self.logs.clone(), date: Some(self.date.clone()), skip };
        let day = match jellymoon_core::day(&day) {
            Ok(day) => day,
            Err(error) => return Some(self.failed(kept, &words(&error))),
        };
        if !self.fresh && kept.is_some_and(|kept| kept.data.read.turns == day.turns.len()) {
            return None;
        }
        let options = PageOptions {
            model: self.model.clone(),
            dir: self.home.join("reading"),
            limit: READING_LIMIT,
            harness: self.harness.clone(),
        };
        let data = match jellymoon_core::page(&day, &options) {
            Ok(data) => data,
            Err(error) => return Some(self.failed(kept, &words(&error))),
        };
        let unkept = self.keep(&data).err();
        let mut shown = scene(&Page { data, unknown: Vec::new(), left_out: LeftOut::default() });
        if let Some(error) = unkept {
            let body = format!("It will be read again the next time Jellymoon opens ({error}).");
            shown.notes.insert(0, Note::Plain { title: "This page could not be kept.".to_string(), body });
        }
        Some(shown)
    }

    /// Keeps the checked page data, one line of JSON as `jellymoon page` prints it, in a folder for this user only.
    /// It is written beside its place and moved there, so a kept page is never half a page.
    fn keep(&self, data: &PageData) -> io::Result<()> {
        let file = self.file();
        private_dir(&self.home.join("pages"))?;
        let beside = file.with_extension("json.new");
        fs::write(&beside, format!("{data}\n"))?;
        fs::rename(&beside, &file)
    }

    /// The page when the day could not be read: one plain note that says why, on the kept page when there is one.
    fn failed(&self, kept: Option<&Page>, why: &str) -> Scene {
        let note = |title: &str| Note::Plain { title: title.to_string(), body: why.to_string() };
        match kept {
            Some(page) => {
                let mut shown = scene(page);
                shown.notes.insert(0, note("This page is from an earlier reading."));
                shown
            }
            None => bare(&self.date, vec![note("Today could not be read.")]),
        }
    }
}

/// Why there is no reading, for the person the page is for. Core's own line follows only where it tells them more.
fn words(error: &Error) -> String {
    let again = "Open Jellymoon again to try once more.";
    match error {
        Error::NoHarness => {
            "Claude Code was not found on this computer. Jellymoon reads your day with your own Claude \
                             Code: install it, or open Jellymoon from a terminal where `claude` runs."
                .to_string()
        }
        Error::Logs(_) => {
            format!("No Claude Code sessions were found on this computer, so there is nothing to read yet ({error}).")
        }
        Error::TooLong(_) => format!("The reading took too long and was stopped. {again}"),
        Error::NotPageData { .. } => {
            format!("The model's answer was not a page, so nothing from it is shown. {again}")
        }
        Error::Call(_) | Error::Failed { .. } | Error::NotAResult | Error::Harness(_) => format!(
            "Claude Code could not do the reading. Check that `claude` answers in a terminal. {again} What happened: \
             {error}."
        ),
        Error::Date | Error::Model | Error::Uninstall(_) => format!("{error}."),
    }
}

/// Makes a folder for this user only where the system has file modes, as core does for its own.
fn private_dir(dir: &Path) -> io::Result<()> {
    let mut builder = fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    std::os::unix::fs::DirBuilderExt::mode(&mut builder, 0o700);
    builder.create(dir)?;
    #[cfg(unix)]
    fs::set_permissions(dir, std::os::unix::fs::PermissionsExt::from_mode(0o700))?;
    Ok(())
}
