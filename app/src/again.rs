//! `Make it again`, carried (ADR 0030, decision 10). Core offers the choice under a keep it refused because the
//! change no longer fits the folder, and under work that waits because it did not fit when its run ended, where
//! `Keep it` would be (`working::waiting`); to a tap on either core says which words and which project
//! (`WorkDone::Again`). Here the wish is made as the entry makes a typed one, by the same calls
//! (`jellymoon_core::wish_offered`, `Today::asking`), so every check and limit of a typed wish holds. Only once
//! core kept the new wish is the work that does not fit tossed, by the call a tap on `Toss it` is carried by; a
//! wish core refuses tosses nothing, and neither does a tap with no installed command to run it. The run is started last: a toss asks git where every copy of the repo
//! stands, and core refuses it while a new run's copy is being made. Every word is core's or a typed wish's.

//!
//! The answer says the new wish is being worked on, and it stays under the old wish after that work has ended.
//! So the new wish's id is kept with the answer where it is made ([`Remade`]), and the answer is drawn in
//! core's words for afterwards once that wish is at work no more ([`Again::drawn`]).
//!
//! When all of it worked (the wish made, the old work tossed, the run started) the old wish steps aside: it is
//! tucked away at once (ADR 0030, decision 11), as the new wish under it, with the same words, is the answer.
//! Whether the work was tossed is core's to say: it keeps the old wish as let go, which a toss writes last.

use std::cell::RefCell;
use std::rc::Rc;

use jellymoon_core::{Wish, WishState, WorkDone, WorkTap};
use sticker_kit::{Note, Scene, State};

use crate::placing::any_of;
use crate::today::Today;
use crate::wishing::{UNKEPT, of};
use crate::working::Ask;

/// The made wish a run was started for by `Make it again`, by its id as its little one carries it, what core
/// says in place of its answer once that wish is worked on no more, and whether the old work was tossed.
pub(crate) type Since = Option<(String, WorkDone, bool)>;

/// Core's answer that a wish was made again, with the wish it speaks of: the new wish's id, the answer's note
/// as it says that wish is being worked on, and the same note as core says it afterwards. And whether the old
/// work was tossed: the old wish then steps aside (`drawing::Open::remade`).
pub(crate) struct Remade {
    wish: String,
    going: Note,
    since: Note,
    pub(crate) tossed: bool,
}

impl Remade {
    /// The answer whose notes are `going`, of the new wish `wish`, and `since`, the notes of it afterwards:
    /// an answer is one note. None when either has none.
    pub(crate) fn of(wish: String, going: &[Note], since: Vec<Note>, tossed: bool) -> Option<Box<Remade>> {
        let (going, since) = (going.first()?.clone(), since.into_iter().next()?);
        Some(Box::new(Remade { wish, going, since, tossed }))
    }
}

/// The answers of one window that say a wish was made again: the window's own state, held with what is open
/// (`drawing::Open`). No call to core, and nothing written.
#[derive(Clone, Default)]
pub(crate) struct Again(Rc<RefCell<Vec<Remade>>>);

impl Again {
    pub(crate) fn keep(&self, remade: Option<Box<Remade>>) {
        self.0.borrow_mut().extend(remade.map(|one| *one));
    }

    /// The answer on `scene` under the made wish `wish` that says a wish was made again, as core gave it.
    pub(crate) fn under(&self, scene: &Scene, wish: &str) -> Option<Note> {
        let is = |note: &&Note| any_of(note, wish) && self.0.borrow().iter().any(|one| one.going == **note);
        scene.notes.iter().find(is).cloned()
    }

    /// `scene` as it is drawn: an answer that says a wish was made again and is being worked on says only that
    /// it was made again once that wish, on the scene, is at work no more. The note is found by being that
    /// answer, whole, and the wish by its id; while the wish works, and of a wish the scene does not hold, the
    /// answer is as core gave it.
    pub(crate) fn drawn(&self, scene: &Scene) -> Scene {
        let mut drawn = scene.clone();
        for remade in self.0.borrow().iter() {
            let made = crate::placing::made(scene).iter();
            let ended = made.filter(|one| one.id == remade.wish).any(|one| one.state != State::Busy);
            for note in drawn.notes.iter_mut().filter(|note| ended && **note == remade.going) {
                *note = remade.since.clone();
            }
        }
        drawn
    }
}

impl Today {
    /// Core's answer to a tap on a choice of `wish`, by `ask`. An answer that the wish is to be made again is
    /// carried on here: the new wish, then the toss, asked of core by `ask` as `Toss it` is, and then the run.
    /// With it, when that run was started, the new wish, the answer for afterwards and whether the old work
    /// was tossed ([`Since`]): core keeps the old wish as let go, which a toss writes last, when all of it was
    /// done. No sentence of the answer is read for it.
    pub(crate) fn asked(&self, wish: &str, tap: WorkTap, ask: &impl Ask) -> (WorkDone, Since) {
        let (said, project) = match ask(self, wish, tap) {
            WorkDone::Again(said, project) => (said, project),
            done => return (done, None),
        };
        let unmade =
            |refused: jellymoon_core::Error| (WorkDone::unmade(UNKEPT, of(&said, &format!("{refused}."))), None);
        // With no installed command no run could be started: nothing is made, and nothing is tossed.
        if let Err(refused) = jellymoon_core::installed_command(&self.bin.clone().unwrap_or_default()) {
            return unmade(refused);
        }
        let made = match jellymoon_core::wish_offered(&self.read(), &said, &project) {
            Ok(made) => made.wish,
            Err(refused) => return unmade(refused),
        };
        let tossed = ask(self, wish, WorkTap::Toss);
        let let_go = |one: &Wish| crate::scene::drawn_id(&one.id) == wish && one.state == WishState::LetGo;
        let gone = jellymoon_core::made(&self.home, &self.date).iter().any(let_go);
        let unstarted = (self.asking(made.id.clone(), made.said))();
        // Only a wish whose run was started is said to be worked on, so only that answer has an afterwards.
        let since = if unstarted.is_none() { tossed.remade_since() } else { None };
        (tossed.remade(unstarted), since.map(|since| (crate::scene::drawn_id(&made.id), since, gone)))
    }
}
