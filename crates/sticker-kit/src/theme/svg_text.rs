//! What a picture writes in its paths and shapes, held before the parser makes a tree of it. These are the cheap
//! refusals of shapes that are known to cost the parser dear, each with a line that says what is wrong. They are
//! not the proof that a picture is safe to draw: the trial is (`trial.rs`), and all of this runs inside it.
//!
//! What the parser does by the length of a value and not by the nesting of elements
//! (`docs/adr/0015-the-art-door-reads-with-the-renderers-parser.md`, "What the parser does by the length of a value"):
//!
//! 1. `svgtypes::SimplifyingPathParser::next` calls itself for every segment it draws nothing for. There are two:
//!    a close that follows a close, and an arc `kurbo` makes no curve of (its sweep rounds to nothing, when its
//!    ends are far closer than its radius). An arc that follows a close is not one: a move is drawn before it.
//!    Every other segment draws at least one. A thousand such in a row overflow a thread's stack.
//! 2. An arc is drawn as a number of curves that grows with its radius, all made before one is given back, and
//!    taken from the front of a list one at a time: by the library's sum a radius of 1e36 is 1.5 million curves,
//!    from 30 bytes.
//! 3. A `circle`, an `ellipse` and a rounded `rect` are made of the same arcs, by their size: `r="3e38"` is four
//!    million curves, and so is `rx="100%"` of a `viewBox` that wide.
//! 4. The stroker does not end soon on a curve under a stroke thousands of times wider than the curve is long.
//!
//! So a `d` is walked here with the parser library's own plain tokenizer, `svgtypes::PathParser`, which is a loop
//! and calls nothing back.

use std::str::FromStr;

use svgtypes::{Length, LengthUnit, PathParser, PathSegment, PointsParser, ViewBox};

/// The most commands in the paths of one file, a pair of `points` counted as one.
pub const MAX_ART_COMMANDS: usize = 2048;

/// The most segments in a row in one path that the parser draws nothing for: a close after a close, and an arc
/// whose ends are closer than a millionth of its radius. It calls itself once for each. In a debug build a stack
/// of 256 KiB takes 80 of them and one of 128 KiB takes 32.
pub const MAX_ART_IDLE: usize = 32;

/// The largest number a path, a list of points, a `viewBox` or a shape's place, size and stroke may write.
pub const MAX_ART_WRITTEN: f64 = 1_000_000.0;

/// How many times larger one radius of an arc may be than the other. The parser makes a radius larger until the
/// arc reaches its end, by as much as the other is larger than it, and the curves grow with the radius.
pub const MAX_ART_FLAT: f64 = 1000.0;

/// An arc whose ends are closer than this part of its larger radius is one the parser may make no curve of.
const IDLE_ARC: f64 = 1e-6;

/// The attributes that place and size a shape, and the width of its stroke.
const PLACED: [&str; 14] =
    ["x", "y", "x1", "y1", "x2", "y2", "cx", "cy", "r", "rx", "ry", "width", "height", "stroke-width"];

/// What the paths of one file have written so far.
#[derive(Default)]
pub(super) struct Written {
    commands: usize,
}

impl Written {
    /// Holds one attribute's value, or says why the file is refused.
    pub(super) fn attribute(&mut self, name: &str, value: &str) -> Result<(), String> {
        match name {
            "d" => self.path(value)?,
            "points" => {
                for (x, y) in PointsParser::from(value) {
                    self.commands += 1;
                    within(&[x, y])?;
                }
            }
            // A `viewBox` the parser cannot read is none to it.
            "viewBox" => {
                let read = ViewBox::from_str(value).map_or([0.0; 4], |view| [view.x, view.y, view.w, view.h]);
                within(&read)?;
            }
            _ if PLACED.contains(&name) => placed(name, value)?,
            _ => {}
        }
        if self.commands > MAX_ART_COMMANDS {
            return Err(format!("its paths hold more than {MAX_ART_COMMANDS} commands"));
        }
        Ok(())
    }

