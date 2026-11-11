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

/// Why the SVG is refused, or `None` when it is only a picture.
pub fn svg_refusal(svg: &str) -> Option<String> {
    let text = svg.to_ascii_lowercase();
    // With no character reference, what the checks below read is what the renderer reads.
    let banned = [("<!doctype", "a DOCTYPE"), ("<!entity", "an entity declaration"), ("&#", "a character reference")];
    for (needle, what) in banned {
        if text.contains(needle) {
            return Some(format!("it contains {what}"));
        }
    }
    tags(&text).err()
}

/// Walks every tag once and stops at the first thing a picture has no use for.
fn tags(text: &str) -> Result<(), String> {
    let (mut elements, mut depth) = (0, 0);
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
            let (after, open) = attributes(&rest[name_end..], name)?;
            depth += usize::from(open);
            if depth > MAX_ART_DEPTH {
                return Err(format!("its elements are nested more than {MAX_ART_DEPTH} deep"));
            }
            rest = after;
        }
    }
    Ok(())
}

/// Reads one tag's attributes: the text after the tag, and whether the element stays open.
fn attributes<'a>(mut rest: &'a str, element: &str) -> Result<(&'a str, bool), String> {
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
        if place >= WORDS {
            numbers(name, value, element == "svg" && name != "version")?;
        }
        rest = after;
    }
}

/// Holds an attribute's numbers to plain digits with no exponent, of at most [`MAX_ART_NUMBER`]. A transform may
/// only be `translate` and `rotate`. On the `<svg` itself (`root`) the most is [`MAX_ART_SIDE`], and `width`, `height`
/// and the last two of `viewBox` are sizes: 1 or more, with no unit but `px`.
fn numbers(name: &str, value: &str, root: bool) -> Result<(), String> {
    let most = if root { MAX_ART_SIDE } else { MAX_ART_NUMBER };
    let digits = |text: &str| text.len() - text.trim_start_matches(|c: char| c.is_ascii_digit()).len();
    let mut rest = value;
    while let Some(at) = rest.find(|c: char| c.is_ascii_digit() || c == '.') {
        let mut len = digits(&rest[at..]);
        if let Some(decimals) = rest[at + len..].strip_prefix('.') {
            len += 1 + digits(decimals);
        }
        let number = rest[at..at + len].parse::<f32>();
        rest = &rest[at + len..];
        if !number.is_ok_and(|number| number <= most) || rest.starts_with('e') {
            return Err(format!("`{name}=\"{value}\"` holds a number that is not plain digits of at most {most}"));
        }
    }
    let words = value.replace(|c: char| !c.is_ascii_alphabetic(), "");
    if name.ends_with("transform") && !words.replace("translate", "").replace("rotate", "").is_empty() {
        return Err(format!("`{name}=\"{value}\"` is more than `translate` and `rotate`"));
    }
    let size =
        |text: &str| text.bytes().all(|b| b.is_ascii_digit() || b == b'.') && text.parse().is_ok_and(|n: f32| n >= 1.0);
    let parts: Vec<&str> = value.split([' ', ',']).filter(|part| !part.is_empty()).collect();
    let fine = match name {
        _ if !root => true,
        "viewbox" => parts.len() == 4 && size(parts[2]) && size(parts[3]),
        "width" | "height" => size(value.strip_suffix("px").unwrap_or(value)),
        _ => true,
    };
    if fine { Ok(()) } else { Err(format!("`{name}=\"{value}\"` is not a size from 1 to {most}")) }
}
