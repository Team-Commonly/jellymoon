//! A change that was kept by itself, on the page (F92, ADR 0028; `docs/development/app.md`, "A change kept by
//! itself"). A finished change goes into the person's folder with no yes asked wherever core takes none, and
//! the wish's note is then `kept-<wish>`: core's title that the folder has the change, core's line of how many
//! files it is, and the one choice core names, which undoes it. A tap on it is carried as every tap on work is
//! (`working.rs`): core's answer is a note, and under it the work's own note is back, which waits for a yes, or
//! this note again when core says the change stays. Every word and every rule is core's. No GPUI.

use jellymoon_core::{KeptItself, Stands};
use sticker_kit::{Kept, Note, Picture};

use crate::answering::with_more;
use crate::today::Today;
use crate::told::told;
use crate::working::text;

/// The start of the id of the note of a change that was kept by itself, followed by the wish's id.
pub(crate) const KEPT: &str = "kept-";

/// The note of a change that was kept by itself: core's title, core's lines and the choice core names. It is
/// drawn, its sheets in the folder (ADR 0029), only while core says that the folder has the change and its files
/// were read: where the folder stands somewhere else by now, a picture of the folder would say more than core
/// does, so the note stays sentences. The counts are core's.
pub(crate) fn kept_note(kept: &KeptItself) -> Note {
    let (waits, id) = (&kept.waits, format!("{KEPT}{}", kept.waits.wish));
    if !waits.read || kept.stands == Stands::Elsewhere {
        return text(id, kept.title(), kept.lines(), kept.choices());
    }
    let (made, changed, removed) = (waits.made, waits.changed, waits.removed);
    Note::Drawn {
        id,
        picture: Picture::Sheets { made, changed, removed, state: Kept::In },
        words: [kept.title().to_string()].into_iter().chain(kept.lines()).collect(),
        open: false,
        options: kept.choices().into_iter().map(str::to_string).collect(),
        field: None,
    }
}

impl Today {
    /// The notes of the made wish `id` whose change was kept by itself and can be undone: what its run says it
    /// did, then the note that the folder has the change, with the choice to undo it and the offer of a
    /// follow-up. None of any other wish.
    pub(crate) fn kept_notes(&self, id: &str) -> Option<Vec<Note>> {
        let kept = jellymoon_core::kept_itself(&self.home, &self.run_of(id))?;
        let note = with_more(kept_note(&kept), id, self.follows(id));
        Some(told(&kept.waits, false).into_iter().chain([note]).collect())
    }

    /// The notes under core's answer to a tap on the undo: the work's own note, which waits again, or the note
    /// of the kept change when core says it stays.
    pub(crate) fn after_undo(&self, wish: &str) -> Vec<Note> {
        self.work_notes(wish, false).or_else(|| self.kept_notes(wish)).unwrap_or_default()
    }
}
