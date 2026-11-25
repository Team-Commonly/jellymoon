//! A sticker-journal page: flat sticker characters, stars and taped notes in
//! one column on grid paper.
//!
//! Three plain-data halves and one drawing half:
//! - [`scene`]: what is on the page.
//! - [`theme`]: how it looks, loaded from a TOML file or built in.
//! - [`layout`]: a pure function from scene, theme and viewport to placed pieces, in the column's own coordinates.
//! - [`draw`]: a GPUI view that paints the placed pieces (feature `gpui`).
//!
//! Beside a scene a page may hold one [`Entry`], a line to type: [`entry`] is its rules, as plain data.
//! [`edge`] is a second, smaller scene: one character at a corner of the screen and its bubble.

pub mod art;
pub mod color;
pub mod edge;
pub mod entry;
mod fonts;
pub mod geom;
pub mod layout;
pub mod motion;
pub mod pencil;
mod sample;
pub mod scene;
pub mod scroll;
pub mod text;
pub mod theme;

#[cfg(feature = "gpui")]
pub mod draw;

pub use color::Color;
pub use edge::{Corner, EDGE_GAP, EDGE_MARGIN, Edge, EdgeMood, EdgeText, Tone, place_beside, place_corner, place_pair};
pub use fonts::BundledMeasure;
pub use geom::{Rect, Viewport};
pub use layout::{
    EdgeKind, EdgePiece, EdgePlaced, EdgeWindow, MAX_BUBBLE_LINES, MAX_BUBBLE_TEXTS, MAX_PIECES, MAX_QUOTED_LINES,
    MAX_TEXT_LINES, PieceKind, Placed, TextRun, edge_windows, entry_top, layout, layout_edge, layout_entry, print,
    scroll_most,
};
pub use pencil::{Ink, Part, Pencil, Sketch, Stroke};
pub use scene::{
    Bar, Character, Entry, Field, Kept, MAX_STRIP_PANELS, MAX_TALLY_MARKS, MAX_TALLY_ROWS, Mark, Mood, Note,
    PICTURE_PLACE, Panel, Picture, Scene, Shows, Stage, State, TallyRow, ToldRow, speaker,
};
pub use scroll::Scroll;
pub use text::{FixedAdvance, Measure, Role};
pub use theme::{Loaded, Theme, ThemeError};
