//! The entry in the view: the keys and the platform's input method wired to [`EntryState`], which holds every rule.
//! Here are only the focus, the text with its caret and its composition underlined, and what takes the clicks.
//! The field and the chips themselves are flat shapes (`shapes.rs`), and the hint is laid-out text like any other.

use std::ops::Range;

use gpui::{
    AnyElement, App, Bounds, Context, CursorStyle, ElementInputHandler, Entity, EntityInputHandler, FocusHandle,
    KeyDownEvent, MouseButton, MouseDownEvent, Pixels, Point, ShapedLine, TextAlign, TextRun, UTF16Selection,
    UnderlineStyle, Window, canvas, div, fill, point, prelude::*, px, size,
};

use super::measure::{font_for, hsla};
use super::{Page, Stale};
use crate::entry::EntryState;
use crate::layout::{PieceKind, Placed, line_height, page_scale};
use crate::scene::Entry;
use crate::text::Role;
use crate::theme::Theme;

/// What was entered: the entry's id, the text with no space at either end, and the chosen option by its name,
/// never by its place: the options can change before the handler acts (`None` for an entry with no options).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EntryMade {
    pub id: String,
    pub text: String,
    pub option: Option<String>,
}

type EntryHandler = Box<dyn Fn(&EntryMade, &mut Window, &mut App)>;

#[derive(Default)]
pub(super) struct EntryView {
    pub what: Option<Entry>,
    pub state: EntryState,
    on_made: Option<EntryHandler>,
    /// The entry was taken away while it held words: it goes as soon as it holds none.
    leaving: bool,
    focus: Option<FocusHandle>,
    /// Whether the entry had the keys when the frame was built.
    pub lit: bool,
    /// The text as it was last painted, where, and how far left it was moved to keep the caret in the field. The
    /// input method asks where a character is, and a click asks which character is there.
    line: Option<ShapedLine>,
    at: Bounds<Pixels>,
    scroll: Pixels,
}

impl EntryView {
    /// Shapes what is typed, the composition underlined. Returns the line, the caret's place in it and its height.
    fn shape(
        &mut self,
        theme: &Theme,
        k: f32,
        at: Bounds<Pixels>,
        window: &Window,
    ) -> (Option<ShapedLine>, Pixels, f32) {
        let (state, font_size) = (&self.state, Role::RowMain.size(theme, k));
        let ink = hsla(super::shapes::words(&theme.palette, state.waiting()));
        let under = UnderlineStyle { thickness: px(1.0), color: Some(ink), wavy: false };
        let marked = state.marked().unwrap_or(0..0);
        let runs: Vec<TextRun> =
            [(marked.start, None), (marked.len(), Some(under)), (state.text().len() - marked.end, None)]
                .into_iter()
                .filter(|(len, _)| *len > 0)
                .map(|(len, underline)| TextRun {
                    len,
                    font: font_for(theme, Role::RowMain),
                    color: ink,
                    background_color: None,
                    underline,
                    strikethrough: None,
                })
                .collect();
        let shaped = |text: &str| window.text_system().shape_line(text.to_string().into(), px(font_size), &runs, None);
        let line = Some(state.text()).filter(|text| !text.is_empty()).map(shaped);
        let caret = line.as_ref().map_or(px(0.0), |line| line.x_for_index(state.caret()));
        // The caret stays inside the field: a line longer than it is moved left by what the caret is past its end.
        self.scroll = px((f32::from(caret) - f32::from(at.size.width) + 2.0).max(0.0));
        (self.line, self.at) = (line.clone(), at);
        (line, caret, line_height(font_size))
    }
}

/// Paints the typed line and the caret, and gives the window this entry as what the platform's text input talks to.
fn paint(at: Bounds<Pixels>, page: &Entity<Page>, focus: &FocusHandle, window: &mut Window, cx: &mut App) {
    window.handle_input(focus, ElementInputHandler::new(at, page.clone()), cx);
    let ((line, caret, height), scroll, colour, waiting) = page.update(cx, |page, _| {
        let Page { entry, theme, viewport, .. } = page;
        let shaped = entry.shape(theme, page_scale(theme, *viewport), at, window);
        (shaped, entry.scroll, hsla(theme.palette.caret), entry.state.waiting())
    });
    let origin = point(at.left() - scroll, at.top() + (at.size.height - px(height)) / 2.0);
    if let Some(line) = line {
        let _ = line.paint(origin, px(height), TextAlign::Left, None, window, cx);
    }
    if focus.is_focused(window) && !waiting {
        window.paint_quad(fill(Bounds::new(point(origin.x + caret, origin.y), size(px(1.5), px(height))), colour));
    }
}

impl Page {
    /// Puts an entry on the page, in its own spot ([`crate::layout_entry`]).
    pub fn with_entry(mut self, entry: Entry) -> Self {
        self.entry.state = EntryState::of(&entry);
        self.entry.what = Some(entry);
        self
    }

