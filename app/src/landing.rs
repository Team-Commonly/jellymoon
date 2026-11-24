//! A change that was kept by itself, on the page (F92, ADR 0028; `docs/development/app.md`, "A change kept by
//! itself"). A finished change goes into the person's folder with no yes asked wherever core takes none, and
//! the wish's note is then `kept-<wish>`: core's title that the folder has the change, core's line of how many
//! files it is, and the one choice core names, which undoes it. A tap on it is carried as every tap on work is
//! (`working.rs`): core's answer is a note, and under it the work's own note is back, which waits for a yes, or
//! this note again when core says the change stays. Every word and every rule is core's. No GPUI.

use sticker_kit::Note;

use crate::answering::with_more;
use crate::today::Today;
use crate::told::told;
use crate::working::text;

/// The start of the id of the note of a change that was kept by itself, followed by the wish's id.
pub(crate) const KEPT: &str = "kept-";

impl Today {
    /// The notes of the made wish `id` whose change was kept by itself and can be undone: what its run says it
    /// did, then the note that the folder has the change, with the choice to undo it and the offer of a
    /// follow-up. None of any other wish.
    pub(crate) fn kept_notes(&self, id: &str) -> Option<Vec<Note>> {
        let kept = jellymoon_core::kept_itself(&self.home, &self.run_of(id))?;
        let note = text(format!("{KEPT}{id}"), kept.title(), kept.lines(), kept.choices(false));
        Some(told(&kept, false).into_iter().chain([with_more(note, id, self.follows(id))]).collect())
    }

    /// The notes under core's answer to a tap on the undo: the work's own note, which waits again, or the note
    /// of the kept change when core says it stays.
    pub(crate) fn after_undo(&self, wish: &str) -> Vec<Note> {
        self.work_notes(wish, false).or_else(|| self.kept_notes(wish)).unwrap_or_default()
    }
}
