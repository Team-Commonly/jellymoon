//! A wish's work, drawn (ADR 0029; `docs/development/app.md`, "A wish at work" and "Work that waits"). A run at
//! work is a track (`live.rs` makes its note) and work that waits is its sheets ([`sheets`]): notes of the kit's
//! `Note::Drawn`, whose sentences are not on the page until the person taps the picture or its `?`. Which drawn
//! notes are open is the window's own state ([`Open`], held by `showing::Showing`): no call to core, and nothing
//! written.
//!
//! One pure pass ([`pass`]) says how a scene is drawn with that state, and [`column`] is the one place it is
//! applied, just before the scene is put in its column's order: every scene the window shows goes through
//! `Showing::column`, and `layout`, which has no tap, prints every drawn note open ([`every`]). While the drawn
//! note of a wish's work is closed, what its run says it did (`told-<wish>`) is not on the page either: a
//! finished wish is first one picture with its choices. No word is lost: a tap brings every sentence, and the
//! told note; a second folds them away. A choice means what it meant: its words and its place are core's.
//!
//! The same pass leaves out the notes of the made wishes that are tucked away (`drawing/tucked.rs`, ADR 0030,
//! decision 11): which those are is held with what is open, and `layout` tucks none away.

use std::cell::{Cell, RefCell};
use std::collections::BTreeSet;
use std::rc::Rc;

use jellymoon_core::{PageData, Waits};
use sticker_kit::{Field, Kept, Note, Picture, Scene};

use crate::again::{Again, Remade};
use crate::landing::KEPT;
pub use crate::live::tracked;
use crate::told::{TOLD, rows_shown};
use crate::working::{FILE, WORK};

mod ended;
mod folded;
mod tucked;
pub use folded::FOLD;
use tucked::Tucked;

/// The note of work that waits, drawn: a sheet for each file core counted, in a copy still, with core's choices
/// in core's order. Its words are core's title and then `lines`, core's lines of the work, each a sentence of its
/// own. The counts are core's: nothing is counted here. Work core says did not fit the folder is drawn so.
pub(crate) fn sheets(waits: &Waits, lines: Vec<String>) -> Note {
    let (made, changed, removed) = (waits.made, waits.changed, waits.removed);
    let state = if waits.unfit() { Kept::Unfit } else { Kept::Waiting };
    Note::Drawn {
        id: format!("{WORK}{}", waits.wish),
        picture: Picture::Sheets { made, changed, removed, state },
        words: [waits.title().to_string()].into_iter().chain(lines).collect(),
        open: false,
        options: waits.choices(false).into_iter().map(str::to_string).collect(),
        field: None,
    }
}

/// What a tap on a choice of a note is made of: the note's texts under its title, its options and its field. A
/// drawn note's first word is its title, as [`sheets`] makes it. None of a note that has no choice.
pub(crate) fn held(note: &Note) -> Option<(&[String], &[String], Option<&Field>)> {
    match note {
        Note::Text { body, options, field, .. } => Some((body, options, field.as_ref())),
        Note::Drawn { words, options, field, .. } => {
            Some((words.get(1..).unwrap_or_default(), options, field.as_ref()))
        }
        _ => None,
    }
}

/// The wish whose finished work the drawn note `id` is: work that waits, or a change kept by itself.
fn wish_of(id: &str) -> Option<&str> {
    id.strip_prefix(WORK).or_else(|| id.strip_prefix(KEPT))
}

fn drawn_ids(scene: &Scene) -> impl Iterator<Item = &str> {
    scene.notes.iter().filter_map(|note| match note {
        Note::Drawn { id, .. } => Some(id.as_str()),
        _ => None,
    })
}

/// The ids of every drawn note of a scene: what `layout` prints open, as it has no tap.
pub fn every(scene: &Scene) -> BTreeSet<String> {
    drawn_ids(scene).map(str::to_string).collect()
}

/// The scene as it is drawn while the drawn notes whose ids are in `open` are open: each drawn note open or
/// closed by that, and no `told-<wish>` while the drawn `work-<wish>`, or the drawn `kept-<wish>` of a change kept
/// by itself, is on the scene and closed. Where a wish's work is not a drawn note (a question, a keep asked once
/// more, a kept change the folder no longer shows) its told note stays. The offer of a follow-up is under the
/// newest made wish of which one may be made, as on a page nobody tapped ([`passed`]).
pub fn pass(scene: &Scene, open: &BTreeSet<String>) -> Scene {
    passed(scene, open, None, &BTreeSet::new())
}

