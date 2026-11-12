//! What the window does with a key, a tap, Enter, a step of the reading and a look, with no window: each act is a
//! call here that gives work for another thread, and a second call that takes what the work gave and says what to
//! show. `window.rs` only runs the work off its thread and hands each scene to the kit's view, so the app's tests
//! make the same calls in the same order (`tests/showing.rs`). No GPUI.

use std::cell::RefCell;
use std::rc::Rc;

use sticker_kit::{Scene, Theme};

use crate::stopping::{Chosen, Stopped, Stopping};
use crate::taps::OnShow;
use crate::today::{Later, Next};
use crate::wishing::{Answer, Asking, Look, Offering, Reading, Said, Seen, Wishing};

/// The day's page on show, shared by the window's handlers, which all run on its thread. A page-data file has no
/// day: every act on it is nothing.
#[derive(Clone, Default)]
pub struct Showing(Rc<RefCell<Option<OnShow>>>);

/// What the window does once core has answered an Enter.
pub struct Entered {
    /// What the entry is told: kept, and it empties; not kept, and its words stay.
    pub kept: bool,
    /// The scene to show, when there is a day.
    pub scene: Option<Scene>,
    /// The asking for the wish's run, when core kept the wish: work for another thread, answered with
    /// [`Showing::asked`].
    pub asking: Option<Asking>,
}

impl Showing {
    pub fn of(day: Option<OnShow>) -> Self {
        Showing(Rc::new(RefCell::new(day)))
    }

    fn with<T>(&self, act: impl FnOnce(&mut OnShow) -> T) -> Option<T> {
        self.0.borrow_mut().as_mut().map(act)
    }

    /// The entry of the first page, as work for another thread.
    pub fn offering(&self) -> Option<Offering> {
        self.with(|on| on.offering())
    }

    /// A step of the day's reading with the entry of the page it gives, as work for another thread.
    pub fn reading(&self, step: Later) -> Option<Reading> {
        self.with(|on| on.today.clone().offered(step))
    }

    /// What a step of the reading gave: the scene to show.
    pub fn read(&self, next: &Next) -> Option<Scene> {
        self.with(|on| on.show(next))
    }

    /// One look at the day, as work for another thread.
    pub fn look(&self) -> Option<Look> {
        self.with(|on| on.look())
    }

    /// What a look, or a tap on a character, gave: the scene to show. None when the scene on show has changed since.
    pub fn seen(&self, seen: Seen) -> Option<Scene> {
        self.with(|on| on.seen(seen)).flatten()
    }

    /// Enter in the entry, as work for another thread.
    pub fn wishing(&self, typed: &str, project: Option<&str>) -> Option<Wishing> {
        self.with(|on| on.wishing(typed, project))
    }

    /// Core's answer to an Enter, or none when there was no day to ask. The entry is always answered, so it never
    /// waits for ever, and it is told its words were kept only when core kept the wish.
    pub fn answered(&self, answer: Option<Answer>) -> Entered {
        let (scene, asking) = answer.and_then(|answer| self.with(|on| on.answered(answer))).unzip();
        let asking = asking.flatten();
        Entered { kept: asking.is_some(), scene, asking }
    }

    /// What the asking for a wish's run had to say: the scene to show, when it said something.
    pub fn asked(&self, said: Option<Said>) -> Option<Scene> {
        self.with(|on| on.asked(said)).flatten()
    }

    /// A tap on a character, as work for another thread. What it gives goes to [`Showing::seen`].
    pub fn poked(&self, id: &str) -> Option<Look> {
        self.with(|on| on.poked(id))
    }

    /// A tap on an option: the scene to show at once, and the stop when it was the choice to stop a run, as work
    /// for another thread. None for a tap that is nothing.
    pub fn chose(&self, note: &str, index: usize) -> Option<(Scene, Option<Stopping>)> {
        match self.with(|on| on.chose(note, index))? {
            Chosen::Stop(scene, stop) => Some((scene, Some(stop))),
            Chosen::Tapped(scene) => Some((scene, None)),
            Chosen::Nothing => None,
        }
    }

    /// What a stop gave: the scene to show.
    pub fn stopped(&self, stopped: Stopped) -> Option<Scene> {
        self.with(|on| on.stopped(stopped))
    }
}

/// Whether a key switches the theme: `t` does, unless the entry has the keys, where it is a letter of a wish.
pub fn switches_theme(key: &str, entry_has_keys: bool) -> bool {
    key == "t" && !entry_has_keys
}

/// The built-in theme `t` switches to from the theme named `name`: the plain one, and from that the sticker one.
pub fn other_theme(name: &str) -> Theme {
    if name == "plain" { Theme::sticker() } else { Theme::plain() }
}

/// The window's line for a tap: the piece's id, and the option when it was one. Never a word of the page.
pub fn tap_line(id: &str, option: Option<usize>) -> String {
    option.map_or(format!("tap id={id}"), |index| format!("tap id={id} option={index}"))
}

/// The window's line for an Enter: the entry's id and how many characters were entered, never the words.
pub fn entry_line(id: &str, typed: &str) -> String {
    format!("entry id={id} characters={}", typed.chars().count())
}
