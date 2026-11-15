//! The jellyfish on the screen: two pop-up windows of the kit's edge scene (ADR 0019, decision 4). Its own is
//! opened once, where the kit's placing says, in the room the system leaves on the primary display. The bubble's
//! is opened while core's state has words a person sees, and removed when it has none. Neither is ever moved or
//! resized, and neither is asked for the keys (`popup.rs`). Core says what is drawn (`JellyDrawn`); `jelly.rs`
//! turns that into the kit's scene. This file is wiring: a test opens both on GPUI's test platform.

use std::rc::Rc;

use gpui::{App, Bounds, DisplayId, WindowHandle, point, prelude::*, px, size};
use jellymoon_core::JellyDrawn;
use sticker_kit::draw::{EdgePart, EdgeView};
use sticker_kit::{Edge, Rect, Theme, Viewport};

use crate::{jelly, popup};

/// The primary display as the system says it is: its size, the room the menu bar and the Dock leave on it, from
/// its own top left, and where that top left is among the displays.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Room {
    pub screen: Viewport,
    pub left: Rect,
    pub origin: (f32, f32),
    pub display: Option<DisplayId>,
}

impl Room {
    /// The room on the primary display. With no display there is no room: both windows are then placed at the
    /// top left of a screen of nothing, and the system puts them where it can.
    pub fn primary(cx: &App) -> Room {
        let Some(display) = cx.primary_display() else { return Room::default() };
        let (whole, left) = (display.bounds(), display.visible_bounds());
        let (x, y) = (f32::from(whole.origin.x), f32::from(whole.origin.y));
        Room {
            screen: Viewport { w: whole.size.width.into(), h: whole.size.height.into() },
            left: Rect::new(
                f32::from(left.origin.x) - x,
                f32::from(left.origin.y) - y,
                left.size.width.into(),
                left.size.height.into(),
            ),
            origin: (x, y),
            display: Some(display.id()),
        }
    }

    /// Where a window that stands at `stands` on this display is among the displays.
    fn bounds(&self, stands: Rect) -> Bounds<gpui::Pixels> {
        let at = point(px(self.origin.0 + stands.x), px(self.origin.1 + stands.y));
        Bounds::new(at, size(px(stands.w), px(stands.h)))
    }
}

/// The two windows. The jellyfish's is open for as long as this is; the bubble's while there are words.
pub struct Jellyfish {
    character: WindowHandle<EdgeView>,
    bubble: Option<WindowHandle<EdgeView>>,
    theme: Theme,
    room: Room,
    /// Where each window stands on the display, the bubble's whether it is open or not.
    stands: (Rect, Rect),
}

impl Jellyfish {
    /// Opens the jellyfish, quiet, in `room`. `tapped` is called for a tap on the jellyfish itself.
    pub fn open(theme: Theme, room: Room, tapped: Rc<dyn Fn(&mut App)>, cx: &mut App) -> gpui::Result<Jellyfish> {
        let stands = jelly::stands(&theme, room.screen, room.left);
        let (at, quiet) = (room.bounds(stands.0), theme.clone());
        let character = cx.open_window(popup::options(popup::JELLY, at, room.display), move |_, cx| {
            let view = EdgeView::new(EdgePart::Character, Edge::default(), quiet, jelly::CORNER);
            cx.new(|_| view.on_tap(move |_, cx| tapped(cx)))
        })?;
        Ok(Jellyfish { character, bubble: None, theme, room, stands })
    }

    /// Draws what core says: the jellyfish in its state, and the bubble opened, drawn again or removed.
    pub fn draw(&mut self, drawn: &JellyDrawn, cx: &mut App) {
        let edge = jelly::edge(drawn);
        let words = edge.speaks();
        let for_character = edge.clone();
        let _ = self.character.update(cx, |view, _, cx| view.set_edge(for_character, cx));
        match (self.bubble, words) {
            (Some(open), true) => {
                let _ = open.update(cx, |view, _, cx| view.set_edge(edge, cx));
            }
            (Some(open), false) => {
                let _ = open.update(cx, |_, window, _| window.remove_window());
                self.bubble = None;
            }
            (None, true) => self.bubble = self.speak(edge, cx).ok(),
            (None, false) => {}
        }
    }

    fn speak(&self, edge: Edge, cx: &mut App) -> gpui::Result<WindowHandle<EdgeView>> {
        let (at, theme) = (self.room.bounds(self.stands.1), self.theme.clone());
        cx.open_window(popup::options(popup::BUBBLE, at, self.room.display), move |_, cx| {
            cx.new(|_| EdgeView::new(EdgePart::Bubble, edge, theme, jelly::CORNER))
        })
    }

    /// Removes both windows.
    pub fn close(&mut self, cx: &mut App) {
        for open in [Some(self.character), self.bubble.take()].into_iter().flatten() {
            let _ = open.update(cx, |_, window, _| window.remove_window());
        }
    }

    pub fn character(&self) -> WindowHandle<EdgeView> {
        self.character
    }

    /// The bubble's window, while it is open.
    pub fn bubble(&self) -> Option<WindowHandle<EdgeView>> {
        self.bubble
    }

    /// Where the jellyfish stands on its display, and where the bubble stands whenever it is open.
    pub fn stands(&self) -> (Rect, Rect) {
        self.stands
    }
}
