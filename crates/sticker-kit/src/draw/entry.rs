//! The entry in the view: the keys and the platform's input method wired to [`EntryState`], which holds every rule.
//! Here are only the focus, the text with its caret and its composition underlined, and what takes the clicks.
//! The field and the chips themselves are flat shapes (`shapes.rs`), and the hint is laid-out text like any other.
//! The field of a note (`field.rs`) is typed in the same way, by the same code: every method here that takes a
//! [`Which`] is the entry's and a field's alike.

use gpui::{
    AnyElement, App, Bounds, Context, CursorStyle, ElementId, ElementInputHandler, Entity, FocusHandle, KeyDownEvent,
    MouseButton, MouseDownEvent, Pixels, ShapedLine, TextAlign, TextRun, UnderlineStyle, Window, canvas, div, fill,
    point, prelude::*, px, size,
};

use super::field::Which;
use super::measure::{font_for, hsla};
use super::{Page, Stale, Tap};
use crate::entry::{EntryState, MAX_ENTRY_OPTIONS, Made};
use crate::geom::Rect;
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
    pub focus: Option<FocusHandle>,
    /// Whether the entry had the keys when the frame was built.
    pub lit: bool,
    /// The text as it was last painted, where, and how far left it was moved to keep the caret in the field. The
    /// input method asks where a character is, and a click asks which character is there.
    pub line: Option<ShapedLine>,
    pub at: Bounds<Pixels>,
    pub scroll: Pixels,
    /// Of a note's field: its key and its offer, which it is told by when the scene changes (`field.rs`), and
    /// the note's id Enter last gave its words with, which the answer names whatever note has the field then.
    pub key: String,
    pub offer: String,
    pub asked_as: String,
}

impl EntryView {
    pub fn focused(&self, window: &Window) -> bool {
        self.focus.as_ref().is_some_and(|focus| focus.is_focused(window))
    }

    /// Whether this is the field of the note `id`.
    pub fn names(&self, id: &str) -> bool {
        self.what.as_ref().is_some_and(|what| what.id == id)
    }

    /// Whether this field waits for the answer to what Enter gave with the note's id `id`.
    pub fn asked(&self, id: &str) -> bool {
        self.state.waiting() && self.asked_as == id
    }

    /// Shapes what is typed, the composition underlined: or, while an answer is owed and nothing was typed since,
    /// the words kept aside, pale as the hint is, with the caret where an entry that shows its hint has it. Those
    /// are only painted: the line kept for a click and for the input method is of the typed text alone.
    /// Returns the line, the caret's place in it and its height.
    fn shape(
        &mut self,
        theme: &Theme,
        k: f32,
        at: Bounds<Pixels>,
        window: &Window,
    ) -> (Option<ShapedLine>, Pixels, f32) {
        let (state, font_size) = (&self.state, Role::RowMain.size(theme, k));
        let aside = state.owed().filter(|_| state.text().is_empty());
        let (shown, typed) = (aside.unwrap_or(state.text()), aside.is_none());
        let ink = hsla(super::shapes::words(&theme.palette, !typed));
        let under = UnderlineStyle { thickness: px(1.0), color: Some(ink), wavy: false };
        let marked = state.marked().unwrap_or(0..0);
        let runs: Vec<TextRun> = [(marked.start, None), (marked.len(), Some(under)), (shown.len() - marked.end, None)]
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
        let line = Some(shown).filter(|text| !text.is_empty()).map(shaped);
        let caret = line.as_ref().filter(|_| typed).map_or(px(0.0), |line| line.x_for_index(state.caret()));
        // The caret stays inside the field: a line longer than it is moved left by what the caret is past its end.
        self.scroll = px((f32::from(caret) - f32::from(at.size.width) + 2.0).max(0.0));
        (self.line, self.at) = (line.clone().filter(|_| typed), at);
        (line, caret, line_height(font_size))
    }
}

