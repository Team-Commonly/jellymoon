//! Where the pieces of an edge scene go: the character and its count in one window, the bubble with its texts
//! and its ring in a second. A pure function of the scene, the theme and a text measurer. Each window has one
//! size for a theme, whatever is said in it, so a window is opened and removed and never resized: a text longer
//! than its lines is cut with an ellipsis ([`wrap`]).

use serde::{Deserialize, Serialize};

use super::{MAX_TEXT_CHARS, TextRun, line_height, run};
use crate::edge::{Corner, Edge, Tone};
use crate::geom::{Rect, Viewport};
use crate::text::{Measure, Role, clean, has_visible, wrap};
use crate::theme::Theme;

/// The most texts a bubble is drawn with: the first that hold something a person sees.
pub const MAX_BUBBLE_TEXTS: usize = 4;
/// The most lines a bubble is drawn with, over all its texts.
pub const MAX_BUBBLE_LINES: usize = 8;

/// The side of the character's window, and the width of the bubble's.
const CHARACTER: f32 = 96.0;
const BUBBLE_WIDTH: f32 = 300.0;
/// Between the bubble's edge and its texts, between two of its texts, and around a quoted one.
const PAD: f32 = 14.0;
const BETWEEN: f32 = 6.0;
const APART: f32 = 10.0;
const RING: f32 = 26.0;
/// The range an edge scene's text is drawn in, whatever a theme asks: its windows are small and never resized, so
/// a larger text would leave them and a smaller one could not be read.
const TEXT_SIZES: (f32, f32) = (9.0, 22.0);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeKind {
    Character,
    /// The number on the character; its text is the number.
    Count,
    /// The bubble; its text is every line drawn in it.
    Bubble,
    Ring,
}

/// One placed piece of an edge scene, in logical pixels from its own window's top left.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EdgePiece {
    pub kind: EdgeKind,
    pub rect: Rect,
    pub text: Vec<TextRun>,
}

/// One window of an edge scene: its size, and the pieces in it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EdgeWindow {
    pub size: Viewport,
    pub pieces: Vec<EdgePiece>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EdgePlaced {
    pub character: EdgeWindow,
    /// None when the scene's bubble holds nothing a person sees.
    pub bubble: Option<EdgeWindow>,
    /// How full the ring is, from 1 to 0, when one is drawn.
    pub ring: Option<f32>,
}

impl Tone {
    fn role(self) -> Role {
        match self {
            Tone::Plain => Role::Body,
            Tone::Strong => Role::RowMain,
            Tone::Small => Role::RowSub,
            Tone::Quoted => Role::Quoted,
        }
    }

    /// The size its text is drawn at in `theme`, held to [`TEXT_SIZES`]. A size that is no number is the least.
    fn size(self, theme: &Theme) -> f32 {
        sized(self.role(), theme)
    }

    /// The most lines one text of this tone takes.
    fn lines(self) -> usize {
        match self {
            Tone::Plain => 4,
            Tone::Strong | Tone::Quoted => 3,
            Tone::Small => 1,
        }
    }
}

/// The size a text of `role` is drawn at in an edge scene.
fn sized(role: Role, theme: &Theme) -> f32 {
    let size = role.size(theme, 1.0);
    if size.is_finite() { size.clamp(TEXT_SIZES.0, TEXT_SIZES.1) } else { TEXT_SIZES.0 }
}

impl Edge {
    /// Whether the bubble holds something a person sees: whether there is a bubble, and a second window.
    pub fn speaks(&self) -> bool {
        self.bubble.iter().any(|said| has_visible(&clean(&said.text, MAX_TEXT_CHARS)))
    }
}

/// The sizes of the two windows of an edge scene in `theme`: the character's, and the bubble's, which is as
/// tall as its most lines in the theme's largest bubble text. Neither depends on what is said.
pub fn edge_windows(theme: &Theme) -> (Viewport, Viewport) {
    let tallest = [Tone::Plain, Tone::Strong, Tone::Small, Tone::Quoted].map(|tone| line_height(tone.size(theme)));
    let line = tallest.into_iter().fold(0.0, f32::max);
    let gaps = (MAX_BUBBLE_TEXTS - 1) as f32 * APART;
    let bubble = Viewport { w: BUBBLE_WIDTH, h: 2.0 * PAD + MAX_BUBBLE_LINES as f32 * line + gaps };
    (Viewport { w: CHARACTER, h: CHARACTER }, bubble)
}

