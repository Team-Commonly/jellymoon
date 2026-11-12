//! A wish typed in the window (F42). The rules are core's: `jellymoon_core::wishable` says which projects a wish
//! can be made for (a folder of that name was typed in that day and is not switched off, asked of the reader as a
//! run asks it), `wish_offered` says which words are a wish and keeps them for such a project, `installed_command`
//! says whether the installed `jellymoon` may be started, and `run_apart` starts it as `jellymoon run`, a process
//! of its own. Nothing of a run is in this process: the app asks, and then only looks.
//!
//! What is carried to core is the text of the entry and the name on the chip that was chosen: a name, never a
//! path and never a place among the chips. The folder is found by `jellymoon run`, as it is for
//! `jellymoon wish --run` (`docs/development/run.md`). No GPUI: the window holds an [`OnShow`]. Every call to core
//! here is in a piece of work the window runs off its own thread ([`Offering`], [`Reading`], [`Wishing`],
//! [`Asking`], [`Look`]); what the window's thread does with an answer ([`OnShow::answered`], [`OnShow::asked`],
//! [`OnShow::seen`]) opens no file.

use jellymoon_core::{DayOptions, MADE_SAID_MAX, RunOptions, Started};
use sticker_kit::entry::MAX_ENTRY_OPTIONS;
use sticker_kit::{Entry, Note, Scene};

use crate::read::Page;
use crate::stopping;
use crate::taps::{OnShow, UNTOLD};
use crate::today::{Later, Next, Today};

/// The id of the entry a wish is typed in.
pub const ENTRY: &str = "wish";

const HINT: &str = "Type a wish: Tab for its project, Enter";

/// The title of the note for words that were not kept as a wish. Its body names the words, then core's reason.
pub const UNKEPT: &str = "That wish was not kept.";

/// The title of the note for a wish that is kept and has no run. Its body names the wish, then core's reason.
pub const UNSTARTED: &str = "Your wish is kept, and no run was started for it.";

/// The most characters of a wish that its note names it by: the start of it, as core cleans and cuts it.
const NAMED_MAX: usize = 40;

