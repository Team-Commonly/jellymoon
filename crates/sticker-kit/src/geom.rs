//! Rectangles in logical pixels, origin at the top left.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Viewport {
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    pub fn right(&self) -> f32 {
        self.x + self.w
    }

    pub fn bottom(&self) -> f32 {
        self.y + self.h
    }

    pub fn center(&self) -> (f32, f32) {
        (self.x + self.w / 2.0, self.y + self.h / 2.0)
    }

    /// True when the two share any area; touching edges do not count.
    pub fn overlaps(&self, other: &Rect) -> bool {
        self.x < other.right() && other.x < self.right() && self.y < other.bottom() && other.y < self.bottom()
    }

    pub fn inside(&self, outer: &Rect) -> bool {
        self.x >= outer.x && self.y >= outer.y && self.right() <= outer.right() && self.bottom() <= outer.bottom()
    }

    pub fn grown(&self, by: f32) -> Rect {
        Rect::new(self.x - by, self.y - by, self.w + 2.0 * by, self.h + 2.0 * by)
    }

    pub fn moved(&self, dx: f32, dy: f32) -> Rect {
        Rect::new(self.x + dx, self.y + dy, self.w, self.h)
    }

    /// The same rectangle scaled about its own centre.
    pub fn scaled(&self, by: f32) -> Rect {
        let (cx, cy) = self.center();
        Rect::new(cx - self.w * by / 2.0, cy - self.h * by / 2.0, self.w * by, self.h * by)
    }

    /// The four corners after turning the rectangle about its centre.
    pub fn corners(&self, degrees: f32) -> [(f32, f32); 4] {
        let (cx, cy) = self.center();
        let (hw, hh) = (self.w / 2.0, self.h / 2.0);
        [(-hw, -hh), (hw, -hh), (hw, hh), (-hw, hh)].map(|(x, y)| turn((cx + x, cy + y), (cx, cy), degrees))
    }

    /// The level box that holds the turned rectangle.
    pub fn turned_bounds(&self, degrees: f32) -> Rect {
        let pts = self.corners(degrees);
        let min_x = pts.iter().map(|p| p.0).fold(f32::INFINITY, f32::min);
        let max_x = pts.iter().map(|p| p.0).fold(f32::NEG_INFINITY, f32::max);
        let min_y = pts.iter().map(|p| p.1).fold(f32::INFINITY, f32::min);
        let max_y = pts.iter().map(|p| p.1).fold(f32::NEG_INFINITY, f32::max);
        Rect::new(min_x, min_y, max_x - min_x, max_y - min_y)
    }
}

/// The part of a convex polygon with `min <= x <= max`.
pub fn clip_x(polygon: &[(f32, f32)], min: f32, max: f32) -> Vec<(f32, f32)> {
    let lower = clip_half(polygon, min, 1.0);
    clip_half(&lower, -max, -1.0)
}

/// Keeps the side where `sign * x >= bound`.
fn clip_half(polygon: &[(f32, f32)], bound: f32, sign: f32) -> Vec<(f32, f32)> {
    let mut out = Vec::with_capacity(polygon.len() + 2);
    for (i, &a) in polygon.iter().enumerate() {
        let b = polygon[(i + 1) % polygon.len()];
        let (da, db) = (sign * a.0 - bound, sign * b.0 - bound);
        if da >= 0.0 {
            out.push(a);
        }
        if (da >= 0.0) != (db >= 0.0) {
            let u = da / (da - db);
            out.push((a.0 + (b.0 - a.0) * u, a.1 + (b.1 - a.1) * u));
        }
    }
    out
}

/// Turns `p` clockwise on screen about `about`.
pub fn turn(p: (f32, f32), about: (f32, f32), degrees: f32) -> (f32, f32) {
    let (s, c) = degrees.to_radians().sin_cos();
    let (dx, dy) = (p.0 - about.0, p.1 - about.1);
    (about.0 + dx * c - dy * s, about.1 + dx * s + dy * c)
}
