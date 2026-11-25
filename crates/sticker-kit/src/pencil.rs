//! A pencil: strokes as a hand draws them, as plain geometry. Nothing here knows GPUI, the clock or a random
//! source. A stroke bows a little and its ends overshoot a little, by numbers made from the note's id and the
//! stroke's place in its picture, so the same scene gives the same strokes every frame and every run. Each stroke
//! is laid twice, a wide faint pass under a narrow darker one: that is the graphite.
//!
//! A stroke has a start and a length in seconds from when its note first came onto a page; at a time it is
//! drawn from its start to the part that time has reached ([`Stroke::passes`]). The strokes of a picture follow
//! one another, and a whole picture takes [`DRAWN_IN`] seconds at most.

use serde::{Deserialize, Serialize};

use crate::geom::Rect;

pub type Point = (f32, f32);

/// The most a point of a stroke leaves the line it was asked for, in design units: its bow, its overshoot at an
/// end and the wobble of its wide pass.
pub const REACH: f32 = 5.0;
/// The most seconds a picture takes to draw itself.
pub const DRAWN_IN: f32 = 1.5;
/// The narrow pass's width in design units, and how many times wider the faint pass under it is.
pub const WIDTH: f32 = 1.5;
pub const SOFT: f32 = 2.4;

const OVER: f32 = 2.4;
const BOW: f32 = 1.5;

/// Which of the theme's pencils a stroke is drawn with. `Quiet` is the palette's `quiet`: a thing that is gone.
/// Which of the theme's pencils a stroke is drawn with.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Ink {
    #[default]
    Pencil,
    Added,
    Taken,
    Quiet,
    /// The palette's `star`: what fills a star.
    Star,
    /// The palette's `accent`: the mark of a thing that asks for the person.
    Asks,
}

/// One stroke: the narrow pass, the wide faint pass under it, and when it is drawn.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Stroke {
    pub ink: Ink,
    pub line: Vec<Point>,
    pub soft: Vec<Point>,
    /// The narrow pass's width in logical pixels.
    pub width: f32,
    pub start: f32,
    pub length: f32,
    /// Whether the stroke is of the one thing that keeps moving once its picture is drawn.
    pub rocks: bool,
}

/// A named part of a picture and where it is: what a test or a print reads a picture by.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Part {
    pub name: String,
    pub rect: Rect,
}

/// A picture as laid out: what it is and counts, in words nobody draws, its parts and its strokes in the order
/// a hand draws them.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Sketch {
    pub what: String,
    pub parts: Vec<Part>,
    pub strokes: Vec<Stroke>,
}

impl Sketch {
    /// How long the picture takes to draw itself.
    pub fn seconds(&self) -> f32 {
        self.strokes.iter().map(|stroke| stroke.start + stroke.length).fold(0.0, f32::max)
    }

    pub fn count(&self, name: &str) -> usize {
        self.parts.iter().filter(|part| part.name == name).count()
    }
}

impl Stroke {
    /// How much of the stroke is drawn `t` seconds after its note came, from 0 to 1. With no time, all of it.
    pub fn part(&self, t: Option<f32>) -> f32 {
        let Some(t) = t else { return 1.0 };
        if t <= self.start {
            return 0.0;
        }
        if self.length > 0.0 { ((t - self.start) / self.length).min(1.0) } else { 1.0 }
    }

    /// The two passes as far as `t` has drawn them, the wide faint one first. None before the stroke begins.
    pub fn passes(&self, t: Option<f32>) -> Option<[Vec<Point>; 2]> {
        let part = self.part(t);
        if part >= 1.0 {
            return Some([self.soft.clone(), self.line.clone()]);
        }
        (part > 0.0).then(|| [cut(&self.soft, part), cut(&self.line, part)])
    }
}

fn far(a: Point, b: Point) -> f32 {
    (b.0 - a.0).hypot(b.1 - a.1)
}

fn long(points: &[Point]) -> f32 {
    points.windows(2).map(|pair| far(pair[0], pair[1])).sum()
}

/// The first `part` of a line of points, by length.
fn cut(points: &[Point], part: f32) -> Vec<Point> {
    let mut left = long(points) * part.clamp(0.0, 1.0);
    let mut out: Vec<Point> = points.first().copied().into_iter().collect();
    for pair in points.windows(2) {
        let step = far(pair[0], pair[1]);
        if step <= left || step == 0.0 {
            out.push(pair[1]);
            left -= step;
            continue;
        }
        let u = left / step;
        out.push((pair[0].0 + (pair[1].0 - pair[0].0) * u, pair[0].1 + (pair[1].1 - pair[0].1) * u));
        break;
    }
    out
}

/// Numbers from -1 to 1 that follow from a note's id, a stroke's place and which pass it is, and nothing else.
struct Hand(u64);

