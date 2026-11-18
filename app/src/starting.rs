//! A project given to Jellymoon in the window (F64, F66; ADR 0020, decisions 3 and 4). A folder dropped on the
//! window, or chosen with the system's picker, goes to core's `add_project`; a name typed in the entry a day with
//! no project on offer has, to core's `new_project`. Core answers with the project's name or a refusal, and that
//! is one note. The folder's path is carried to core and never drawn. Every call to core is in work the window
//! runs off its own thread ([`Listing`]); what the window's thread does with its answer ([`OnShow::listed`])
//! opens no file. No GPUI.

use std::path::PathBuf;

use sticker_kit::{Entry, Note, Scene};

use crate::scene;
use crate::taps::OnShow;
use crate::today::Next;
use crate::wishing::{Said, Seen, newest};

/// The id of the entry a new project is named in.
pub const NEW: &str = "new";

/// The one choice of the entry a day with no project on offer has. It is ADR 0020's, and like any chip a tap
/// on it gives the field the keys: the cursor is in the field a new project is named in.
pub const START: &str = "Start something new";

/// The choice beside every entry's field that does what Enter does.
pub const GO: &str = "Start";

/// What the entry offers besides its chips: the system's folder picker.
pub const CHOOSE: &str = "Choose a folder";

/// The hint of the entry a new project is named in: the folder first, as the page's note says it, then a name.
pub const NEW_HINT: &str = "Choose a folder below, or type a name for a new project";

/// The note of `Start` tapped while the entry holds no words: nothing was started, and what to do first.
pub const UNREADY: &str = "Nothing to start yet.";
const UNREADY_NEW: &str = "Choose the folder you want to work in, or type a name for a new project first.";
const UNREADY_WISH: &str = "Type what you want done in the field first.";

/// The most characters of a project's name: one folder's name, as core takes it.
const NAME_MAX: usize = 100;

/// The titles of the notes core's answer is put in.
pub const ADDED: &str = "A project was added.";
pub const MADE: &str = "Your new project is ready.";
pub const UNADDED: &str = "That folder was not added.";
pub const UNMADE: &str = "No project was made.";

/// What the person gave: a folder, dropped or chosen, or the name of a project to make.
pub enum Given {
    Folder(PathBuf),
    Name(String),
}

/// What core answered, and what the page is then: work done off the window's thread.
pub struct Listed {
    seen: Seen,
    said: Said,
    /// A day that had no page has the one core gives it now that a project is on offer.
    next: Option<Next>,
    entry: Option<Entry>,
    /// The project's name, when core listed it.
    name: Option<String>,
}

/// A project given, carried to core: the window runs it off its own thread.
pub type Listing = Box<dyn FnOnce() -> Listed + Send>;

/// What the window shows once core has answered.
pub struct Shown {
    pub scene: Scene,
    pub entry: Option<Entry>,
    /// The project's name, when core listed it: its chip is the chosen one, and the entry has the keys.
    pub name: Option<String>,
}

/// The entry of a day with no project on offer: one choice, a field for a name, and the folder picker, which is
/// the loud choice: a folder the person works in is the first way in.
pub fn new_entry() -> Entry {
    let (options, action, go) = (vec![START.to_string()], CHOOSE.into(), GO.into());
    Entry { id: NEW.into(), hint: NEW_HINT.into(), options, most: NAME_MAX, action, go, loud: true }
}

/// What Enter in the entry `id` gives core as a project: a name, in the entry of a day with no project on offer.
/// None in the entry of a wish: its words are a wish's.
pub fn named(id: &str, typed: &str) -> Option<Given> {
    (id == NEW).then(|| Given::Name(typed.to_string()))
}

/// The empty page of a day with no project on offer: its one note says to choose a folder first, or to name a new
/// project, in the order the entry's hint does (`starting::NEW_HINT`). The window and the picture share this.
pub fn no_project(shown: &mut Scene) {
    for note in &mut shown.notes {
        if let Note::Plain { title, body } = note
            && title == scene::NOTHING
        {
            *body = scene::NO_PROJECT.to_string();
        }
    }
}

/// Core's refusal in the app's words: what core says of `--repo` is said of the folder, and nothing else changes.
fn refusal(why: &str) -> String {
    let why = why.strip_prefix("--repo ").map_or(why.to_string(), |rest| format!("that folder {rest}"));
    let why = why.strip_prefix("project new ").map_or(why.clone(), |rest| format!("a new project {rest}"));
    let mut letters = why.chars();
    letters.next().map_or(String::new(), |first| format!("{}{}.", first.to_uppercase(), letters.as_str()))
}