    /// Called with what was entered when Enter is pressed in the entry. The entry keeps its words and waits, taking
    /// no key, until the handler's answer comes ([`Page::answer_entry`]): a handler always answers. Like `on_tap`,
    /// it is called while the page is in use: a handler that changes the page does so in `cx.defer`.
    pub fn on_entry(mut self, handler: impl Fn(&EntryMade, &mut Window, &mut App) + 'static) -> Self {
        self.entry.on_made = Some(Box::new(handler));
        self
    }

    /// Changes or removes the entry. What is already typed stays, held to the new entry's bounds. An entry that is
    /// removed while it holds words, or waits for its answer, stays on the page as it is, with the chips it had,
    /// and goes once it is empty and does not wait: words a person typed never go with it, and a refusal always
    /// has them to leave. An entry given meanwhile takes its place, with the words.
    pub fn set_entry(&mut self, entry: Option<Entry>, cx: &mut Context<Self>) {
        match &entry {
            Some(entry) => {
                let was = self.entry.what.as_ref().map(|was| was.options.as_slice()).unwrap_or_default();
                self.entry.state.keep_option(was, &entry.options);
                self.entry.state.fit(entry);
            }
            None if self.entry.state.holds() => {
                self.entry.leaving = true;
                return;
            }
            None => self.entry.state = EntryState::default(),
        }
        (self.entry.what, self.entry.leaving) = (entry, false);
        self.stale = Stale::Spring;
        cx.notify();
    }

    /// The answer to what Enter gave: kept, and the entry is empty again; not kept, and its words stay. Either way
    /// it takes keys again.
    pub fn answer_entry(&mut self, kept: bool, window: &Window, cx: &mut Context<Self>) {
        self.entry.state.answer(kept);
        self.entry_changed(window, cx);
    }

    /// What is typed in the entry now.
    pub fn entry(&self) -> &EntryState {
        &self.entry.state
    }

    /// Whether the entry has the keys. While it does, a letter key is text for it: a window that acts on plain
    /// keys of its own (a `t` to switch theme) asks this first and leaves the key alone.
    pub fn entry_focused(&self, window: &Window) -> bool {
        self.entry.what.is_some() && self.entry.focus.as_ref().is_some_and(|focus| focus.is_focused(window))
    }

    /// Gives the entry the keys, as a click on it does. Escape gives them back.
    pub fn focus_entry(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let focus = self.entry.focus.get_or_insert_with(|| cx.focus_handle()).clone();
        window.focus(&focus, cx);
    }

    /// The entry's own elements, over everything else: the typed line in the field, and what takes the clicks on
    /// the field and on each chip. None for a page with no entry.
    pub(super) fn entry_layer(&mut self, window: &Window, cx: &mut Context<Self>) -> Vec<AnyElement> {
        if self.entry.what.is_none() {
            return Vec::new();
        }
        let focus = self.entry.focus.get_or_insert_with(|| cx.focus_handle()).clone();
        self.entry.lit = focus.is_focused(window);
        let pad = px(14.0 * page_scale(&self.theme, self.viewport));
        let ours = self.placed.iter().filter(|piece| matches!(piece.piece, PieceKind::Entry | PieceKind::Chip));
        ours.map(|piece| {
            let (rect, index) = (piece.rect, piece.index);
            let at = div().absolute().left(px(rect.x)).top(px(rect.y)).w(px(rect.w)).h(px(rect.h));
            if piece.piece == PieceKind::Chip {
                let chosen = cx.listener(move |page, _, window, cx| {
                    page.entry.state.choose(index);
                    page.focus_entry(window, cx);
                    cx.notify();
                });
                return at.id(("chip", index)).cursor_pointer().on_click(chosen).into_any_element();
            }
            let (page, focus) = (cx.entity(), focus.clone());
            at.id("entry")
                .debug_selector(|| "Entry".into())
                .track_focus(&focus)
                .cursor(CursorStyle::IBeam)
                .overflow_hidden()
                .px(pad)
                .on_mouse_down(MouseButton::Left, cx.listener(Self::entry_pressed))
                .on_key_down(cx.listener(Self::entry_key))
                .child(canvas(|_, _, _| (), move |at, (), window, cx| paint(at, &page, &focus, window, cx)).size_full())
                .into_any_element()
        })
        .collect()
    }

    /// A press anywhere else on the page gives the keys back.
    pub(super) fn entry_left(&mut self, event: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let (x, y) = (f32::from(event.position.x), f32::from(event.position.y));
        let on = |piece: &Placed| {
            let r = piece.rect;
            matches!(piece.piece, PieceKind::Entry | PieceKind::Chip)
                && r.x <= x
                && x < r.right()
                && r.y <= y
                && y < r.bottom()
        };
        if self.entry_focused(window) && !self.placed.iter().any(on) {
            window.blur(cx);
        }
    }

    /// The press itself gives the field the keys: GPUI does that for an element that tracks a focus handle.
    fn entry_pressed(&mut self, event: &MouseDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        let view = &mut self.entry;
        if let Some(line) = &view.line {
            view.state.set_caret(line.closest_index_for_x(event.position.x - view.at.left() + view.scroll));
        }
        cx.notify();
    }

