//! A theme's SVG may only be a flat picture: shapes, colours and gradients. The check is an allow-list, so that what
//! a file costs to draw is bounded here and not left to the renderer. Every element must be one of [`ELEMENTS`] and
//! every attribute one of [`ATTRIBUTES`], which leaves out whatever runs code, reaches outside the file, holds a
//! stylesheet, or multiplies drawing work (`use`, `pattern`, `mask`, `filter`, a group's `opacity`, dashes). Every
//! number is plain and small, a transform only moves or turns, and the file is held to a number of elements, a depth
//! and a size.
//!
//! The kit's own art (`art.rs`) is built in code from numbers and colours and does not come through here. It alone
//! blurs a shadow, scales and fades a group: the renderer panics on a blurred group whose box overflows and keeps a
//! layer of memory for every faded group, and a file could ask for either.

/// The elements a picture is made of. Any other element is refused.
pub const ELEMENTS: [&str; 15] = [
    "svg",
    "g",
    "defs",
    "title",
    "desc",
    "path",
    "rect",
    "circle",
    "ellipse",
    "line",
    "polyline",
    "polygon",
    "lineargradient",
    "radialgradient",
    "stop",
];

/// The attributes an element may carry, in lower case. Any other attribute is refused: `style`, `href`, `filter`,
/// `opacity`, every `on…` handler, `xml:base`, `stroke-dasharray` and `stroke-miterlimit` among them.
pub const ATTRIBUTES: [&str; 33] = [
    "xmlns",
    "id",
    "fill",
    "stroke",
    "stop-color",
    "fill-rule",
    "stroke-linecap",
    "stroke-linejoin",
    "gradientunits",
    "version",
    "width",
    "height",
    "viewbox",
    "x",
    "y",
    "x1",
    "y1",
    "x2",
    "y2",
    "cx",
    "cy",
    "r",
    "rx",
    "ry",
    "d",
    "points",
    "transform",
    "gradienttransform",
    "fill-opacity",
    "stroke-width",
    "stroke-opacity",
    "offset",
    "stop-opacity",
];

/// How many of [`ATTRIBUTES`], from the start, hold words and colours. The rest hold numbers.
const WORDS: usize = 9;

/// The most elements in one art file, the `<svg` counted. The kit's largest piece has under 40.
pub const MAX_ART_ELEMENTS: usize = 100;

/// How deep elements may nest, the `<svg` counted. The kit's own art goes 5 deep.
pub const MAX_ART_DEPTH: usize = 12;

/// The largest width and height of a picture, in its own pixels. It is drawn at twice that.
pub const MAX_ART_SIDE: f32 = 256.0;

/// The largest number anywhere else in a picture: a coordinate, a radius, a stroke's width, a move.
pub const MAX_ART_NUMBER: f32 = 4096.0;

/// The longest `d` or `points` value, in bytes: 4 KiB.
pub const MAX_ART_PATH_BYTES: usize = 4096;

/// The letters a `d` may hold: the path commands, in lower case. No other number may stand beside a letter.
const PATH_COMMANDS: &str = "mlhvcsqtaz";

/// The smallest width and height of a `viewBox`. A smaller one draws every unit hundreds of pixels wide.
pub const MIN_ART_VIEW: f32 = 8.0;

/// The most numbers in the `d` and `points` values of one art file, and of a theme's art files together. A number is
/// at most one line, or a share of one curve, for the renderer to walk on every row of pixels it crosses.
pub const MAX_ART_NUMBERS: usize = 1024;

/// The most paint in one art file, and in a theme's art files together: 64 shapes on 256 by 256. See [`ArtCost`].
pub const MAX_ART_PAINT: u64 = 64 * 256 * 256;

/// The elements that are painted. The others group, name or colour them.
const SHAPES: [&str; 7] = ["path", "rect", "circle", "ellipse", "line", "polyline", "polygon"];

/// What an art file costs to draw, counted from its text. Drawing time follows these two and nothing else the door
/// lets through: a shape is painted over as much of the picture as it likes, and a gradient costs about four fills.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ArtCost {
    /// The numbers in every `d` and `points`.
    pub numbers: usize,
    /// The shapes times the picture's width times its height, and four times that in a file with a gradient.
    pub paint: u64,
}

impl ArtCost {
    /// Why this much is refused, or `None` when it is inside [`MAX_ART_NUMBERS`] and [`MAX_ART_PAINT`].
    pub fn refusal(&self) -> Option<String> {
        if self.numbers > MAX_ART_NUMBERS {
            return Some(format!("its paths hold more than {MAX_ART_NUMBERS} numbers"));
        }
        let paint = self.paint;
        (paint > MAX_ART_PAINT).then(|| {
            format!("its paint (shapes x width x height, x 4 with a gradient) is {paint}, over {MAX_ART_PAINT}")
        })
    }
}

