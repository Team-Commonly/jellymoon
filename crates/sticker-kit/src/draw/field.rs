//! The field of a text note, or of a drawn one, in the view: one line a person types in under it, held and typed
//! in as the entry is (`entry.rs`), by the same code. A page has one for each note of its scene with a field, by
//! the id of the note it is under now. What was typed stays while a note of the scene has the field, whatever
//! else of the scene changes. A field is the same field while its key, or with no key its note's id, and its
//! offer are the same: an offer that changes is another field, and the words typed for the first are not
//! words for it.
//!
//! A field no note has any more goes, with two exceptions that are kept apart, not drawn and with no keys: one
//! that waits for its answer, so a refusal has the words to leave when the note has its field again, and one
//! with a key that holds words, so a note that takes another's place for a while costs nobody what they typed.

use gpui::{Context, Window};

use super::Page;
use super::entry::EntryView;
use crate::entry::EntryState;
use crate::scene::{Entry, Field, Note, Scene};

/// Which of a page's lines is meant: the entry, or the field of a note by its place among the fields.
pub(super) type Which = Option<usize>;

/// The most fields that are kept apart from a scene at once. The ones that wait are kept first.
pub const MAX_KEPT_FIELDS: usize = 8;

/// The fields of a scene's notes, in the notes' order, and the fields kept apart from it. One that `was` there,
/// on the scene or apart, keeps what was typed in it, held to the most its note now says.
pub(super) fn fit(scene: &Scene, mut was: Vec<EntryView>) -> (Vec<EntryView>, Vec<EntryView>) {
    let told_by = |id: &str, key: &str| if key.is_empty() { id.to_string() } else { key.to_string() };
    let same = |view: &EntryView, id: &str, field: &Field| {
        let told = view.what.as_ref().map(|what| told_by(&what.id, &view.key));
        view.offer == field.offer && told == Some(told_by(id, &field.key))
    };
    let mut held = Vec::new();
    for note in &scene.notes {
        let (Note::Text { id, field: Some(field), .. } | Note::Drawn { id, field: Some(field), .. }) = note else {
            continue;
        };
        let entry = Entry {
            id: id.clone(),
            hint: field.hint.clone(),
            options: Vec::new(),
            most: field.most,
            action: String::new(),
            go: String::new(),
            loud: false,
        };
        let at = was.iter().position(|view| same(view, id, field));
        let mut view = at.map(|at| was.remove(at)).unwrap_or_default();
        view.state.fit(&entry);
        (view.what, view.key, view.offer) = (Some(entry), field.key.clone(), field.offer.clone());
        held.push(view);
    }
    was.retain(|view| view.state.waiting() || !view.key.is_empty() && view.state.holds());
    was.sort_by_key(|view| !view.state.waiting());
    was.truncate(MAX_KEPT_FIELDS);
    (held, was)
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

    /// The answer to what Enter gave: kept, and the words kept aside are forgotten; not kept, and they come back
    /// in front of what was typed since ([`EntryState::answer`]). Kept, an Enter pressed while the answer was
    /// owed is given now as one pressed now is, to the same handler ([`EntryState::answer_and_enter`]), unless
    /// [`Page::forget_enter`] was called first.
    pub fn answer_entry(&mut self, kept: bool, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(made) = self.entry.state.answer_and_enter(kept) {
            self.entry_made(None, made, window, cx);
        }
        self.entry_changed(window, cx);
    }

    /// Forgets the Enter the entry remembered while its answer is owed: for a handler whose answer leaves the
    /// page where an Enter nobody is pressing now should not act. The words typed since stay.
    pub fn forget_enter(&mut self) {
        self.entry.state.forget_enter();
    }

    /// As [`Page::answer_entry`], for a page whose window was closed since Enter: there is no text input of the
    /// platform's to tell, and the page that is shown again does not wait. An Enter the entry remembered is
    /// forgotten: a handler is called in a window.
    pub fn answer_entry_closed(&mut self, kept: bool, cx: &mut Context<Self>) {
        self.entry.state.answer(kept);
        self.entry_settled(cx);
    }

    /// The answer to what Enter gave with the note's id `id`, as [`Page::answer_entry`] is the entry's, by the
    /// same rule for kept and not kept. The field is found by that id wherever it is now: under its note, under a
    /// note that took its place, or kept apart. Nothing when none waits so. A field is given no Enter it
    /// remembered: one pressed while its answer was owed is forgotten here, as it always was.
    pub fn answer_field(&mut self, id: &str, kept: bool, window: &Window, cx: &mut Context<Self>) {
        self.field_answered(id, kept);
        self.entry_changed(window, cx);
    }

    /// As [`Page::answer_field`], for a page whose window was closed since Enter: there is no text input of the
    /// platform's to tell, and the page that is shown again has a field that does not wait.
    pub fn answer_field_closed(&mut self, id: &str, kept: bool, cx: &mut Context<Self>) {
        self.field_answered(id, kept);
        self.entry_settled(cx);
    }

    fn field_answered(&mut self, id: &str, kept: bool) {
        if let Some(view) = self.fields.iter_mut().chain(&mut self.apart).find(|view| view.asked(id)) {
            view.state.answer(kept);
        }
    }
}
