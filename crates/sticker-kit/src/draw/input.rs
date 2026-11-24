//! What the platform's text input asks of a page and does to it: of the entry, or of the field of a note when
//! that one has the keys ([`Page::keyed`]). The rules are [`EntryState`]'s; here its answers are only passed on.

use std::ops::Range;

use gpui::{Bounds, Context, EntityInputHandler, Pixels, Point, UTF16Selection, Window, point, px};

use super::Page;
use crate::entry::EntryState;

impl Page {
    /// What is typed where the keys are.
    fn typed_in(&mut self, window: &Window) -> &mut EntryState {
        let which = self.keyed(window);
        &mut self.view_mut(which).state
    }
}

/// It counts in UTF-16; [`EntryState`] converts.
impl EntityInputHandler for Page {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        adjusted: &mut Option<Range<usize>>,
        window: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<String> {
        let state = &self.view(self.keyed(window)).state;
        let range = state.from_utf16(range_utf16);
        adjusted.replace(state.to_utf16(range.clone()));
        Some(state.text()[range].to_string())
    }

    fn selected_text_range(&mut self, _: bool, window: &mut Window, _: &mut Context<Self>) -> Option<UTF16Selection> {
        let state = &self.view(self.keyed(window)).state;
        Some(UTF16Selection { range: state.to_utf16(state.caret()..state.caret()), reversed: false })
    }

    fn marked_text_range(&self, window: &mut Window, _: &mut Context<Self>) -> Option<Range<usize>> {
        let state = &self.view(self.keyed(window)).state;
        state.marked().map(|marked| state.to_utf16(marked))
    }

    fn unmark_text(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.typed_in(window).unmark();
        self.entry_changed(window, cx);
    }

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.typed_in(window).replace(range_utf16, text);
        self.entry_changed(window, cx);
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        new_selected_range_utf16: Option<Range<usize>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.typed_in(window).mark(range_utf16, new_text, new_selected_range_utf16);
        self.entry_changed(window, cx);
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        field: Bounds<Pixels>,
        window: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let view = self.view(self.keyed(window));
        let range = view.state.from_utf16(range_utf16);
        let x =
            |at: usize| field.left() - view.scroll + view.line.as_ref().map_or(px(0.0), |line| line.x_for_index(at));
        Some(Bounds::from_corners(point(x(range.start), field.top()), point(x(range.end), field.bottom())))
    }

    fn character_index_for_point(
        &mut self,
        at: Point<Pixels>,
        window: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<usize> {
        let view = self.view(self.keyed(window));
        let index = view.line.as_ref()?.closest_index_for_x(at.x - view.at.left() + view.scroll);
        Some(view.state.to_utf16(index..index).start)
    }

    fn set_selected_text_range(&mut self, range_utf16: Range<usize>, window: &mut Window, cx: &mut Context<Self>) {
        let state = self.typed_in(window);
        state.set_caret(state.from_utf16(range_utf16).end);
        self.entry_changed(window, cx);
    }

    fn text_length_utf16(&mut self, window: &mut Window, _: &mut Context<Self>) -> Option<usize> {
        Some(self.view(self.keyed(window)).state.len_utf16())
    }

    /// While an answer is owed too: the text and every range here are of what is typed, never of the words kept
    /// aside, so a composition begins after Enter as at any other time.
    fn accepts_text_input(&self, window: &mut Window, _: &mut Context<Self>) -> bool {
        self.view(self.keyed(window)).what.is_some()
    }
}
