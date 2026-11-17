//! An edge scene: one character in a small window at a corner of the screen, and a bubble in a second window
//! beside it while it has something to say. Plain data, and pure functions that say where the two windows stand.
//! Nothing here asks the system anything: the screen, and the room a menu bar, a dock or a task bar leaves on it,
//! come in as numbers.

use serde::{Deserialize, Serialize};

use crate::geom::{Rect, Viewport};

/// What stands at the screen's edge.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Edge {
    pub mood: EdgeMood,
    /// A number drawn on the character; 0 draws none. Over 99 it is drawn as `99+`.
    pub count: usize,
    /// The bubble's texts, in order: the first [`crate::layout::MAX_BUBBLE_TEXTS`] that hold something a person
    /// sees. With none there is no bubble, and no second window.
    pub bubble: Vec<EdgeText>,
    /// How full the bubble's ring is, from 1, full, to 0, empty. None draws no ring, and so does no bubble. The
    /// kit reads no clock: whoever counts the time gives the share that is left.
    pub ring: Option<f32>,
}

/// How the character holds itself. A mood changes how it is drawn and never where anything stands.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeMood {
    #[default]
    Floats,
    LeansIn,
    HoldsUp,
    Swims,
    Waves,
    Glows,
    Droops,
}

/// One text of a bubble. It is drawn as text, cleaned and cut like every text of the kit, and as nothing else.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct EdgeText {
    pub text: String,
    pub tone: Tone,
}

/// Whose voice a bubble's text is in, which sets its role and how many lines it may take.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tone {
    /// The scene's own words.
    #[default]
    Plain,
    /// What the bubble is about, written larger.
    Strong,
    /// One small line: a name.
    Small,
    /// Someone else's words: a role of its own, with room above and below, so they are never read as the scene's.
    Quoted,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Corner {
    TopLeft,
    TopRight,
    BottomLeft,
    #[default]
    BottomRight,
}

impl Corner {
    fn right(self) -> bool {
        matches!(self, Corner::TopRight | Corner::BottomRight)
    }

    pub(crate) fn bottom(self) -> bool {
        matches!(self, Corner::BottomLeft | Corner::BottomRight)
    }
}

/// How far a corner window stands from the two edges of its corner.
pub const EDGE_MARGIN: f32 = 16.0;
/// How far the bubble's window stands from the character's.
pub const EDGE_GAP: f32 = 8.0;

/// A number that is one, and not below nothing.
fn length(value: f32) -> f32 {
    if value.is_finite() { value.max(0.0) } else { 0.0 }
}

/// The room windows may stand in: the part of `left` that is on the screen, where `left` is what the system
/// leaves of it. The whole screen when that is nothing, or is not numbers.
fn room(screen: Viewport, left: Rect) -> Rect {
    let whole = Rect::new(0.0, 0.0, length(screen.w), length(screen.h));
    if ![left.x, left.y, left.w, left.h].iter().all(|side| side.is_finite()) {
        return whole;
    }
    let (x, y) = (left.x.max(0.0), left.y.max(0.0));
    let (right, bottom) = (left.right().min(whole.w), left.bottom().min(whole.h));
    if right > x && bottom > y { Rect::new(x, y, right - x, bottom - y) } else { whole }
}

/// Where a span of `size` starts, at `wanted`, held inside the room's span from `from`, `room` long. A span longer
/// than the room starts where the room does, so its start is on the screen.
fn held(wanted: f32, size: f32, from: f32, room: f32) -> f32 {
    wanted.min(from + room - size).max(from)
}

/// Where a window of `size` stands in a corner of the screen, `margin` from the corner's two edges and inside the
/// room the system leaves. The margin gives way first: a window as large as the room fills it. A window larger
/// than the room keeps its size and starts at the room's top left. The window is never resized.
pub fn place_corner(screen: Viewport, left: Rect, size: Viewport, corner: Corner, margin: f32) -> Rect {
    let (room, margin, w, h) = (room(screen, left), length(margin), length(size.w), length(size.h));
    let x = if corner.right() { room.right() - margin - w } else { room.x + margin };
    let y = if corner.bottom() { room.bottom() - margin - h } else { room.y + margin };
    Rect::new(held(x, w, room.x, room.w), held(y, h, room.y, room.h), w, h)
}

/// Where a second window of `size` stands beside `anchor`, a window [`place_corner`] placed: on the side away
/// from the corner's edge, `gap` off, level with the anchor's lower edge in a lower corner and with its upper
/// edge in an upper one. When the room is too narrow for that, it stands over the anchor in a lower corner and
/// under it in an upper one, flush with the anchor's outer edge. It is held inside the room like the first.
pub fn place_beside(screen: Viewport, left: Rect, anchor: Rect, size: Viewport, corner: Corner, gap: f32) -> Rect {
    let (room, gap, w, h) = (room(screen, left), length(gap), length(size.w), length(size.h));
    let beside = if corner.right() { anchor.x - gap - w } else { anchor.right() + gap };
    let (x, y) = if beside >= room.x && beside + w <= room.right() {
        (beside, if corner.bottom() { anchor.bottom() - h } else { anchor.y })
    } else {
        let x = if corner.right() { anchor.right() - w } else { anchor.x };
        (x, if corner.bottom() { anchor.y - gap - h } else { anchor.bottom() + gap })
    };
    Rect::new(held(x, w, room.x, room.w), held(y, h, room.y, room.h), w, h)
}

/// Where two windows stand at a corner: the first as [`place_corner`] places it, the second beside it as
/// [`place_beside`] does. In a room that holds both apart only with less than `margin`, the first window's
/// margin gives way before the second is laid over the first: beside it when the room is wide enough for that,
/// else over or under it. The margin is one number for both of the corner's edges, so it is made smaller at both,
/// by what the tighter way of standing apart needs, though one edge alone would do. The first stands where this
/// says with a second window or with none, so neither is ever moved for the other.
pub fn place_pair(
    screen: Viewport,
    left: Rect,
    first: Viewport,
    second: Viewport,
    corner: Corner,
    margin: f32,
    gap: f32,
) -> (Rect, Rect) {
    let (held, margin) = (room(screen, left), length(margin));
    // What is left of the room's width with the two side by side, and of its height with one over the other.
    let beside = held.w - length(first.w) - length(gap) - length(second.w);
    let over = held.h - length(first.h) - length(gap) - length(second.h);
    let spare = beside.max(over);
    let margin = if spare >= 0.0 { margin.min(spare) } else { margin };
    let anchor = place_corner(screen, left, first, corner, margin);
    (anchor, place_beside(screen, left, anchor, second, corner, gap))
}
