//! The shell (ADR 0011, section 4): it draws page data with the kit and decides nothing.
//!
//! - [`read`]: a page-data file into core's page data, plus what the file held that this app has no piece for.
//! - [`scene`]: page data into the kit's scene. Pure, and no GPUI.
//! - [`lines`]: the scene laid out and printed, one placed piece a line, measured from the kit's bundled fonts.
//! - [`taps`]: what a tap on a day's page does: a ruling through core, or the delivery hook turned on.
//! - [`wishing`]: a wish typed in the window: kept by core, its run asked of core, and the page looked at again.
//! - [`today`]: the day's page, kept in the user's own folder and read again through core when the day has moved on.
//! - [`showing`]: what the window does with a key, a tap, Enter, a reading step and a look, with no window.
//! - [`window`]: the kit's view of the scene in a window (feature `window`): wiring only.

/// The smallest size a page is drawn at. The fullest page core can write fits it; below it the text is too small.
pub const SMALLEST: sticker_kit::Viewport = sticker_kit::Viewport { w: 960.0, h: 640.0 };

/// The longest side `--size` takes, in pixels.
pub const LARGEST_SIDE: f32 = 16384.0;

/// The first argument by which this program knows it was started as the trial of a theme's picture, and not as
/// the app: `main` hands it to the kit before it does anything else.
pub const TRIAL: &str = "--try-art";

/// How the kit starts a trial of a theme's picture: this very program again, and the folder its answers are
/// kept in, `art-trials` in Jellymoon's own folder. With no such folder every start tries the pictures again; with
/// no path to this program no trial starts, and the kit draws its own art.
pub fn trial() -> sticker_kit::theme::Trial {
    let kept = jellymoon_core::own_folder().ok().flatten().map(|home| home.join("art-trials"));
    sticker_kit::theme::Trial::new(std::env::current_exe().unwrap_or_default(), TRIAL, kept)
}

/// The line that says a picture of the theme's is not drawn, and why.
pub fn refused_art(refused: &sticker_kit::ThemeError) -> String {
    format!("theme: {refused}; the built-in art is drawn").replace(char::is_control, " ")
}

pub mod lines;
pub mod read;
pub mod scene;
pub mod showing;
pub mod stopping;
pub mod taps;
pub mod today;
#[cfg(feature = "window")]
pub mod window;
pub mod wishing;