/// Where every piece of an edge scene goes, each in its own window. The bubble hugs the side of its window that
/// `corner` names (its lower edge in a lower corner), where the character is. A theme that [`Theme::check`]
/// refuses gets no piece.
pub fn layout_edge(edge: &Edge, theme: &Theme, corner: Corner, measure: &dyn Measure) -> EdgePlaced {
    let (character, bubble) = edge_windows(theme);
    if theme.check().is_err() {
        return EdgePlaced { character: EdgeWindow { size: character, pieces: Vec::new() }, bubble: None, ring: None };
    }
    let piece = |kind, rect| EdgePiece { kind, rect, text: Vec::new() };
    let mut pieces = vec![piece(EdgeKind::Character, Rect::new(8.0, 8.0, CHARACTER - 16.0, CHARACTER - 16.0))];
    if edge.count > 0 {
        let (role, size) = (Role::RowSub, sized(Role::RowSub, theme));
        let text = if edge.count > 99 { "99+".to_string() } else { edge.count.to_string() };
        let (width, height) = (measure.width(&text, role, size), line_height(size));
        let rect = Rect::new(CHARACTER - 4.0 - (width + 12.0), 4.0, width + 12.0, height + 4.0);
        let text = vec![run(text, role, rect.x + 6.0, rect.y + rect.h / 2.0, width, size)];
        pieces.push(EdgePiece { kind: EdgeKind::Count, rect, text });
    }
    let said = spoken(edge, theme, corner, bubble, measure);
    let ring = said.as_ref().and(edge.ring).map(|ring| if ring.is_finite() { ring.clamp(0.0, 1.0) } else { 0.0 });
    EdgePlaced { character: EdgeWindow { size: character, pieces }, bubble: said, ring }
}

/// The bubble's window: the bubble with its lines, and its ring when the scene has one.
fn spoken(edge: &Edge, theme: &Theme, corner: Corner, window: Viewport, measure: &dyn Measure) -> Option<EdgeWindow> {
    let texts = edge.bubble.iter().map(|said| (said.tone, clean(&said.text, MAX_TEXT_CHARS)));
    let texts: Vec<(Tone, String)> = texts.filter(|(_, text)| has_visible(text)).take(MAX_BUBBLE_TEXTS).collect();
    if texts.is_empty() {
        return None;
    }
    // The ring has a column of its own, so no line runs under it.
    let inner = window.w - 2.0 * PAD - if edge.ring.is_some() { RING + BETWEEN } else { 0.0 };
    let (mut runs, mut cursor, mut left, mut before) = (Vec::new(), PAD, MAX_BUBBLE_LINES, None);
    for (at, (tone, text)) in texts.iter().enumerate() {
        // Every text still to come keeps one line, so none is left out for the length of an earlier one.
        let most = tone.lines().min(left - (texts.len() - at - 1));
        let (role, size) = (tone.role(), tone.size(theme));
        if let Some(before) = before {
            cursor += if *tone == Tone::Quoted || before == Tone::Quoted { APART } else { BETWEEN };
        }
        let lines = wrap(text, inner, most, role, size, measure);
        left -= lines.len().min(most);
        for (line, width) in lines {
            runs.push(run(line, role, PAD, cursor + line_height(size) / 2.0, width, size));
            cursor += line_height(size);
        }
        before = Some(*tone);
    }
    // In a lower corner the bubble stands on its window's lower edge, and its lines move down with it.
    let height = (cursor + PAD).max(2.0 * PAD + RING).min(window.h);
    let top = if corner.bottom() { window.h - height } else { 0.0 };
    runs.iter_mut().for_each(|run| run.rect = run.rect.moved(0.0, top));
    let mut pieces =
        vec![EdgePiece { kind: EdgeKind::Bubble, rect: Rect::new(0.0, top, window.w, height), text: runs }];
    if edge.ring.is_some() {
        let rect = Rect::new(window.w - PAD - RING, top + PAD, RING, RING);
        pieces.push(EdgePiece { kind: EdgeKind::Ring, rect, text: Vec::new() });
    }
    Some(EdgeWindow { size: window, pieces })
}
