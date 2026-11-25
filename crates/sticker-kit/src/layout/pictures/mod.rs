//! The pictures of a drawn note, each laid out from its top left in the column's design units and drawn by the
//! pencil: a run at work (`track.rs`), finished work and where it is (`sheets.rs`), changed lines to one scale
//! (`bars.rs`), rows of a label and counted marks (`tally.rs`). A picture is a `Picture` piece: its strokes and named parts are its sketch, and every word it
//! draws is a caption in the theme's handwriting, measured as every text is. A picture drawn small, on one line
//! under what a person typed, is `small.rs`.

mod bars;
mod sheets;
mod small;
mod tally;
mod track;

pub(super) use small::mark;

use super::{Ctx, PieceKind, Placed, TextRun, line_height, piece, run, told};
use crate::geom::Rect;
use crate::pencil::{Part, Pencil, Point, Sketch};
use crate::scene::Picture;
use crate::text::Role;

/// A picture being drawn: the pencil, the parts named so far, the captions, and how far down it reaches.
pub(super) struct Draft<'a, 'c> {
    cx: &'a Ctx<'c>,
    pub pencil: Pencil,
    parts: Vec<Part>,
    runs: Vec<TextRun>,
    /// The picture's top left corner and width; its height is what it comes to hold.
    pub at: Rect,
    low: f32,
}

impl Draft<'_, '_> {
    /// A design length in logical pixels.
    pub fn u(&self, units: f32) -> f32 {
        self.cx.px(units)
    }

    /// The picture's width in design units: the column's, less a card's sides.
    pub fn wide(&self) -> f32 {
        self.cx.theme.metrics.note_width - 2.0 * super::drawn::SIDE
    }

    /// A point `x` and `y` design units from the picture's top left.
    pub fn point(&self, x: f32, y: f32) -> Point {
        (self.at.x + self.u(x), self.at.y + self.u(y))
    }

    /// A rectangle in design units from the picture's top left.
    pub fn rect(&self, x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect::new(self.at.x + self.u(x), self.at.y + self.u(y), self.u(w), self.u(h))
    }

    /// Names what was just drawn in `rect`.
    pub fn part(&mut self, name: &str, rect: Rect) {
        self.low = self.low.max(rect.bottom());
        self.parts.push(Part { name: name.to_string(), rect });
    }

    /// A caption on one line with its top at `top`, cut to the picture's width: from `x` with a `lean` of 0,
    /// centred on it with 0.5, ending at it with 1. Never past the picture's sides.
    pub fn caption(&mut self, text: &str, (x, top): Point, lean: f32) {
        let fitted = self.cx.line(text, Role::Caption, self.at.w);
        self.write(fitted, (x, top), lean);
    }

    fn write(&mut self, (text, width): (String, f32), (x, top): Point, lean: f32) {
        if text.is_empty() {
            return;
        }
        let (size, most) = (self.cx.size(Role::Caption), (self.at.right() - width).max(self.at.x));
        let (left, high) = ((x - width * lean).clamp(self.at.x, most), line_height(size));
        self.runs.push(run(text, Role::Caption, left, top + high / 2.0, width, size));
        self.low = self.low.max(top + high);
    }

    /// How high a caption's line is.
    pub fn line(&self) -> f32 {
        line_height(self.cx.size(Role::Caption))
    }

    /// Someone else's name for a thing, as a caption, by the rule a told row's name is drawn by: only when it
    /// holds no blank, unseen or control character and is whole on one line of `room`. Says whether it was drawn.
    pub fn name(&mut self, name: &str, room: f32, at: Point, lean: f32) -> bool {
        let fitted = told::plain(name).then(|| told::one_line(self.cx, name, room, Role::Caption)).flatten();
        let drawn = fitted.is_some();
        fitted.into_iter().for_each(|fitted| self.write(fitted, at, lean));
        drawn
    }
}

/// The picture of the note `id`, from `at`'s top left and as wide as `at`: a `Picture` piece as high as what it
/// holds.
pub(super) fn draw(cx: &Ctx, id: &str, picture: &Picture, at: Rect) -> Placed {
    let pencil = Pencil::new(id, cx.k);
    let mut draft = Draft { cx, pencil, parts: Vec::new(), runs: Vec::new(), at, low: at.y };
    let what = match picture {
        Picture::Track { steps, stage, file } => track::draw(&mut draft, *steps, *stage, file),
        Picture::Sheets { made, changed, removed, state } => {
            sheets::draw(&mut draft, [*made, *changed, *removed], *state)
        }
        Picture::Bars { rows, more } => bars::draw(&mut draft, rows, *more),
        Picture::Tally { title, rows, more, under, open } => tally::draw(&mut draft, title, rows, (more, under), *open),
    };
    let rect = Rect::new(at.x, at.y, at.w, draft.low - at.y + cx.px(4.0));
    let mut placed = piece(PieceKind::Picture, id, 0, rect);
    placed.text = draft.runs;
    placed.sketch = Some(Sketch { what, parts: draft.parts, strokes: draft.pencil.finish() });
    placed
}