/// The note of core's answer: the project's name, or core's refusal.
fn said(given: &Given, answer: Result<(String, Option<&str>), String>) -> (Said, Option<String>) {
    let folder = matches!(given, Given::Folder(_));
    match answer {
        Ok((name, also)) => {
            let title = if folder { ADDED } else { MADE };
            let also = also.map_or(String::new(), |also| format!(" {}.", refusal(also)));
            ((title, format!("“{name}” is ready: type what you want done.{also}")), Some(name))
        }
        Err(why) => ((if folder { UNADDED } else { UNMADE }, refusal(&why)), None),
    }
}

impl OnShow {
    /// `Start` tapped while the entry `id` held no words: the scene on show with one note that says what to do
    /// first, before the others. The notes standing stay (a wish not kept, a run not started, a question
    /// unanswered); only an earlier "Nothing to start yet." goes, so a second tap adds no second.
    pub fn unready(&mut self, id: &str) -> Scene {
        let body = if id == NEW { UNREADY_NEW } else { UNREADY_WISH };
        let notes = &mut self.scene.notes;
        notes.retain(|note| !matches!(note, Note::Plain { title, .. } if title == UNREADY));
        notes.insert(0, Note::Plain { title: UNREADY.to_string(), body: body.to_string() });
        self.scene.clone()
    }

    /// A folder or a name given, as work for another thread: core lists the folder or makes the project, and the
    /// page and its entry are asked again, so the project has a chip as soon as core has answered.
    pub fn listing(&self, given: Given) -> Listing {
        let (today, page, from) = (self.today.clone(), self.page.clone(), self.scene.clone());
        Box::new(move || {
            let (home, day) = (&today.home, Some(today.date.as_str()));
            let answer = match &given {
                Given::Folder(folder) => jellymoon_core::add_project(home, folder, day).map(|known| (known, None)),
                Given::Name(name) => jellymoon_core::new_project(home, name, day),
            };
            let answer = answer.map(|(known, also)| (known.name, also)).map_err(|refused| refused.to_string());
            let (said, name) = said(&given, answer);
            // A day with no page has one once a project is on offer: the first step of a reading gives it, and
            // makes no call. A step that would call the model is not taken.
            let next = page.is_none().then(|| today.clone().later(None)()).flatten().filter(|next| next.page.is_some());
            let page = next.as_ref().map_or(page, |next| next.page.clone());
            let entry = today.entry(page.as_ref());
            let mut now = match &next {
                Some(next) => today.under(&from, next.scene.clone()),
                None => today.again(page.as_ref(), &from),
            };
            newest(&mut now, Some(said.clone()));
            let next = next.map(|next| Next { later: None, ..next });
            Listed { seen: Seen { from, now }, said, next, entry, name }
        })
    }

    /// What a [`Listing`] gave. When the scene on show changed while core was asked, only the note is put on it.
    pub fn listed(&mut self, listed: Listed) -> Shown {
        if self.scene == listed.seen.from {
            self.scene = listed.seen.now;
            if let Some(next) = listed.next {
                self.page = next.page;
            }
        } else {
            newest(&mut self.scene, Some(listed.said));
        }
        Shown { scene: self.scene.clone(), entry: listed.entry, name: listed.name }
    }
}

#[cfg(test)]
mod tests {
    use jellymoon_core::{PageData, PageRead};
    use sticker_kit::Note;

    use super::*;
    use crate::read::{LeftOut, Page};

    #[test]
    fn the_empty_page_of_a_day_with_no_project_says_the_folder_first() {
        let read = PageRead { turns: 0, projects: vec![], no_logs: false };
        let data = PageData { day: "2026-03-10".to_string(), wishes: vec![], clashes: vec![], read };
        let mut shown = scene::scene(&Page { data, unknown: vec![], left_out: LeftOut::default() });
        assert!(matches!(&shown.notes[..], [Note::Plain { body, .. }] if body == scene::NOTHING_HERE));
        no_project(&mut shown);
        assert!(matches!(&shown.notes[..], [Note::Plain { body, .. }] if body == scene::NO_PROJECT));
        let said = scene::NO_PROJECT.to_lowercase();
        let hint = NEW_HINT.to_lowercase();
        assert!(said.find("folder") < said.find("name") && hint.find("folder") < hint.find("name"));
    }
}
