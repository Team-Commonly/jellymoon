//! A wish typed in the window (F42). The rules are core's: `jellymoon_core::wish` says which words are a wish and
//! keeps them, `installed_command` says whether the installed `jellymoon` may be started, and `run_apart` starts it
//! as `jellymoon run`, a process of its own. Nothing of a run is in this process: the app asks, and then only looks.
//!
//! What is carried to core is the text of the entry and the name of the project whose chip was chosen: a name of
//! the page's own `read.projects`, never a path. The folder is found by `jellymoon run`, as it is for
//! `jellymoon wish --run` (`docs/development/run.md`). No GPUI: the window holds an [`OnShow`] and calls
//! [`OnShow::wished`], runs the [`Asking`] it gives off its own thread, and looks once a second ([`OnShow::look`]).

use jellymoon_core::{MADE_SAID_MAX, RunOptions, Started};
use sticker_kit::{Entry, Note, Scene};

use crate::read::Page;
use crate::taps::{OnShow, UNTOLD};
use crate::today::Today;

/// The id of the entry a wish is typed in.
pub const ENTRY: &str = "wish";

const HINT: &str = "Wish for something: type it, choose its project with Tab, press Enter";

/// The title of the note for words that were not kept as a wish. Core's reason is its body.
pub const UNKEPT: &str = "That wish was not kept.";

/// The title of the note for a wish that is kept and has no run. Core's reason is its body.
pub const UNSTARTED: &str = "Your wish is kept, and no run was started for it.";

/// A note's title and body.
pub type Said = (&'static str, String);

/// The asking for a made wish's run. It can take as long as a copy of the repo takes to make, so the window runs
/// it off its own thread. None: the run was started, and a look shows how it goes. Else why there is none.
pub type Asking = Box<dyn FnOnce() -> Option<Said> + Send>;

/// One look at what is kept: the scene it was made from, and the scene to show in its place.
pub struct Seen {
    from: Scene,
    now: Scene,
}

/// A look, which reads files and may end a run that was cut off: the window runs it off its own thread. None when
/// what is shown is what is kept.
pub type Look = Box<dyn FnOnce() -> Option<Seen> + Send>;

/// The entry of a day's page: one option for each project of the page's reading, which the kit holds to its first
/// six, and core's own cap on a wish's length. None for a day with no page, or a page that read no project: a
/// wish needs a project, and the app names none of its own.
pub fn entry(page: Option<&Page>) -> Option<Entry> {
    let projects = &page?.data.read.projects;
    let entry = || Entry { id: ENTRY.into(), hint: HINT.into(), options: projects.clone(), most: MADE_SAID_MAX };
    (!projects.is_empty()).then(entry)
}

impl OnShow {
    /// The entry of the page on show.
    pub fn entry(&self) -> Option<Entry> {
        entry(self.page.as_ref())
    }

    /// Enter in the entry: `typed` for the project of chip `option` (from 0). The scene to show at once, and the
    /// asking for the wish's run when core kept the wish. Words core refuses write nothing and ask for nothing:
    /// the scene's first note says why, in core's words. No reading is made: a made wish is drawn from what core
    /// keeps of it, in the working state.
    pub fn wished(&mut self, typed: &str, option: Option<usize>) -> (Scene, Option<Asking>) {
        let offered = self.page.as_ref().map(|page| page.data.read.projects.as_slice()).unwrap_or_default();
        let project = option.and_then(|option| offered.get(option));
        let made = match project {
            Some(project) => jellymoon_core::wish(&self.today.home, Some(&self.today.date), typed, project)
                .map_err(|refused| format!("{refused}.")),
            None => Err("No project of this page was chosen, so nothing was sent.".to_string()),
        };
        match made {
            Ok(made) => (self.say(None), Some(self.today.asking(made.wish.id))),
            Err(why) => (self.say(Some((UNKEPT, why))), None),
        }
    }

    /// What an [`Asking`] gave: the scene to show when there is something to say.
    pub fn asked(&mut self, said: Option<Said>) -> Option<Scene> {
        said.map(|said| self.say(Some(said)))
    }

    /// The scene drawn again from what is kept now, with the note of the last wish, when it left one, in place of
    /// the note of the wish before.
    fn say(&mut self, said: Option<Said>) -> Scene {
        let of_a_wish =
            |note: &Note| matches!(note, Note::Plain { title, .. } if [UNKEPT, UNSTARTED].contains(&&**title));
        self.scene.notes.retain(|note| !of_a_wish(note));
        self.scene = self.today.again(self.page.as_ref(), &self.scene);
        if let Some((title, body)) = said {
            self.scene.notes.insert(0, Note::Plain { title: title.to_string(), body });
        }
        self.scene.clone()
    }

    /// The work of one look at the day: what core keeps now, drawn under the notes on show.
    pub fn look(&self) -> Look {
        let (today, page, from) = (self.today.clone(), self.page.clone(), self.scene.clone());
        Box::new(move || {
            let now = today.again(page.as_ref(), &from);
            (now != from).then_some(Seen { from, now })
        })
    }

    /// What a look gave: the scene to show. None when the scene on show changed while the look was made (a tap, a
    /// wish, a reading): that look is dropped, and the next is made from the new scene.
    pub fn seen(&mut self, seen: Seen) -> Option<Scene> {
        (self.scene == seen.from).then(|| {
            self.scene = seen.now;
            self.scene.clone()
        })
    }
}

impl Today {
    /// `drawn` drawn again from what is kept now: the page, or with no page the day's made wishes, under the notes
    /// the app had put over it. Those are the plain notes `drawn` begins with that the page does not draw itself:
    /// a reading that is going or failed, a tap that did nothing, a wish that was not kept or has no run. So a look
    /// never takes a note away: it goes with the next wish, tap or reading.
    pub(crate) fn again(&self, page: Option<&Page>, drawn: &Scene) -> Scene {
        let mut shown = page.map_or_else(|| self.made_alone(), |page| self.draw(page));
        // Two notes of a page's own come and go with what is kept: neither is carried over as the app's.
        let own = |title: &str| [crate::scene::NOTHING, UNTOLD].contains(&title);
        let over =
            |note: &&Note| matches!(note, Note::Plain { title, .. } if !own(title)) && !shown.notes.contains(note);
        let over: Vec<Note> = drawn.notes.iter().take_while(over).cloned().collect();
        shown.notes.splice(0..0, over);
        shown
    }

    /// The asking for the run of the made wish `wish`: core's check of the installed command, then core's
    /// `run_apart` with it. The folder is not named: `jellymoon run` finds the one of the wish's project in the
    /// day's logs, with the folders switched off left out.
    fn asking(&self, wish: String) -> Asking {
        let today = self.clone();
        Box::new(move || {
            let options = RunOptions {
                wish,
                day: Some(today.date),
                repo: None,
                logs: Some(today.logs),
                harness: today.harness,
                skip: today.skip,
            };
            // With no home there is no folder to look in, and core says so of the empty one.
            let command = jellymoon_core::installed_command(&today.bin.unwrap_or_default());
            let why = match command.and_then(|command| jellymoon_core::run_apart(&today.home, &command, &options)) {
                Ok(Started { run: Some(_), .. }) => return None,
                Ok(Started { why, .. }) => why,
                Err(refused) => refused.to_string(),
            };
            Some((UNSTARTED, format!("{why}.")))
        })
    }
}
