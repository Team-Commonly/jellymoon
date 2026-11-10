//! The shell (ADR 0011, section 4): it draws page data with the kit and decides nothing.
//!
//! - [`read`]: a page-data file into core's page data, plus what the file held that this app has no piece for.
//! - [`scene`]: page data into the kit's scene. Pure, and no GPUI.
//! - [`lines`]: the scene laid out and printed, one placed piece a line, measured from the kit's bundled fonts.
//! - [`window`]: the kit's view of the scene in a window (feature `window`).

/// The smallest size a page is drawn at. The fullest page core can write fits it; below it the text is too small.
pub const SMALLEST: sticker_kit::Viewport = sticker_kit::Viewport { w: 960.0, h: 640.0 };

pub mod lines;
pub mod read;
pub mod scene;
#[cfg(feature = "window")]
pub mod window;
