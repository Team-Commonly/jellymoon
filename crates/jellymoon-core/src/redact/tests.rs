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
    assert_eq!(credentials("a jmr_AbC123 token\u{1b}[2J"), "a jmr_… token", "credentials also does what string does");
}

// A megabyte of each shape is cleaned in a time that grows with its length: no step looks back or walks outward.
#[test]
fn a_megabyte_of_the_worst_shapes_is_cleaned_in_seconds() {
    let mixed = "aB3".repeat(13);
    let shapes = [
        ("a long run, a slash and a dot, over and over in one word", format!("{mixed}aB3/x.")),
        ("one path of a megabyte", format!("/{mixed}")),
        ("the start of every pattern and none finished", format!("\u{1b}]0;\u{1b}[1;-----BEGIN-eyJ{mixed}.x://u:p.")),
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

const SAID: usize = 700;

fn hub_key() -> String {
    key("gh", "p_", "aB3x", 6)
}

#[test]
fn the_window_is_the_first_characters_and_is_cut_between_two_of_them() {
    assert_eq!(start("ééé", 2), ("éé", false), "counted in characters, cut on a boundary");
    assert_eq!(start("ééé", 3), ("ééé", true));
    assert_eq!(start("", 3), ("", true));
    let long = "é".repeat(SAID + WINDOW + 5);
    assert_eq!(start(&long, SAID + WINDOW).0.chars().count(), SAID + WINDOW);
}

#[test]
fn a_key_that_straddles_the_shown_limit_is_replaced_whole() {
    let key = hub_key();
    for before in [SAID - 30, SAID - 12, SAID - 2] {
        let text = format!("{} {key} and more", "x".repeat(before - 1));
        let said = shown(&text, SAID);
        assert_eq!(said, cut(&format!("{} {REPLACED} and more", "x".repeat(before - 1)), SAID), "{before}");
        assert!(!said.contains("gh"), "{before}");
    }
    // The same when the text runs on past the window: the key is matched whole inside it, then the cut is made.
    let text = format!("{} {key} {}", "x".repeat(SAID - 13), "and more ".repeat(WINDOW));
    assert_eq!(shown(&text, SAID), format!("{} {REPLACED} …", "x".repeat(SAID - 13)));
}

// A key the window cuts is far past what is shown, unless what stood before it was taken out or squeezed. Then the
// word at the window's end is dropped, since the rest of it was never read.
#[test]
fn a_key_that_starts_just_before_the_windows_end_leaves_nothing_visible() {
    let (key, seen) = (hub_key(), 10); // how many of the key's characters the window holds
    let fill = |lead: &str, filler: &str, limit: usize| {
        let filler: String = filler.chars().cycle().take(limit + WINDOW - lead.chars().count() - seen).collect();
        format!("{lead}{filler}{key} and more")
    };
    assert_eq!(
        shown(&fill("say ", "word ", SAID), SAID).chars().count(),
        SAID,
        "words before it: it is far past the cut"
    );
    let spaces = format!("say{}…", " ".repeat(SAID - 4));
    let cases = [
        ("invisible characters before it", "say ", "\u{200b}", "say…", "say…"),
        ("escape sequences before it", "say ", "\u{1b}[0m", "say…", "say…"),
        ("spaces before it", "say ", " ", spaces.as_str(), "say…"),
        ("nothing else in the text", "\u{200b}", " ", "", ""),
        ("one word with it", "say:", "\u{200b}", "", ""),
    ];
    for (name, lead, filler, want_shown, want_line) in cases {
        assert_eq!(shown(&fill(lead, filler, SAID), SAID), want_shown, "{name}");
        assert_eq!(line(&fill(lead, filler, 300), 300), want_line, "{name}, on one line");
    }
    // The same for a password in a URL and a key of unknown make, which have no prefix to be known by.
    let password = format!("see x://u:{}@host", ["hunter", "2secret"].concat());
    let unknown = format!("use {}", "aB3+/x9Z".repeat(5));
    for (text, kept) in [(password, 20), (unknown, 34)] {
        let cut_short = format!("{}{}{}", &text[..4], "\u{200b}".repeat(SAID + WINDOW - kept), &text[4..]);
        assert_eq!(shown(&cut_short, SAID), format!("{}…", &text[..3]), "{text}");
    }
    // A long text with no spaces at all (a paste, or prose in Chinese) is not such a case: it is cut as any other.
    let prose = "字".repeat(SAID + WINDOW + 100);
    assert_eq!(shown(&prose, SAID), format!("{}…", "字".repeat(SAID - 1)));
}

#[test]
fn a_pem_key_with_no_end_mark_is_replaced_to_the_end_of_what_was_read() {
    let header = format!("-----BEGIN RSA {}-----", ["PRIVATE", "KEY"].join(" "));
    let body = "MIIEabc9\n".repeat(WINDOW); // nine times the window: the END mark is never read
    let end = header.replace("BEGIN", "END");
    let cut_by_the_window = format!("my key: {header}\n{body}{end} and more");
    assert_eq!(shown(&cut_by_the_window, SAID), "my key: [redacted]…");
    assert_eq!(line(&cut_by_the_window, 300), "my key: [redacted]…");
    let cut_by_the_log = format!("my key: {header}\nMIIEabc9\nMIIE");
    assert_eq!(shown(&cut_by_the_log, SAID), "my key: [redacted]");
    let straddling = format!("{} {header}\n{body}", "x".repeat(SAID - 20));
    assert_eq!(shown(&straddling, SAID), format!("{} {REPLACED}…", "x".repeat(SAID - 20)), "across the shown limit");
    let whole = format!("my key: {header}\nMIIEabc9\n{end} and more");
    assert_eq!(shown(&whole, SAID), "my key: [redacted] and more");
}

// Megabytes whose start is plain cost what the window costs. By construction: what is cleaned is what `start` returns,
// and that is `limit + WINDOW` characters whatever the text's length. The time is a second check, with room for a slow
// machine.
#[test]
fn megabytes_after_a_plain_start_are_not_read() {
    let shape = "aB3/x. -----BEGIN eyJ-\u{1b}]";
    let text = format!("{}{}", "plain words ".repeat(100), shape.repeat((32 << 20) / shape.len()));
    assert!(start(&text, SAID + WINDOW).0.len() <= 4 * (SAID + WINDOW));
    let began = std::time::Instant::now();
    let said = shown(&text, SAID);
    let took = began.elapsed();
    assert!(took < std::time::Duration::from_millis(1500), "{took:?}");
    assert_eq!(said, shown(&text[..2 * (SAID + WINDOW)], SAID), "the same as cleaning a short text with this start");
    assert!(said.starts_with("plain words plain") && said.chars().count() == SAID);
}
