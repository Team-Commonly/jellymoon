//! What the window does with a key, a tap, Enter, a step of the reading and a look, with no window: each act is a
//! call here that gives work for another thread, and a second call that takes what the work gave and says what to
//! show. `window.rs` only runs the work off its thread and hands each scene to the kit's view, so the app's tests
//! make the same calls in the same order (`tests/f43/showing.rs`). No GPUI.

use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;
use std::time::Instant;

use sticker_kit::{Scene, Theme};

use crate::drawing::Open;
use crate::newer::Watch;
use crate::placing::{any_of, made, put, whose};
use crate::scene::drawn_id;
use crate::starting::{Given, Listed, Listing, Shown};
use crate::stopping::Chosen;
use crate::taps::OnShow;
use crate::today::{Later, Next};
use crate::wishing::{Answer, Asking, Look, Offering, Reading, Said, Seen, Wishing};

mod after;
mod turns;
pub use after::{After, Ended};
use turns::Turns;

/// The day's page on show, shared by the window's handlers, which all run on its thread. A page-data file has no
/// day: every act on it is nothing. With it, the project core listed while no entry offered it (F64): a folder
/// dropped or a name entered while the day is still being read. And which made wishes' notes a look makes again
/// (`showing/turns.rs`). And which drawn notes the person opened, which made wish they tapped last and which made
/// wishes are tucked away (`drawing.rs`).
/// And the watch of the bundle the window runs from (`newer.rs`).
#[derive(Clone, Default)]
pub struct Showing(Rc<RefCell<Option<OnShow>>>, Rc<RefCell<Option<String>>>, Rc<RefCell<Turns>>, Open, Rc<Watch>);

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
        let open = day.as_ref().map_or_else(Open::default, |on| Open::opening(&on.scene));
        Showing(Rc::new(RefCell::new(day)), Rc::default(), Rc::default(), open, Rc::new(Watch::running()))
    }

    /// The same day in a window that runs from `bundle`, whose version is read now: a test's bundle is a folder
    /// of its own.
    pub fn bundled(self, bundle: Option<&Path>) -> Self {
        Showing(self.0, self.1, self.2, self.3, Rc::new(Watch::of(bundle)))
    }

    /// The project whose chip is chosen as soon as an entry offers it, or none.
    pub fn wait_for(&self, name: Option<String>) {
        *self.1.borrow_mut() = name;
    }

    /// The project waiting for an entry that offers its chip.
    pub fn waiting(&self) -> Option<String> {
        self.1.borrow().clone()
    }

    fn with<T>(&self, act: impl FnOnce(&mut OnShow) -> T) -> Option<T> {
        self.0.borrow_mut().as_mut().map(act)
    }

    /// The scene on show, when there is a day. The window gives its view every scene it is given here, and the
    /// kit's view gives none out, so a test of the window reads what is on show from this.
    pub fn scene(&self) -> Option<Scene> {
        self.with(|on| on.scene.clone())
    }

    /// `scene` as the kit's view is given it: its drawn notes open or closed as the person left them, in the order
    /// of its column (`drawing.rs`). Every scene the window shows passes here, and no other place orders one.
    /// Once a newer build was found installed, its note is the column's last (`newer.rs`): it is on no scene
    /// that is kept, so no tap, choice or offer knows of it.
    pub fn column(&self, scene: &Scene) -> Scene {
        self.3.column(&Scene { notes: self.4.noted(scene.notes.clone()), ..scene.clone() })
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

    /// One look at the day, as work for another thread. Every made wish that has ended keeps its notes on the
    /// page (`placing.rs`, ADR 0030), and a look shows the ones it has. It makes them again, by the calls to core
    /// a tap on the wish's little one makes, only for the wishes `Turns::asks` names: every ended wish when the
    /// window opens, when a made wish's run ends and after a choice on any wish's work was answered, each not
    /// while a note of it is in the person's hand nor in the `turns::HOLD` looks after the person acted on it.
    /// With none to make again, and no run found ended, core is asked nothing of a wish that has ended.
    pub fn look(&self) -> Option<Look> {
        self.look_at(Instant::now())
    }

    /// [`Showing::look`] made at `now`. At most once a minute the look also reads the version of the bundle on
    /// disk, after its own work and off the window's thread with it (`newer.rs`). The look that finds a newer
    /// build gives the scene on show to be drawn again when it has nothing else to show.
    pub fn look_at(&self, now: Instant) -> Option<Look> {
        self.with(|on| {
            let (whom, held) = self.2.borrow_mut().asks(&on.scene);
            let (today, page, from) = (on.today.clone(), on.page.clone(), on.scene.clone());
            let Some(check) = self.4.check(now) else {
                return Box::new(move || today.looked(page.as_ref(), from, &whom, &held)) as Look;
            };
            Box::new(move || {
                let seen = today.looked(page.as_ref(), from.clone(), &whom, &held);
                let again = check().then(|| Seen { from: from.clone(), now: from });
                seen.or(again)
            })
        })
    }

    /// What a look, or a tap on a character, gave: the scene to show. None when the scene on show has changed since.
    pub fn seen(&self, seen: Seen) -> Option<Scene> {
        let scene = self.with(|on| on.seen(seen)).flatten();
        self.2.borrow_mut().saw(scene.as_ref());
        scene
    }

    /// Enter in the entry, as work for another thread.
    pub fn wishing(&self, typed: &str, project: Option<&str>) -> Option<Wishing> {
        self.with(|on| on.shaping(typed, project))
    }

    /// Enter in the field of the note `note`, as work for another thread: an answer to a run that asks, or a
    /// follow-up. With an answer comes the scene to show at once, in which the wish is at work. None when that
    /// note has no field on show, and then nothing is asked of core.
    pub fn typing(&self, note: &str, typed: &str) -> Option<(Option<Scene>, Wishing)> {
        let typing = self.with(|on| on.typing(note, typed)).flatten();
        typing.inspect(|_| self.acted(note))
    }

    /// Core's answer to an Enter, or none when there was no day to ask. The entry, or the field, is always
    /// answered, so it never waits for ever, and it is told its words were kept only when core took them. An
    /// answer that asks for a run is of a new made wish, typed in the entry or as a follow-up: every settled
    /// wish tucks away then (`drawing/tucked.rs`).
    pub fn answered(&self, answer: Option<Answer>) -> Entered {
        let (kept, earlier) = answer.as_ref().map_or((false, 0), |answer| (answer.kept(), answer.earlier()));
        let (scene, asking) = answer.and_then(|answer| self.with(|on| on.answered(answer))).unzip();
        let asking: Option<Asking> = asking.flatten();
        if let (Some(scene), Some(_)) = (&scene, &asking) {
            self.3.made(scene, None, earlier);
        }
        Entered { kept, scene, asking }
    }

    /// Whether the entry may give the Enter it remembered while its answer was owed (the kit's
    /// `Page::answer_entry`), asked once that answer is on show: not over a sharper wish that waits for the
    /// person's choice (`shaping::waits`). The window has the entry forget it then, and its words stay typed.
    pub fn takes_enter(&self) -> bool {
        !self.with(|on| crate::shaping::waits(&on.scene)).unwrap_or(false)
    }

    /// `Start` tapped while the entry `id` held no words: the scene to show.
    pub fn unready(&self, id: &str) -> Option<Scene> {
        self.with(|on| on.unready(id))
    }

    /// A folder dropped or chosen, or a new project's name entered, as work for another thread.
    pub fn listing(&self, given: Given) -> Option<Listing> {
        self.with(|on| on.listing(given))
    }

    /// What a [`Listing`] gave: the scene, the entry, and the project's name when core listed it.
    pub fn listed(&self, listed: Listed) -> Option<Shown> {
        self.with(|on| on.listed(listed))
    }

    /// What the asking for a wish's run had to say: the scene to show, when it said something.
    pub fn asked(&self, said: Option<Said>) -> Option<Scene> {
        self.with(|on| on.asked(said)).flatten()
    }

    /// The person acted on the note `note`: its made wish has the offer of a follow-up, and its notes are held.
    fn acted(&self, note: &str) {
        let wish = self.with(|on| whose(&on.scene, note)).flatten();
        wish.iter().for_each(|wish| self.3.offer(wish.clone()));
        self.2.borrow_mut().acted(wish);
    }

    /// A tap on a character, as work for another thread. What it gives goes to [`Showing::seen`]. Of a made wish
    /// it is also the tap that takes the offer of a follow-up under that wish and opens or closes the drawn note
    /// of its finished work (`drawing::Open`), which the scene it gives is drawn with. Of a made wish that was
    /// tucked away it brings the notes back: the scene on show is given to be drawn again even when core says
    /// nothing new of the wish. Core's answer that the wish was made again, which that tap's notes leave out,
    /// stays under them (`again.rs`).
    pub fn poked(&self, id: &str) -> Option<Look> {
        self.with(|on| {
            let back = made(&on.scene).iter().any(|one| one.id == id) && self.3.poked(&on.scene, id);
            let (look, from) = (on.poked(id), on.scene.clone());
            if !back {
                return look;
            }
            // The answer that the wish was made again comes back with the notes: the new wish is under it.
            let (said, id) = (self.3.answer(&from, id), id.to_string());
            Box::new(move || {
                let mut seen = look().unwrap_or_else(|| Seen { now: from.clone(), from });
                if let Some(said) = said.filter(|said| !seen.now.notes.contains(said)) {
                    let notes = seen.now.notes.iter().filter(|note| any_of(note, &id)).cloned().chain([said]);
                    let notes = notes.collect();
                    put(&mut seen.now, &id, notes);
                }
                Some(seen)
            }) as Look
        })
    }

    /// The project of the reading's wish `id`, whose chip a tap on its character chooses as a click on the chip
    /// does (`window.rs`): the next wish typed goes on there. None of a made wish, which chooses no chip.
    pub fn chip_of(&self, id: &str) -> Option<String> {
        let project = |on: &mut OnShow| {
            let read = &on.page.as_ref()?.data.wishes;
            read.iter().find(|wish| drawn_id(&wish.id) == id).map(|wish| wish.project.clone())
        };
        self.with(project)?
    }

    /// A tap on an option: the scene to show at once, and, when it was the choice to stop a run or a choice of
    /// work that waits, core's part of it as work for another thread. None for a tap that is nothing. A tap on a
    /// drawn note's picture opens the note or closes it: the scene is the one on show, and core is not asked.
    ///
    /// `words` are the option's as they were drawn when it was tapped (the kit's tap carries them). A tap means
    /// what was seen: when the note of that id on show now has other words at that place, or is gone, the page
    /// changed under the pointer, so nothing is carried to core, nothing is written, and the scene on show is
    /// given back to be drawn again. A tap on a picture carries no words.
    pub fn chose(&self, note: &str, index: usize, words: &str) -> Option<(Scene, Option<After>)> {
        if index == sticker_kit::PICTURE_PLACE {
            let wish = self.with(|on| whose(&on.scene, note)).flatten();
            wish.into_iter().for_each(|wish| self.3.offer(wish));
            return self.scene().filter(|scene| self.3.tapped(scene, note)).map(|scene| (scene, None));
        }
        let on_show = self.scene()?;
        if on_show.option(note, index) != Some(words) {
            return Some((on_show, None));
        }
        // A stop holds nothing: the stopped wish has the notes of an ended wish at the next look.
        let chosen = match self.with(|on| on.chose(note, index))? {
            Chosen::Stop(scene, stop) => return Some((scene, Some(After::Stop(stop)))),
            Chosen::Work(scene, work) => Some((scene, Some(After::Work(work)))),
            Chosen::Tapped(scene) => Some((scene, None)),
            Chosen::Nothing => None,
        };
        chosen.inspect(|_| self.acted(note))
    }

    /// What the work a tap left gave: the scene to show. Core's answer to a choice on a wish's work may change
    /// what core says of every other wish (a keep or an undo moves the folder under their changes), so every
    /// ended wish is owed its notes, and the next look makes them again; the answered wish's own are held, from
    /// here again, so an answer that was long in coming is read as long as a quick one. An answer that made a
    /// wish (a sharper wish gone with, `Make it again`) tucks every settled wish away but the answered one.
    /// A wish that `Make it again` made again, its old work tossed, steps aside too (`Open::remade`).
    pub fn ended(&self, ended: Ended) -> Option<Scene> {
        self.with(|on| match ended {
            Ended::Stopped(stopped) => on.stopped(stopped),
            Ended::Worked(mut worked) => {
                let wish = worked.wish.clone();
                self.3.remade(&wish, worked.remade.take());
                let (now, before) = (made(&worked.seen.now).len(), made(&worked.seen.from).len());
                let scene = self.3.showed(on.worked(worked));
                if now > before {
                    self.3.made(&scene, Some(&wish), now - 1);
                }
                let mut turns = self.2.borrow_mut();
                turns.owe(&scene);
                turns.acted(Some(wish));
                scene
            }
        })
    }
}

/// Whether a key switches the theme: `t` with the platform's command key (Ctrl off the Mac), which no field takes.
/// A bare `t` is always a letter: the entry has the keys whenever nothing else does.
pub fn switches_theme(key: &str, command: bool) -> bool {
    key == "t" && command
}

/// The built-in theme the command key and T switches to from the theme named `name`: the plain one, and from that the sticker one.
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

#[cfg(test)]
mod tests;