/// Paints the typed line and the caret, and gives the window this page as what the platform's text input talks
/// to: the window takes it from the one that has the keys.
fn paint(
    at: Bounds<Pixels>,
    (page, which): (&Entity<Page>, Which),
    focus: &FocusHandle,
    window: &mut Window,
    cx: &mut App,
) {
    window.handle_input(focus, ElementInputHandler::new(at, page.clone()), cx);
    let ((line, caret, height), scroll, colour) = page.update(cx, |page, _| {
        let Page { entry, fields, theme, viewport, .. } = page;
        let view = which.and_then(|at| fields.get_mut(at)).unwrap_or(entry);
        let shaped = view.shape(theme, page_scale(theme, *viewport), at, window);
        (shaped, view.scroll, hsla(theme.palette.caret))
    });
    let origin = point(at.left() - scroll, at.top() + (at.size.height - px(height)) / 2.0);
    if let Some(line) = line {
        let _ = line.paint(origin, px(height), TextAlign::Left, None, window, cx);
    }
    if focus.is_focused(window) {
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

    /// Called with what was entered when Enter is pressed in the entry. The entry keeps the words aside, and takes
    /// every key, until the handler's answer comes ([`Page::answer_entry`]): a handler always answers. Like `on_tap`,
    /// it is called while the page is in use: a handler that changes the page does so in `cx.defer`.
    pub fn on_entry(mut self, handler: impl Fn(&EntryMade, &mut Window, &mut App) + 'static) -> Self {
        self.entry.on_made = Some(Box::new(handler));
        self
    }

    /// Changes or removes the entry. What is already typed stays, held to the new entry's bounds. An entry that is
    /// removed while it holds words, or is owed an answer, stays on the page as it is, with the chips it had,
    /// and goes once it is empty and owed none: words a person typed never go with it, and a refusal always
    /// has them to give back. An entry given meanwhile takes its place, with the words.
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

    /// Chooses the entry's option of that name, when it is one of the options offered: false when it is not.
    pub fn choose_option(&mut self, name: &str, cx: &mut Context<Self>) -> bool {
        let mut offered = self.entry.what.iter().flat_map(|what| what.options.iter().take(MAX_ENTRY_OPTIONS));
        let at = offered.position(|one| one == name);
        if let Some(at) = at {
            self.entry.state.choose(at);
            cx.notify();
        }
        at.is_some()
    }

    /// What is typed in the entry now.
    pub fn entry(&self) -> &EntryState {
        &self.entry.state
    }

    /// Whether the entry has the keys, or the field of a note. On a page with an entry one always does once it is
    /// drawn, so a letter is always text: a window's own key needs a modifier no field takes.
    pub fn entry_focused(&self, window: &Window) -> bool {
        (self.entry.what.is_some() && self.entry.focused(window)) || self.keyed(window).is_some()
    }

    /// Gives the entry the keys, as a click on it does. Escape gives them back.
    pub fn focus_entry(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.focus_view(None, window, cx);
    }

    pub(super) fn focus_view(&mut self, which: Which, window: &mut Window, cx: &mut Context<Self>) {
        let focus = self.view_mut(which).focus.get_or_insert_with(|| cx.focus_handle()).clone();
        window.focus(&focus, cx);
    }

    /// The elements of each note's field, which move with the column, and of the entry, over everything else:
    /// the typed line in the field, and what takes the clicks on the field, on its `go`, on each chip and on a
    /// field's offer. None for a page with neither.
    pub(super) fn entry_layer(&mut self, window: &mut Window, cx: &mut Context<Self>) -> [Vec<AnyElement>; 2] {
        // The keys are the entry's whenever nothing else on the page holds them, so nothing typed goes nowhere.
        if self.entry.what.is_some() && !self.entry_focused(window) {
            self.focus_view(None, window, cx);
        }
        for view in std::iter::once(&mut self.entry).chain(&mut self.fields) {
            view.lit = view.focus.get_or_insert_with(|| cx.focus_handle()).is_focused(window);
        }
        let pad = px(14.0 * page_scale(&self.theme, self.viewport));
        let typed = |piece: &&Placed| {
            matches!(
                piece.piece,
                PieceKind::Entry | PieceKind::Chip | PieceKind::Field | PieceKind::Offer | PieceKind::Go
            )
        };
        let (mut fields, mut entry) = (Vec::new(), Vec::new());
        for piece in self.placed.iter().filter(typed) {
            let rect = piece.rect.moved(0.0, -self.lift(piece));
            let into = if piece.piece.in_band() { &mut entry } else { &mut fields };
            into.extend(self.typed_element(piece, rect, pad, cx));
        }
        [fields, entry]
    }

    /// What takes the clicks and the keys of one piece of the entry or of a note's field, at `rect`.
    fn typed_element(&self, piece: &Placed, rect: Rect, pad: Pixels, cx: &mut Context<Self>) -> Option<AnyElement> {
        // A field's pieces are its note's: the entry's are nobody's but the entry's.
        let which = match piece.piece {
            PieceKind::Field | PieceKind::Offer => Some(self.fields.iter().position(|view| view.names(&piece.id))?),
            _ => None,
        };
        let index = piece.index;
        let at = div().absolute().left(px(rect.x)).top(px(rect.y)).w(px(rect.w)).h(px(rect.h));
        if piece.piece == PieceKind::Go {
            let went = cx.listener(|page, _, window, cx| page.entry_go(window, cx));
            return Some(at.id("go").cursor_pointer().on_click(went).into_any_element());
        }
        if matches!(piece.piece, PieceKind::Chip | PieceKind::Offer) {
            let chosen = cx.listener(move |page, _, window, cx| {
                if which.is_none() {
                    page.entry.state.choose(index);
                }
                page.focus_view(which, window, cx);
                cx.notify();
            });
            let id: ElementId = which.map_or(("chip", index).into(), |field| ("offer", field).into());
            return Some(at.id(id).cursor_pointer().on_click(chosen).into_any_element());
        }
        let (page, focus) = (cx.entity(), self.view(which).focus.clone()?);
        let (id, name): (ElementId, String) = match which {
            Some(field) => (("field", field).into(), format!("Field:{}", piece.id)),
            None => ("entry".into(), "Entry".into()),
        };
        let pressed = cx.listener(move |page, event, _, cx| page.entry_pressed(which, event, cx));
        let field = at.id(id).debug_selector(move || name).track_focus(&focus);
        let drawn = move |at, (), window: &mut Window, cx: &mut App| paint(at, (&page, which), &focus, window, cx);
        let field = field.cursor(CursorStyle::IBeam).overflow_hidden().px(pad);
        let field = field.on_mouse_down(MouseButton::Left, pressed).on_key_down(cx.listener(Self::entry_key));
        Some(field.child(canvas(|_, _, _| (), drawn).size_full()).into_any_element())
    }

    /// A press anywhere off the fields gives the entry the keys: a tap on a note's option or a character too.
    pub(super) fn entry_left(&mut self, event: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let (x, y) = (f32::from(event.position.x), f32::from(event.position.y));
        let on = |piece: &Placed| {
            let r = piece.rect.moved(0.0, -self.lift(piece));
            matches!(
                piece.piece,
                PieceKind::Entry | PieceKind::Chip | PieceKind::Field | PieceKind::Offer | PieceKind::Go
            ) && r.x <= x
                && x < r.right()
                && r.y <= y
                && y < r.bottom()
        };
        if !self.placed.iter().any(on) {
            self.focus_view(None, window, cx);
        }
    }

    /// The press itself gives the field the keys: GPUI does that for an element that tracks a focus handle.
    fn entry_pressed(&mut self, which: Which, event: &MouseDownEvent, cx: &mut Context<Self>) {
        let view = self.view_mut(which);
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
        let (command, which) = (held.secondary(cx), self.keyed(window));
        let state = &mut self.view_mut(which).state;
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
            "enter" => self.entry_entered(which, window, cx),
            // From a note's field the keys go to the entry; in the entry they stay: the page is never left
            // with no holder.
            "escape" => self.focus_view(None, window, cx),
            "v" if command => {
                let pasted = cx.read_from_clipboard().and_then(|item| item.text());
                state.replace(None, &pasted.unwrap_or_default());
            }
            _ => return,
        }
        cx.stop_propagation();
        self.entry_changed(window, cx);
    }

    /// A tap on the entry's `go`, which gives the entry the keys: what Enter does there. With no words in it,
    /// nothing is entered and the tap is `Tap::Go`, so whoever made the entry can say what to type first.
    fn entry_go(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.focus_view(None, window, cx);
        // An entry that is owed an answer holds words too: its Enter gives nothing, and the tap is no `Tap::Go`.
        if self.entry.state.holds() {
            return self.entry_entered(None, window, cx);
        }
        let id = self.entry.what.as_ref().map(|what| what.id.clone()).unwrap_or_default();
        if let Some(handler) = &self.on_tap {
            handler(&Tap::Go { id }, window, cx);
        }
        cx.notify();
    }

    /// Enter where the keys are. What a note's field gives carries the note's id, and goes to the entry's handler.
    fn entry_entered(&mut self, which: Which, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(made) = self.view_mut(which).state.enter() {
            self.entry_made(which, made, window, cx);
        }
    }

    /// What Enter gave where `which` is, handed to the entry's handler.
    pub(super) fn entry_made(&mut self, which: Which, made: Made, window: &mut Window, cx: &mut Context<Self>) {
        let view = self.view_mut(which);
        view.asked_as = view.what.as_ref().map(|what| what.id.clone()).unwrap_or_default();
        cx.notify();
        match (&self.entry.on_made, &self.view(which).what) {
            (Some(handler), Some(entry)) => {
                let option = made.option.and_then(|at| entry.options.get(at).cloned());
                handler(&EntryMade { id: entry.id.clone(), text: made.text, option }, window, cx);
            }
            // Nobody was given the words, so nobody will answer: they are taken as kept.
            _ => self.view_mut(which).state.answer(true),
        }
    }

    /// Tells the platform's text input that the text or the caret is no longer what it last saw, and draws again.
    /// The line told of is the one with the keys: the entry, or the field of a note.
    pub(super) fn entry_changed(&mut self, window: &Window, cx: &mut Context<Self>) {
        if let Some(focus) = &self.view(self.keyed(window)).focus {
            window.notify_text_input_changed(focus);
        }
        self.entry_settled(cx);
    }

    /// Draws again. An entry that was taken away while it held words goes here, once it holds none.
    pub(super) fn entry_settled(&mut self, cx: &mut Context<Self>) {
        if self.entry.leaving && !self.entry.state.holds() {
            (self.entry.what, self.entry.state, self.entry.leaving) = (None, EntryState::default(), false);
            self.stale = Stale::Spring;
        }
        cx.notify();
    }
}
