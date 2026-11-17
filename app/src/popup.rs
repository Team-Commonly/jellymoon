//! What each of the jellyfish's two windows is asked for, as data (ADR 0019, decision 4): a pop-up one sees
//! through, which never takes the keys or the focus and is never moved or resized by a person. `layout --jelly`
//! prints it with each window, and [`options`] is the one place it becomes what GPUI is asked: a pop-up, above
//! other windows and on every space, with no title bar. That a system honours it only a person at a screen can
//! see (spike 3).
//!
//! Not all that such a look can find is answered here. `tap` is what is printed; whether a view is given a tap
//! is `jellyfish.rs`, and the part of the window that takes it is the kit's box for the character
//! (`draw/edge.rs`). The app comes to the front on a tap in `shell.rs` (`show_page`). GPUI's options have
//! nothing to say of a shadow, so a shadow round a window one sees through cannot be answered here at all.

use serde::Serialize;

/// What a window of the jellyfish is asked for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Opens {
    /// Whether the window takes the keys and the focus when it is opened.
    pub focus: bool,
    /// Whether what is behind the window is seen where nothing is drawn.
    pub see_through: bool,
    /// Whether a person can move it, and whether a person can resize it.
    pub moves: bool,
    pub resizes: bool,
    /// What a tap in it does.
    pub tap: Tapped,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Tapped {
    /// The page is shown, and nothing else happens.
    Page,
    /// The window takes no tap.
    #[serde(rename = "none")]
    Nothing,
}

/// The jellyfish's own window: a tap on the jellyfish shows the page.
pub const JELLY: Opens = Opens { focus: false, see_through: true, moves: false, resizes: false, tap: Tapped::Page };

/// The bubble's window: it takes no tap.
pub const BUBBLE: Opens = Opens { tap: Tapped::Nothing, ..JELLY };

/// What GPUI is asked for a window that is `asked`, at `bounds` on `display`: no title bar, and never minimized.
#[cfg(feature = "window")]
pub fn options(
    asked: Opens,
    bounds: gpui::Bounds<gpui::Pixels>,
    display: Option<gpui::DisplayId>,
) -> gpui::WindowOptions {
    use gpui::{WindowBackgroundAppearance, WindowBounds, WindowKind, WindowOptions};
    let (see_through, opaque) = (WindowBackgroundAppearance::Transparent, WindowBackgroundAppearance::Opaque);
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        titlebar: None,
        focus: asked.focus,
        show: true,
        kind: WindowKind::PopUp,
        is_movable: asked.moves,
        is_resizable: asked.resizes,
        is_minimizable: false,
        display_id: display,
        window_background: if asked.see_through { see_through } else { opaque },
        ..Default::default()
    }
}

#[cfg(all(test, feature = "window"))]
mod tests {
    use gpui::{Bounds, WindowBackgroundAppearance, WindowBounds, WindowKind, point, px, size};

    use super::*;

    /// This holds what each window is asked for. What a system does with it is spike 3's to see.
    #[test]
    fn both_windows_are_asked_as_pop_ups_one_sees_through_that_never_take_the_focus() {
        let bounds = Bounds::new(point(px(1328.0), px(788.0)), size(px(96.0), px(96.0)));
        for asked in [JELLY, BUBBLE] {
            let options = options(asked, bounds, None);
            assert!(matches!(options.kind, WindowKind::PopUp), "{asked:?}");
            assert!(matches!(options.window_background, WindowBackgroundAppearance::Transparent), "{asked:?}");
            assert!(!options.focus && options.show && options.titlebar.is_none(), "{asked:?}");
            assert!(!options.is_movable && !options.is_resizable && !options.is_minimizable, "{asked:?}");
            assert!(matches!(options.window_bounds, Some(WindowBounds::Windowed(at)) if at == bounds), "{asked:?}");
            assert!(options.display_id.is_none() && options.window_min_size.is_none());
        }
        // Only the tap differs, and a window that is asked otherwise is opened otherwise.
        let tapped = Opens { tap: Tapped::Page, ..BUBBLE };
        assert_eq!((JELLY.tap, BUBBLE.tap, tapped), (Tapped::Page, Tapped::Nothing, JELLY));
        let plain =
            options(Opens { focus: true, see_through: false, moves: true, resizes: true, ..JELLY }, bounds, None);
        assert!(plain.focus && plain.is_movable && plain.is_resizable);
        assert!(matches!(plain.window_background, WindowBackgroundAppearance::Opaque));
    }

    #[test]
    fn what_a_window_is_asked_is_printed_with_these_names() {
        let printed = serde_json::to_string(&JELLY).unwrap();
        let want = r#"{"focus":false,"see_through":true,"moves":false,"resizes":false,"tap":"page"}"#;
        assert_eq!(printed, want);
        assert!(serde_json::to_string(&BUBBLE).unwrap().ends_with(r#""tap":"none"}"#));
    }
}
