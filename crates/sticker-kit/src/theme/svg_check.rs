//! A theme's SVG may only be a flat picture: shapes, colours and gradients. The door reads it with the renderer's own
//! parser (`usvg`, at the version GPUI draws with: `docs/adr/0015-the-art-door-reads-with-the-renderers-parser.md`)
//! and judges the tree that parser makes, which is the tree the renderer draws. So there is no second reading of a
//! file for the two to differ in: a size, a shape and a stroke are what `usvg` says they are.
//!
//! Three things are checked, each an allow-list. The file's bytes, before any parse (`load.rs`). The XML document:
//! every element is one of [`ELEMENTS`] and every attribute one of [`ATTRIBUTES`], which bounds what the parser is
//! asked to build and leaves out what it would load (`image`, the fonts of `text`). Then the tree: groups and paths
//! only, no layer, each path near the picture and its stroke no thinner than a drawn pixel, and the two counts of
//! [`ArtCost`].
//!
//! The kit's own art (`art.rs`) is built in code from numbers and colours and does not come through here. It alone
//! blurs a shadow, scales and fades a group: the renderer panics on a blurred group whose box overflows and keeps a
//! layer of memory for every faded group, and a file could ask for either.

use usvg::roxmltree::{Document, ParsingOptions};
use usvg::{Group, ImageHrefResolver, Node, Options, Paint, Path, Tree};

/// The elements a picture is made of, as SVG writes them. Any other element is refused.
pub const ELEMENTS: &str =
    "svg g defs title desc path rect circle ellipse line polyline polygon linearGradient radialGradient stop";

/// The attributes an element may carry. Any other is refused: `style`, `href`, `filter`, `opacity`, every `on…`
/// handler, `xml:base`, `stroke-dasharray` and `stroke-miterlimit` among them.
pub const ATTRIBUTES: &str = "id fill stroke stop-color fill-rule stroke-linecap stroke-linejoin gradientUnits version \
     width height viewBox x y x1 y1 x2 y2 cx cy r rx ry d points transform gradientTransform fill-opacity \
     stroke-width stroke-opacity offset stop-opacity";

const SVG: &str = "http://www.w3.org/2000/svg";

/// The most elements in one art file, the `<svg` counted. The kit's largest piece has under 40.
pub const MAX_ART_ELEMENTS: usize = 100;

/// How deep elements may nest, the `<svg` counted. The kit's own art goes 5 deep.
pub const MAX_ART_DEPTH: usize = 12;

/// The most bytes in the `d` and `points` values of one file together: 16 KiB. It bounds what the parser spends on
/// a file before its numbers can be counted.
pub const MAX_ART_PATH_BYTES: usize = 16 * 1024;

/// The largest width and height of a picture, as the renderer sizes it. It is drawn at twice that.
pub const MAX_ART_SIDE: f32 = 256.0;

/// How far past the picture's edges a path may reach with its stroke, in the picture's own pixels. A shape far
/// larger than its picture is walked all along for the little of it that shows.
pub const MAX_ART_REACH: f32 = 4096.0;

/// The thinnest stroke, in the picture's own pixels: just over half of one, which is one drawn pixel. At a drawn
/// pixel or under the renderer draws a hairline, ten times slower by the line (measured).
pub const MIN_ART_STROKE: f32 = 0.51;

/// The most numbers in the paths of one art file, and of a theme's art files together: 1536 points. Every shape is
/// counted as the path it is drawn as, so 99 circles are 2574 of them. See [`ArtCost`].
pub const MAX_ART_NUMBERS: usize = 3072;

/// The most paint in one art file, and in a theme's art files together: 64 fills of 256 by 256. See [`ArtCost`].
pub const MAX_ART_PAINT: u64 = 64 * 256 * 256;

/// What an art file costs to draw, counted on the tree the renderer draws. Drawing time follows these two and
/// nothing else the door lets through.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ArtCost {
    /// Two for every point of every path, once the parser has made each shape a path: a `rect` is 4 points, a
    /// `circle` 13, an arc as many as the curves it is drawn as.
    pub numbers: usize,
    /// The picture's width times its height, for every fill and every stroke: a shape may paint all of the picture
    /// however small it is written. A gradient counts as four.
    pub paint: u64,
}

