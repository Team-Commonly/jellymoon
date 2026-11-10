//! Everything a session log or a model wrote passes through [`shown`] or [`line`] before it is printed. Each keeps
//! only the start of a text ([`WINDOW`]), so no step's cost grows with a text's length. [`credentials`] then replaces
//! what is shaped like a key or a password; [`string`], which it runs first, shortens Jellymoon's own tokens and drops
//! control and invisible characters, so a text can neither leak a token nor move a terminal's cursor nor reorder a line.

use std::sync::LazyLock;

use regex::{Captures, Regex};

/// What [`credentials`] leaves where a credential was.
const REPLACED: &str = "[redacted]";

/// Compiles a pattern that is a constant of this crate, with `\b` meaning an ASCII word boundary.
#[allow(clippy::expect_used)] // a constant: the tests run every pattern, so one that does not compile fails there
pub(crate) fn pattern(source: &str) -> Regex {
    Regex::new(&source.replace(r"\b", r"(?-u:\b)")).expect("a constant pattern compiles")
}

/// A Jellymoon (`jmu_`, `jmc_`, `jmr_`) or Commonly (`cm_`) token. It takes an ellipsis [`string`] already left, so a
/// text that passes through twice is the same as one that passed once.
static TOKEN: LazyLock<Regex> = LazyLock::new(|| pattern(r"(jm[ucr]_|\bcm_)[A-Za-z0-9_-]*…?"));

/// The vendor key shapes worth naming: model provider, Google, GitLab, Hugging Face, Stripe and GitHub keys, Slack
/// tokens and AWS key ids. Every branch is a prefix and a counted run, so one that has started either matches or ends
/// within 40 characters: none stays open across a later match of another, which would make the pass rescan the text.
static KEY: LazyLock<Regex> = LazyLock::new(|| {
    pattern(concat!(
        r"\bsk-[A-Za-z0-9_-]{16,}|\bAIza[A-Za-z0-9_-]{35}|\bglpat-[A-Za-z0-9_-]{20,}|\bhf_[A-Za-z0-9]{30,}",
        r"|\b[sr]k_live_[A-Za-z0-9]{16,}|\bgh[pousr]_[A-Za-z0-9]{20,}|\bgithub_pat_[A-Za-z0-9_]{20,}",
        r"|\bxox[a-z]-[A-Za-z0-9-]{10,}|\bAKIA[0-9A-Z]{16}\b",
    ))
});

/// A JWT. Its first part can run on for any length before its dot, so it is a pattern and a pass of its own.
static JWT: LazyLock<Regex> = LazyLock::new(|| pattern(r"\beyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]*"));

/// A PEM private key, from its BEGIN line to its END mark, or to the end of the text when there is none: a key cut
/// short, by a log or by [`WINDOW`], is still replaced from its header on. A pass of its own for the same reason as
/// [`JWT`]: the label in its header can be any length.
static PEM: LazyLock<Regex> =
    LazyLock::new(|| pattern(r"(?s:-----BEGIN [A-Z ]*PRIVATE KEY-----.*?(?:-----END [A-Z ]*PRIVATE KEY-----|$))"));

/// 40 or more characters of the base64 alphabets with no break: long enough to be a key of unknown make.
static LONG_RUN: LazyLock<Regex> = LazyLock::new(|| pattern(r"[A-Za-z0-9+/=_-]{40,}"));

/// The same within a path, where a slash ends a run, so each segment is a run of its own.
static LONG_SEGMENT: LazyLock<Regex> = LazyLock::new(|| pattern(r"[A-Za-z0-9+=_-]{40,}"));

