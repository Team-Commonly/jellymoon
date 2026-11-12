//! A theme's SVG may only be a flat picture: shapes, colours and gradients. The door reads it with the renderer's own
//! parser (`usvg`, at the version GPUI draws with: `docs/adr/0015-the-art-door-reads-with-the-renderers-parser.md`)
//! and judges the tree that parser makes, which is the tree the renderer draws. So there is no second reading of a
//! file for the two to differ in: a size, a shape and a stroke are what `usvg` says they are.
//!
//! Three things are checked, each an allow-list. The file's bytes, before any parse (`load.rs`). The XML document:
//! every element is one of [`ELEMENTS`] and every attribute one of [`ATTRIBUTES`], which bounds what the parser is
//! asked to build and leaves out what it would load (`image`, the fonts of `text`); and what its paths and shapes
//! write, which bounds what the parser does by the length of a value (`svg_text.rs`). Then the tree: groups and
//! paths only, no layer, each path near the picture and its stroke no thinner than a drawn pixel, and the two
//! counts of [`ArtCost`].
//!
//! The kit's own art (`art.rs`) is built in code from numbers and colours and does not come through here. It alone
//! blurs a shadow, scales and fades a group: the renderer panics on a blurred group whose box overflows and keeps a
//! layer of memory for every faded group, and a file could ask for either.

use usvg::roxmltree::{Document, ParsingOptions};
use usvg::tiny_skia_path::{PathSegment, Point};
use usvg::{Group, ImageHrefResolver, Node, Options, Paint, Path, Tree};

use super::svg_text::Written;

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

/// The smallest width and height of a picture. It is drawn at twice its size, cut to whole pixels: under half a
/// pixel a side that is none, and the renderer refuses it.
pub const MIN_ART_SIDE: f32 = 0.5;

/// How far past the picture's edges a path may reach with its stroke, in the picture's own pixels: the path's box
/// after every transform, and twice the stroke's drawn width, which is as far as a mitre's point goes. A shape far
/// larger than its picture is walked all along for the little of it that shows.
pub const MAX_ART_REACH: f32 = 4096.0;

/// The thinnest stroke, in the picture's own pixels: just over half of one, which is one drawn pixel. At a drawn
/// pixel or under the renderer draws a hairline, ten times slower by the line (measured).
pub const MIN_ART_STROKE: f32 = 0.51;

/// The most numbers in the paths of one art file, and of a theme's art files together: 1536 points. Every shape is
/// counted as the path it is drawn as, so 99 circles are 2574 of them. See [`ArtCost`].
pub const MAX_ART_NUMBERS: usize = 3072;

/// The most paint in one art file, and in a theme's art files together: 64 fills of 256 by 256 with no edge that
/// crosses a row, or 56 that cover the picture. See [`ArtCost`].
pub const MAX_ART_PAINT: u64 = 64 * 256 * 256;

/// What one row costs that one point's edge crosses, in paint: as much as 16 pixels of a plain fill. The renderer
/// walks every edge of a fill down every row it crosses and paints the picture in the pieces the edges cut a row
/// into, so a ragged shape as tall as its picture is far dearer than its area (measured: `README.md`).
pub const ROW_PAINT: u64 = 16;

/// What an art file costs to draw, counted on the tree the renderer draws. Each way the kit knows to spend the two
/// to the last unit is drawn in `tests/draw.rs`, and the times are in the README: they bound what was measured,
/// and are not a proof that no dearer picture is inside them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ArtCost {
    /// Two for every point of every path, once the parser has made each shape a path: a `rect` is 4 points, a
    /// `circle` 13, an arc as many as the curves it is drawn as.
    pub numbers: usize,
    /// The picture's width times its height, for every fill and every stroke: a shape may paint all of the picture
    /// however small it is written. And [`ROW_PAINT`] for every row of the picture an edge of the shape crosses,
    /// twice for a stroke, which has two sides. A gradient counts all of it four times.
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
                "its paint (fills and strokes x width x height, and {ROW_PAINT} for each row an edge crosses; x 4 for \
                 a gradient) is {paint}, over {MAX_ART_PAINT}"
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
    let read = Tree::from_xmltree(&document, &options());
    counted(&read.map_err(|e| format!("it is not a picture the renderer reads: {e}"))?)
}

/// The renderer loads the image an `<image` names and the fonts a `<text` asks for. Neither element passes
/// `listed`, so nothing is asked of either; with these an image is not loaded all the same.
fn options() -> Options<'static> {
    let resolver = ImageHrefResolver { resolve_data: Box::new(|_, _, _| None), resolve_string: Box::new(|_, _| None) };
    Options { image_href_resolver: resolver, ..Options::default() }
}

/// Judges the tree the renderer draws, and counts it.
fn counted(tree: &Tree) -> Result<ArtCost, String> {
    let (wide, high) = (tree.size().width(), tree.size().height());
    if wide > MAX_ART_SIDE || high > MAX_ART_SIDE {
        return Err(format!("the renderer sizes it {wide} by {high}: a picture is at most {MAX_ART_SIDE} a side"));
    }
    if wide < MIN_ART_SIDE || high < MIN_ART_SIDE {
        return Err(format!("the renderer sizes it {wide} by {high}: a picture is at least {MIN_ART_SIDE} a side"));
    }
    let mut tally = Tally { area: wide.ceil() as u64 * high.ceil() as u64, wide, high, cost: ArtCost::default() };
    tally.group(tree.root())?;
    tally.cost.refusal().map_or(Ok(tally.cost), Err)
}

