//! Everything a session log or a model wrote passes through here before it is printed. [`credentials`] replaces what is
//! shaped like a key or a password; [`string`], which it runs first, shortens Jellymoon's own tokens and drops control
//! and invisible characters, so a text can neither leak a token nor move the cursor of a terminal nor reorder a line.

use std::ops::Range;
use std::sync::LazyLock;

use regex::Regex;

/// What [`credentials`] leaves where a credential was.
pub(crate) const REPLACED: &str = "[redacted]";

/// Compiles a pattern that is a constant of this crate, with `\b` meaning an ASCII word boundary.
#[allow(clippy::expect_used)] // a constant: the tests run every pattern, so one that does not compile fails there
pub(crate) fn pattern(source: &str) -> Regex {
    Regex::new(&source.replace(r"\b", r"(?-u:\b)")).expect("a constant pattern compiles")
}

/// A Jellymoon (`jmu_`, `jmc_`, `jmr_`) or Commonly (`cm_`) token. It takes an ellipsis [`string`] already left, so a
/// text that passes through twice is the same as one that passed once.
static TOKEN: LazyLock<Regex> = LazyLock::new(|| pattern(r"(jm[ucr]_|\bcm_)[A-Za-z0-9_-]*…?"));

/// The vendor key shapes worth naming: model provider, Google, GitLab, Hugging Face, Stripe and GitHub keys, Slack
/// tokens, AWS key ids, JWTs and PEM private keys.
static KEY: LazyLock<Regex> = LazyLock::new(|| {
    pattern(concat!(
        r"\bsk-[A-Za-z0-9_-]{16,}|\bAIza[A-Za-z0-9_-]{35}|\bglpat-[A-Za-z0-9_-]{20,}|\bhf_[A-Za-z0-9]{30,}",
        r"|\b[sr]k_live_[A-Za-z0-9]{16,}|\bgh[pousr]_[A-Za-z0-9]{20,}|\bgithub_pat_[A-Za-z0-9_]{20,}",
        r"|\bxox[a-z]-[A-Za-z0-9-]{10,}|\bAKIA[0-9A-Z]{16}\b|\beyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]*",
        r"|(?s:-----BEGIN [A-Z ]*PRIVATE KEY-----.*?(?:-----END [A-Z ]*PRIVATE KEY-----|$))",
    ))
});

/// 40 or more characters of the base64 alphabets with no break: long enough to be a key of unknown make.
static LONG_RUN: LazyLock<Regex> = LazyLock::new(|| pattern(r"[A-Za-z0-9+/=_-]{40,}"));

/// The password in `scheme://user:password@host`. Only the password is replaced.
static URL_PASSWORD: LazyLock<Regex> =
    LazyLock::new(|| pattern(r"([A-Za-z][A-Za-z0-9+.-]*://[^\t\n\f\r /:@]*:)[^\t\n\f\r /@]+@"));

