//! What the window does with a key, a tap, Enter, a step of the reading and a look, with no window: each act is a
//! call here that gives work for another thread, and a second call that takes what the work gave and says what to
//! show. `window.rs` only runs the work off its thread and hands each scene to the kit's view, so the app's tests
//! make the same calls in the same order (`tests/f43/showing.rs`). No GPUI.

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use sticker_kit::{Character, Note, Scene, State, Theme};

use crate::scene::drawn_id;
use crate::starting::{Given, Listed, Listing, Shown};
use crate::stopping::{Chosen, Stopped, Stopping};
use crate::taps::OnShow;
use crate::today::{Later, Next};
use crate::wishing::{Answer, Asking, Look, Offering, Reading, Said, Seen, Wishing};
use crate::working::{self, Worked, Working};

/// The day's page on show, shared by the window's handlers, which all run on its thread. A page-data file has no
/// day: every act on it is nothing. With it, the project core listed while no entry offered it (F64): a folder
/// dropped or a name entered while the day is still being read. And whose work a look may show by itself.
#[derive(Clone, Default)]
pub struct Showing(Rc<RefCell<Option<OnShow>>>, Rc<RefCell<Option<String>>>, Rc<RefCell<Turns>>);

/// Whose work that waits a look may show with no tap. Kept on the window's thread.
#[derive(Default)]
struct Turns {
    /// The work that waited when the window opened is looked for by one look that is shown, and by no other. It
    /// shows the newest wish's, and every other wish it finds work of has its turn after it (`later`).
    first: First,
    /// The made wishes of the last look. None before the first: a wish there then is not new to this window.
    known: Option<BTreeSet<String>>,
    /// The wishes this window saw made or working, whose work was not shown since and is not known to be none.
    turn: BTreeSet<String>,
    /// The wishes a look asked core of that have no work waiting, told from the look's thread.
    none: Arc<Mutex<Vec<String>>>,
    /// The wishes the first look found work of and did not show, told from its thread: their turn is to come.
    later: Arc<Mutex<Vec<String>>>,
    /// The looks still to be made before one may show work: [`HOLD`] after a tap on an option, an Enter in a
    /// field, core's answer to either, and a look made while a note was in hand.
    hold: usize,
}

/// The looks that show no work after a tap on an option or core's answer to it. A look is made once a second, so
/// work comes three to four seconds after the answer is on the page: past a slow second tap, or a tap made again
/// by someone not sure the first was taken, on options that would stand where the tapped ones stood.
const HOLD: usize = 3;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum First {
    #[default]
    Due,
    Asked,
    Done,
}

/// What one look may ask core of.
struct Asks {
    first: bool,
    turn: BTreeSet<String>,
    /// The made wishes of the scene the look was made of: no other wish's work is put on it.
    made: BTreeSet<String>,
    none: Arc<Mutex<Vec<String>>>,
    later: Arc<Mutex<Vec<String>>>,
}

/// The characters of the day's made wishes: the last of the page's, one for each row of the note of them.
fn made(scene: &Scene) -> &[Character] {
    let rows = |note: &Note| match note {
        Note::Whole { title, rows } if title == crate::scene::MADE => Some(rows.len()),
        _ => None,
    };
    let count = scene.notes.iter().find_map(rows).unwrap_or(0);
    &scene.characters[scene.characters.len().saturating_sub(count)..]
}

