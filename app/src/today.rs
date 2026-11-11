//! A day's page for the app that was opened with no page file. The page is kept per day in the user's own folder,
//! shown from there at once, and read again through core when the day has turns the kept page did not see. No GPUI:
//! the window only shows the scenes this hands it, each when the work before it is done ([`Next`]).
//!
//! Only core's `day` and `page` are called, with the folders and the harness the `jellymoon` command would use, so
//! core's rules hold unchanged. What is kept is the checked page data, as `jellymoon page` prints it, and nothing
//! else. Nothing is printed here: the scenes go to the window, or to `layout`, which prints the page's own words on
//! stdout as it does for any page.

use std::path::PathBuf;
use std::time::Duration;
use std::{env, io};

use jellymoon_core::{DayOptions, Error, PageData, PageOptions, ReaderDay};
use sticker_kit::{Note, Scene};

use crate::read::{self, LeftOut, Page};
use crate::scene::bare;

/// How long the reading call may take: the `jellymoon` command's own limit.
const READING_LIMIT: Duration = Duration::from_secs(180);

/// A scene to show, and the work that gives the one after it.
pub struct Next {
    pub scene: Scene,
    /// The page data the scene shows, when it shows a page: what a tap on it acts on.
    pub page: Option<Page>,
    pub later: Option<Later>,
}

/// Work that gives what to show next, or none when what is shown stays. Counting a day's turns can take seconds and
/// a reading call minutes, so the window runs it off its own thread.
pub type Later = Box<dyn FnOnce() -> Option<Next> + Send>;

/// One day to show, and where everything it needs is.
#[derive(Debug, Clone)]
pub struct Today {
    /// The local calendar day, as YYYY-MM-DD.
    pub date: String,
    /// The Claude Code projects directory.
    pub logs: PathBuf,
    /// Names of the folders the person switched off, besides those in `skip.txt`, which core reads.
    pub skip: Vec<String>,
    pub model: String,
    /// Where jellymoon keeps its own files.
    pub home: PathBuf,
    /// The harness program. None is `claude`, found on `PATH` or in a well-known place.
    pub harness: Option<PathBuf>,
    /// Read again even when the kept page saw every turn.
    pub fresh: bool,
    /// Claude Code's settings file, where "Turn on delivery" adds the hook. None when no folder of its is known.
    pub settings: Option<PathBuf>,
    /// The folder `install.sh` puts the `jellymoon` command in, which the hook runs. None when there is no home.
    pub bin: Option<PathBuf>,
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
    /// user's home is `HOME` (`USERPROFILE` on Windows). A home that is not an absolute path is no home: where it
    /// led would depend on the folder the app was started in.
    pub fn from_env(asked: Asked) -> Result<Today, String> {
        let named = |variable: &str| env::var_os(variable).filter(|path| !path.is_empty()).map(PathBuf::from);
        // The homes are core's: the variable as a full path and nothing else, so a run with an empty environment (a
        // test's) is refused here and can never reach a real person's logs.
        let user = jellymoon_core::user_home;
        let home = jellymoon_core::own_folder().map_err(|e| e.to_string())?;
        if asked.logs.as_ref().is_some_and(|logs| logs.to_string_lossy().trim().is_empty()) {
            return Err("--logs needs a folder".to_string());
        }
        let config = || named("CLAUDE_CONFIG_DIR").or_else(|| Some(user()?.join(".claude")));
        let logs = asked.logs.or_else(|| Some(config()?.join("projects")));
        // The folder `install.sh` puts the command in, under the name it gives it.
        let bin = named("JELLYMOON_BIN_DIR").or_else(|| Some(user()?.join(".local").join("bin")));
        Ok(Today {
            date: date(asked.date.as_deref()).ok_or("--date must be a day as YYYY-MM-DD")?,
            logs: logs.ok_or("finding the session logs: no home directory; name them with --logs")?,
            skip: jellymoon_core::skip_names(&asked.skip)?,
            model: asked.model.unwrap_or_else(|| "sonnet".to_string()),
            home: home.ok_or("no home directory: set JELLYMOON_HOME")?,
            harness: named("JELLYMOON_CLAUDE"),
            fresh: asked.fresh,
            settings: config().map(|config| config.join("settings.json")),
            bin,
        })
    }

    /// Where the day's page is kept.
    fn file(&self) -> PathBuf {
        self.home.join("pages").join(format!("{}.json", self.date))
    }

    /// The kept page of the day, when there is one that reads as page data of that day. Core says whether the file
    /// may be used (a plain file only this user can write to, of a page's size); then it comes through the same door
    /// as any page file.
    pub fn kept(&self) -> Option<Page> {
        let bytes = jellymoon_core::kept_page(&self.home, &self.date).ok()?;
        read::bytes(bytes).ok().filter(|page| page.data.day == self.date)
    }

    /// What the window shows at once, before a log is opened: the kept page, or the waiting scene. The kept page goes
    /// on to [`Today::later`].
    pub fn first(&self) -> (Scene, Option<Page>) {
        match self.kept() {
            Some(page) => (self.draw(&page), Some(page)),
            None => (waiting(&self.date), None),
        }
    }

    /// What the window shows in the end, for `layout`: the kept page when it is current, else what reading gave.
    pub fn last(&self) -> Scene {
        let (mut shown, kept) = self.first();
        let mut later = Some(self.clone().later(kept));
        while let Some(next) = later.take().and_then(|work| work()) {
            (shown, later) = (next.scene, next.later);
        }
        shown
    }