/// A character nobody sees that changes what is seen or found: a control character other than newline and tab, a bidi
/// control (it reorders the line it is in), a soft hyphen, a zero-width space, a word joiner, a byte-order mark. The
/// zero-width joiner and non-joiner stay: emoji and several scripts are written with them.
fn hidden(c: char) -> bool {
    let unseen = matches!(c, '\u{ad}' | '\u{61c}' | '\u{200b}' | '\u{200e}' | '\u{200f}' | '\u{2060}' | '\u{feff}');
    c.is_control() && !matches!(c, '\n' | '\t')
        || unseen
        || matches!(c, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
}

/// Strips the [`hidden`] characters and shortens every token to its prefix and an ellipsis. The characters go first, so
/// one planted inside a token or a key cannot hide the rest of it.
pub(crate) fn string(text: &str) -> String {
    let shown: String = text.chars().filter(|c| !hidden(*c)).collect();
    TOKEN.replace_all(&shown, "${1}…").into_owned()
}

/// [`string`] for text a person or an agent wrote, which can hold any vendor's key: it also replaces every known key
/// shape, the password in a URL, and any long unbroken run that mixes digits with both letter cases. A git hash (one
/// case) is left alone. So is a path (see [`is_path`]): it is judged one segment at a time, so a long folder path with a
/// digit and a capital somewhere in it survives, and a long mixed segment inside it does not.
pub(crate) fn credentials(text: &str) -> String {
    let text = string(text);
    let text = KEY.replace_all(&text, REPLACED);
    let text = URL_PASSWORD.replace_all(&text, format!("${{1}}{REPLACED}@"));
    let (mut shown, mut from) = (String::with_capacity(text.len()), 0);
    for run in LONG_RUN.find_iter(&text) {
        shown.push_str(&text[from..run.start()]);
        shown.push_str(&long_run(&text, run.range()));
        from = run.end();
    }
    shown + &text[from..]
}

/// Judges the run at `at`: whole, unless it is a path, whose segments are each judged alone.
fn long_run(text: &str, at: Range<usize>) -> String {
    let run = &text[at.clone()];
    if run.contains(['+', '=']) || !run.contains('/') || !is_path(text, at) {
        return mixed(run).to_string();
    }
    let segments: Vec<&str> =
        run.split('/').map(|segment| if segment.len() >= 40 { mixed(segment) } else { segment }).collect();
    segments.join("/")
}

/// Whether a run with a slash in it shows a sign of being a path. A base64 secret has slashes too, so the slash alone
/// is none. The signs are read off the word the run is in, which is the run and the dots, tildes and backslashes that
/// touch it: it starts with `/` or `~` (which `./x`, `../x`, `~/x` and a URL's `//host/x` all do), it follows a
/// Windows drive (`C:`), or one of its segments has a dot in it (`index.tsx`, `..`). A full stop that ends the word is
/// the sentence's, not the path's.
fn is_path(text: &str, at: Range<usize>) -> bool {
    let wordy = |c: char| c.is_ascii_alphanumeric() || "+/=_-.~\\".contains(c);
    let before = text[..at.start].trim_end_matches(wordy);
    let after = text[at.end..].trim_start_matches(wordy);
    let word = text[before.len()..text.len() - after.len()].trim_end_matches('.');
    let on_a_drive = before.strip_suffix(':').is_some_and(|drive| drive.ends_with(|c: char| c.is_ascii_alphabetic()));
    word.starts_with(['/', '~'])
        || on_a_drive && word.starts_with('\\')
        || word.split(['/', '\\']).any(|segment| segment.contains('.'))
}

/// Replaces a run that has digits and both letter cases.
fn mixed(run: &str) -> &str {
    let has = |kind: fn(&u8) -> bool| run.as_bytes().iter().any(kind);
    if has(u8::is_ascii_digit) && has(u8::is_ascii_lowercase) && has(u8::is_ascii_uppercase) { REPLACED } else { run }
}

/// Shortens `text` to `max` characters; a text that was cut ends in an ellipsis.
pub(crate) fn cut(text: &str, max: usize) -> String {
    match text.char_indices().nth(max.saturating_sub(1)) {
        Some((end, _)) if text.chars().count() > max => format!("{}…", &text[..end]),
        _ => text.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The shapes are joined here so no scanner reads one in this file.
    fn key(prefix: &str, rest: &str, piece: &str, times: usize) -> String {
        format!("{prefix}{rest}{}", piece.repeat(times))
    }

    // Item 19.
    #[test]
    fn every_key_shape_is_replaced() {
        let jwt = format!("ey{}{}.{}.sig", "J", "hbGci".repeat(3), "eyJzd".repeat(3));
        let pem = format!("-----BEGIN RSA {0}-----\nMIIEabc\n-----END RSA {0}-----", ["PRIVATE", "KEY"].join(" "));
        let secrets = [
            ("model key", key("sk", "-proj-", "aB3x", 6)),
            ("hub", key("gh", "o_", "aB3x", 6)),
            ("hub pat", key("github", "_pat_", "aB3_", 6)),
            ("slack", format!("{}abcd", key("xo", "xb-", "1234-", 3))),
            ("aws id", key("AK", "IA", "Q7", 8)),
            ("jwt", jwt),
            ("pem", pem),
            ("unknown make", "aB3+/x9Z".repeat(5)),
            ("google", key("AI", "za", "Sy9_b", 7)),
            ("gitlab", key("gl", "pat-", "aB3x", 5)),
            ("hugging face", key("h", "f_", "aB3xY", 7)),
            ("stripe", key("sk", "_live_", "aB3x", 6)),
            ("stripe restricted", key("rk", "_live_", "aB3x", 6)),
            ("slashes and no plus, 40 long", "aB3/x9Zq".repeat(5)),
        ];
        for (name, secret) in secrets {
            assert_eq!(credentials(&format!("use {secret} now")), "use [redacted] now", "{name}");
        }
        let pem_cut_off = format!("key: -----BEGIN {}-----\nMIIEabc and the rest", ["PRIVATE", "KEY"].join(" "));
        assert_eq!(
            credentials(&pem_cut_off),
            "key: [redacted]",
            "a key with no end mark is replaced to the end of the text"
        );
        let slashed = "wJa1r/XUtn".repeat(4);
        assert_eq!(credentials(&format!("the key is {slashed}. Keep it")), "the key is [redacted]. Keep it");
        assert_eq!(credentials(&format!("aws:{slashed}")), "aws:[redacted]", "a colon alone makes no drive path");
    }

    // Item 19: the password only, and a long mixed segment of a path.
    #[test]
    fn a_url_password_and_a_long_mixed_path_segment_are_replaced() {
        let text = format!(
            "see postgres://app:{}@db.internal:5432/main and /api/{}/logs",
            ["hunter", "2secret"].concat(),
            "aB3x".repeat(12)
        );
        assert_eq!(credentials(&text), "see postgres://app:[redacted]@db.internal:5432/main and /api/[redacted]/logs");
    }

    // Item 19: what is not a credential.
    #[test]
    fn a_path_a_commit_hash_and_plain_words_are_left_alone() {
        let paths = [
            "/srv/checkouts/workspace/MyProject2/internal/handlers/v2",
            "src/components/UserProfile2/SettingsPanel3/index.tsx",
            "./src/components/UserProfile2/SettingsPanel3/index",
            "../checkouts/workspace/MyProject2/internal/handlers/v2",
            "~/checkouts/workspace/MyProject2/internal/handlers/v2",
            "~dev/checkouts/workspace/MyProject2/internal/handlers/v2",
            r"C:\work\checkouts\Workspace2\MyProject2\internal\handlers\v2",
            "D:/work/checkouts/Workspace2/MyProject2/internal/handlers/v2",
            r"E:\work\checkouts/Workspace2/MyProject2/internal/handlers/v2",
            "https://example.com/docs/guide",
            "mailto:someone@example.com at 10:30",
        ];
        for path in paths {
            assert_eq!(credentials(&format!("open {path} now")), format!("open {path} now"));
        }
        // The cost of judging a run with no sign of a path whole: a long relative path with no dot in it, when it
        // mixes digits with both cases, is replaced too.
        assert_eq!(credentials("open src/components/UserProfile2/SettingsPanel3/index now"), "open [redacted] now");
        let hash = "0123456789abcdef".repeat(3);
        let plain = format!(
            "commit {hash} in docs/development/a-long-folder-name/another-long-folder-name/file.md, sk-short, ghp_x"
        );
        assert_eq!(credentials(&plain), plain);
        assert_eq!(
            credentials("a jmr_AbC123 token\u{1b}[2J"),
            "a jmr_… token[2J",
            "credentials also does what string does"
        );
    }

    // Item 20.
    #[test]
    fn redacting_twice_is_redacting_once() {
        let once = string("jmu_AbC123 and a bare jmr_ and cm_0123");
        assert_eq!(once, "jmu_… and a bare jmr_… and cm_…");
        assert_eq!(string(&once), once);
        let once = credentials(&format!("use {} and jmc_AbC123", key("gh", "p_", "aB3x", 6)));
        assert_eq!(credentials(&once), once);
    }

    // Item 21.
    #[test]
    fn every_token_prefix_is_shortened() {
        for prefix in ["jmu_", "jmc_", "jmr_"] {
            assert_eq!(string(&format!("before {prefix}AbC123xyz_-9Q after")), format!("before {prefix}… after"));
        }
        assert_eq!(string("jmx_AbC123xyz and jm_whoami"), "jmx_AbC123xyz and jm_whoami", "not a token");
        assert_eq!(string("token cm_0123456789abcdef end"), "token cm_… end");
        assert_eq!(string("the vcm_ prefix and pcm_x"), "the vcm_ prefix and pcm_x", "a word that only ends in cm_");
    }

    // Item 22.
    #[test]
    fn control_characters_are_stripped_and_newline_and_tab_kept() {
        let cases = [
            ("ansi escape", "a\u{1b}[2Jb\u{1b}]0;title\u{7}c", "a[2Jb]0;titlec"),
            ("bare CR", "line one\rrewritten", "line onerewritten"),
            ("NUL", "a\0b", "ab"),
            ("DEL", "a\u{7f}b", "ab"),
            ("C1 control", "a\u{9b}b", "ab"),
            ("kept", "a\tb\nc", "a\tb\nc"),
        ];
        for (name, text, want) in cases {
            assert_eq!(string(text), want, "{name}");
        }
    }

    // Item 23.
    #[test]
    fn a_token_split_by_control_characters_is_still_caught() {
        assert_eq!(string("jmc_ab\0cd\ref\u{1b}gh"), "jmc_…");
    }

    #[test]
    fn bidi_controls_and_invisible_characters_are_stripped_before_a_key_is_looked_for() {
        let gone = "\u{61c}\u{200e}\u{200f}\u{202a}\u{202b}\u{202c}\u{202d}\u{202e}\u{2066}\u{2067}\u{2068}\u{2069}\u{ad}\u{200b}\u{2060}\u{feff}";
        assert_eq!(string(&gone.chars().map(|c| format!("x{c}")).collect::<String>()), "x".repeat(16));
        assert_eq!(string("ab\u{202e}dc"), "abdc", "a label cannot be turned round");
        let joined = "\u{645}\u{200c}\u{6cc} and a\u{200d}b";
        assert_eq!(string(joined), joined, "the joiners are kept: emoji and several scripts need them");
        let split = format!("gh\u{200b}p_{}\u{ad}{}", "aB3x".repeat(3), "aB3x".repeat(3));
        assert_eq!(credentials(&format!("use {split} now")), "use [redacted] now");
        assert_eq!(string("jmc_ab\u{200b}cd"), "jmc_…");
    }

    #[test]
    fn a_cut_text_has_exactly_the_length_and_ends_in_an_ellipsis() {
        assert_eq!(cut("abcdef", 6), "abcdef");
        assert_eq!(cut("abcdefg", 6), "abcde…");
        assert_eq!(cut("ééééééé", 6), "ééééé…", "counted in characters, not bytes");
        assert_eq!(cut("", 6), "");
    }
}