    /// The keys that are not text. Every other key is left to go on, to the platform's text input and from there
    /// to [`EntityInputHandler::replace_text_in_range`]. Tab is the key that changes the option because no other
    /// piece of a page takes the keys, so there is nowhere for it to move on to.
    fn entry_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let (key, held) = (event.keystroke.key.as_str(), event.keystroke.modifiers);
        let command = held.secondary(cx);
        let state = &mut self.entry.state;
        // An entry that waits takes no key at all: Escape too, so the keys stay the entry's and none is the window's.
        if state.waiting() {
            return;
        }
        match key {
            "backspace" => state.backspace(),
            "delete" => state.delete(),
            "home" => state.home(),
            "end" => state.end(),
            "left" if command => state.home(),
            "right" if command => state.end(),
            "left" => state.left(),
            "right" => state.right(),
            "tab" if held.shift => state.previous_option(),
            "tab" => state.next_option(),
            "enter" => self.entry_entered(window, cx),
            "escape" => window.blur(cx),
            "v" if command => {
                let pasted = cx.read_from_clipboard().and_then(|item| item.text());
                state.replace(None, &pasted.unwrap_or_default());
            }
            _ => return,
        }
        cx.stop_propagation();
        self.entry_changed(window, cx);
    }

    fn entry_entered(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(made) = self.entry.state.enter() else {
            return;
        };
        cx.notify();
        match (&self.entry.on_made, &self.entry.what) {
            (Some(handler), Some(entry)) => {
                let option = made.option.and_then(|at| entry.options.get(at).cloned());
                handler(&EntryMade { id: entry.id.clone(), text: made.text, option }, window, cx);
            }
            // Nobody was given the words, so nobody will answer: they are taken as kept.
            _ => self.entry.state.answer(true),
        }
    }

    /// Tells the platform's text input that the text or the caret is no longer what it last saw, and draws again.
    /// An entry that was taken away while it held words goes here, once it holds none.
    fn entry_changed(&mut self, window: &Window, cx: &mut Context<Self>) {
        if self.entry.leaving && !self.entry.state.holds() {
            (self.entry.what, self.entry.state, self.entry.leaving) = (None, EntryState::default(), false);
            self.stale = Stale::Spring;
        }
        if let Some(focus) = &self.entry.focus {
            window.notify_text_input_changed(focus);
        }
        cx.notify();
    }
}

/// What the platform's text input asks of the entry and does to it. It counts in UTF-16; [`EntryState`] converts.
impl EntityInputHandler for Page {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        adjusted: &mut Option<Range<usize>>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<String> {
        let state = &self.entry.state;
        let range = state.from_utf16(range_utf16);
        adjusted.replace(state.to_utf16(range.clone()));
        Some(state.text()[range].to_string())
    }

    fn selected_text_range(&mut self, _: bool, _: &mut Window, _: &mut Context<Self>) -> Option<UTF16Selection> {
        let state = &self.entry.state;
        Some(UTF16Selection { range: state.to_utf16(state.caret()..state.caret()), reversed: false })
    }

    fn marked_text_range(&self, _: &mut Window, _: &mut Context<Self>) -> Option<Range<usize>> {
        self.entry.state.marked().map(|marked| self.entry.state.to_utf16(marked))
    }

    fn unmark_text(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.entry.state.unmark();
        self.entry_changed(window, cx);
    }

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.entry.state.replace(range_utf16, text);
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
        self.entry.state.mark(range_utf16, new_text, new_selected_range_utf16);
        self.entry_changed(window, cx);
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        field: Bounds<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let view = &self.entry;
        let range = view.state.from_utf16(range_utf16);
        let x =
            |at: usize| field.left() - view.scroll + view.line.as_ref().map_or(px(0.0), |line| line.x_for_index(at));
        Some(Bounds::from_corners(point(x(range.start), field.top()), point(x(range.end), field.bottom())))
    }

    fn character_index_for_point(&mut self, at: Point<Pixels>, _: &mut Window, _: &mut Context<Self>) -> Option<usize> {
        let view = &self.entry;
        let index = view.line.as_ref()?.closest_index_for_x(at.x - view.at.left() + view.scroll);
        Some(view.state.to_utf16(index..index).start)
    }

    fn set_selected_text_range(&mut self, range_utf16: Range<usize>, window: &mut Window, cx: &mut Context<Self>) {
        let to = self.entry.state.from_utf16(range_utf16).end;
        self.entry.state.set_caret(to);
        self.entry_changed(window, cx);
    }

    fn text_length_utf16(&mut self, _: &mut Window, _: &mut Context<Self>) -> Option<usize> {
        Some(self.entry.state.len_utf16())
    }

    /// Not while the entry waits for its answer: no composition can begin then.
    fn accepts_text_input(&self, _: &mut Window, _: &mut Context<Self>) -> bool {
        self.entry.what.is_some() && !self.entry.state.waiting()
    }
}
