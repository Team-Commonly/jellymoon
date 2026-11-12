//! A sticker-journal page: flat sticker characters, stars and taped notes on
//! grid paper.
//!
//! Three plain-data halves and one drawing half:
//! - [`scene`]: what is on the page.
//! - [`theme`]: how it looks, loaded from a TOML file or built in.
//! - [`layout`]: a pure function from scene, theme and viewport to placed pieces.
//! - [`draw`]: a GPUI view that paints the placed pieces (feature `gpui`).
//!
//! Beside a scene a page may hold one [`Entry`], a line to type: [`entry`] is its rules, as plain data.

pub mod art;
pub mod color;
pub mod entry;
mod fonts;
pub mod geom;
pub mod layout;
pub mod motion;
mod sample;
pub mod scene;
pub mod text;
pub mod theme;

#[cfg(feature = "gpui")]
pub mod draw;

pub use color::Color;
pub use fonts::BundledMeasure;
pub use geom::{Rect, Viewport};
pub use layout::{MAX_NOTES, MAX_PIECES, MORE_NOTES_ID, PieceKind, Placed, TextRun, layout, layout_entry};
pub use scene::{Character, Entry, LegendItem, Mark, Mood, Note, Scene, State};
pub use text::{FixedAdvance, Measure, Role};
pub use theme::{Loaded, Theme, ThemeError};
