//! The app's windows as one thing (ADR 0019, decision 4): the page's window, and the jellyfish with its bubble.
//! Closing the page's window leaves the app and the jellyfish; a tap on the jellyfish, or opening the app again,
//! shows the page, the same view in a new window; Quit ends all of it. Once a second what core keeps for the
//! jellyfish is looked at off the window's thread, and drawn.
//!
//! This file is wiring. `window.rs` opens [`wire`] on a screen; `tests/f57/window.rs` opens it on GPUI's test
//! platform, where there is none.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use gpui::{App, AsyncApp, Entity, WindowHandle};
use jellymoon_core::JellyDrawn;
use sticker_kit::draw::{EdgeView, Page};
use sticker_kit::{Rect, Scene, Theme, Viewport};

use crate::jellyfish::{Jellyfish, Room};
use crate::showing::Showing;
use crate::taps::OnShow;
use crate::today::Later;
use crate::window::{self, LOOK_EVERY, say};

/// A look at what core keeps for the jellyfish. It reads files, so it is run off the window's thread.
pub type JellyLook = Arc<dyn Fn() -> JellyDrawn + Send + Sync>;

/// What the app is asked to open.
pub struct Asked {
    pub scene: Scene,
    pub theme: Theme,
    pub viewport: Viewport,
    pub day: Option<(OnShow, Later)>,
    /// None for a page with no day: its jellyfish is quiet, and looks at nothing.
    pub jelly: Option<JellyLook>,
    /// How long until the app ends by itself, for checks.
    pub quit_after: Option<Duration>,
}

struct State {
    view: Entity<Page>,
    viewport: Viewport,
    /// None when it could not be opened, and once the app has ended.
    jellyfish: Option<Jellyfish>,
    ended: bool,
}

/// What [`wire`] opened. Every handler runs on the window's thread, so one of them at a time has it.
#[derive(Clone)]
pub struct Opened(Rc<RefCell<State>>);

/// Opens the page's window and the jellyfish in `room`, and starts what goes on by itself: a day's reading, the
/// look at the day and the look for the jellyfish, each once a second, and the end after `quit_after`.
pub fn wire(asked: Asked, room: Room, cx: &mut App) -> gpui::Result<Opened> {
    let Asked { scene, theme, viewport, day, jelly, quit_after } = asked;
    let (shown, later) = day.unzip();
    let shown = Showing::of(shown);
    let view = window::view(scene, theme.clone(), shown.clone(), cx);
    window::show(view.clone(), viewport, cx)?;
    let page = view.downgrade();
    let opened = Opened(Rc::new(RefCell::new(State { view, viewport, jellyfish: None, ended: false })));
    // The jellyfish is in the middle of handling its tap: the page is shown once that is done.
    let tapped = opened.clone();
    let tapped: Rc<dyn Fn(&mut App)> = Rc::new(move |cx| {
        let shows = tapped.clone();
        cx.defer(move |cx| shows.show_page(cx));
    });
    match Jellyfish::open(theme, room, tapped, cx) {
        Ok(jellyfish) => opened.0.borrow_mut().jellyfish = Some(jellyfish),
        Err(error) => say(&format!("no jellyfish: {error}")),
    }
    let closing = opened.clone();
    cx.on_window_closed(move |cx, _| closing.closed(cx)).detach();
    cx.observe_keystrokes(|event, window, cx| window::key(&event.keystroke.key, window, cx)).detach();
    let (reading, read_into) = (shown.clone(), page.clone());
    cx.spawn(async move |cx| window::reads(reading, later, read_into, cx).await).detach();
    cx.spawn(async move |cx| window::looks(shown, page, cx).await).detach();
    if let Some(look) = jelly {
        let looking = opened.clone();
        cx.spawn(async move |cx| looking.looks(look, cx).await).detach();
    }
    if let Some(limit) = quit_after {
        let ending = opened.clone();
        cx.spawn(async move |cx| {
            cx.background_executor().timer(limit).await;
            cx.update(|cx| ending.quit(cx));
        })
        .detach();
    }
    Ok(opened)
}

impl Opened {
    /// The page's window. None while it is closed.
    pub fn page(&self, cx: &App) -> Option<WindowHandle<Page>> {
        window::page_window(cx)
    }

    /// The jellyfish's window, and where it and its bubble stand on the display.
    pub fn jelly(&self) -> Option<(WindowHandle<EdgeView>, (Rect, Rect))> {
        self.0.borrow().jellyfish.as_ref().map(|jellyfish| (jellyfish.character(), jellyfish.stands()))
    }

    /// The bubble's window, while there are words.
    pub fn bubble(&self) -> Option<WindowHandle<EdgeView>> {
        self.0.borrow().jellyfish.as_ref().and_then(Jellyfish::bubble)
    }

    /// Whether Quit, or the end asked for, has ended the app.
    pub fn ended(&self) -> bool {
        self.0.borrow().ended
    }

    /// Shows the page: its window is opened again when it was closed, on the view it had, and brought to the
    /// front either way. What a tap on the jellyfish does, and opening the app again.
    pub fn show_page(&self, cx: &mut App) {
        if self.ended() {
            return;
        }
        match window::page_window(cx) {
            Some(open) => {
                let _ = open.update(cx, |_, window, _| window.activate_window());
            }
            None => {
                let (view, viewport) = (self.0.borrow().view.clone(), self.0.borrow().viewport);
                if let Err(error) = window::show(view, viewport, cx) {
                    say(&format!("no window: {error}"));
                }
            }
        }
        cx.activate(true);
    }

    /// Draws the jellyfish as core says it is. It is taken out while it is drawn, so nothing GPUI calls while a
    /// window is opened finds it held.
    pub fn draw(&self, drawn: &JellyDrawn, cx: &mut App) {
        let Some(mut jellyfish) = self.0.borrow_mut().jellyfish.take() else { return };
        jellyfish.draw(drawn, cx);
        self.0.borrow_mut().jellyfish = Some(jellyfish);
    }

    /// Quit: both of the jellyfish's windows and the page's are removed, and the app ends.
    pub fn quit(&self, cx: &mut App) {
        if std::mem::replace(&mut self.0.borrow_mut().ended, true) {
            return;
        }
        let jellyfish = self.0.borrow_mut().jellyfish.take();
        if let Some(mut jellyfish) = jellyfish {
            jellyfish.close(cx);
        }
        if let Some(page) = window::page_window(cx) {
            let _ = page.update(cx, |_, window, _| window.remove_window());
        }
        cx.quit();
    }

    /// A window was closed. The app lives on with the jellyfish; with no window left at all (a jellyfish that
    /// could not be opened) there is nothing to open the page from again, and it ends.
    fn closed(&self, cx: &mut App) {
        if cx.windows().is_empty() {
            self.quit(cx);
        }
    }

    /// Looks for the jellyfish now and then once a second, off the window's thread, and draws what each look gave.
    async fn looks(self, look: JellyLook, cx: &mut AsyncApp) {
        while !self.ended() {
            let work = look.clone();
            let drawn = cx.background_executor().spawn(async move { work() }).await;
            cx.update(|cx| self.draw(&drawn, cx));
            cx.background_executor().timer(LOOK_EVERY).await;
        }
    }
}