impl ArtCost {
    /// This and more, together.
    pub fn with(self, more: ArtCost) -> ArtCost {
        ArtCost { numbers: self.numbers + more.numbers, paint: self.paint + more.paint }
    }

    /// Why this much is refused, or `None` when it is inside [`MAX_ART_NUMBERS`] and [`MAX_ART_PAINT`].
    pub fn refusal(&self) -> Option<String> {
        if self.numbers > MAX_ART_NUMBERS {
            return Some(format!("its paths hold more than {MAX_ART_NUMBERS} numbers"));
        }
        let paint = self.paint;
        (paint > MAX_ART_PAINT).then(|| {
            format!(
                "its paint (fills and strokes x width x height, x 4 for a gradient) is {paint}, over {MAX_ART_PAINT}"
            )
        })
    }
}

/// Why the SVG is refused, or `None` when it is only a picture.
pub fn svg_refusal(svg: &str) -> Option<String> {
    art_cost(svg).err()
}

/// What the SVG costs to draw, or why it is refused: for what it holds, or for costing too much.
pub fn art_cost(svg: &str) -> Result<ArtCost, String> {
    // The renderer allows a DTD. A file with one is refused here, so that no entity is ever expanded.
    let xml = ParsingOptions { allow_dtd: false, ..ParsingOptions::default() };
    let document =
        Document::parse_with_options(svg, xml).map_err(|e| format!("it is not XML the renderer reads: {e}"))?;
    listed(&document)?;
    // The renderer loads the image an `<image` names and the fonts a `<text` asks for. Neither element passes
    // `listed`, so nothing is asked of either; an image is refused here all the same.
    let resolver = ImageHrefResolver { resolve_data: Box::new(|_, _, _| None), resolve_string: Box::new(|_, _| None) };
    let options = Options { image_href_resolver: resolver, ..Options::default() };
    let read = Tree::from_xmltree(&document, &options);
    counted(&read.map_err(|e| format!("it is not a picture the renderer reads: {e}"))?)
}

/// Judges the tree the renderer draws, and counts it.
fn counted(tree: &Tree) -> Result<ArtCost, String> {
    let (wide, high) = (tree.size().width(), tree.size().height());
    if wide > MAX_ART_SIDE || high > MAX_ART_SIDE {
        return Err(format!("the renderer sizes it {wide} by {high}: a picture is at most {MAX_ART_SIDE} a side"));
    }
    let mut tally = Tally { area: wide.ceil() as u64 * high.ceil() as u64, wide, high, cost: ArtCost::default() };
    tally.group(tree.root())?;
    tally.cost.refusal().map_or(Ok(tally.cost), Err)
}

/// Holds the document to the two lists, a count and a depth, before the parser builds a picture from it.
fn listed(document: &Document) -> Result<(), String> {
    let mut path_bytes = 0;
    for (nth, element) in document.descendants().filter(|node| node.is_element()).enumerate() {
        let name = element.tag_name();
        if name.namespace() != Some(SVG) {
            return Err(format!("its `<{}` is not in SVG's namespace", name.name()));
        }
        if !ELEMENTS.split(' ').any(|known| known == name.name()) {
            return Err(format!("it contains the element `<{}`", name.name()));
        }
        if nth >= MAX_ART_ELEMENTS {
            return Err(format!("it has more than {MAX_ART_ELEMENTS} elements"));
        }
        if name.name() == "svg" && nth > 0 {
            return Err("it contains a second `<svg`".into());
        }
        // Counted after the elements are, so the walk up is over a hundred of them at most.
        if element.ancestors().filter(|node| node.is_element()).count() > MAX_ART_DEPTH {
            return Err(format!("its elements are nested more than {MAX_ART_DEPTH} deep"));
        }
        for attribute in element.attributes() {
            let name = attribute.name();
            if attribute.namespace().is_some() || !ATTRIBUTES.split_whitespace().any(|known| known == name) {
                return Err(format!("it contains the attribute `{name}`"));
            }
            path_bytes += if matches!(name, "d" | "points") { attribute.value().len() } else { 0 };
        }
    }
    if path_bytes > MAX_ART_PATH_BYTES {
        return Err(format!("its `d` and `points` values are over {MAX_ART_PATH_BYTES} bytes together"));
    }
    Ok(())
}

