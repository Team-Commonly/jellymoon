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

use crate::window::say;
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
    /// Why the bubble could not be opened at the last draw, as it was said. None when it was opened, drawn in or
    /// not wanted.
    said: Option<String>,
}

/// Whether a draw's failure to open the bubble is to be said. `failure` is why it could not be opened, and none
/// when it was opened, drawn in or not wanted. A failure is said when it is not the one last said: once for a
/// row of the same failure, since the look draws every second, and again when the reason changes. A draw with
/// none ends the row, a quiet one too.
fn news(said: &mut Option<String>, failure: Option<String>) -> bool {
    let new = failure.is_some() && *said != failure;
    *said = failure;
    new
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
        Ok(Jellyfish { character, bubble: None, theme, room, stands, said: None })
    }

    /// Draws what core says: the jellyfish in its state, and the bubble opened, drawn again or removed.
    pub fn draw(&mut self, drawn: &JellyDrawn, cx: &mut App) {
        let edge = jelly::edge(drawn);
        let for_character = edge.clone();
        let _ = self.character.update(cx, |view, _, cx| view.set_edge(for_character, cx));
        let open = self.bubble.take();
        let (bubble, failure) = if edge.speaks() {
            // A bubble that something else removed cannot be drawn in: it is opened again, as for the first words.
            let again = edge.clone();
            let open = open.filter(|open| open.update(cx, |view, _, cx| view.set_edge(again, cx)).is_ok());
            match open.map_or_else(|| self.speak(edge, cx), Ok) {
                Ok(open) => (Some(open), None),
                Err(why) => (None, Some(why)),
            }
        } else {
            if let Some(open) = open {
                let _ = open.update(cx, |_, window, _| window.remove_window());
            }
            (None, None)
        };
        // An open that fails is said on stderr, and tried again at the next draw.
        if news(&mut self.said, failure)
            && let Some(why) = &self.said
        {
            say(&format!("no bubble: {why}"));
        }
        self.bubble = bubble;
    }

    /// Opens the bubble, or says why it could not be opened.
    fn speak(&self, edge: Edge, cx: &mut App) -> Result<WindowHandle<EdgeView>, String> {
        let (at, theme) = (self.room.bounds(self.stands.1), self.theme.clone());
        let opened = cx.open_window(popup::options(popup::BUBBLE, at, self.room.display), move |_, cx| {
            cx.new(|_| EdgeView::new(EdgePart::Bubble, edge, theme, jelly::CORNER))
        });
        opened.map_err(|error| error.to_string())
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

#[cfg(test)]
mod tests {
    use super::news;

    // No open fails on GPUI's test platform, so the rule is held here: the review of F57, finding 8, and of its
    // notes, finding 3. Every draw goes through it, a quiet one as a draw with no failure.
    #[test]
    fn a_failed_open_is_said_once_for_a_row_of_the_same_failure_and_again_for_another_or_after_a_draw_with_none() {
        let mut last = None;
        let failed = |why: &str| Some(why.to_string());
        let draws = [
            (failed("no display"), true),
            (failed("no display"), false),
            (failed("no memory"), true),
            (failed("no memory"), false),
            (failed("no display"), true),
            (None, false),
            (failed("no display"), true),
            (failed("no display"), false),
            (None, false),
            (None, false),
            (failed("no memory"), true),
        ];
        let said: Vec<bool> = draws.iter().map(|(failure, _)| news(&mut last, failure.clone())).collect();
        assert_eq!(said, draws.map(|(_, said)| said));
        assert_eq!(last, failed("no memory"), "what was last said is kept, to tell the next failure from it");
    }
}
