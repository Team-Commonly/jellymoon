//! The kit's own art, as SVG text built from theme colours: each piece is a
//! flat shape with a white rim and a soft shadow, like a die-cut sticker.
//! Only numbers and colours go in; no scene text ever reaches an SVG.

use std::f32::consts::PI;

use crate::color::Color;
use crate::scene::Mood;
use crate::theme::{Hue, Palette};

const BODY: &str = "M-100 96 C-100 20,-58 -14,0 -14 C58 -14,100 20,100 96 C100 118,86 132,66 128 \
Q49.5 141 33 128 Q16.5 141 0 128 Q-16.5 141 -33 128 Q-49.5 141 -66 128 C-86 132,-100 118,-100 96 Z";

/// Room around a piece's shape for its rim and shadow, in pixels.
pub fn pad(rim: f32) -> f32 {
    rim + 12.0
}

fn fill(color: Color) -> String {
    format!("fill=\"{}\" fill-opacity=\"{:.3}\"", color.hex_rgb(), color.opacity())
}

/// Wraps a shape drawn in its own units: a blurred shadow of the outline,
/// the outline again as a wide rim, then the art.
/// `origin` is where the shape's (0, 0) lands and `unit` its scale.
fn sticker(
    size: (f32, f32),
    origin: (f32, f32),
    unit: f32,
    rim: f32,
    palette: &Palette,
    outline: &str,
    art: &str,
) -> String {
    let (w, h) = size;
    let stroke = 2.0 * rim / unit;
    let shadow = palette.shadow;
    let rim_hex = palette.rim.hex_rgb();
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w:.2}\" height=\"{h:.2}\" viewBox=\"0 0 {w:.2} {h:.2}\">\
<defs><filter id=\"soft\" x=\"-40%\" y=\"-40%\" width=\"180%\" height=\"180%\">\
<feGaussianBlur stdDeviation=\"{blur:.3}\"/></filter></defs>\
<g transform=\"translate({ox:.2} {oy:.2}) scale({unit:.4})\">\
<g transform=\"translate(0 {drop:.3})\" filter=\"url(#soft)\" opacity=\"{alpha:.3}\" \
fill=\"{sh}\" stroke=\"{sh}\" stroke-width=\"{stroke:.3}\" stroke-linejoin=\"round\">{outline}</g>\
<g fill=\"{rim_hex}\" stroke=\"{rim_hex}\" stroke-width=\"{stroke:.3}\" stroke-linejoin=\"round\">{outline}</g>\
{art}</g></svg>",
        ox = origin.0,
        oy = origin.1,
        blur = 3.5 / unit,
        drop = 4.5 / unit,
        alpha = shadow.opacity(),
        sh = shadow.hex_rgb(),
    )
}

fn feet() -> String {
    [-51, -17, 17, 51]
        .map(|x| format!("<rect x=\"{}\" y=\"112\" width=\"22\" height=\"42\" rx=\"11\"/>", x - 11))
        .concat()
}

fn face(mood: Mood, ink: &str) -> String {
    let eyes: String = [-36, 36]
        .map(|x| {
            format!(
                "<circle cx=\"{x}\" cy=\"70\" r=\"11.5\" fill=\"{ink}\"/>\
<circle cx=\"{}\" cy=\"65.5\" r=\"4.2\" fill=\"#FFFFFF\"/>\
<circle cx=\"{}\" cy=\"74.5\" r=\"1.9\" fill=\"#FFFFFF\"/>",
                x as f32 - 3.5,
                x + 4
            )
        })
        .concat();
    let mouth = match mood {
        Mood::Happy => format!(
            "<path d=\"M-9 85 Q-4.5 93 0 85 Q4.5 93 9 85\" fill=\"none\" stroke=\"{ink}\" \
stroke-width=\"3.6\" stroke-linecap=\"round\" stroke-linejoin=\"round\"/>"
        ),
        Mood::Cross => format!(
            "<ellipse cx=\"0\" cy=\"89\" rx=\"6\" ry=\"7\" fill=\"{ink}\"/>\
<path d=\"M-49 50 L-27 56 M49 50 L27 56\" stroke=\"{ink}\" stroke-width=\"4\" stroke-linecap=\"round\"/>"
        ),
    };
    eyes + &mouth
}