    fn path(&mut self, d: &str) -> Result<(), String> {
        // `closed`: the last thing drawn was a close, as the parser remembers it. `at` and `began`: where the
        // path is, and where its piece began, when every segment so far was drawn. `off`: how far from `at`
        // the parser may stand, since it stays where it stood at an arc it draws nothing for; `began_off` is
        // that for `began`.
        let (mut idle, mut closed) = (0, false);
        let (mut at, mut began) = ((0.0, 0.0), (0.0, 0.0));
        let (mut off, mut began_off) = (0.0, 0.0);
        // The parser draws what came before a segment it cannot read, and stops there.
        for segment in PathParser::from(d).map_while(Result::ok) {
            self.commands += 1;
            within(&numbers(segment))?;
            let to = end(segment, at, began);
            let chord = (to.0 - at.0).hypot(to.1 - at.1);
            let nothing = match segment {
                PathSegment::ClosePath { .. } => closed,
                PathSegment::EllipticalArc { rx, ry, .. } => {
                    let (small, large) = (rx.abs().min(ry.abs()), rx.abs().max(ry.abs()));
                    // An arc with a radius of nothing is a line to the parser.
                    if small > 0.0 && large > MAX_ART_FLAT * small {
                        return Err(format!("an arc's radii are more than {MAX_ART_FLAT} times apart"));
                    }
                    // An arc to the very place the parser stands at is a line to it, and drawn. Where it
                    // may stand elsewhere, an arc to here may be one that goes nowhere from there.
                    !closed && chord <= IDLE_ARC * large + off && (chord > 0.0 || off > 0.0)
                }
                _ => false,
            };
            idle = if nothing { idle + 1 } else { 0 };
            closed = matches!(segment, PathSegment::ClosePath { .. });
            off = match segment {
                PathSegment::ClosePath { .. } => began_off,
                // The parser is at this arc's end or where it stood: the gate on PR 154 wrote 1022 arcs to one
                // place, and a door that only moved on took each after the first for an arc that is drawn.
                // Twice the chord and a part of the place: a sum of two numbers is rounded.
                PathSegment::EllipticalArc { .. } if nothing => {
                    off + 2.0 * chord + f64::EPSILON * (to.0.abs() + to.1.abs())
                }
                _ if lands(segment) => 0.0,
                _ => off,
            };
            if matches!(segment, PathSegment::MoveTo { .. }) {
                (began, began_off) = (to, off);
            }
            at = to;
            if idle > MAX_ART_IDLE {
                return Err(format!(
                    "a path holds more than {MAX_ART_IDLE} closes, or arcs that go nowhere, in a row, which the \
                     parser draws nothing for"
                ));
            }
        }
        Ok(())
    }
}

/// Whether a segment ends at a place it writes whole, wherever the path stood before it.
fn lands(segment: PathSegment) -> bool {
    matches!(
        segment,
        PathSegment::MoveTo { abs: true, .. }
            | PathSegment::LineTo { abs: true, .. }
            | PathSegment::CurveTo { abs: true, .. }
            | PathSegment::SmoothCurveTo { abs: true, .. }
            | PathSegment::Quadratic { abs: true, .. }
            | PathSegment::SmoothQuadratic { abs: true, .. }
            | PathSegment::EllipticalArc { abs: true, .. }
    )
}

/// A shape's place, size or stroke: a number within the limit, or a part of what holds it from none to all of
/// it, as drawing programs write a gradient's ends, a background and a round corner. What holds it is the
/// `viewBox`, whose four numbers are within the limit themselves, so the part is. A length the parser cannot
/// read is none to it, and a unit makes one at most 96 times larger.
fn placed(name: &str, value: &str) -> Result<(), String> {
    let Ok(length) = Length::from_str(value) else { return Ok(()) };
    if length.unit == LengthUnit::Percent && !(0.0..=100.0).contains(&length.number) {
        return Err(format!("its `{name}` is a percent outside 0 to 100"));
    }
    within(&[length.number])
}

/// Where a segment ends, from where the path is (`at`) and where its piece began.
fn end(segment: PathSegment, at: (f64, f64), began: (f64, f64)) -> (f64, f64) {
    let from = |abs: bool, x: f64, y: f64| if abs { (x, y) } else { (at.0 + x, at.1 + y) };
    match segment {
        PathSegment::MoveTo { abs, x, y }
        | PathSegment::LineTo { abs, x, y }
        | PathSegment::CurveTo { abs, x, y, .. }
        | PathSegment::SmoothCurveTo { abs, x, y, .. }
        | PathSegment::Quadratic { abs, x, y, .. }
        | PathSegment::SmoothQuadratic { abs, x, y }
        | PathSegment::EllipticalArc { abs, x, y, .. } => from(abs, x, y),
        PathSegment::HorizontalLineTo { abs, x } => (if abs { x } else { at.0 + x }, at.1),
        PathSegment::VerticalLineTo { abs, y } => (at.0, if abs { y } else { at.1 + y }),
        PathSegment::ClosePath { .. } => began,
    }
}

/// The lengths and places a segment writes. An arc's turn and flags are neither.
fn numbers(segment: PathSegment) -> [f64; 6] {
    match segment {
        PathSegment::MoveTo { x, y, .. }
        | PathSegment::LineTo { x, y, .. }
        | PathSegment::SmoothQuadratic { x, y, .. } => [x, y, 0.0, 0.0, 0.0, 0.0],
        PathSegment::HorizontalLineTo { x: one, .. } | PathSegment::VerticalLineTo { y: one, .. } => {
            [one, 0.0, 0.0, 0.0, 0.0, 0.0]
        }
        PathSegment::CurveTo { x1, y1, x2, y2, x, y, .. } => [x1, y1, x2, y2, x, y],
        PathSegment::SmoothCurveTo { x2: a, y2: b, x, y, .. }
        | PathSegment::Quadratic { x1: a, y1: b, x, y, .. }
        | PathSegment::EllipticalArc { rx: a, ry: b, x, y, .. } => [a, b, x, y, 0.0, 0.0],
        PathSegment::ClosePath { .. } => [0.0; 6],
    }
}

fn within(written: &[f64]) -> Result<(), String> {
    // Written this way round, what is not a number is refused too.
    if written.iter().all(|number| number.abs() <= MAX_ART_WRITTEN) {
        return Ok(());
    }
    Err(format!("it writes a number over {MAX_ART_WRITTEN}"))
}