/// Holds the document to the two lists, a count and a depth, before the parser builds a picture from it.
fn listed(document: &Document) -> Result<(), String> {
    let (mut path_bytes, mut written) = (0, Written::default());
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
            if path_bytes > MAX_ART_PATH_BYTES {
                return Err(format!("its `d` and `points` values are over {MAX_ART_PATH_BYTES} bytes together"));
            }
            written.attribute(name, attribute.value())?;
        }
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
        let rows = self.rows(path);
        if let Some(fill) = path.fill() {
            self.paint(fill.paint(), rows)?;
        }
        // How far past the path its stroke is drawn.
        let mut past = 0.0;
        if let Some(stroke) = path.stroke() {
            // How long the renderer draws a length of 1 of this path, along each axis.
            let to = path.abs_transform();
            let (along, across) = (to.sx.hypot(to.ky), to.kx.hypot(to.sy));
            let drawn = stroke.width().get() * along.min(across);
            if stroke.dasharray().is_some() || drawn < MIN_ART_STROKE {
                return Err(format!("a stroke is dashed, or drawn {drawn} wide, thinner than {MIN_ART_STROKE}"));
            }
            // Half its width, and at the point of a mitre as many times that as the mitre's limit. The renderer's
            // own box of a stroke is not asked: under a slant it is the path's box and no more.
            past = stroke.width().get() * along.max(across) * (stroke.miterlimit().get() / 2.0).max(1.0);
            // A stroke has two sides.
            self.paint(stroke.paint(), 2 * rows)?;
        }
        let around = path.abs_bounding_box();
        let near = around.left().min(around.top()) - past;
        let far = (around.right() - self.wide).max(around.bottom() - self.high) + past;
        // Written this way round, a box that is not a number is refused too.
        if near >= -MAX_ART_REACH && far <= MAX_ART_REACH {
            return Ok(());
        }
        Err(format!("a shape reaches more than {MAX_ART_REACH} past the picture's edge"))
    }

    /// One fill or one stroke: the picture's area, and [`ROW_PAINT`] for each of `rows`.
    fn paint(&mut self, paint: &Paint, rows: u64) -> Result<(), String> {
        let times = match paint {
            Paint::Color(_) => 1,
            Paint::LinearGradient(_) | Paint::RadialGradient(_) => 4,
            Paint::Pattern(_) => return Err("it paints with a pattern".into()),
        };
        self.cost.paint += times * (self.area + ROW_PAINT * rows);
        Ok(())
    }

    /// How many of the picture's rows the edges of a path cross, one after another, where the path is drawn. A
    /// curve is counted by the points that pull it, which is no less, and every piece as a fill closes it.
    fn rows(&self, path: &Path) -> u64 {
        let to = path.abs_transform();
        let row = |point: Point| (to.ky * point.x + to.sy * point.y + to.ty).clamp(0.0, self.high);
        // The row a piece began on, the row its last point is on, and the rows crossed so far.
        let (mut began, mut on, mut rows) = (0.0_f32, 0.0_f32, 0.0_f32);
        for segment in path.data().segments() {
            let through = match segment {
                PathSegment::MoveTo(point) => {
                    rows += (began - on).abs();
                    (began, on) = (row(point), row(point));
                    continue;
                }
                PathSegment::LineTo(point) => [point; 3],
                PathSegment::QuadTo(pull, point) => [pull, point, point],
                PathSegment::CubicTo(one, two, point) => [one, two, point],
                PathSegment::Close => {
                    rows += (began - on).abs();
                    on = began;
                    continue;
                }
            };
            for point in through {
                rows += (row(point) - on).abs();
                on = row(point);
            }
        }
        (rows + (began - on).abs()).ceil() as u64
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::{Group, Node, Options, Tree, counted, options};

    /// A picture that holds another, written into it.
    const IMAGE: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 8 8"><image width="8" height="8" href="data:image/svg+xml;utf8,&lt;svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 8 8'&gt;&lt;rect width='8' height='8'/&gt;&lt;/svg&gt;"/></svg>"#;

    fn images(group: &Group) -> usize {
        let held = |node: &Node| match node {
            Node::Group(inner) => images(inner),
            Node::Image(_) => 1,
            _ => 0,
        };
        group.children().iter().map(held).sum()
    }

    // `listed` lets no `<image` through, so no file reaches either of these today.
    #[test]
    fn a_tree_that_holds_an_image_is_refused_and_the_door_s_own_options_load_none() {
        let loaded = Tree::from_str(IMAGE, &Options::default()).unwrap();
        assert_eq!(images(loaded.root()), 1, "the renderer's own options load it");
        assert_eq!(counted(&loaded), Err("it holds text or an image".to_string()));
        let unloaded = Tree::from_str(IMAGE, &options()).unwrap();
        assert_eq!(images(unloaded.root()), 0, "the door's load nothing");
        assert_eq!(counted(&unloaded).map(|cost| cost.paint), Ok(0));
    }

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
        assert_eq!(judged(rect), Ok(64 + 16 * 16));
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
