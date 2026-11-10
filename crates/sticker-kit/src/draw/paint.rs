//! The flat layer under the stickers: paper, grid, holes, tape, notes and
//! tags. Level shapes are quads; tilted paper and tape are drawn paths,
//! the only things GPUI can turn.

use gpui::{BorderStyle, Bounds, BoxShadow, Corners, PathBuilder, Pixels, Point, Window, fill, point, px, quad, size};

use super::measure::hsla;
use crate::color::Color;
use crate::geom::{Rect, clip_x, turn};

#[derive(Clone, Copy, Debug)]
pub(crate) struct Shadow {
    pub color: Color,
    pub drop: f32,
    pub blur: f32,
    pub spread: f32,
}

#[derive(Clone, Debug)]
pub(crate) enum Shape {
    /// A level rounded rectangle, optionally with a grid drawn inside.
    Slab {
        rect: Rect,
        radius: f32,
        fill: Color,
        edge: Option<(Color, f32)>,
        shadow: Option<Shadow>,
        grid: Option<(f32, Color)>,
    },
    /// A sheet of paper that may be tilted.
    Sheet { rect: Rect, tilt: f32, fill: Color, shadow: Shadow },
    /// A strip of two-colour striped tape that may be tilted.
    Tape { rect: Rect, tilt: f32, a: Color, b: Color },
}

fn bounds(rect: Rect, origin: Point<Pixels>) -> Bounds<Pixels> {
    Bounds::new(point(origin.x + px(rect.x), origin.y + px(rect.y)), size(px(rect.w), px(rect.h)))
}

fn box_shadow(shadow: Shadow) -> BoxShadow {
    BoxShadow {
        color: hsla(shadow.color),
        offset: point(px(0.0), px(shadow.drop)),
        blur_radius: px(shadow.blur),
        spread_radius: px(shadow.spread),
        inset: false,
    }
}

pub(crate) fn paint(shapes: &[Shape], origin: Point<Pixels>, window: &mut Window) {
    for shape in shapes {
        match shape {
            Shape::Slab { rect, radius, fill, edge, shadow, grid } => {
                let at = bounds(*rect, origin);
                let radii = Corners::all(px(*radius));
                if let Some(shadow) = shadow {
                    window.paint_drop_shadows(at, radii, &[box_shadow(*shadow)]);
                }
                let (edge_color, edge_width) = edge.unwrap_or((*fill, 0.0));
                window.paint_quad(quad(
                    at,
                    radii,
                    hsla(*fill),
                    px(edge_width),
                    hsla(edge_color),
                    BorderStyle::default(),
                ));
                if let Some((step, color)) = grid {
                    paint_grid(*rect, *step, *color, origin, window);
                }
            }
            Shape::Sheet { rect, tilt, fill, shadow } => paint_sheet(*rect, *tilt, *fill, *shadow, origin, window),
            Shape::Tape { rect, tilt, a, b } => paint_tape(*rect, *tilt, *a, *b, origin, window),
        }
    }
}

fn paint_grid(rect: Rect, step: f32, color: Color, origin: Point<Pixels>, window: &mut Window) {
    if step < 2.0 || color.a == 0 {
        return;
    }
    let ink = hsla(color);
    let mut x = rect.x + step;
    while x < rect.right() - 1.0 {
        window.paint_quad(fill(bounds(Rect::new(x, rect.y, 1.0, rect.h), origin), ink));
        x += step;
    }
    let mut y = rect.y + step;
    while y < rect.bottom() - 1.0 {
        window.paint_quad(fill(bounds(Rect::new(rect.x, y, rect.w, 1.0), origin), ink));
        y += step;
    }
}

fn paint_polygon(points: &[(f32, f32)], color: Color, origin: Point<Pixels>, window: &mut Window) {
    if points.len() < 3 {
        return;
    }
    let points: Vec<Point<Pixels>> = points.iter().map(|p| point(origin.x + px(p.0), origin.y + px(p.1))).collect();
    let mut builder = PathBuilder::fill();
    builder.add_polygon(&points, true);
    if let Ok(path) = builder.build() {
        window.paint_path(path, hsla(color));
    }
}

fn paint_sheet(rect: Rect, tilt: f32, fill: Color, shadow: Shadow, origin: Point<Pixels>, window: &mut Window) {
    // A path cannot be blurred, so the shadow is a level blurred box under
    // the sheet, pulled in far enough that a small tilt still covers it.
    let at = bounds(rect, origin);
    window.paint_drop_shadows(at, Corners::all(px(0.0)), &[box_shadow(shadow)]);
    if tilt == 0.0 {
        window.paint_quad(gpui::fill(at, hsla(fill)));
    } else {
        paint_polygon(&rect.corners(tilt), fill, origin, window);
    }
}

fn paint_tape(rect: Rect, tilt: f32, a: Color, b: Color, origin: Point<Pixels>, window: &mut Window) {
    let centre = rect.center();
    let mut base = a;
    base.a = 235;
    paint_polygon(&rect.corners(tilt), base, origin, window);
    if a == b {
        return;
    }
    let mut stripe_color = b;
    stripe_color.a = 235;
    let band = (rect.h * 0.4).max(4.0);
    let mut start = 0.0;
    while start < rect.w + rect.h {
        let stripe = [(start, 0.0), (start + band, 0.0), (start + band - rect.h, rect.h), (start - rect.h, rect.h)];
        let turned: Vec<(f32, f32)> = clip_x(&stripe, 0.0, rect.w)
            .into_iter()
            .map(|(u, v)| turn((rect.x + u, rect.y + v), centre, tilt))
            .collect();
        paint_polygon(&turned, stripe_color, origin, window);
        start += 2.0 * band;
    }
}
