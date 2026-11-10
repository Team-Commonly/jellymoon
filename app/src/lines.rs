//! What `jellymoon-app layout` prints: the scene laid out by the kit, one placed piece a line. Text is measured from
//! the kit's bundled font files, never by the operating system, so the numbers are the same on every machine.

use serde::Serialize;
use sticker_kit::{BundledMeasure, PieceKind, Placed, Rect, Role, Scene, State, Theme, Viewport, layout};

/// One placed piece. `x`, `y`, `w` and `h` are logical pixels from the window's top left; `tilt` is degrees clockwise.
#[derive(Serialize)]
struct Line<'a> {
    kind: PieceKind,
    /// The wish's id, `clash-<n>`, `note-<n>`, or the piece's own name.
    id: &'a str,
    index: usize,
    #[serde(flatten)]
    at: At,
    tilt: f64,
    /// A character only. How it is drawn (greyed, with a bubble, with a badge) is its state, not a place.
    #[serde(skip_serializing_if = "Option::is_none")]
    state: Option<State>,
    text: Vec<Run<'a>>,
}

#[derive(Serialize)]
struct Run<'a> {
    role: Role,
    text: &'a str,
    #[serde(flatten)]
    at: At,
}

#[derive(Serialize)]
struct At {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

/// Every placed piece as one line of JSON, in the order the kit placed them.
pub fn lines(scene: &Scene, theme: &Theme, viewport: Viewport) -> String {
    let placed = layout(scene, theme, viewport, &BundledMeasure::new(theme));
    placed.iter().filter_map(|piece| Some(format!("{}\n", serde_json::to_string(&line(piece, scene)).ok()?))).collect()
}

fn line<'a>(piece: &'a Placed, scene: &Scene) -> Line<'a> {
    let who = scene.characters.get(piece.index).filter(|_| piece.piece == PieceKind::Character);
    Line {
        kind: piece.piece,
        id: &piece.id,
        index: piece.index,
        at: at(piece.rect),
        tilt: hundredths(piece.tilt_degrees),
        state: who.map(|who| who.state),
        text: piece.text.iter().map(|run| Run { role: run.role, text: &run.text, at: at(run.rect) }).collect(),
    }
}

fn at(rect: Rect) -> At {
    At { x: hundredths(rect.x), y: hundredths(rect.y), w: hundredths(rect.w), h: hundredths(rect.h) }
}

/// Logical pixels to two decimals: enough to place a piece, and no noise from the last bits of a float.
fn hundredths(value: f32) -> f64 {
    (f64::from(value) * 100.0).round() / 100.0 + 0.0
}