impl Turns {
    /// A look is about to be made of `scene`: what it may ask core of, or none while a note on the page is in
    /// the person's hand ([`working::in_hand`]). Then the look is the plain one, every turn is kept, and the
    /// [`HOLD`] looks after it show no work, however the page leaves the hand: by a tap, or by a look that took a
    /// stop's or a clash's choice away.
    fn asks(&mut self, scene: &Scene) -> Option<Asks> {
        let made = made(scene);
        let ids = |of: &dyn Fn(&Character) -> bool| -> Vec<String> {
            made.iter().filter(|wish| of(wish)).map(|wish| wish.id.clone()).collect()
        };
        if let Some(known) = &self.known {
            self.turn.extend(ids(&|wish| !known.contains(&wish.id)));
        }
        let all: BTreeSet<String> = ids(&|_| true).into_iter().collect();
        self.known = Some(all.clone());
        if let Ok(mut later) = self.later.lock() {
            self.turn.extend(later.drain(..));
        }
        if let Ok(mut none) = self.none.lock() {
            for id in none.drain(..) {
                self.turn.remove(&id);
            }
        }
        for id in working::shown(scene) {
            self.turn.remove(id);
        }
        self.turn.extend(ids(&|wish| wish.state == State::Busy));
        if self.first == First::Asked {
            self.first = First::Done;
        }
        if working::in_hand(scene) {
            self.hold = HOLD;
            return None;
        }
        if self.hold > 0 {
            self.hold -= 1;
            return None;
        }
        let first = self.first == First::Due;
        if first {
            self.first = First::Asked;
        }
        let (none, later) = (self.none.clone(), self.later.clone());
        Some(Asks { first, turn: self.turn.clone(), made: all, none, later })
    }

    /// The person tapped an option or pressed Enter in a field, or core answered one: no look shows work until
    /// [`HOLD`] more have been made. Every turn is kept. A look in flight then is dropped by `OnShow::seen`, as
    /// the act changed the scene it was made of.
    fn acted(&mut self) {
        self.hold = HOLD;
    }

    /// What a look or a tap gave was shown as `scene`, or dropped. Work on show has had its turn. A dropped
    /// look that `carried` work is made again: the first look is owed still. What a run says it did, put on the
    /// page with no work under it, has nothing to choose and so is never in hand: it holds the [`HOLD`] looks
    /// after it itself, so no other wish's work comes over it before it can be read.
    fn saw(&mut self, scene: Option<&Scene>, carried: bool) {
        match scene {
            Some(scene) => {
                for id in working::shown(scene) {
                    self.turn.remove(id);
                }
                if carried && working::told_on(scene) {
                    self.hold = HOLD;
                }
            }
            None if carried && self.first == First::Asked => self.first = First::Due,
            None => {}
        }
    }
}

impl Asks {
    /// The wishes to ask core of, in the order asked, on the page as the look drew it: none whose run goes. The
    /// first look asks the wishes that are done, the newest first; then each wish whose turn it is, in core's order.
    fn whom(&self, now: &Scene) -> Vec<String> {
        let made = made(now);
        let done = made.iter().rev().filter(|wish| self.first && wish.state == State::Idle);
        let ended = made.iter().filter(|wish| wish.state != State::Busy && self.turn.contains(&wish.id));
        let mut whom: Vec<String> = Vec::new();
        for wish in done.chain(ended) {
            if self.made.contains(&wish.id) && !whom.contains(&wish.id) {
                whom.push(wish.id.clone());
            }
        }
        whom
    }

    /// Tells the window's thread of the wish `id`. In `none`: it has no work waiting, and is not asked of again
    /// for this end of its run. In `later`: the first look found its work and shows another's, so its turn comes.
    fn tell(list: &Mutex<Vec<String>>, id: String) {
        if let Ok(mut list) = list.lock() {
            list.push(id);
        }
    }
}

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

/// What a tap on an option left to do: work for another thread.
pub enum After {
    Stop(Stopping),
    Work(Working),
}

/// What an [`After`] gave.
pub enum Ended {
    Stopped(Stopped),
    Worked(Worked),
}

impl After {
    /// The work itself: it asks core, which reads files and a repo, or waits for a run to end.
    pub fn run(self) -> Ended {
        match self {
            After::Stop(stop) => Ended::Stopped(stop()),
            After::Work(work) => Ended::Worked(work()),
        }
    }
}

