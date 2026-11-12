//! What a picture writes in its paths and shapes, held before the parser makes a tree of it. The door shares the
//! renderer's parser, so the two never differ on a picture; but the parser itself does three things by the length
//! of a value and not by the nesting of elements, and a tree cannot be judged once making it has ended the program
//! (`docs/adr/0015-the-art-door-reads-with-the-renderers-parser.md`, "What the parser does by the length of a value").
//!
//! 1. `svgtypes::SimplifyingPathParser::next` calls itself for every segment it draws nothing for. There are two:
//!    a close that follows a close, and an arc `kurbo` makes no curve of (its sweep rounds to nothing, when its
//!    ends are far closer than its radius). An arc that follows a close is not one: a move is drawn before it.
//!    Every other segment draws at least one. A thousand such in a row overflow a thread's stack.
//! 2. An arc is drawn as a number of curves that grows with its radius, all made before one is given back, and
//!    taken from the front of a list one at a time: by the library's sum a radius of 1e36 is 1.5 million curves,
//!    from 30 bytes.
//! 3. A `circle`, an `ellipse` and a rounded `rect` are made of the same arcs, by their written size: `r="3e38"`
//!    is four million curves.
//!
//! So a `d` is walked here with the parser library's own plain tokenizer, `svgtypes::PathParser`, which is a loop
//! and calls nothing back: the door has no reading of a path of its own.

use std::str::FromStr;

use svgtypes::{Length, PathParser, PathSegment, PointsParser};

/// The most commands in the paths of one file, a pair of `points` counted as one.
pub const MAX_ART_COMMANDS: usize = 2048;

/// The most closes and arcs in a row in one path with nothing else between them: each may be one the parser
/// draws nothing for, and it calls itself once for each. In a debug build a stack of 256 KiB takes 80 of them and
/// one of 128 KiB takes 32.
pub const MAX_ART_IDLE: usize = 32;

/// The largest number a path, a list of points or a shape's place and size may write. At that size a circle is
/// made of 32 curves at most (a unit makes a length up to 96 times larger).
pub const MAX_ART_WRITTEN: f64 = 1_000_000.0;

/// How many times larger one radius of an arc may be than the other. The parser makes a radius larger until the
/// arc reaches its end, by as much as the other is larger than it, and the curves grow with the radius. With this
/// and [`MAX_ART_WRITTEN`] an arc is made of 64 curves at most.
pub const MAX_ART_FLAT: f64 = 1000.0;

/// The attributes that place and size a shape.
const PLACED: &str = "x y x1 y1 x2 y2 cx cy r rx ry width height";

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
            // A length the parser cannot read is none to it. A unit makes it at most 96 times larger.
            _ if PLACED.split(' ').any(|placed| placed == name) => {
                within(&[Length::from_str(value).map_or(0.0, |length| length.number)])?;
            }
            _ => {}
        }
        if self.commands > MAX_ART_COMMANDS {
            return Err(format!("its paths hold more than {MAX_ART_COMMANDS} commands"));
        }
        Ok(())
    }

    fn path(&mut self, d: &str) -> Result<(), String> {
        // `closed`: the last thing drawn was a close, as the parser remembers it.
        let (mut idle, mut closed) = (0, false);
        // The parser draws what came before a segment it cannot read, and stops there.
        for segment in PathParser::from(d).map_while(Result::ok) {
            self.commands += 1;
            within(&numbers(segment))?;
            if let PathSegment::EllipticalArc { rx, ry, .. } = segment {
                let (small, large) = (rx.abs().min(ry.abs()), rx.abs().max(ry.abs()));
                // An arc with a radius of nothing is a line to the parser.
                if small > 0.0 && large > MAX_ART_FLAT * small {
                    return Err(format!("an arc's radii are more than {MAX_ART_FLAT} times apart"));
                }
            }
            idle = match segment {
                PathSegment::ClosePath { .. } if closed => idle + 1,
                PathSegment::EllipticalArc { .. } if !closed => idle + 1,
                _ => 0,
            };
            closed = matches!(segment, PathSegment::ClosePath { .. });
            if idle > MAX_ART_IDLE {
                return Err(format!(
                    "a path holds more than {MAX_ART_IDLE} closes or arcs in a row, which the parser may draw \
                     nothing for"
                ));
            }
        }
        Ok(())
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
