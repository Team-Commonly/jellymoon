//! What `jellymoon-app layout --jelly` prints: the jellyfish and its bubble as the kit's edge scene places them,
//! one line of JSON each. Core says which state is drawn and what the bubble holds (`JellyDrawn`); here that is
//! turned into the kit's scene, one name for one name, and nothing is decided. No window is opened.

use jellymoon_core::{JellyDrawn, JellyState, Whose};
use serde::Serialize;
use sticker_kit::{
    BundledMeasure, Corner, EDGE_GAP, EDGE_MARGIN, Edge, EdgeKind, EdgeMood, EdgePiece, EdgeWindow, Rect, Said, Theme,
    Tone, Viewport, layout_edge, place_beside, place_corner,
};

use crate::lines::{At, Run, at, run};

/// The corner the jellyfish lives in (ADR 0019, decision 4).
const CORNER: Corner = Corner::BottomRight;

/// One printed line: a window, where it stands on the screen, or a piece, from its own window's top left.
#[derive(Serialize)]
struct Line<'a> {
    kind: Kind,
    /// The window: `jelly`, or `bubble`.
    id: &'static str,
    #[serde(flatten)]
    at: At,
    /// The jellyfish only: core's state, and the mood it is drawn in.
    #[serde(skip_serializing_if = "Option::is_none")]
    state: Option<JellyState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    mood: Option<EdgeMood>,
    /// The ring only: how full it is, from 1 to 0.
    #[serde(skip_serializing_if = "Option::is_none")]
    ring: Option<f64>,
    /// A piece only: its runs of text.
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<Vec<Run<'a>>>,
}

#[derive(Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum Kind {
    Window,
    Character,
    Count,
    Bubble,
    Ring,
}

/// The kit's scene for what core says is drawn: a mood for each state, a tone for each voice.
pub fn edge(drawn: &JellyDrawn) -> Edge {
    let mood = match drawn.state {
        JellyState::Quiet => EdgeMood::Floats,
        JellyState::Listening => EdgeMood::LeansIn,
        JellyState::Heard => EdgeMood::HoldsUp,
        JellyState::Working => EdgeMood::Swims,
        JellyState::Asks => EdgeMood::Waves,
        JellyState::Waits => EdgeMood::Glows,
        JellyState::Cannot => EdgeMood::Droops,
    };
    let tone = |whose| match whose {
        Whose::Yours => Tone::Strong,
        Whose::Project => Tone::Small,
        Whose::Run => Tone::Quoted,
        Whose::Jellymoon => Tone::Plain,
    };
    let bubble = drawn.bubble.iter().map(|said| Said { text: said.text.clone(), tone: tone(said.whose) });
    Edge {
        mood,
        count: drawn.count.unwrap_or(0),
        bubble: bubble.collect(),
        ring: drawn.ring.map(|left| f32::from(left) / 1000.0),
    }
}

/// `1440x900` as a screen: each side a whole number of pixels, from 1 to [`crate::LARGEST_SIDE`].
pub fn screen(given: &str) -> Option<Viewport> {
    let (w, h) = given.split_once('x')?;
    let side =
        |text: &str| text.parse::<u16>().ok().map(f32::from).filter(|side| (1.0..=crate::LARGEST_SIDE).contains(side));
    Some(Viewport { w: side(w)?, h: side(h)? })
}

/// The jellyfish's window and its pieces, then the bubble's when there is one, each as one line of JSON. The room
/// windows stand in is the whole screen here: the room a system leaves is given by whoever opens them.
pub fn lines(drawn: &JellyDrawn, theme: &Theme, screen: Viewport) -> String {
    let scene = edge(drawn);
    let placed = layout_edge(&scene, theme, CORNER, &BundledMeasure::new(theme));
    let room = Rect::new(0.0, 0.0, screen.w, screen.h);
    let jelly = place_corner(screen, room, placed.character.size, CORNER, EDGE_MARGIN);
    let mut out = window("jelly", jelly, &placed.character, drawn.state, &scene, None);
    if let Some(bubble) = &placed.bubble {
        let beside = place_beside(screen, room, jelly, bubble.size, CORNER, EDGE_GAP);
        out.extend(window("bubble", beside, bubble, drawn.state, &scene, placed.ring));
    }
    out.iter().filter_map(|line| Some(format!("{}\n", serde_json::to_string(line).ok()?))).collect()
}

fn window<'a>(
    id: &'static str,
    stands: Rect,
    window: &'a EdgeWindow,
    state: JellyState,
    scene: &Edge,
    ring: Option<f32>,
) -> Vec<Line<'a>> {
    let bare = |kind, rect| Line { kind, id, at: at(rect), state: None, mood: None, ring: None, text: None };
    let piece = |piece: &'a EdgePiece| {
        let kind = match piece.kind {
            EdgeKind::Character => Kind::Character,
            EdgeKind::Count => Kind::Count,
            EdgeKind::Bubble => Kind::Bubble,
            EdgeKind::Ring => Kind::Ring,
        };
        let who = kind == Kind::Character;
        Line {
            state: who.then_some(state),
            mood: who.then_some(scene.mood),
            ring: ring.filter(|_| kind == Kind::Ring).map(|ring| (f64::from(ring) * 1000.0).round() / 1000.0),
            text: Some(piece.text.iter().map(run).collect()),
            ..bare(kind, piece.rect)
        }
    };
    std::iter::once(bare(Kind::Window, stands)).chain(window.pieces.iter().map(piece)).collect()
}