impl Showing {
    pub fn of(day: Option<OnShow>) -> Self {
        Showing(Rc::new(RefCell::new(day)), Rc::default(), Rc::default())
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

    /// One look at the day, as work for another thread. A look that finds a wish's run ended shows the work of
    /// it that waits, as a tap on its character does ([`Showing::poked`]), by the calls to core a tap makes, each
    /// wish asked of once. Not while a note on the page is in the person's hand, and not in the [`HOLD`] looks
    /// after the page left the hand, the person chose or typed or core answered them: then the work waits its
    /// turn. Of several, the first in core's order is shown. The first look shown also finds the work that
    /// waited when the window opened: it shows the newest wish's, and the others have their turn after it, as
    /// work of a run that ended under this window does, so a keep or a toss is followed by the next that waits.
    /// What a run that changed nothing says it did (told alone) is shown the same way, but only when no wish
    /// asked has work that waits: the look asks on past it, and a wish passed over so keeps its turn.
    pub fn look(&self) -> Option<Look> {
        self.with(|on| {
            let Some(asks) = self.2.borrow_mut().asks(&on.scene) else { return on.look() };
            let (today, page, from) = (on.today.clone(), on.page.clone(), on.scene.clone());
            Box::new(move || {
                let (mut now, mut shown) = (today.again(page.as_ref(), &from), None);
                let mut found = false;
                for id in asks.whom(&now) {
                    match today.waiting_over(page.as_ref(), &now, &id) {
                        // The first look asks on past the work it shows, for the work that waits its turn.
                        Some((_, true)) if found => Asks::tell(&asks.later, id),
                        Some((work, true)) => {
                            (shown, found) = (Some(work), true);
                            if !asks.first {
                                break;
                            }
                        }
                        Some((alone, false)) if !found => shown = shown.or(Some(alone)),
                        Some(_) => {}
                        None => Asks::tell(&asks.none, id),
                    }
                }
                now = shown.unwrap_or(now);
                (now != from).then_some(Seen { from, now })
            }) as Look
        })
    }

    /// What a look, or a tap on a character, gave: the scene to show. None when the scene on show has changed since.
    pub fn seen(&self, seen: Seen) -> Option<Scene> {
        let was: Vec<&str> = working::shown(&seen.from).collect();
        let carried = working::shown(&seen.now).any(|id| !was.contains(&id));
        let scene = self.with(|on| on.seen(seen)).flatten();
        self.2.borrow_mut().saw(scene.as_ref(), carried);
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
        typing.inspect(|_| self.2.borrow_mut().acted())
    }

    /// Core's answer to an Enter, or none when there was no day to ask. The entry, or the field, is always
    /// answered, so it never waits for ever, and it is told its words were kept only when core took them.
    pub fn answered(&self, answer: Option<Answer>) -> Entered {
        let kept = answer.as_ref().is_some_and(Answer::kept);
        let (scene, asking) = answer.and_then(|answer| self.with(|on| on.answered(answer))).unzip();
        self.2.borrow_mut().acted();
        Entered { kept, scene, asking: asking.flatten() }
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

    /// A tap on a character, as work for another thread. What it gives goes to [`Showing::seen`].
    pub fn poked(&self, id: &str) -> Option<Look> {
        self.with(|on| on.poked(id))
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
    /// work that waits, core's part of it as work for another thread. None for a tap that is nothing.
    pub fn chose(&self, note: &str, index: usize) -> Option<(Scene, Option<After>)> {
        let chosen = match self.with(|on| on.chose(note, index))? {
            Chosen::Stop(scene, stop) => Some((scene, Some(After::Stop(stop)))),
            Chosen::Work(scene, work) => Some((scene, Some(After::Work(work)))),
            Chosen::Tapped(scene) => Some((scene, None)),
            Chosen::Nothing => None,
        };
        chosen.inspect(|_| self.2.borrow_mut().acted())
    }

    /// What the work a tap left gave: the scene to show. The looks held back are counted from here again, so an
    /// answer that was long in coming is on the page as long as a quick one before work is shown under it.
    pub fn ended(&self, ended: Ended) -> Option<Scene> {
        self.2.borrow_mut().acted();
        self.with(|on| match ended {
            Ended::Stopped(stopped) => on.stopped(stopped),
            Ended::Worked(worked) => on.worked(worked),
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
