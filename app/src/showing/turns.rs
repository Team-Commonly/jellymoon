//! Which made wishes' notes a look makes again (ADR 0030, "When the notes are made again"). Every made wish that
//! has ended keeps its notes on the page, and a look shows the ones it has: core is asked of a wish again only
//! when what it would say may have changed. That is when the window opens, when the run of any made wish ends
//! (a change that landed takes the undo of an older one away), and when a choice on any wish's work was answered.
//! Between those a look asks core nothing of a wish that has ended. The look that finds a run ended makes the
//! notes again itself (`Today::looked`), off the window's thread; what is kept here is kept on the window's.

use std::collections::{BTreeMap, BTreeSet};

use sticker_kit::{Scene, State};

use crate::placing::made;

/// The looks that leave a wish's notes as they are after a tap on one of them, an Enter under one, or core's
/// answer to either. A look is made once a second, so the answer is read for three seconds before anything of
/// that wish is made again under it. The hold is the wish's own: every other wish's notes are made again at once.
pub(super) const HOLD: usize = 3;

/// The made wishes of a scene, each with whether its run goes.
type Known = BTreeMap<String, bool>;

fn known(scene: &Scene) -> Known {
    made(scene).iter().map(|one| (one.id.clone(), one.state == State::Busy)).collect()
}

/// Whether a wish of `now` has ended that `before` did not know as ended: its run ended, or it is new.
fn ended_since(before: &Known, now: &Known) -> bool {
    now.iter().any(|(id, working)| !working && before.get(id) != Some(&false))
}

#[derive(Default)]
pub(super) struct Turns {
    /// The made wishes of the last scene looked at or shown by a look. None before the first look.
    known: Option<Known>,
    /// The wishes whose notes the next look that may makes again.
    owed: BTreeSet<String>,
    /// The wishes the last look was given. They are owed again when its scene is dropped.
    asked: Vec<String>,
    /// For a wish the person acted on, the looks still to be made before one makes its notes again.
    hold: BTreeMap<String, usize>,
}

impl Turns {
    /// A look is about to be made of `scene`: the made wishes whose notes it makes again, none of them working,
    /// and the wishes in their hold, whose notes it leaves even when it finds a run ended. A wish that has ended
    /// and was not known so (the window's first look, a run a tap stopped) owes every ended wish its notes. A
    /// wish in its hold, or with a note in the person's hand, stays owed.
    pub(super) fn asks(&mut self, scene: &Scene) -> (Vec<String>, Vec<String>) {
        let now = known(scene);
        let before = self.known.replace(now.clone());
        if before.is_none_or(|before| ended_since(&before, &now)) {
            self.owe(scene);
        }
        self.owed.retain(|id| now.contains_key(id));
        let free = |id: &&String| {
            !self.hold.contains_key(*id) && now.get(*id) == Some(&false) && !crate::placing::in_hand(scene, id)
        };
        let asked: Vec<String> = self.owed.iter().filter(free).cloned().collect();
        let held = self.hold.keys().cloned().collect();
        self.hold.retain(|_, left| {
            *left -= 1;
            *left > 0
        });
        self.owed.retain(|id| !asked.contains(id));
        self.asked.clone_from(&asked);
        (asked, held)
    }

    /// Every made wish of `scene` that has ended is owed its notes: core's answer to a choice, or a run's end,
    /// may have changed what core says of any of them.
    pub(super) fn owe(&mut self, scene: &Scene) {
        let ended = made(scene).iter().filter(|one| one.state != State::Busy);
        self.owed.extend(ended.map(|one| one.id.clone()));
    }

    /// The person tapped an option of a note of `wish` or pressed Enter under one, or core answered a choice
    /// on its work: no look makes that wish's notes again until [`HOLD`] more have been made.
    pub(super) fn acted(&mut self, wish: Option<String>) {
        self.hold.extend(wish.map(|wish| (wish, HOLD)));
    }

    /// What a look or a tap on a character gave was shown, as `shown`, or dropped as the page had changed under
    /// it. The wishes a dropped look was given are owed their notes still. A look that found a run ended made
    /// the notes of every ended wish again but of those in their hold (`Today::looked`), which are owed theirs.
    pub(super) fn saw(&mut self, shown: Option<&Scene>) {
        let Some(shown) = shown else { return self.owed.extend(self.asked.iter().cloned()) };
        let Some(before) = &mut self.known else { return };
        let now = known(shown);
        if ended_since(before, &now) {
            self.owed.extend(self.hold.keys().filter(|id| now.get(*id) == Some(&false)).cloned());
        }
        *before = now;
    }
}
