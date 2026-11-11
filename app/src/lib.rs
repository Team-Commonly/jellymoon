//! The shell (ADR 0011, section 4): it draws page data with the kit and decides nothing.
//!
//! - [`read`]: a page-data file into core's page data, plus what the file held that this app has no piece for.
//! - [`scene`]: page data into the kit's scene. Pure, and no GPUI.
//! - [`lines`]: the scene laid out and printed, one placed piece a line, measured from the kit's bundled fonts.
//! - [`taps`]: what a tap on a day's page does: a ruling through core, or the delivery hook turned on.
//! - [`today`]: the day's page, kept in the user's own folder and read again through core when the day has moved on.
//! - [`window`]: the kit's view of the scene in a window (feature `window`).

/// The smallest size a page is drawn at. The fullest page core can write fits it; below it the text is too small.
pub const SMALLEST: sticker_kit::Viewport = sticker_kit::Viewport { w: 960.0, h: 640.0 };

/// The longest side `--size` takes, in pixels.
pub const LARGEST_SIDE: f32 = 16384.0;

pub mod lines;
pub mod read;
pub mod scene;
pub mod taps;
pub mod today;
#[cfg(feature = "window")]
pub mod window;
