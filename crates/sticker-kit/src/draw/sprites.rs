//! Sticker art as images. Each distinct sticker is rasterised once from
//! its SVG and then only moved and scaled, which keeps a frame cheap.

use std::collections::HashMap;
use std::sync::Arc;

use gpui::{App, Image, ImageFormat};

use crate::art;
use crate::geom::Rect;
use crate::layout::{PieceKind, Placed};
use crate::scene::{Character, Mood, Scene, State};
use crate::theme::{Accepted, Theme};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct Key {
    kind: PieceKind,
    hue: u8,
    mood: Mood,
    dimmed: bool,
    size: u32,
    half_rims: u32,
}

#[derive(Default)]
pub(crate) struct Sprites {
    drawn: HashMap<Key, Arc<Image>>,
    /// Art the theme supplies from its own folder, by piece name: only what a trial accepted.
    supplied: HashMap<&'static str, Arc<Image>>,
}

/// Sizes art is rasterised at, so a window being resized reuses images
/// instead of making one per pixel of width.
const SIZES: [f32; 12] = [16.0, 24.0, 32.0, 48.0, 64.0, 96.0, 128.0, 200.0, 280.0, 400.0, 560.0, 800.0];
const MOST: usize = 96;
/// The widest rim an image is rasterised with, in half pixels: 64 px, whatever a theme and a window multiply out to.
const MOST_HALF_RIMS: f32 = 128.0;

fn svg_image(svg: String) -> Arc<Image> {
    Arc::new(Image::from_bytes(ImageFormat::Svg, svg.into_bytes()))
}

fn piece_name(kind: PieceKind) -> &'static str {
    match kind {
        PieceKind::Hero => "hero",
        PieceKind::Character => "character",
        PieceKind::Star => "star",
        PieceKind::Bubble => "bubble",
        _ => "badge",
    }
}

impl Sprites {
    /// Forgets every image and takes the theme's own art afresh. A picture is an [`Accepted`] one or it is not
    /// given to the renderer: this reads no file.
    pub fn reset(&mut self, art: &[Accepted], cx: &mut App) {
        self.forget(cx);
        for image in self.supplied.drain().map(|entry| entry.1) {
            image.remove_asset(cx);
        }
        for accepted in art {
            self.supplied.insert(accepted.piece(), svg_image(accepted.svg().to_string()));
        }
    }

    fn forget(&mut self, cx: &mut App) {
        for image in self.drawn.drain().map(|entry| entry.1) {
            image.remove_asset(cx);
        }
    }

    /// The image for a piece and the rectangle it covers, which is larger
    /// than the piece by the room its rim and shadow need.
    pub fn get(&mut self, piece: &Placed, scene: &Scene, theme: &Theme, rim: f32, cx: &mut App) -> (Arc<Image>, Rect) {
        if let Some(image) = self.supplied.get(piece_name(piece.piece)) {
            return (image.clone(), piece.rect);
        }
        let width = piece.rect.w.max(1.0);
        let (size, half_rims) = raster(width, rim);
        let grow = size / width;
        let who = match piece.piece {
            PieceKind::Hero => scene.hero.as_ref(),
            PieceKind::Character => scene.characters.get(piece.index),
            _ => None,
        };
        let key = Key {
            kind: piece.piece,
            hue: who.map_or(0, |c| c.hue),
            mood: who.map_or(Mood::Happy, |c| c.mood),
            dimmed: who.is_some_and(|c| c.state == State::Dimmed),
            size: size as u32,
            half_rims,
        };
        let rim_at_size = half_rims as f32 / 2.0;
        if self.drawn.len() >= MOST && !self.drawn.contains_key(&key) {
            self.forget(cx);
        }
        let image =
            self.drawn.entry(key).or_insert_with(|| svg_image(draw(key, who, theme, size, rim_at_size))).clone();
        (image, piece.rect.grown(art::pad(rim_at_size) / grow))
    }
}

/// The size a piece `width` wide is rasterised at, and its rim there in half pixels. A piece wider than the largest
/// size is drawn from that size, scaled up: no theme or window buys more pixels than 800 a side and a rim of 64.
fn raster(width: f32, rim: f32) -> (f32, u32) {
    let size = SIZES.iter().copied().find(|s| *s >= width).unwrap_or(SIZES[SIZES.len() - 1]);
    (size, (rim * size / width * 2.0).round().clamp(1.0, MOST_HALF_RIMS) as u32)
}

fn draw(key: Key, who: Option<&Character>, theme: &Theme, size: f32, rim: f32) -> String {
    let palette = &theme.palette;
    match key.kind {
        PieceKind::Hero | PieceKind::Character => {
            let hue = if key.dimmed { palette.dimmed } else { theme.hue(who.map_or(0, |c| c.hue)) };
            art::character(palette, hue, key.mood, key.dimmed, size, rim)
        }
        PieceKind::Star => art::star(palette, size / 2.0, rim),
        PieceKind::Bubble => art::bubble(palette, size / 2.0, rim),
        _ => art::badge(palette, size / 2.0, rim),
    }
}

#[cfg(test)]
mod tests {
    use super::raster;

    /// What bounds the cost of the kit's own art, counted: the pixels of one image, whatever is asked for.
    #[test]
    fn no_piece_is_rasterised_larger_than_800_with_a_rim_of_64() {
        assert_eq!(raster(92.0, 5.0), (96.0, 10));
        assert_eq!(raster(800.0, 64.0), (800.0, 128));
        for (width, rim) in [(801.0, 5.0), (3600.0, 180.0), (163_840.0, 400_000.0), (f32::MAX, f32::MAX), (1.0, 1e9)] {
            let (size, half_rims) = raster(width, rim);
            assert!(size <= 800.0 && half_rims <= 128, "{width} {rim}: {size} {half_rims}");
        }
    }
}
