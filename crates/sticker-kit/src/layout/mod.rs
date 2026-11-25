//! Where every piece goes. A pure function of the scene, the theme, the
//! viewport and a text measurer; nothing here draws.
//!
//! The page is one column, read from its top: the header, the small characters nobody typed for in rows, then the
//! scene's notes in the scene's order, each a card of one width at its full height (`column.rs`). A piece's place
//! is the column's own: `y` is from the column's top, however far a view has scrolled it. The entry alone is
//! placed in the window ([`layout_entry`]): a band at its bottom the column ends above.

mod column;
mod drawn;
mod edge;
mod entry;
mod little;
mod notes;
mod page;
mod pictures;
mod print;
mod text_note;
mod told;
mod typed;

use serde::{Deserialize, Serialize};

use crate::geom::{Rect, Viewport};
use crate::pencil::Sketch;
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
    /// What a busy character says it is doing: one line on a small bubble over it. `id` is the character's.
    Says,
    Note,
    Option,
    /// The mark in front of a row of a whole list: `id` is the note's, `index` the row.
    Mark,
    Footer,
    /// The field of an entry ([`layout_entry`]); its text is the hint.
    Entry,
    /// An option of an entry: `id` is the entry's, `index` the option.
    Chip,
    /// What stands for the options of an entry past the ones offered, `+N more`: `id` is the entry's, `index`
    /// how many they are. It is words and no option: nothing chooses it.
    More,
    /// An entry's one thing to do that is no option, after its chips: `id` is the entry's. A tap acts on it.
    Action,
    /// The choice at the right end of an entry's field row that does what Enter does: `id` is the entry's.
    Go,
    /// Someone else's words under a text note, on a ground of their own: `id` is the note's. Text and no more.
    /// A person's own typed words are one too ([`crate::Note::Said`]), a bubble with that note's `id`.
    Quote,
    /// The field of a text note, which a person types one line in: `id` is the note's; its text is the hint.
    Field,
    /// The words over a text note's field that say what it is for: `id` is the note's.
    Offer,
    /// The picture of a drawn note ([`crate::Note::Drawn`]): `id` is the note's. Its strokes are its `sketch` and
    /// its text the captions. A tap on it is the note's `id` at [`crate::PICTURE_PLACE`].
    Picture,
    /// The `?` at the top right of a picture whose note has words: `id` is the note's. A tap on it is the picture's.
    Ask,
    /// A picture drawn small under what a person typed ([`crate::Note::Said`]'s `small`): `id` is that note's.
    /// Its strokes are its `sketch` and its text the captions, on one line. A tap on it is the bubble's.
    Small,
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
    /// the big character's turn down the column, or the character's position in the scene.
    pub index: usize,
    pub rect: Rect,
    pub tilt_degrees: f32,
    pub z: i32,
    pub text: Vec<TextRun>,
    /// What a pencil draws of the piece: a picture's strokes, or the ring of its `?`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sketch: Option<Sketch>,
}

pub use edge::{
    EdgeKind, EdgePiece, EdgePlaced, EdgeWindow, MAX_BUBBLE_LINES, MAX_BUBBLE_TEXTS, edge_windows, layout_edge,
};
pub use entry::{entry_top, layout_entry};
pub use print::print;
pub use text_note::MAX_TEXT_LINES;
pub use typed::MAX_QUOTED_LINES;

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
    let mut cx = Ctx::new(theme, viewport, measure);
    column::lay(&mut cx, scene);
    cx.out.truncate(MAX_PIECES);
    cx.out
}

/// How far the column `placed` scrolls in the viewport: what of it is below the top of the entry's band
/// ([`entry_top`]) when its top is at the window's, and nothing when it ends above that.
pub fn scroll_most(placed: &[Placed], theme: &Theme, viewport: Viewport) -> f32 {
    let column = placed.iter().filter(|piece| !piece.piece.in_band());
    let bottom = column.map(|piece| piece.rect.bottom()).fold(0.0, f32::max);
    let under = theme.metrics.margin_y * page_scale(theme, viewport);
    (bottom + under - entry_top(theme, viewport)).max(0.0)
}

pub(crate) struct Ctx<'a> {
    pub theme: &'a Theme,
    pub measure: &'a dyn Measure,
    pub k: f32,
    ox: f32,
    pub viewport: Viewport,
    pub out: Vec<Placed>,
}

impl<'a> Ctx<'a> {
    fn new(theme: &'a Theme, viewport: Viewport, measure: &'a dyn Measure) -> Self {
        let k = page_scale(theme, viewport);
        let ox = (viewport.w - theme.metrics.width * k) / 2.0;
        Ctx { theme, measure, k, ox, viewport, out: Vec::new() }
    }

    /// A design length in logical pixels.
    pub fn px(&self, v: f32) -> f32 {
        v * self.k
    }

    pub fn x(&self, design_x: f32) -> f32 {
        self.ox + design_x * self.k
    }

    /// The column's left edge and its width: every card's.
    pub fn column(&self) -> (f32, f32) {
        let m = &self.theme.metrics;
        (self.x(m.content_left), self.px(m.note_width))
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
    /// Whether a piece of this kind is the entry's: placed in the window, in the band at its bottom, and never
    /// moved with the column.
    pub fn in_band(self) -> bool {
        matches!(self, PieceKind::Entry | PieceKind::Chip | PieceKind::More | PieceKind::Action | PieceKind::Go)
    }

    /// Paint order: larger is nearer the viewer.
    pub fn z(self) -> i32 {
        match self {
            PieceKind::Paper => 0,
            PieceKind::Hole => 1,
            PieceKind::Title | PieceKind::Footer => 3,
            PieceKind::Entry | PieceKind::Chip | PieceKind::More | PieceKind::Action | PieceKind::Go => 4,
            PieceKind::Label => 10,
            PieceKind::Hero | PieceKind::Character => 11,
            PieceKind::Star => 12,
            PieceKind::Bubble | PieceKind::Badge | PieceKind::Says => 13,
            PieceKind::Note => 20,
            PieceKind::Option | PieceKind::Mark | PieceKind::Quote | PieceKind::Field | PieceKind::Offer => 21,
            PieceKind::Picture | PieceKind::Ask | PieceKind::Small => 21,
            PieceKind::Tape => 22,
        }
    }
}

/// A level piece with no text, at its kind's paint order.
pub(crate) fn piece(kind: PieceKind, id: &str, index: usize, rect: Rect) -> Placed {
    let (id, text) = (id.to_string(), Vec::new());
    Placed { piece: kind, id, index, rect, tilt_degrees: 0.0, z: kind.z(), text, sketch: None }
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