    /// The work after the first scene, in two steps. The first counts the day's turns: none when the kept page saw
    /// every one and nobody asked for a new reading, so what is shown stays. Otherwise it gives the kept page with a
    /// note that it is being read again (or the waiting scene), and the second step: the reading call, which gives
    /// the new page, now kept, or why there is none.
    pub fn later(self, kept: Option<Page>) -> Later {
        Box::new(move || self.count(kept))
    }

    fn count(self, kept: Option<Page>) -> Option<Next> {
        let (skip, home) = (self.skip.clone(), Some(self.home.clone()));
        let day = DayOptions { logs: self.logs.clone(), date: Some(self.date.clone()), skip, home };
        let day = match jellymoon_core::day(&day) {
            Ok(day) => day,
            Err(error) => return Some(self.stopped(kept, &words(&error))),
        };
        if !self.fresh && kept.as_ref().is_some_and(|kept| kept.data.read.turns == day.turns.len()) {
            return None;
        }
        let scene = match &kept {
            Some(page) => {
                noted(self.draw(page), "Reading your day again…", "This page will change when the reading is done.")
            }
            None => waiting(&self.date),
        };
        let page = kept.clone();
        let call: Later = Box::new(move || Some(self.call(&day, kept)));
        Some(Next { scene, page, later: Some(call) })
    }

    /// What is shown when the day could not be read: the kept page stays, and can still be tapped.
    fn stopped(&self, kept: Option<Page>, why: &str) -> Next {
        Next { scene: self.failed(kept.as_ref(), why), page: kept, later: None }
    }

    fn call(&self, day: &ReaderDay, kept: Option<Page>) -> Next {
        let options = PageOptions {
            model: self.model.clone(),
            dir: self.home.join("reading"),
            limit: READING_LIMIT,
            harness: self.harness.clone(),
        };
        let data = match jellymoon_core::page(day, &options) {
            Ok(data) => data,
            Err(error) => return self.stopped(kept, &words(&error)),
        };
        let unkept = self.keep(&data).err();
        let page = Page { data, unknown: Vec::new(), left_out: LeftOut::default() };
        let mut shown = self.draw(&page);
        if let Some(error) = unkept {
            let body = format!("It will be read again the next time Jellymoon opens ({error}).");
            shown.notes.insert(0, Note::Plain { title: "This page could not be kept.".to_string(), body });
        }
        Next { scene: shown, page: Some(page), later: None }
    }

    /// Keeps the checked page data, one line of JSON as `jellymoon page` prints it, in a folder for this user only.
    /// It is written beside its place, in a file of this process's own that only this user can read, and moved
    /// there, so a kept page is never half a page and two apps open at once never write the same file.
    fn keep(&self, data: &PageData) -> io::Result<()> {
        jellymoon_core::keep_private(&self.file(), &format!("{data}\n"))
    }

    /// The page when the day could not be read: one plain note that says why, on the kept page when there is one.
    fn failed(&self, kept: Option<&Page>, why: &str) -> Scene {
        match kept {
            Some(page) => noted(self.draw(page), "This page is from an earlier reading.", why),
            None => bare(
                &self.date,
                vec![Note::Plain { title: "Today could not be read.".to_string(), body: why.to_string() }],
            ),
        }
    }
}

/// The scene with one plain note before its own.
fn noted(mut shown: Scene, title: &str, body: &str) -> Scene {
    shown.notes.insert(0, Note::Plain { title: title.to_string(), body: body.to_string() });
    shown
}

/// Why there is no reading, for the person the page is for. Core's own line follows only where it tells them more.
fn words(error: &Error) -> String {
    let again = "Open Jellymoon again to try once more.";
    match error {
        Error::NoHarness(looked) => format!(
            "Claude Code was not found on this computer. Jellymoon reads your day with your own Claude Code: install \
             it, or open Jellymoon from a terminal where `claude` runs. It was looked for in {looked}."
        ),
        Error::NoLogs(looked) => format!(
            "No Claude Code session logs were found on this computer, so there is nothing to read yet. Use Claude \
             Code once and open Jellymoon again. They were looked for in {looked}."
        ),
        Error::Logs(_) => format!("Your Claude Code session logs could not be read ({error})."),
        Error::Skip(why) => format!(
            "The list of folders you switched off (skip.txt in Jellymoon's folder) could not be read, so nothing was \
             read: {why}."
        ),
        Error::TooLong(_) => format!("The reading took too long and was stopped. {again}"),
        Error::NotPageData { .. } => {
            format!("The model's answer was not a page, so nothing from it is shown. {again}")
        }
        Error::Call(_) | Error::Failed { .. } | Error::NotAResult | Error::Harness(_) => format!(
            "Claude Code could not do the reading. Check that `claude` answers in a terminal. {again} What happened: \
             {error}."
        ),
        Error::Date | Error::Model | Error::Home | Error::NotAbsolute | Error::Uninstall(_) => format!("{error}."),
        // A tap's errors are put into words in `taps.rs`; these say what core says.
        Error::Ruling
        | Error::Store(_)
        | Error::Settings(_)
        | Error::NotSettings
        | Error::Elsewhere
        | Error::Changed
        | Error::Tap(_)
        | Error::Command(_)
        | Error::Copy(_) => {
            format!("{error}.")
        }
    }
}