/// [`pass`] in a window where the made wish `tapped` was tapped last: the offer of a follow-up is under that
/// wish, while one may be made of it, and under no other (`answering::offered`, ADR 0030). The settled wishes of
/// `away` are tucked away: none of their notes is drawn (`tucked::shown`), so none of them has the offer.
fn passed(scene: &Scene, open: &BTreeSet<String>, tapped: Option<&str>, away: &BTreeSet<String>) -> Scene {
    let closed: Vec<&str> = drawn_ids(scene).filter(|id| !open.contains(*id)).filter_map(wish_of).collect();
    let folded = |note: &Note| match note {
        Note::Told { id, .. } => id.strip_prefix(TOLD).is_some_and(|wish| closed.contains(&wish)),
        _ => false,
    };
    // While a wish is being read, or a sharper wish waits, the note that nothing was wished for is not drawn.
    let taken = crate::shaping::taken(scene);
    let unsaid = |note: &Note| folded(note) || taken && crate::shaping::nothing(note);
    let mut notes: Vec<Note> = tucked::shown(scene, away).filter(|note| !unsaid(note)).cloned().collect();
    for note in &mut notes {
        if let Note::Drawn { id, open: is, .. } = note {
            *is = open.contains(id);
        }
    }
    let mut drawn = Scene { notes, ..scene.clone() };
    crate::answering::one_offer(&mut drawn, tapped);
    drawn
}

/// [`pass`], and then the scene in the order of its column: the one place a scene is made ready to be drawn.
pub fn column(scene: &Scene, open: &BTreeSet<String>) -> Scene {
    crate::column::column(&pass(scene, open))
}

/// The column of `scene` as a window that opens on it first draws it ([`Open::opening`]): no note open, and
/// every settled made wish but the newest tucked away, each with its mark. For a picture, which has no window.
pub fn first(scene: &Scene) -> Scene {
    Open::opening(scene).column(scene)
}

/// `column`, a scene [`first`] or [`column`] gave, with the day of the page data it was made from folded, or
/// `open` under its tally (`drawing/folded.rs`). For a picture, which has no window.
pub fn fold(column: Scene, data: &PageData, open: bool) -> Scene {
    folded::folded(column, Some(data), open)
}

/// The drawn notes a person opened in one window, by their ids, the made wish they tapped last, and the made
/// wishes that are tucked away. An id that leaves the page leaves the set. And core's answers that a wish was
/// made again, each with the wish it speaks of (`again.rs`). And whether the
/// day is open under its tally: a window starts with it folded (`drawing/folded.rs`).
#[derive(Clone, Default)]
pub struct Open(Rc<RefCell<BTreeSet<String>>>, Rc<RefCell<Option<String>>>, Tucked, Again, Rc<Cell<bool>>);

impl Open {
    /// What a window that opens on `scene` starts with: no note open, and every settled made wish tucked away
    /// but the newest, so a day's page opens on the last wish's result and on what still waits for the person.
    pub(crate) fn opening(scene: &Scene) -> Self {
        let (open, made) = (Open::default(), crate::placing::made(scene));
        open.2.tuck(scene, made.last().map(|one| one.id.as_str()), false, made.len());
        open
    }

    /// A new made wish was kept, and `scene` is on show: every settled made wish tucks away (`Tucked::tuck`),
    /// but `but`, the wish core's answer is under, when the new wish was made from one. `earlier` made wishes
    /// stand before the new one, which `scene` may not hold yet.
    pub(crate) fn made(&self, scene: &Scene, but: Option<&str>, earlier: usize) {
        self.2.tuck(scene, but, true, earlier);
    }

    /// [`column`] of `scene` with what is open now, the offer of a follow-up under the wish tapped last, and
    /// none of the notes of a wish that is tucked away. A note that is not on `scene`, or is tucked away, is open
    /// no more; a look, which gives the notes it found again, closes nothing. An answer that a wish was made
    /// again says that wish is being worked on only while it is (`Again::drawn`). Under the bubble of a wish
    /// that is tucked away stands the mark of how it ended (`drawing/ended.rs`), and under no other.
    pub(crate) fn column(&self, scene: &Scene) -> Scene {
        let scene = &self.3.drawn(scene);
        let away = self.2.away(scene);
        let hidden = |id: &&str| wish_of(id).is_some_and(|wish| away.contains(wish));
        let mut open = self.0.borrow_mut();
        let on: Vec<&str> = drawn_ids(scene).filter(|id| !hidden(id)).collect();
        open.retain(|id| on.contains(&id.as_str()));
        let column = crate::column::column(&passed(scene, &open, self.1.borrow().as_deref(), &away));
        ended::marked(column, scene, &away)
    }

