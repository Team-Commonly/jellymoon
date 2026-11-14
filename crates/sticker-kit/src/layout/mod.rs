//! Where every piece goes. A pure function of the scene, the theme, the
//! viewport and a text measurer; nothing here draws.

mod entry;
mod grid;
mod notes;
mod page;
mod text_note;

use serde::{Deserialize, Serialize};

use crate::geom::{Rect, Viewport};
use crate::scene::Scene;
use crate::text::{Measure, Role, clean, fit};
use crate::theme::Theme;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PieceKind {
    Paper,
    Hole,
    Tape,
    Title,
    Hero,
    Character,
    Star,
    Bubble,
    Badge,
    Label,
    Note,
    Option,
    /// The mark in front of a row of a whole list: `id` is the note's, `index` the row.
    Mark,
    Legend,
    Footer,
    /// The field of an entry ([`layout_entry`]); its text is the hint.
    Entry,
    /// An option of an entry: `id` is the entry's, `index` the option.
    Chip,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TextRun {
    pub rect: Rect,
    pub text: String,
    pub role: Role,
    /// The font size, in logical pixels, the text was measured at and is drawn at. It is the role's own size unless
    /// the layout wrote the text smaller to make room (a whole choice's options).
    pub size: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Placed {
    pub piece: PieceKind,
    /// The scene item this belongs to: a character's or choice note's id,
    /// `note-<n>` for other notes, or the piece's own name.
    pub id: String,
    /// Which one, where an id has several: the star, the option, the note,
    /// the legend entry, or the character's position in the scene.
    pub index: usize,
    pub rect: Rect,
    pub tilt_degrees: f32,
    pub z: i32,
    pub text: Vec<TextRun>,
}

pub use entry::layout_entry;
pub use notes::{MAX_NOTES, MORE_NOTES_ID};
pub use text_note::MAX_TEXT_LINES;

/// Longest text the kit carries for anything that is not a label.
pub const MAX_TEXT_CHARS: usize = 400;

/// The most pieces a layout returns. A backstop against a scene nobody bounded: what would come after is dropped.
pub const MAX_PIECES: usize = 5000;

/// The scale from the theme's design canvas to the viewport.
pub fn page_scale(theme: &Theme, viewport: Viewport) -> f32 {
    let m = &theme.metrics;
    let fit = (viewport.w / m.width.max(1.0)).min(viewport.h / m.height.max(1.0));
    fit.clamp(0.0, m.max_scale.max(0.1))
}

/// Where every piece goes. A theme that [`Theme::check`] refuses gets no piece at all: its numbers drive the loops
/// here and in the view, and the view draws only what is placed.
pub fn layout(scene: &Scene, theme: &Theme, viewport: Viewport, measure: &dyn Measure) -> Vec<Placed> {
    if theme.check().is_err() {
        return Vec::new();
    }
    let mut cx = Ctx::new(theme, viewport, measure, !scene.notes.is_empty());
    page::header(&mut cx, scene);
    let top = page::hero(&mut cx, scene);
    grid::characters(&mut cx, scene, top);
    notes::notes(&mut cx, scene);
    page::legend_and_footer(&mut cx, scene);
    cx.out.truncate(MAX_PIECES);
    cx.out
}

pub(crate) struct Ctx<'a> {
    pub theme: &'a Theme,
    pub measure: &'a dyn Measure,
    pub k: f32,
    ox: f32,
    oy: f32,
    pub has_notes: bool,
    pub out: Vec<Placed>,
}

impl<'a> Ctx<'a> {
    fn new(theme: &'a Theme, viewport: Viewport, measure: &'a dyn Measure, has_notes: bool) -> Self {
        let (k, m) = (page_scale(theme, viewport), &theme.metrics);
        let (ox, oy) = ((viewport.w - m.width * k) / 2.0, (viewport.h - m.height * k) / 2.0);
        Ctx { theme, measure, k, ox, oy, has_notes, out: Vec::new() }
    }

    /// A design length in logical pixels.
    pub fn px(&self, v: f32) -> f32 {
        v * self.k
    }

    pub fn x(&self, design_x: f32) -> f32 {
        self.ox + design_x * self.k
    }

    pub fn y(&self, design_y: f32) -> f32 {
        self.oy + design_y * self.k
    }

    pub fn paper(&self) -> Rect {
        let m = &self.theme.metrics;
        Rect::new(
            self.x(m.margin_x),
            self.y(m.margin_y),
            self.px(m.width - 2.0 * m.margin_x),
            self.px(m.height - 2.0 * m.margin_y),
        )
    }

    /// The right edge the left-hand content may reach.
    pub fn content_right(&self) -> f32 {
        if self.has_notes {
            self.x(self.theme.metrics.note_left) - self.px(24.0)
        } else {
            self.paper().right() - self.px(40.0)
        }
    }

    pub fn size(&self, role: Role) -> f32 {
        role.size(self.theme, self.k)
    }

    /// Cleans `raw` and cuts it to `max_width`. Returns the text and its width.
    pub fn line(&self, raw: &str, role: Role, max_width: f32) -> (String, f32) {
        let limit = match role {
            Role::Label | Role::LabelStruck => self.theme.labels.max_chars,
            _ => MAX_TEXT_CHARS,
        };
        let text = clean(raw, limit);
        fit(&text, max_width.max(0.0), role, self.size(role), self.measure)
    }

    pub fn clean(&self, raw: &str) -> String {
        clean(raw, MAX_TEXT_CHARS)
    }
}

impl PieceKind {
    /// Paint order: larger is nearer the viewer.
    pub fn z(self) -> i32 {
        match self {
            PieceKind::Paper => 0,
            PieceKind::Hole => 1,
            PieceKind::Title | PieceKind::Legend | PieceKind::Footer => 3,
            PieceKind::Entry | PieceKind::Chip => 4,
            PieceKind::Label => 10,
            PieceKind::Hero | PieceKind::Character => 11,
            PieceKind::Star => 12,
            PieceKind::Bubble | PieceKind::Badge => 13,
            PieceKind::Note => 20,
            PieceKind::Option | PieceKind::Mark => 21,
            PieceKind::Tape => 22,
        }
    }
}

/// A level piece with no text, at its kind's paint order.
pub(crate) fn piece(kind: PieceKind, id: &str, index: usize, rect: Rect) -> Placed {
    Placed { piece: kind, id: id.to_string(), index, rect, tilt_degrees: 0.0, z: kind.z(), text: Vec::new() }
}

/// A run whose box is `width` wide and one line tall, starting at `x`, with
/// the line centred on `mid_y`.
pub(crate) fn run(text: String, role: Role, x: f32, mid_y: f32, width: f32, size: f32) -> TextRun {
    let height = line_height(size);
    TextRun { rect: Rect::new(x, mid_y - height / 2.0, width, height), text, role, size }
}

pub(crate) fn line_height(size: f32) -> f32 {
    (size * 1.3).round()
}