/// A note's title and body.
pub type Said = (&'static str, String);

/// The asking for a made wish's run. It can take as long as a copy of the repo takes to make, so the window runs
/// it off its own thread. None: the run was started, and a look shows how it goes. Else why there is none.
pub type Asking = Box<dyn FnOnce() -> Option<Said> + Send>;

/// One look at what is kept: the scene it was made from, and the scene to show in its place.
pub struct Seen {
    pub(crate) from: Scene,
    pub(crate) now: Scene,
}

/// A look, which reads files and may end a run that was cut off: the window runs it off its own thread. None when
/// what is shown is what is kept.
pub type Look = Box<dyn FnOnce() -> Option<Seen> + Send>;

/// The entry of the page on show. Which projects have a chip is asked of the day's session logs, so the window
/// runs this off its own thread and has no entry until it is done.
pub type Offering = Box<dyn FnOnce() -> Option<Entry> + Send>;

/// A step of the day's reading with the entry of the page it gives.
pub type Reading = Box<dyn FnOnce() -> Option<(Next, Option<Entry>)> + Send>;

/// What core answered to an Enter.
pub struct Answer {
    seen: Seen,
    said: Option<Said>,
    asking: Option<Asking>,
}

/// An Enter carried to core: the window runs it off its own thread, so a day that is locked, a slow disk or many
/// logs never hold the window.
pub type Wishing = Box<dyn FnOnce() -> Answer + Send>;

/// The body of a wish's note: the start of the wish as core cleans it, so nothing a page would hide is shown, and
/// why.
fn of(words: &str, why: &str) -> String {
    format!("“{}”: {why}", jellymoon_core::wish_start(words, NAMED_MAX))
}

fn of_a_wish(note: &Note) -> bool {
    matches!(note, Note::Plain { title, .. } if [UNKEPT, UNSTARTED].contains(&&**title))
}

/// The note of the wish just entered, when it left one, in place of the notes of the wishes before.
fn newest(scene: &mut Scene, said: Option<Said>) {
    scene.notes.retain(|note| !of_a_wish(note));
    if let Some((title, body)) = said {
        scene.notes.insert(0, Note::Plain { title: title.to_string(), body });
    }
}

impl OnShow {
    /// The entry of the page on show, as work for another thread.
    pub fn offering(&self) -> Offering {
        let (today, page) = (self.today.clone(), self.page.clone());
        Box::new(move || today.entry(page.as_ref()))
    }

    /// Enter in the entry: `typed` for the project `project`, the name on the chosen chip, as work for another
    /// thread. Words core refuses, and a project core no longer offers, write nothing and ask for nothing: the
    /// answer's first note says why, in core's words. No reading is made: a made wish is drawn from what core
    /// keeps of it, in the working state.
    pub fn wishing(&self, typed: &str, project: Option<&str>) -> Wishing {
        let (today, page, from) = (self.today.clone(), self.page.clone(), self.scene.clone());
        let (typed, project) = (typed.to_string(), project.map(str::to_string));
        Box::new(move || {
            let made = match &project {
                Some(project) => jellymoon_core::wish_offered(&today.read(), &typed, project)
                    .map_err(|refused| format!("{refused}.")),
                None => Err("No project was chosen, so nothing was sent.".to_string()),
            };
            let (said, asking) = match made {
                Ok(made) => (None, Some(today.asking(made.wish.id, made.wish.said))),
                Err(why) => (Some((UNKEPT, of(&typed, &why))), None),
            };
            let mut now = today.again(page.as_ref(), &from);
            newest(&mut now, said.clone());
            Answer { seen: Seen { from, now }, said, asking }
        })
    }

    /// What a [`Wishing`] gave: the scene to show, and the asking for the wish's run when core kept the wish. With
    /// none the words were not kept, and the entry keeps them. When the scene on show changed while core was asked,
    /// only the note is put on it here, and the next look draws the wish.
    pub fn answered(&mut self, answer: Answer) -> (Scene, Option<Asking>) {
        if self.scene == answer.seen.from {
            self.scene = answer.seen.now;
        } else {
            newest(&mut self.scene, answer.said);
        }
        (self.scene.clone(), answer.asking)
    }

    /// [`OnShow::wishing`] and [`OnShow::answered`] in one, on the caller's thread: for a caller with no window.
    pub fn wished(&mut self, typed: &str, project: Option<&str>) -> (Scene, Option<Asking>) {
        let answer = (self.wishing(typed, project))();
        self.answered(answer)
    }

    /// What an [`Asking`] gave: the scene to show when there is something to say. The note goes under the notes
    /// of wishes entered since, and takes none away.
    pub fn asked(&mut self, said: Option<Said>) -> Option<Scene> {
        let (title, body) = said?;
        let under = self.scene.notes.iter().take_while(|note| of_a_wish(note)).count();
        self.scene.notes.insert(under, Note::Plain { title: title.to_string(), body });
        Some(self.scene.clone())
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

/// An entry with a chip for each of `options`. None with no option: a wish needs a project.
pub fn entry_of(options: Vec<String>) -> Option<Entry> {
    let entry = |options| Entry { id: ENTRY.into(), hint: HINT.into(), options, most: MADE_SAID_MAX };
    (!options.is_empty()).then(|| entry(options))
}

impl Today {
    /// The day's session logs as core reads them for this day, with the names switched off.
    fn read(&self) -> DayOptions {
        let (logs, skip) = (self.logs.clone(), self.skip.clone());
        DayOptions { logs, date: Some(self.date.clone()), skip, home: Some(self.home.clone()) }
    }

    /// The entry of a day's page: one option for each of the page's projects core says a wish can be made for
    /// today (`jellymoon_core::wishable`), the first six, which are the ones the kit offers, and core's own cap on
    /// a wish's length. None for a day with no page, or a page with no such project: the app names none of its
    /// own. It reads the day's session logs, so the window never calls it on its own thread.
    pub fn entry(&self, page: Option<&Page>) -> Option<Entry> {
        let read = page.map(|page| page.data.read.projects.as_slice()).unwrap_or_default();
        let mut offered = jellymoon_core::wishable(&self.read(), read);
        offered.truncate(MAX_ENTRY_OPTIONS);
        entry_of(offered)
    }

    /// A step of the reading with the entry of the page it gives, as work for another thread.
    pub fn offered(self, step: Later) -> Reading {
        Box::new(move || {
            let next = step()?;
            let entry = self.entry(next.page.as_ref());
            Some((next, entry))
        })
    }

    /// `drawn` drawn again from what is kept now: the page, or with no page the day's made wishes, under what the
    /// app had put over it. That is the plain notes `drawn` begins with that the page does not draw itself (a
    /// reading that is going or failed, a tap that did nothing, a wish that was not kept or has no run, a stop),
    /// and among them the choice to stop a run, while its wish is still working. So a look never takes a note
    /// away: it goes with the next wish or tap.
    pub(crate) fn again(&self, page: Option<&Page>, drawn: &Scene) -> Scene {
        let mut shown = page.map_or_else(|| self.made_alone(), |page| self.draw(page));
        // Two notes of a page's own come and go with what is kept: neither is carried over as the app's.
        let own = |title: &str| [crate::scene::NOTHING, UNTOLD].contains(&title);
        let over = |note: &&Note| {
            stopping::asks(note)
                || matches!(note, Note::Plain { title, .. } if !own(title)) && !shown.notes.contains(note)
        };
        let over: Vec<Note> =
            drawn.notes.iter().take_while(over).filter(|note| stopping::holds(note, &shown)).cloned().collect();
        shown.notes.splice(0..0, over);
        shown
    }

    /// `next`, the scene a reading gave, under the notes of `drawn` that are about a wish or a stop: a reading
    /// that ends says nothing of those, so it takes none of them away.
    pub(crate) fn under(&self, drawn: &Scene, mut next: Scene) -> Scene {
        let stays = |note: &&Note| of_a_wish(note) || stopping::said(note) || stopping::asks(note);
        let over: Vec<Note> =
            drawn.notes.iter().filter(stays).filter(|note| stopping::holds(note, &next)).cloned().collect();
        next.notes.splice(0..0, over);
        next
    }

    /// The asking for the run of the made wish `wish`, whose kept words are `said`: core's check of the installed
    /// command, then core's `run_apart` with it. The folder is not named: `jellymoon run` finds the one of the
    /// wish's project in the day's logs, with the folders switched off left out.
    pub fn asking(&self, wish: String, said: String) -> Asking {
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
            Some((UNSTARTED, of(&said, &format!("{why}."))))
        })
    }
}
