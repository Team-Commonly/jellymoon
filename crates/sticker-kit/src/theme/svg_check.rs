//! A theme's SVG may only be a picture. Anything that could run code or reach outside the file is refused before the
//! file is used: every element must be one of [`ELEMENTS`], and no attribute may be an event handler or `xml:base`.

/// The elements a picture is made of. Any other element is refused.
pub const ELEMENTS: [&str; 30] = [
    "svg",
    "g",
    "defs",
    "symbol",
    "use",
    "title",
    "desc",
    "style",
    "path",
    "rect",
    "circle",
    "ellipse",
    "line",
    "polyline",
    "polygon",
    "text",
    "tspan",
    "lineargradient",
    "radialgradient",
    "stop",
    "clippath",
    "mask",
    "pattern",
    "filter",
    "fegaussianblur",
    "feoffset",
    "femerge",
    "femergenode",
    "feflood",
    "fecomposite",
];

/// Why the SVG is refused, or `None` when it is only a picture.
pub fn svg_refusal(svg: &str) -> Option<String> {
    let text = svg.to_ascii_lowercase();
    let banned = [
        ("<!doctype", "a DOCTYPE"),
        ("<!entity", "an entity declaration"),
        ("<?xml-stylesheet", "an external stylesheet"),
        ("@import", "a stylesheet import"),
        ("javascript:", "a javascript: address"),
    ];
    for (needle, what) in banned {
        if text.contains(needle) {
            return Some(format!("it contains {what}"));
        }
    }
    if let Some(why) = tags(&text) {
        return Some(why);
    }
    if outside_reference(&text, "href", &['=']) || outside_reference(&text, "url", &['(']) {
        return Some("it refers to something outside the file".into());
    }
    None
}

/// Walks every tag once: the first element or attribute a picture has no use for.
fn tags(text: &str) -> Option<String> {
    let mut rest = text;
    while let Some(at) = rest.find('<') {
        rest = &rest[at + 1..];
        if let Some(inside) = rest.strip_prefix("!--") {
            rest = inside.split_once("-->").map_or("", |(_, after)| after);
        } else if let Some(inside) = rest.strip_prefix("![cdata[") {
            rest = inside.split_once("]]>").map_or("", |(_, after)| after);
        } else if rest.starts_with("?xml ") || rest.starts_with('/') {
            continue;
        } else {
            let name_end = rest.find(|c: char| c.is_whitespace() || c == '>' || c == '/').unwrap_or(rest.len());
            let name = &rest[..name_end];
            if !ELEMENTS.contains(&name) {
                return Some(format!("it contains the element `<{name}`"));
            }
            let (why, after) = attributes(&rest[name_end..]);
            if why.is_some() {
                return why;
            }
            rest = after;
        }
    }
    None
}

/// Reads one tag's attributes, whatever stands between them: a refusal, and the text after the tag.
fn attributes(mut rest: &str) -> (Option<String>, &str) {
    loop {
        rest = rest.trim_start_matches(|c: char| c.is_whitespace() || c == '/');
        if rest.is_empty() || rest.starts_with('>') {
            return (None, rest);
        }
        let end = rest.find(|c: char| c.is_whitespace() || matches!(c, '=' | '>' | '/')).unwrap_or(rest.len());
        let name = &rest[..end];
        if name.starts_with("on") {
            return (Some(format!("it contains the event handler `{name}`")), rest);
        }
        if name == "xml:base" || name.is_empty() {
            return (Some(format!("it contains the attribute `{name}`")), rest);
        }
        rest = rest[end..].trim_start();
        let Some(value) = rest.strip_prefix('=') else {
            continue;
        };
        let value = value.trim_start();
        rest = match value.chars().next().filter(|quote| matches!(quote, '"' | '\'')) {
            Some(quote) => value[1..].split_once(quote).map_or("", |(_, after)| after),
            None => value.trim_start_matches(|c: char| !c.is_whitespace() && c != '>'),
        };
    }
}
/// True when `word` followed by one of `openers` introduces a value that is
/// not a `#fragment` of this same file.
fn outside_reference(text: &str, word: &str, openers: &[char]) -> bool {
    let mut at = 0;
    while let Some(found) = text[at..].find(word) {
        at += found + word.len();
        let rest = text[at..].trim_start();
        let Some(value) = rest.strip_prefix(openers) else {
            continue;
        };
        let value = value.trim_start_matches(|c: char| c.is_whitespace() || c == '"' || c == '\'');
        if !value.starts_with('#') {
            return true;
        }
    }
    false
}
