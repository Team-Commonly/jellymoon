//! Everything a session log or a model wrote passes through here before it is printed. [`credentials`] replaces what is
//! shaped like a key or a password; [`string`], which it runs first, shortens Jellymoon's own tokens and drops control
//! and invisible characters, so a text can neither leak a token nor move the cursor of a terminal nor reorder a line.

use std::sync::LazyLock;

use regex::{Captures, Regex};

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
/// short, by a log, is still replaced from its header on. A pass of its own for the same reason as
/// [`JWT`]: the label in its header can be any length.
static PEM: LazyLock<Regex> =
    LazyLock::new(|| pattern(r"(?s:-----BEGIN [A-Z ]*PRIVATE KEY-----.*?(?:-----END [A-Z ]*PRIVATE KEY-----|$))"));

/// 40 or more characters of the base64 alphabets with no break: long enough to be a key of unknown make.
static LONG_RUN: LazyLock<Regex> = LazyLock::new(|| pattern(r"[A-Za-z0-9+/=_-]{40,}"));

/// The same within a path, where a slash ends a run, so each segment is a run of its own.
static LONG_SEGMENT: LazyLock<Regex> = LazyLock::new(|| pattern(r"[A-Za-z0-9+=_-]{40,}"));

/// How a word that is a path or a link starts: after any opening quotes and brackets, with `/`, `./`, `../`, `~/`, a
/// Windows drive (`C:\` or `C:/`) or a URL scheme (`https://`).
static PATH_START: LazyLock<Regex> =
    LazyLock::new(|| pattern(r#"^["'`(\[{<]*(?:\.{0,2}/|~/|[A-Za-z]:[\\/]|[A-Za-z][A-Za-z0-9+.-]*://)"#));

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
pub(crate) fn string(text: &str) -> String {
    TOKEN.replace_all(&UNSEEN.replace_all(text, ""), "${1}…").into_owned()
}

/// [`string`] for text a person or an agent wrote, which can hold any vendor's key: it also replaces every known key
/// shape, the password in a URL, and, word by word (a word is what stands between two spaces), any [`LONG_RUN`] that
/// mixes digits with both letter cases. A git hash (one case) is left alone. A base64 secret has slashes too, so a
/// slash or a dot inside a word does not make it a path: only its [`PATH_START`] does, and then each segment is judged
/// alone, so a long folder path survives and a long mixed segment inside it does not.
pub(crate) fn credentials(text: &str) -> String {
    let text = string(text);
    let text = PEM.replace_all(&text, REPLACED);
    let text = JWT.replace_all(&text, REPLACED);
    let text = KEY.replace_all(&text, REPLACED);
    let text = URL_PASSWORD.replace_all(&text, format!("${{1}}{REPLACED}@"));
    let mut shown = String::with_capacity(text.len());
    for piece in text.split_inclusive(char::is_whitespace) {
        let word = piece.trim_end();
        let runs = if PATH_START.is_match(word) { &LONG_SEGMENT } else { &LONG_RUN };
        shown.push_str(&runs.replace_all(word, |run: &Captures| mixed(&run[0]).to_string()));
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
pub(crate) fn cut(text: &str, max: usize) -> String {
    match text.char_indices().nth(max.saturating_sub(1)) {
        Some((end, _)) if text.chars().count() > max => format!("{}…", &text[..end]),
        _ => text.to_string(),
    }
}

/// Makes words that are not the person's (a model's, a harness's) safe to show: [`credentials`], on one line, cut to `max`.
pub(crate) fn line(text: &str, max: usize) -> String {
    cut(&credentials(text).split_whitespace().collect::<Vec<_>>().join(" "), max)
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
            let coloured = format!("use \u{1b}[32m{secret}\u{1b}[0m now");
            assert_eq!(credentials(&coloured), "use [redacted] now", "{name} after a colour code");
            // An invisible separator, the Mongolian vowel separator and a tag character, each planted inside the key.
            for unseen in ['\u{2063}', '\u{180e}', '\u{e0041}'] {
                let split = format!("use {}{unseen}{} now", &secret[..3], &secret[3..]);
                assert_eq!(credentials(&split), "use [redacted] now", "{name} split by {unseen:?}");
            }
        }
        let pem_cut_off = format!("key: -----BEGIN {}-----\nMIIEabc and the rest", ["PRIVATE", "KEY"].join(" "));
        assert_eq!(
            credentials(&pem_cut_off),
            "key: [redacted]",
            "a key with no end mark is replaced to the end of the text"
        );
        // A secret of 40 characters with slashes in it, and what each neighbour makes of it (reader.md states each).
        let slashed = "wJa1r/XUtn".repeat(4);
        let cases = [
            ("the key is {}. Keep it", "the key is [redacted]. Keep it", "a full stop after it"),
            ("aws:{}", "aws:[redacted]", "a colon alone makes no path"),
            ("~{}", "~[redacted]", "a tilde with no slash makes no path"),
            ("notes.txt/{}", "notes.[redacted]", "a file name before it makes no path"),
            ("{}.txt", "[redacted].txt", "an extension after it makes no path"),
            ("`{}`", "`[redacted]`", "a quote mark makes no path"),
            // Missed, and stated: in a word that starts as a path or a link, each piece between slashes is under 40.
            ("https://example.com/{}", "https://example.com/{}", "inside a URL's path"),
            ("/{}", "/{}", "after a leading slash"),
        ];
        for (text, want, name) in cases {
            assert_eq!(credentials(&text.replace("{}", &slashed)), want.replace("{}", &slashed), "{name}");
        }
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
            "./src/components/UserProfile2/SettingsPanel3/index",
            "../checkouts/workspace/MyProject2/internal/handlers/v2",
            "~/checkouts/workspace/MyProject2/internal/handlers/v2",
            r"C:\work\checkouts\Workspace2\MyProject2\internal\handlers\v2",
            "D:/work/checkouts/Workspace2/MyProject2/internal/handlers/v2",
            r"E:\work\checkouts/Workspace2/MyProject2/internal/handlers/v2",
            "https://example.com/Docs2/guides/Getting-Started/install/v2/notes",
            "(`./src/components/UserProfile2/SettingsPanel3/index.tsx`)",
            "mailto:someone@example.com at 10:30",
        ];
        for path in paths {
            assert_eq!(credentials(&format!("open {path} now")), format!("open {path} now"));
        }
        // The cost of telling a path by how its word starts: a word that only has a slash or a dot in it is judged as
        // whole runs, and a run of 40 or more that mixes digits with both cases is replaced.
        let replaced = [
            ("feature/JIRA-1234-AddUserProfileSettingsPanel", "[redacted]"),
            ("src/components/UserProfile2/SettingsPanel3/index", "[redacted]"),
            ("src/components/UserProfile2/SettingsPanel3/index.tsx", "[redacted].tsx"),
            ("~dev/checkouts/workspace/MyProject2/internal/handlers/v2", "~[redacted]"),
            ("--out=/srv/checkouts/workspace/MyProject2/internal/handlers", "[redacted]"),
        ];
        for (word, want) in replaced {
            assert_eq!(credentials(&format!("open {word} now")), format!("open {want} now"));
        }
        let hash = "0123456789abcdef".repeat(3);
        let plain = format!(
            "commit {hash} in docs/development/a-long-folder-name/another-long-folder-name/file.md, sk-short, ghp_x"
        );
        assert_eq!(credentials(&plain), plain);
        assert_eq!(
            credentials("a jmr_AbC123 token\u{1b}[2J"),
            "a jmr_… token",
            "credentials also does what string does"
        );
    }

    // A megabyte of each shape is cleaned in a time that grows with its length: no step looks back or walks outward.
    #[test]
    fn a_megabyte_of_the_worst_shapes_is_cleaned_in_seconds() {
        let mixed = "aB3".repeat(13);
        let shapes = [
            ("a long run, a slash and a dot, over and over in one word", format!("{mixed}aB3/x.")),
            ("one path of a megabyte", format!("/{mixed}")),
            (
                "the start of every pattern and none finished",
                format!("\u{1b}]0;\u{1b}[1;-----BEGIN-eyJ{mixed}.x://u:p."),
            ),
            ("short words", "aB3 ".to_string()),
            // A shape that never ends, with whole keys of another shape inside it (the third review's finding).
            ("an unfinished JWT with an AWS key id in it", format!("eyJ-{}-", key("AK", "IA", "Q7", 8))),
            ("an unfinished JWT with a hub key in it", format!("eyJ-{}-", key("gh", "p_", "aB3x", 6))),
            ("an unfinished PEM header with an AWS key id in it", format!("-----BEGIN {} ", key("AK", "IA", "QZ", 8))),
        ];
        for (name, shape) in shapes {
            let text = shape.repeat((1 << 20) / shape.len());
            let start = std::time::Instant::now();
            let shown = credentials(&text);
            assert!(start.elapsed() < std::time::Duration::from_secs(5), "{name}: {:?}", start.elapsed());
            assert!(!shown.is_empty(), "{name}");
        }
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
            ("whole sequences", "a\u{1b}[2Jb\u{1b}]0;title\u{7}c\u{1b}]8;;x\u{1b}\\d\u{9b}1;32me\u{1b}[?25l", "abcde"),
            ("a sequence with no end loses its escape only", "a\u{1b}[12;\u{1b}]0;title", "a[12;]0;title"),
            ("bare CR", "line one\rrewritten", "line onerewritten"),
            ("NUL", "a\0b", "ab"),
            ("DEL", "a\u{7f}b", "ab"),
            ("C1 control", "a\u{85}b", "ab"),
            ("a one-character CSI takes its final byte, as a terminal does", "a\u{9b}b c", "a c"),
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
        let gone = "\u{61c}\u{200e}\u{200f}\u{202a}\u{202b}\u{202c}\u{202d}\u{202e}\u{2066}\u{2067}\u{2068}\u{2069}\u{ad}\u{200b}\u{2060}\u{feff}\u{2061}\u{2064}\u{180e}\u{fff9}\u{e0000}\u{e0001}\u{e007f}";
        assert_eq!(string(&gone.chars().map(|c| format!("x{c}")).collect::<String>()), "x".repeat(23));
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