    /// [`Open::column`] of the page made from `data`, with what the reading put on it folded into its tally, or
    /// open under the tally once the tally was tapped (`drawing/folded.rs`). A scene with no page data, and a
    /// day the reading has no wish on, are the column as it is.
    pub(crate) fn day(&self, scene: &Scene, data: Option<&PageData>) -> Scene {
        folded::folded(self.column(scene), data, self.4.get())
    }

    /// A tap on the picture of the note `note`: when that is the tally, the day opens, or folds when it was
    /// open. Whether it was the tally.
    pub(crate) fn fold(&self, note: &str) -> bool {
        if note == FOLD {
            self.4.set(!self.4.get());
        }
        note == FOLD
    }

    /// Core answered a tap on a choice of the made wish `wish` that a wish was made again and is being worked
    /// on: the answer is kept with that wish. When the old work was tossed too, `wish` steps aside: it is
    /// tucked away at once (`Tucked::aside`), though the answer is under it and the new wish is the newest.
    pub(crate) fn remade(&self, wish: &str, remade: Option<Box<Remade>>) {
        if remade.as_ref().is_some_and(|one| one.tossed) {
            self.2.aside(wish);
        }
        self.3.keep(remade);
    }

    /// The answer on `scene` under the made wish `wish` that says a wish was made again (`Again::under`).
    pub(crate) fn answer(&self, scene: &Scene, wish: &str) -> Option<Note> {
        self.3.under(scene, wish)
    }

    /// A tap on a note of the made wish `wish`, on its picture, or on its little one or its typed words: the
    /// offer of a follow-up goes under that wish.
    pub(crate) fn offer(&self, wish: String) {
        *self.1.borrow_mut() = Some(wish);
    }

    /// A tap on the little one of the made wish `wish`, or on its typed words: the offer goes under it
    /// ([`Open::offer`]), and the drawn note of its finished work on `scene` is opened, or closed when it is open.
    /// Of a wish that was tucked away the tap brings the notes back, as they first come, and that is all it
    /// does here: whether it did.
    pub(crate) fn poked(&self, scene: &Scene, wish: &str) -> bool {
        self.offer(wish.to_string());
        if self.2.back(wish) {
            return true;
        }
        let finished: Vec<String> =
            drawn_ids(scene).filter(|id| wish_of(id) == Some(wish)).map(str::to_string).collect();
        for note in finished {
            self.tapped(scene, &note);
        }
        false
    }

    /// A tap on the picture of the note `note`, or on its `?`: open, it is closed, and closed, it is opened.
    /// Whether `scene` holds that drawn note; of any other id nothing changes.
    pub(crate) fn tapped(&self, scene: &Scene, note: &str) -> bool {
        let on = drawn_ids(scene).any(|id| id == note);
        let mut open = self.0.borrow_mut();
        if on && !open.remove(note) {
            open.insert(note.to_string());
        }
        on
    }

    /// `scene`, core's answer to a tap on a choice of work, given back as it is. Where it holds what `Show me`
    /// brings (a file of a wish's work, or the rows of what its run says it changed) that wish's drawn note is
    /// opened: the person asked to be shown, and closed, the rows would not be on the page.
    pub(crate) fn showed(&self, scene: Scene) -> Scene {
        let brought = |note: &Note, wish: &str| match note {
            Note::Text { id, .. } => {
                let of = id.strip_prefix(FILE).and_then(|rest| rest.split_once('-'));
                of.is_some_and(|(_, of)| of == wish)
            }
            Note::Told { id, .. } => id.strip_prefix(TOLD) == Some(wish) && rows_shown(note),
            _ => false,
        };
        let shown =
            |id: &&str| id.strip_prefix(WORK).is_some_and(|wish| scene.notes.iter().any(|one| brought(one, wish)));
        let opened: Vec<String> = drawn_ids(&scene).filter(shown).map(str::to_string).collect();
        self.0.borrow_mut().extend(opened);
        scene
    }
}

#[cfg(test)]
mod tests;