/// Where a word turns into a path or a link. At its start, after any opening quotes and brackets: `/`, `./`, `../`,
/// `~/` or a Windows drive (`C:\` or `C:/`). Anywhere in it: a URL scheme (`https://`), as in a markdown link.
static PATH_START: LazyLock<Regex> =
    LazyLock::new(|| pattern(r#"^["'`(\[{<]*(?:\.{0,2}/|~/|[A-Za-z]:[\\/])|[A-Za-z][A-Za-z0-9+.-]*://"#));

/// What ends a path or a link inside a word: a closing quote, a closing bracket or a comma. What follows it in the
/// word (the next value of a JSON object, say) is no part of the path.
const PATH_END: [char; 8] = ['"', '\'', '`', ')', ']', '}', '>', ','];

/// The password in `scheme://user:password@host`. Only the password is replaced.
static URL_PASSWORD: LazyLock<Regex> =
    LazyLock::new(|| pattern(r"([A-Za-z][A-Za-z0-9+.-]*://[^\t\n\f\r /:@]*:)[^\t\n\f\r /@]+@"));

/// What nobody sees and a terminal obeys. First a whole escape sequence: CSI (ESC `[` or U+009B, parameter bytes,
/// intermediate bytes, a final byte) or OSC (ESC `]` up to BEL or ESC `\`). Then any control character other than
/// newline and tab, and any format character (category Cf: the bidi controls, zero-width space, soft hyphen, word
/// joiner, byte-order mark, tag characters and the rest) other than the joiners U+200C and U+200D, which emoji and
/// several scripts are written with. A sequence that is cut short or broken loses only those characters.
static UNSEEN: LazyLock<Regex> = LazyLock::new(|| {
    pattern(concat!(
        r"(?:\x1b\[|\u{9b})[0-?]*[ -/]*[@-~]|\x1b\][^\x07\x1b]*(?:\x07|\x1b\\)",
        r"|[\p{Cc}--[\n\t]]|[\p{Cf}--[\u{200c}\u{200d}]]|[\u{e0000}-\u{e007f}]",
    ))
});

/// Strips what is [`UNSEEN`] and shortens every token to its prefix and an ellipsis. The stripping goes first, so a
/// colour code before a key, or a character planted inside one, cannot hide it.
fn string(text: &str) -> String {
    TOKEN.replace_all(&UNSEEN.replace_all(text, ""), "${1}…").into_owned()
}

/// [`string`] for text a person or an agent wrote, which can hold any vendor's key: it also replaces every known key
/// shape, the password in a URL, and, word by word (a word is what stands between two spaces), any [`LONG_RUN`] that
/// mixes digits with both letter cases. A git hash (one case) is left alone. A base64 secret has slashes too, so a
/// slash or a dot inside a word does not make it a path: only its [`PATH_START`] does, and from there on each segment
/// is judged alone, so a long folder path survives and a long mixed segment inside it does not.
fn credentials(text: &str) -> String {
    let text = string(text);
    let text = PEM.replace_all(&text, REPLACED);
    let text = JWT.replace_all(&text, REPLACED);
    let text = KEY.replace_all(&text, REPLACED);
    let text = URL_PASSWORD.replace_all(&text, format!("${{1}}{REPLACED}@"));
    let mut shown = String::with_capacity(text.len());
    for piece in text.split_inclusive(char::is_whitespace) {
        let word = piece.trim_end();
        let path = PATH_START.find(word).map_or(word.len()..word.len(), |start| {
            let end = word[start.end()..].find(PATH_END).map_or(word.len(), |at| start.end() + at);
            start.start()..end
        });
        shown.push_str(&LONG_RUN.replace_all(word, |run: &Captures| {
            let (at, run) = run.get(0).map_or((0, ""), |run| (run.start(), run.as_str()));
            if !path.contains(&at) {
                return mixed(run).to_string();
            }
            LONG_SEGMENT.replace_all(run, |segment: &Captures| mixed(&segment[0]).to_string()).into_owned()
        }));
        shown.push_str(&piece[word.len()..]);
    }
    shown
}

/// Replaces a run that has digits and both letter cases.
fn mixed(run: &str) -> &str {
    let has = |kind: fn(&u8) -> bool| run.as_bytes().iter().any(kind);
    if has(u8::is_ascii_digit) && has(u8::is_ascii_lowercase) && has(u8::is_ascii_uppercase) { REPLACED } else { run }
}

/// Shortens `text` to `max` characters; a text that was cut ends in an ellipsis.
fn cut(text: &str, max: usize) -> String {
    match text.char_indices().nth(max.saturating_sub(1)) {
        Some((end, _)) if text.chars().count() > max => format!("{}…", &text[..end]),
        _ => text.to_string(),
    }
}

/// How far past what will be shown a text is read, in characters. Nothing after `limit + WINDOW` characters is
/// cleaned or kept, so the cost of cleaning does not grow with a text's length. Far longer than any key but a PEM
/// one, which [`PEM`] replaces from its header on.
const WINDOW: usize = 16_384;

/// The longest secret promised whole. A text the window cut must still be this much longer than what is shown once it
/// is cleaned; otherwise so much was taken out that the window's end is near, and the word there is not shown.
const SECRET_MAX: usize = WINDOW / 2;

/// The first `max` characters of `text`, and whether that is all of it. One pass that stops at the cut.
fn start(text: &str, max: usize) -> (&str, bool) {
    match text.char_indices().nth(max) {
        Some((end, _)) => (&text[..end], false),
        None => (text, true),
    }
}

/// Makes a text safe to show, at most `limit` characters of it: the only way into this module for a text that is
/// printed. It keeps the first `limit + WINDOW` characters, cleans those ([`credentials`]), drops the spaces around
/// them and cuts to `limit`. A secret that starts in what is shown and is shorter than [`SECRET_MAX`] lies whole
/// inside the window, so it is matched whole before the cut.
pub(crate) fn shown(text: &str, limit: usize) -> String {
    bounded(text, limit, |cleaned| cleaned.trim().to_string())
}

/// [`shown`] for words that are not the person's (a model's, a harness's): on one line, each run of spaces made one.
pub(crate) fn line(text: &str, limit: usize) -> String {
    bounded(text, limit, |cleaned| cleaned.split_whitespace().collect::<Vec<_>>().join(" "))
}

fn bounded(text: &str, limit: usize, tidy: fn(&str) -> String) -> String {
    let (window, whole) = start(text, limit + WINDOW);
    let cleaned = credentials(window);
    let tidied = tidy(&cleaned);
    if whole {
        return cut(&tidied, limit);
    }
    // The window cut the text, and its last word may be the head of a secret whose rest was never read. That word is
    // far past the cut unless cleaning took out or squeezed most of the window; then only what stands before it is
    // kept. A last word that is the mark and nothing more (a PEM key's replacement) shows nothing, so it stays.
    let marked = cleaned.split_whitespace().next_back() == Some(REPLACED);
    let reaches = marked || tidied.chars().count() >= limit + SECRET_MAX;
    let kept = if reaches { tidied } else { tidy(cleaned.rfind(char::is_whitespace).map_or("", |at| &cleaned[..at])) };
    if kept.is_empty() { kept } else { cut(&format!("{kept}…"), limit) }
}

#[cfg(test)]
mod tests;