/// A character whose body is `width` pixels wide. The image is larger than
/// the body by `pad(rim)` on every side.
pub fn character(palette: &Palette, hue: Hue, mood: Mood, dimmed: bool, width: f32, rim: f32) -> String {
    let unit = width / 200.0;
    let pad = pad(rim);
    let (base, dark, light) = (hue.base.hex_rgb(), hue.dark.hex_rgb(), hue.light.hex_rgb());
    let outline = format!("{}<path d=\"{BODY}\"/>", feet());
    let cheeks: String = [-66, 66]
        .map(|x| format!("<ellipse cx=\"{x}\" cy=\"88\" rx=\"13\" ry=\"7.5\" {}/>", fill(palette.cheek)))
        .concat();
    let art = format!(
        "<g opacity=\"{opacity}\"><g fill=\"{dark}\">{feet}</g><path d=\"{BODY}\" fill=\"{dark}\"/>\
<path d=\"{BODY}\" fill=\"{base}\" transform=\"translate(0 -5) scale(.94 .93)\"/>\
<ellipse cx=\"-46\" cy=\"18\" rx=\"24\" ry=\"12\" fill=\"{light}\" transform=\"rotate(-30 -46 18)\"/>\
<circle cx=\"-74\" cy=\"46\" r=\"5.5\" fill=\"{light}\"/>{cheeks}{face}</g>",
        opacity = if dimmed { "0.6" } else { "1" },
        feet = feet(),
        face = face(mood, &palette.ink.hex_rgb()),
    );
    let size = (width + 2.0 * pad, 168.0 * unit + 2.0 * pad);
    sticker(size, (pad + 100.0 * unit, pad + 14.0 * unit), unit, rim, palette, &outline, &art)
}

/// A piece drawn inside a circle of `radius` pixels, centred in its image.
fn round(palette: &Palette, radius: f32, rim: f32, outline: &str, art: &str) -> String {
    let side = 2.0 * (radius + pad(rim));
    sticker((side, side), (side / 2.0, side / 2.0), radius / 10.0, rim, palette, outline, art)
}

pub fn star(palette: &Palette, radius: f32, rim: f32) -> String {
    let points: Vec<String> = (0..10)
        .map(|i| {
            let angle = -PI / 2.0 + i as f32 * PI / 5.0;
            let r = if i % 2 == 0 { 10.0 } else { 5.0 };
            format!("{:.2},{:.2}", r * angle.cos(), r * angle.sin())
        })
        .collect();
    let shape = format!("<polygon points=\"{}\"/>", points.join(" "));
    let gold = palette.star.hex_rgb();
    let art = format!(
        "<g fill=\"{gold}\" stroke=\"{gold}\" stroke-width=\"3\" stroke-linejoin=\"round\">{shape}</g>\
<circle cx=\"-2.3\" cy=\"-2.3\" r=\"1.9\" fill=\"{}\"/>",
        palette.star_shine.hex_rgb()
    );
    let outline = format!("<g stroke-width=\"3\">{shape}</g>{shape}");
    round(palette, radius, rim, &outline, &art)
}

/// The "busy" bubble: a white disc with three dots.
pub fn bubble(palette: &Palette, radius: f32, rim: f32) -> String {
    let dots: String = [-4.5, 0.0, 4.5]
        .map(|x| format!("<circle cx=\"{x}\" cy=\"0\" r=\"1.6\" fill=\"{}\"/>", palette.soft.hex_rgb()))
        .concat();
    let art = format!(
        "<circle r=\"9.1\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.8\"/>{dots}",
        palette.bubble.hex_rgb(),
        palette.bubble_ring.hex_rgb()
    );
    round(palette, radius, rim, "<circle r=\"10\"/>", &art)
}

/// The "needs you" badge: an accent disc with a question mark drawn as a
/// path, so it needs no font.
pub fn badge(palette: &Palette, radius: f32, rim: f32) -> String {
    let art = format!(
        "<circle r=\"10\" fill=\"{}\"/>\
<path d=\"M-2.9 -2.6 C-2.9 -6.4,2.9 -6.4,2.9 -2.6 C2.9 -0.3,0 -0.4,0 2\" fill=\"none\" \
stroke=\"#FFFFFF\" stroke-width=\"2\" stroke-linecap=\"round\"/>\
<circle cx=\"0\" cy=\"5.2\" r=\"1.25\" fill=\"#FFFFFF\"/>",
        palette.accent.hex_rgb()
    );
    round(palette, radius, rim, "<circle r=\"10\"/>", &art)
}
