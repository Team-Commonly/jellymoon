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

/// What a stop said, in place of the first note that says a run is being stopped.
fn say(scene: &mut Scene, (title, body): (&'static str, String)) {
    let stopping = |note: &Note| matches!(note, Note::Plain { title, .. } if title == STOPPING);
    if let Some(at) = scene.notes.iter().position(stopping) {
        scene.notes.remove(at);
    }
    scene.notes.insert(0, Note::Plain { title: title.to_string(), body });
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
    /// A tap on the character `id`, as work for another thread: the choice to stop, over the scene on show, when
    /// core says that made wish has a run going. None, and nothing written, for any other character, for a wish
    /// with no run going, and when the choice is on show already. It is applied with [`OnShow::seen`].
    pub fn poked(&self, id: &str) -> Look {
        let (today, from, id) = (self.today.clone(), self.scene.clone(), id.to_string());
        Box::new(move || {
            let on_show = from.notes.iter().any(|note| wish_of(note) == Some(&id));
            let going = !on_show && jellymoon_core::going(&today.home, &today.date).contains(&id);
            going.then(|| {
                let mut now = from.clone();
                now.notes.insert(0, choice(&id));
                Seen { from, now }
            })
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
