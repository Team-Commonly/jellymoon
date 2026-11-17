//! The field of a text note in the view: one line a person types in under the note, held and typed in as the
//! entry is (`entry.rs`), by the same code. A page has one for each note of its scene that has a field, found by
//! the note's id. What was typed stays while the note keeps its field, whatever else of the scene changes; a field
//! goes with its note, and with the note's field.

use gpui::{Context, Window};

use super::Page;
use super::entry::EntryView;
use crate::entry::EntryState;
use crate::scene::{Entry, Note, Scene};

/// Which of a page's lines is meant: the entry, or the field of a note by its place among the fields.
pub(super) type Which = Option<usize>;

/// The fields of a scene's notes, in the notes' order. One that `was` there keeps what was typed in it, held to
/// the most its note now says.
pub(super) fn fit(scene: &Scene, mut was: Vec<EntryView>) -> Vec<EntryView> {
    let wanted = scene.notes.iter().filter_map(|note| match note {
        Note::Text { id, field: Some(field), .. } => {
            Some(Entry { id: id.clone(), hint: field.hint.clone(), options: Vec::new(), most: field.most })
        }
        _ => None,
    });
    let held = wanted.map(|entry| {
        let at = was.iter().position(|view| view.names(&entry.id));
        let mut view = at.map(|at| was.swap_remove(at)).unwrap_or_default();
        view.state.fit(&entry);
        view.what = Some(entry);
        view
    });
    held.collect()
}

impl Page {
    /// Which line has the keys: the field of a note, and else the entry.
    pub(super) fn keyed(&self, window: &Window) -> Which {
        self.fields.iter().position(|view| view.focused(window))
    }

    pub(super) fn view(&self, which: Which) -> &EntryView {
        which.and_then(|at| self.fields.get(at)).unwrap_or(&self.entry)
    }

    pub(super) fn view_mut(&mut self, which: Which) -> &mut EntryView {
        let Page { entry, fields, .. } = self;
        which.and_then(|at| fields.get_mut(at)).unwrap_or(entry)
    }

    /// What is typed in the field of the note `id` now. None for a note with no field.
    pub fn field(&self, id: &str) -> Option<&EntryState> {
        self.fields.iter().find(|view| view.names(id)).map(|view| &view.state)
    }

    /// The answer to what Enter gave in the field of the note `id`, as [`Page::answer_entry`] is the entry's:
    /// kept, and the field is empty again; not kept, and its words stay. Nothing for a note whose field is gone.
    pub fn answer_field(&mut self, id: &str, kept: bool, window: &Window, cx: &mut Context<Self>) {
        if let Some(view) = self.fields.iter_mut().find(|view| view.names(id)) {
            view.state.answer(kept);
        }
        self.entry_changed(window, cx);
    }

    /// As [`Page::answer_field`], for a page whose window was closed since Enter: there is no text input of the
    /// platform's to tell, and the page that is shown again has a field that does not wait.
    pub fn answer_field_closed(&mut self, id: &str, kept: bool, cx: &mut Context<Self>) {
        if let Some(view) = self.fields.iter_mut().find(|view| view.names(id)) {
            view.state.answer(kept);
        }
        self.entry_settled(cx);
    }
}
