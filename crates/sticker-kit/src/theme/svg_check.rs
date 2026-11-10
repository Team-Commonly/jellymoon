//! A theme's SVG may only be a picture. Anything that could run code or
//! reach outside the file is refused before the file is used.

/// Why the SVG is refused, or `None` when it is only a picture.
pub fn svg_refusal(svg: &str) -> Option<String> {
    let text = svg.to_ascii_lowercase();
    let banned = [
        ("<script", "a script element"),
        ("<foreignobject", "a foreignObject element"),
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
    if let Some(name) = event_handler(&text) {
        return Some(format!("it contains the event handler `{name}`"));
    }
    if outside_reference(&text, "href", &['=']) || outside_reference(&text, "url", &['(']) {
        return Some("it refers to something outside the file".into());
    }
    None
}

/// The first attribute that looks like `onload=`, `onclick =` and so on.
fn event_handler(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut at = 0;
    while let Some(found) = text[at..].find("on") {
        let start = at + found;
        at = start + 2;
        let after_space = start > 0 && bytes[start - 1].is_ascii_whitespace();
        if !after_space {
            continue;
        }
        let letters = bytes[at..].iter().take_while(|b| b.is_ascii_alphabetic()).count();
        let rest = text[at + letters..].trim_start();
        if letters > 0 && rest.starts_with('=') {
            return Some(text[start..at + letters].to_string());
        }
    }
    None
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