impl Hand {
    fn of(id: &str, place: usize, pass: u64) -> Self {
        let mix = |hash: u64, byte: u8| (hash ^ u64::from(byte)).wrapping_mul(0x100_0000_01B3);
        let id = id.bytes().fold(0xCBF2_9CE4_8422_2325_u64, mix);
        Hand(id ^ (place as u64 + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ pass.wrapping_mul(0xD1B5_4A32_D192_ED03))
    }

    fn next(&mut self) -> f32 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        ((z ^ (z >> 31)) >> 40) as f32 / (1u64 << 23) as f32 - 1.0
    }
}

/// Draws the strokes of one picture, in the order they are asked for.
pub struct Pencil {
    id: String,
    /// Design units to logical pixels.
    k: f32,
    ink: Ink,
    rocks: bool,
    strokes: Vec<Stroke>,
}

impl Pencil {
    pub fn new(id: &str, k: f32) -> Self {
        Pencil { id: id.to_string(), k, ink: Ink::Pencil, rocks: false, strokes: Vec::new() }
    }

    pub fn ink(&mut self, ink: Ink) {
        self.ink = ink;
    }

    /// Whether the strokes that follow are of the thing that rocks once the picture is drawn.
    pub fn rocking(&mut self, rocks: bool) {
        self.rocks = rocks;
    }

    /// The hand for the next stroke: its numbers follow from the note's id and the stroke's place in the picture.
    fn hand(&self, pass: u64) -> Hand {
        Hand::of(&self.id, self.strokes.len(), pass)
    }

    /// A straight stroke as a hand draws it: bowed, and past both its ends.
    pub fn line(&mut self, a: Point, b: Point) {
        let (k, reach) = (self.k, far(a, b));
        let mut hand = self.hand(0);
        let (dx, dy) = if reach > 0.0 { ((b.0 - a.0) / reach, (b.1 - a.1) / reach) } else { (1.0, 0.0) };
        let over = |by: f32| (reach * 0.1).min(OVER * k) * (0.3 + 0.35 * (by + 1.0));
        let (before, after) = (over(hand.next()), over(hand.next()));
        let bow = (reach * 0.04).min(BOW * k) * hand.next();
        let steps = ((reach / (6.0 * k.max(0.01))) as usize).clamp(2, 12);
        let line: Vec<Point> = (0..=steps)
            .map(|step| {
                let u = step as f32 / steps as f32;
                let along = -before + (reach + before + after) * u;
                let side = bow * (std::f32::consts::PI * u).sin() + 0.25 * k * hand.next();
                (a.0 + dx * along - dy * side, a.1 + dy * along + dx * side)
            })
            .collect();
        self.lay(line);
    }

    /// A stroke through `ideal`, each point a little off where it was asked for: a circle, a squiggle.
    pub fn path(&mut self, ideal: &[Point]) {
        let mut hand = self.hand(0);
        let off = 0.5 * self.k;
        self.lay(ideal.iter().map(|point| (point.0 + off * hand.next(), point.1 + off * hand.next())).collect());
    }

    /// A stroke through `ideal` as wide as `width`, with no wobble of its own: what fills a small shape with the
    /// ink. A wide stroke that wobbled would show each turn as a spike.
    pub fn fill(&mut self, ideal: &[Point], width: f32) {
        self.lay(ideal.to_vec());
        if let Some(last) = self.strokes.last_mut() {
            last.width = width;
        }
    }

    /// Lays the stroke twice: `line`, and a wide pass under it whose wobble differs a little.
    fn lay(&mut self, line: Vec<Point>) {
        let mut hand = self.hand(1);
        let off = 0.5 * self.k;
        let soft = line.iter().map(|point| (point.0 + off * hand.next(), point.1 + off * hand.next())).collect();
        // Until `finish`, `start` and `length` are in pixels drawn: every stroke costs its length and a lift.
        let start = self.strokes.last().map_or(0.0, |last| last.start + last.length);
        let length = long(&line) + 10.0 * self.k;
        let (ink, rocks, width) = (self.ink, self.rocks, WIDTH * self.k);
        self.strokes.push(Stroke { ink, line, soft, width, start, length, rocks });
    }

    /// The strokes, each with its turn in the seconds the picture takes.
    pub fn finish(mut self) -> Vec<Stroke> {
        let whole = self.strokes.last().map_or(0.0, |last| last.start + last.length);
        let seconds = DRAWN_IN.min(0.3 * self.strokes.len() as f32);
        let rate = if whole > 0.0 { seconds / whole } else { 0.0 };
        for stroke in &mut self.strokes {
            (stroke.start, stroke.length) = (stroke.start * rate, stroke.length * rate);
        }
        self.strokes
    }