/// What is counted while the tags are walked.
#[derive(Default)]
struct Tally {
    numbers: usize,
    shapes: u64,
    gradient: bool,
    /// The picture's `width` and `height`, and the size in its `viewBox`.
    size: [Option<f32>; 2],
    view: [Option<f32>; 2],
}

impl Tally {
    /// Keeps the sizes the `<svg` states. They were checked as sizes already.
    fn root(&mut self, name: &str, value: &str) {
        let side = |text: &str| text.trim_end_matches("px").parse::<f32>().ok();
        match name {
            "width" => self.size[0] = side(value),
            "height" => self.size[1] = side(value),
            "viewbox" => {
                let mut last = value.split([' ', ',']).filter(|part| !part.is_empty()).skip(2).map(side);
                self.view = [last.next().flatten(), last.next().flatten()];
            }
            _ => {}
        }
    }

    /// A picture with no size of its own takes its `viewBox`'s; with neither it is counted at the largest.
    fn cost(&self) -> ArtCost {
        let side = |i: usize| self.size[i].or(self.view[i]).unwrap_or(MAX_ART_SIDE).ceil() as u64;
        let fills = if self.gradient { 4 } else { 1 };
        ArtCost { numbers: self.numbers, paint: self.shapes * fills * side(0) * side(1) }
    }
}

/// Why the SVG is refused, or `None` when it is only a picture.
pub fn svg_refusal(svg: &str) -> Option<String> {
    art_cost(svg).err()
}

/// What the SVG costs to draw, or why it is refused: for what it holds, or for costing too much.
pub fn art_cost(svg: &str) -> Result<ArtCost, String> {
    let text = svg.to_ascii_lowercase();
    // With no character reference, what the checks below read is what the renderer reads.
    let banned = [("<!doctype", "a DOCTYPE"), ("<!entity", "an entity declaration"), ("&#", "a character reference")];
    for (needle, what) in banned {
        if text.contains(needle) {
            return Err(format!("it contains {what}"));
        }
    }
    let cost = tags(&text)?;
    cost.refusal().map_or(Ok(cost), Err)
}

/// Walks every tag once and stops at the first thing a picture has no use for.
fn tags(text: &str) -> Result<ArtCost, String> {
    let (mut elements, mut depth, mut tally) = (0, 0, Tally::default());
    let mut rest = text;
    while let Some(at) = rest.find('<') {
        rest = &rest[at + 1..];
        if let Some(inside) = rest.strip_prefix("!--") {
            rest = inside.split_once("-->").map_or("", |(_, after)| after);
        } else if let Some(inside) = rest.strip_prefix("![cdata[") {
            rest = inside.split_once("]]>").map_or("", |(_, after)| after);
        } else if rest.starts_with('/') {
            depth = usize::saturating_sub(depth, 1);
        } else if !rest.starts_with("?xml ") {
            let name_end = rest.find(|c: char| c.is_whitespace() || c == '>' || c == '/').unwrap_or(rest.len());
            let name = &rest[..name_end];
            if !ELEMENTS.contains(&name) {
                return Err(format!("it contains the element `<{name}`"));
            }
            elements += 1;
            if elements > MAX_ART_ELEMENTS {
                return Err(format!("it has more than {MAX_ART_ELEMENTS} elements"));
            }
            if name == "svg" && elements > 1 {
                return Err("it contains a second `<svg`".into());
            }
            tally.shapes += u64::from(SHAPES.contains(&name));
            tally.gradient |= name.ends_with("gradient");
            let (after, open) = attributes(&rest[name_end..], name, &mut tally)?;
            depth += usize::from(open);
            if depth > MAX_ART_DEPTH {
                return Err(format!("its elements are nested more than {MAX_ART_DEPTH} deep"));
            }
            rest = after;
        }
    }
    Ok(tally.cost())
}

