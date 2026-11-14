//! A tap that stops a made wish's run (F43). The rules are core's: `jellymoon_core::going` says which made wishes
//! have a run going, and `stop_run` stops one and says how it ended, or refuses and writes nothing. Here a tap on
//! a wish's character shows the choice, and a tap on the choice is carried to core. Both read files, and a stop
//! waits for the run to end, so each is work the window runs off its own thread; what the window's thread does
//! with the answer opens no file. No GPUI.
//!
//! No word of a note here is the agent's, a log's or the wish's: the titles are this file's, and a body is core's
//! fixed line.

use jellymoon_core::RunOutcome;
use sticker_kit::{Note, Scene, State};

use crate::taps::OnShow;
use crate::today::Today;
use crate::wishing::{Look, Seen};
use crate::working::{self, Working};

/// The one option of the choice a tap on a working made wish's character shows.
pub const STOP: &str = "Stop it";

/// The title of the note shown from the tap on [`STOP`] until core has answered.
pub const STOPPING: &str = "That run is being stopped.";

/// The title of the note for a run a stop ended. Core's line is its body.
pub const STOPPED: &str = "That run was stopped.";

/// The title of the note for a run that had ended another way when it was to be stopped. Core's line is its body.
pub const ENDED: &str = "That run had ended, so nothing was stopped.";

/// The title of the note for a stop core refused. Core's line is its body.
pub const UNSTOPPED: &str = "That stop ended no run.";

/// The start of the id of the choice to stop a wish's run: `stop-m1` for the made wish `m1`.
const STOPS: &str = "stop-";