/// What is counted while the tree is walked.
struct Tally {
    area: u64,
    wide: f32,
    high: f32,
    cost: ArtCost,
}

impl Tally {
    /// Groups and paths only. A group that fades, clips, masks, filters or blends is a layer of memory to the
    /// renderer, and whatever is neither a group nor a path is text or an image.
    fn group(&mut self, group: &Group) -> Result<(), String> {
        for node in group.children() {
            match node {
                Node::Group(inner) if inner.should_isolate() => {
                    return Err("it holds a group that is faded, clipped, masked, filtered or blended".into());
                }
                Node::Group(inner) => self.group(inner)?,
                Node::Path(path) => self.path(path)?,
                _ => return Err("it holds text or an image".into()),
            }
        }
        Ok(())
    }

    fn path(&mut self, path: &Path) -> Result<(), String> {
        self.cost.numbers += 2 * path.data().points().len();
        let mut paints = Vec::from_iter(path.fill().map(usvg::Fill::paint));
        if let Some(stroke) = path.stroke() {
            // How long the renderer draws a length of 1 of this path, along each axis: the shorter is taken.
            let to = path.abs_transform();
            let drawn = stroke.width().get() * to.sx.hypot(to.ky).min(to.kx.hypot(to.sy));
            if stroke.dasharray().is_some() || drawn < MIN_ART_STROKE {
                return Err(format!("a stroke is dashed, or drawn {drawn} wide, thinner than {MIN_ART_STROKE}"));
            }
            paints.push(stroke.paint());
        }
        for paint in paints {
            self.cost.paint += match paint {
                Paint::Color(_) => self.area,
                Paint::LinearGradient(_) | Paint::RadialGradient(_) => 4 * self.area,
                Paint::Pattern(_) => return Err("it paints with a pattern".into()),
            };
        }
        let around = path.abs_stroke_bounding_box();
        let (near, far) =
            (around.left().min(around.top()), (around.right() - self.wide).max(around.bottom() - self.high));
        if near < -MAX_ART_REACH || far > MAX_ART_REACH {
            return Err(format!("a shape reaches more than {MAX_ART_REACH} past the picture's edge"));
        }
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::{Options, Tree, counted};

    // The two lists keep each of these out before a tree is made, so no file reaches these refusals today. The
    // tree is judged all the same: it is what is drawn, and a list may one day be longer.
    #[test]
    fn a_tree_with_a_layer_a_dash_or_a_pattern_is_refused_whatever_the_lists_let_through() {
        let judged = |inside: &str| {
            let root = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 8 8">"#;
            counted(&Tree::from_str(&format!("{root}{inside}</svg>"), &Options::default()).unwrap())
                .map(|cost| cost.paint)
        };
        let rect = r#"<rect width="8" height="8"/>"#;
        assert_eq!(judged(rect), Ok(64));
        let layer = Err("it holds a group that is faded, clipped, masked, filtered or blended".to_string());
        assert_eq!(judged(&format!(r#"<g opacity="0.5">{rect}</g>"#)), layer);
        assert_eq!(judged(&format!(r#"<clipPath id="c">{rect}</clipPath><g clip-path="url(#c)">{rect}</g>"#)), layer);
        assert_eq!(judged(&format!(r#"<g style="mix-blend-mode:multiply">{rect}</g>"#)), layer);
        let dashed = judged(r##"<path d="M0 0H8" stroke="#000" stroke-dasharray="1 2"/>"##).unwrap_err();
        assert!(dashed.starts_with("a stroke is dashed"), "{dashed}");
        let pattern = format!(
            r#"<pattern id="p" width="1" height="1">{rect}</pattern><rect width="8" height="8" fill="url(#p)"/>"#
        );
        assert_eq!(judged(&pattern), Err("it paints with a pattern".to_string()));
    }
}