    /// Short strokes with gaps from `a` to `b`.
    pub fn dashes(&mut self, a: Point, b: Point) {
        let (reach, pitch, dash) = (far(a, b), 12.0 * self.k, 7.0 * self.k);
        let at =
            |along: f32| (a.0 + (b.0 - a.0) * along / reach.max(0.01), a.1 + (b.1 - a.1) * along / reach.max(0.01));
        for n in 0..((reach + pitch - dash) / pitch.max(0.01)) as usize {
            self.line(at(n as f32 * pitch), at((n as f32 * pitch + dash).min(reach)));
        }
    }

    /// A box: four strokes, each past its corners. `dashed` draws each side in dashes.
    pub fn frame(&mut self, rect: Rect, dashed: bool) {
        let corners = rect.corners(0.0);
        for side in 0..4 {
            let (a, b) = (corners[side], corners[(side + 1) % 4]);
            if dashed { self.dashes(a, b) } else { self.line(a, b) }
        }
    }

    /// Strokes at 45 degrees inside `rect`, `gap` apart along its edges.
    pub fn hatch(&mut self, rect: Rect, gap: f32) {
        let mut along = gap;
        while gap > 0.0 && along < rect.w + rect.h {
            let top = (rect.x + along.min(rect.w), rect.y + (along - rect.w).max(0.0));
            let low = (rect.x + (along - rect.h).max(0.0), rect.y + along.min(rect.h));
            if far(top, low) > 2.0 * self.k {
                self.line(top, low);
            }
            along += gap;
        }
    }

    /// A circle in one stroke that ends a little past where it began.
    pub fn circle(&mut self, centre: Point, radius: f32) {
        let turn = |n: usize| (-100.0 + 24.0 * n as f32).to_radians();
        let ring: Vec<Point> =
            (0..=16).map(|n| (centre.0 + radius * turn(n).cos(), centre.1 + radius * turn(n).sin())).collect();
        self.path(&ring);
    }

    /// A wavy line from `a` to `b`: a line of writing nobody reads.
    pub fn squiggle(&mut self, a: Point, b: Point) {
        let (reach, k) = (far(a, b), self.k);
        let steps = ((reach / (3.0 * k.max(0.01))) as usize).clamp(2, 40);
        let wave = |step: usize| 1.4 * k * (step as f32 * 1.9).sin();
        let at = |step: usize| {
            let u = step as f32 / steps as f32;
            (a.0 + (b.0 - a.0) * u, a.1 + (b.1 - a.1) * u + wave(step))
        };
        self.path(&(0..=steps).map(at).collect::<Vec<_>>());
    }

    /// A shaft from `a` to `b`, solid or dashed, and a head of two strokes at `b`.
    pub fn arrow(&mut self, a: Point, b: Point, dashed: bool) {
        if dashed {
            self.dashes(a, b)
        } else {
            self.line(a, b)
        }
        let (reach, head) = (far(a, b).max(0.01), 9.0 * self.k);
        let (dx, dy) = ((a.0 - b.0) / reach, (a.1 - b.1) / reach);
        for (sin, cos) in [(0.47, 0.88), (-0.47, 0.88)] {
            self.line((b.0 + head * (dx * cos - dy * sin), b.1 + head * (dx * sin + dy * cos)), b);
        }
    }

    pub fn tick(&mut self, rect: Rect) {
        let low = (rect.x + rect.w * 0.38, rect.bottom());
        self.line((rect.x, rect.y + rect.h * 0.55), low);
        self.line(low, (rect.right(), rect.y));
    }

    pub fn cross(&mut self, rect: Rect) {
        self.line((rect.x, rect.y), (rect.right(), rect.bottom()));
        self.line((rect.right(), rect.y), (rect.x, rect.bottom()));
    }

    pub fn plus(&mut self, rect: Rect) {
        let (x, y) = rect.center();
        self.line((rect.x, y), (rect.right(), y));
        self.line((x, rect.y), (x, rect.bottom()));
    }

    /// `count` tally marks from `at`, `high` tall: four upright and the fifth across them. Returns where each is.
    pub fn tally(&mut self, at: Point, count: usize, high: f32) -> Vec<Rect> {
        let k = self.k;
        let mark = |n: usize| {
            let left = at.0 + (n / 5) as f32 * 36.0 * k;
            if n % 5 < 4 {
                Rect::new(left + (n % 5) as f32 * 7.0 * k, at.1, 1.5 * k, high)
            } else {
                Rect::new(left - 3.0 * k, at.1 + high * 0.2, 28.0 * k, high * 0.55)
            }
        };
        let marks: Vec<Rect> = (0..count).map(mark).collect();
        for (n, mark) in marks.iter().enumerate() {
            if n % 5 < 4 {
                self.line((mark.right(), mark.y), (mark.x, mark.bottom()));
            } else {
                self.line((mark.x, mark.bottom()), (mark.right(), mark.y));
            }
        }
        marks
    }
}