/// What core answered to a stop: the scene the stop was asked from, the scene to show, and the note to say.
pub struct Stopped {
    seen: Seen,
    said: (&'static str, String),
}

/// A stop carried to core, which waits for the run to end: the window runs it off its own thread.
pub type Stopping = Box<dyn FnOnce() -> Stopped + Send>;

fn choice(wish: &str) -> Note {
    Note::Choice {
        id: format!("{STOPS}{wish}"),
        title: "a run is working on this wish".to_string(),
        options: vec![STOP.to_string()],
        footer: "what it did so far is kept for a look".to_string(),
        struck_first: false,
        whole: false,
    }
}

/// The made wish a choice to stop is for.
fn wish_of(note: &Note) -> Option<&str> {
    match note {
        Note::Choice { id, .. } => id.strip_prefix(STOPS),
        _ => None,
    }
}

/// Whether a note is the choice to stop a run.
pub(crate) fn asks(note: &Note) -> bool {
    wish_of(note).is_some()
}

/// Whether a note is what a stop said.
pub(crate) fn said(note: &Note) -> bool {
    matches!(note, Note::Plain { title, .. } if [STOPPING, STOPPED, ENDED, UNSTOPPED].contains(&&**title))
}

/// Whether a note still holds on `shown`: any note but a choice to stop, which holds while its wish is drawn as
/// working.
pub(crate) fn holds(note: &Note, shown: &Scene) -> bool {
    let working = |wish: &str| shown.characters.iter().any(|one| one.id == wish && one.state == State::Busy);
    wish_of(note).is_none_or(working)
}

/// What a stop said, in place of the first note that says a run is being stopped and of what the stops before
/// it said: a page holds one note of a stop that has ended, the newest.
fn say(scene: &mut Scene, (title, body): (&'static str, String)) {
    let stopping = |note: &Note| matches!(note, Note::Plain { title, .. } if title == STOPPING);
    if let Some(at) = scene.notes.iter().position(stopping) {
        scene.notes.remove(at);
    }
    scene.notes.retain(|note| stopping(note) || !said(note));
    scene.notes.insert(0, Note::Plain { title: title.to_string(), body });
}

/// What a tap on an option of a note did.
pub enum Chosen {
    /// It was the choice to stop a run: the scene to show at once, and the stop, as work for another thread.
    Stop(Scene, Stopping),
    /// It was a choice of work that waits (`working.rs`): the scene to show at once, and core's part of it, as
    /// work for another thread.
    Work(Scene, Working),
    /// It was a tap for `taps.rs`: the scene to show.
    Tapped(Scene),
    /// Nothing, and nothing to show: a tap on a choice to stop or a choice of work that is no longer on show (a
    /// second tap on it), or on a page with no day.
    Nothing,
}

impl Today {
    /// `shown` with the choice to stop for every made wish whose run is going, first. `layout` prints these, as
    /// it has no tap: the window shows one after a tap on its wish's character.
    pub fn with_stops(&self, mut shown: Scene) -> Scene {
        let going = jellymoon_core::going(&self.home, &self.date);
        shown.notes.splice(0..0, going.iter().map(|wish| choice(wish)));
        shown
    }
}

impl OnShow {
    /// A tap on option `index` (from 0) of the note `note`: the choice to stop when it is one on show, and
    /// nothing when it is a choice to stop that is not. Any other note is `taps.rs`'s.
    pub fn chose(&mut self, note: &str, index: usize) -> Chosen {
        match self.stopping(note, index) {
            Some((scene, stop)) => Chosen::Stop(scene, stop),
            None if note.starts_with(STOPS) => Chosen::Nothing,
            None => match self.working(note, index) {
                Some((scene, work)) => Chosen::Work(scene, work),
                None if working::names(note) => Chosen::Nothing,
                None => self.tap(note, index).map_or(Chosen::Nothing, Chosen::Tapped),
            },
        }
    }

    /// A tap on the character `id`, as work for another thread: the choice to stop, over the scene on show, when
    /// core says that made wish has a run going, and else the wish opened (`working.rs`): its work that waits, or
    /// its own words and how it stands. None, and nothing written, for a character that is no wish and when what
    /// the tap would show is on show already. It is applied with [`OnShow::seen`].
    pub fn poked(&self, id: &str) -> Look {
        let (today, page, from, id) = (self.today.clone(), self.page.clone(), self.scene.clone(), id.to_string());
        Box::new(move || {
            if from.notes.iter().any(|note| wish_of(note) == Some(&id)) {
                return None;
            }
            if !jellymoon_core::going(&today.home, &today.date).contains(&id) {
                return today.opened(page.as_ref(), from, &id);
            }
            let mut now = from.clone();
            now.notes.insert(0, choice(&id));
            Some(Seen { from, now })
        })
    }

    /// A tap on option `index` (from 0) of the note `note`, when that is a choice to stop on show: the scene to
    /// show at once, which says the run is being stopped in the choice's place, and the stop itself as work for
    /// another thread. None for any other note. The stop asks core, which says how the run ended: stopped, ended
    /// already (nothing is written then), or refused.
    pub fn stopping(&mut self, note: &str, index: usize) -> Option<(Scene, Stopping)> {
        let at = self.scene.notes.iter().position(|one| matches!(one, Note::Choice { id, .. } if id == note))?;
        let wish = wish_of(&self.scene.notes[at]).filter(|_| index == 0)?.to_string();
        let body = "The page shows how it ended as soon as it has.".to_string();
        self.scene.notes[at] = Note::Plain { title: STOPPING.to_string(), body };
        let (today, page, from) = (self.today.clone(), self.page.clone(), self.scene.clone());
        let work = move || {
            let said = match jellymoon_core::stop_run(&today.home, &wish, Some(&today.date)) {
                Ok(ran) if ran.outcome == RunOutcome::Cancelled => (STOPPED, format!("{}.", ran.why)),
                Ok(ran) => (ENDED, format!("{}.", ran.why)),
                Err(refused) => (UNSTOPPED, format!("{refused}.")),
            };
            let mut now = today.again(page.as_ref(), &from);
            say(&mut now, said.clone());
            Stopped { seen: Seen { from, now }, said }
        };
        Some((self.scene.clone(), Box::new(work)))
    }

    /// What a [`Stopping`] gave: the scene to show. When the scene on show changed while the run was stopped, only
    /// the note is put on it here, and the next look draws the wish.
    pub fn stopped(&mut self, stopped: Stopped) -> Scene {
        if self.scene == stopped.seen.from {
            self.scene = stopped.seen.now;
        } else {
            say(&mut self.scene, stopped.said);
        }
        self.scene.clone()
    }
}