/// Reads one tag's attributes: the text after the tag, and whether the element stays open.
fn attributes<'a>(mut rest: &'a str, element: &str, tally: &mut Tally) -> Result<(&'a str, bool), String> {
    loop {
        rest = rest.trim_start();
        if let Some(after) = rest.strip_prefix("/>") {
            return Ok((after, false));
        }
        if let Some(after) = rest.strip_prefix('>') {
            return Ok((after, true));
        }
        let end = rest.find(|c: char| c.is_whitespace() || matches!(c, '=' | '>' | '/')).unwrap_or(rest.len());
        let name = &rest[..end];
        if rest.is_empty() {
            return Err(format!("the tag `<{element}` does not end"));
        }
        let Some(place) = ATTRIBUTES.iter().position(|known| *known == name) else {
            return Err(format!("it contains the attribute `{name}`"));
        };
        let Some(after) = rest[end..].trim_start().strip_prefix('=') else {
            return Err(format!("the attribute `{name}` has no value"));
        };
        let after = after.trim_start();
        let Some(quote) = after.chars().next().filter(|quote| matches!(quote, '"' | '\'')) else {
            return Err(format!("the value of `{name}` is not in quotes"));
        };
        let Some((value, after)) = after[1..].split_once(quote) else {
            return Err(format!("the value of `{name}` does not end"));
        };
        if value.contains('\\') {
            return Err(format!("the value of `{name}` holds a backslash"));
        }
        // The letters `url` are an address wherever they stand, so they may only start `url(#`.
        if value.split("url").skip(1).any(|after| !after.starts_with("(#")) {
            return Err("it refers to something outside the file".into());
        }
        if matches!(name, "d" | "points") && value.len() > MAX_ART_PATH_BYTES {
            return Err(format!("the value of `{name}` is over {MAX_ART_PATH_BYTES} bytes"));
        }
        if place >= WORDS {
            let count = numbers(name, value, element == "svg" && name != "version")?;
            if matches!(name, "d" | "points") {
                tally.numbers += count;
            }
            // Said here and not at the end, so that no file is walked far past what it may hold.
            if tally.numbers > MAX_ART_NUMBERS {
                return Err(format!("its paths hold more than {MAX_ART_NUMBERS} numbers"));
            }
            if element == "svg" {
                tally.root(name, value);
            }
        }
        rest = after;
    }
}

/// Holds an attribute's numbers to plain digits with no exponent, unit or `%`, of at most [`MAX_ART_NUMBER`]. A
/// transform may only be `translate` and `rotate`. On the `<svg` itself (`root`) the most is [`MAX_ART_SIDE`], and
/// `width`, `height` and the last two of `viewBox` are sizes: 1 or more ([`MIN_ART_VIEW`] in a `viewBox`), where alone
/// `px` may follow. Returns how many numbers the value holds.
fn numbers(name: &str, value: &str, root: bool) -> Result<usize, String> {
    let most = if root { MAX_ART_SIDE } else { MAX_ART_NUMBER };
    let digits = |text: &str| text.len() - text.trim_start_matches(|c: char| c.is_ascii_digit()).len();
    let (mut rest, mut count) = (value, 0);
    while let Some(at) = rest.find(|c: char| c.is_ascii_digit() || c == '.') {
        let mut len = digits(&rest[at..]);
        if let Some(decimals) = rest[at + len..].strip_prefix('.') {
            len += 1 + digits(decimals);
        }
        let number = rest[at..at + len].parse::<f32>();
        rest = &rest[at + len..];
        count += 1;
        if !number.is_ok_and(|number| number <= most) || rest.starts_with('e') {
            return Err(format!("`{name}=\"{value}\"` holds a number that is not plain digits of at most {most}"));
        }
    }
    let words = value.replace(|c: char| !c.is_ascii_alphabetic(), "");
    if name.ends_with("transform") && !words.replace("translate", "").replace("rotate", "").is_empty() {
        return Err(format!("`{name}=\"{value}\"` is more than `translate` and `rotate`"));
    }
    let least = if name == "viewbox" { MIN_ART_VIEW } else { 1.0 };
    let size = |text: &str| {
        text.bytes().all(|b| b.is_ascii_digit() || b == b'.') && text.parse().is_ok_and(|n: f32| n >= least)
    };
    let parts: Vec<&str> = value.split([' ', ',']).filter(|part| !part.is_empty()).collect();
    let fine = match name {
        _ if !root => true,
        "viewbox" => parts.len() == 4 && size(parts[2]) && size(parts[3]),
        "width" | "height" => size(value.strip_suffix("px").unwrap_or(value)),
        _ => true,
    };
    if !fine {
        return Err(format!("`{name}=\"{value}\"` is not a size from {least} to {most}"));
    }
    let lettered = match name {
        "d" => !words.chars().all(|c| PATH_COMMANDS.contains(c)),
        "width" | "height" if root => false,
        _ => !name.ends_with("transform") && !words.is_empty(),
    };
    if lettered || value.contains('%') {
        return Err(format!("`{name}` holds a unit or `%`: a number is plain digits"));
    }
    Ok(count)
}
