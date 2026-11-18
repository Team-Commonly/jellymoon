//! The rules of [`super`] that need no repo: the shape a result is read in, its size, what is kept, and the two
//! doors, a text's and a quoted line's. The checks against a run's work are asked at the binary
//! (`crates/jellymoon/tests/f85/checks.rs`).

use std::fs;

use serde_json::json;

use super::*;

const RUN: &str = "r0123456789abcdef";

/// A credential's shape, joined here so no scanner reads one in this file.
fn key() -> String {
    format!("AI{}{}", "za", "kLmNo".repeat(7))
}

/// A result of one piece, whose text is `n` letters.
fn sized(n: usize) -> Value {
    json!({"pieces": [{"type": "did", "text": "x".repeat(n)}]})
}

#[test]
fn a_result_is_read_in_one_shape_and_kept_as_it_was_read() {
    let did = json!({"type": "did", "text": "done"});
    let written = json!({"pieces": [
        did,
        {"type": "change", "file": "a.txt"},
        {"type": "change", "file": "b.txt", "before": [], "after": ["y"]},
        {"type": "decided", "text": ""},
        {"type": "unsure", "text": "u"},
        {"type": "change", "file": "c.txt"},
    ]});
    let kept = r#"{"pieces":[{"type":"did","text":"done"},{"type":"change","file":"a.txt"},{"type":"change","file":"b.txt","after":["y"]},{"type":"decided","text":""},{"type":"unsure","text":"u"},{"type":"change","file":"c.txt"}]}"#;
    assert_eq!(serde_json::to_string(&said(&written).unwrap()).unwrap(), kept);
    // The files asked for as they were are those of the first two changes, and no more.
    assert_eq!(wanted(&said(&written).unwrap()), [&b"a.txt"[..], &b"b.txt"[..]]);
    for not_it in [
        json!(null),
        json!([]),
        json!({}),
        json!({"pieces": {}}),
        json!({"pieces": ["did"]}),
        json!({"pieces": [did], "more": 1}),
        json!({"pieces": [{"type": "checked", "text": "x"}]}),
        json!({"pieces": [{"text": "x"}]}),
        json!({"pieces": [{"type": "did"}]}),
        json!({"pieces": [{"type": "did", "text": 7}]}),
        json!({"pieces": [{"type": "did", "text": "x", "file": "a.txt"}]}),
        json!({"pieces": [{"type": "change", "file": "a.txt", "before": "x"}]}),
        json!({"pieces": [{"type": "change", "after": ["x"]}]}),
        json!({"pieces": [{"type": "change", "file": "a.txt", "text": "x"}]}),
    ] {
        assert_eq!(said(&not_it), None, "{not_it}");
    }
    // Of more than twelve pieces the first twelve are kept.
    let many = said(&json!({"pieces": vec![did.clone(); 13]})).unwrap();
    assert_eq!(many.pieces.len(), 12);
    assert_eq!(said(&json!({"pieces": []})).map(|said| said.pieces.len()), Some(0));
}

#[test]
fn a_result_over_its_size_is_none_and_the_largest_one_is_kept_and_read_back_whole() {
    let bare = serde_json::to_string(&said(&sized(0)).unwrap()).unwrap().len();
    let largest = said(&sized(SAID_MAX - bare - 1)).unwrap();
    assert_eq!(said(&sized(SAID_MAX - bare)), None, "one byte more");
    let home = tempfile::tempdir().unwrap();
    let folder = copy::kept_at(home.path(), RUN);
    fs::create_dir_all(&folder).unwrap();
    assert_eq!(kept(home.path(), RUN), None, "nothing is kept yet");
    keep(home.path(), RUN, &largest);
    assert_eq!(fs::metadata(folder.join(FILE)).unwrap().len(), SAID_MAX as u64);
    assert_eq!(kept(home.path(), RUN), Some(largest));
    // A file of one byte more, and one that is not the shape, are no result.
    let longer = format!("{} ", fs::read_to_string(folder.join(FILE)).unwrap());
    fs::write(folder.join(FILE), longer).unwrap();
    assert_eq!(kept(home.path(), RUN), None);
    fs::write(folder.join(FILE), r#"{"pieces":[],"dropped":0}"#).unwrap();
    assert_eq!(kept(home.path(), RUN), None);
    // Kept where the run has no folder, nothing is written and nothing fails.
    keep(home.path(), "rfedcba9876543210", &said(&sized(1)).unwrap());
    assert_eq!(kept(home.path(), "rfedcba9876543210"), None);
}

#[test]
fn a_text_passes_the_door_and_is_dropped_never_cut() {
    assert_eq!(clean("  It retries\n twice \"now\".  ", 160).as_deref(), Some("It retries twice 'now'."));
    assert_eq!(clean(&format!("the key {} is set", key()), 160).as_deref(), Some("the key [redacted] is set"));
    assert_eq!(clean("a space\u{a0}of another kind", 160).as_deref(), Some("a space of another kind"));
    // A credential a line break splits is one, and is the mark; words on two lines stay two words.
    let (whole, mark) = (key(), Some("use [redacted] here"));
    for split in ["\n", "\r\n", "\u{2028}"] {
        assert_eq!(
            clean(&format!("use {}{split}{} here", &whole[..17], &whole[17..]), 160).as_deref(),
            mark,
            "{split:?}"
        );
        assert_eq!(clean(&format!("fix the upload{split}retry"), 160).as_deref(), Some("fix the upload retry"));
    }
    for none in [
        String::new(),
        " \n\t".to_string(),
        key(),
        format!("\"{}\"", key()),
        "zero\u{200b}width".to_string(),
        "blank\u{2800}here".to_string(),
        "chosen\u{fe0f}".to_string(),
        "fill\u{3164}er".to_string(),
        "red \u{1b}[31malert".to_string(),
        "private \u{e000}".to_string(),
        "+5 more".to_string(),
        "5MORE".to_string(),
        "\u{663} more".to_string(),
    ] {
        assert_eq!(clean(&none, 160), None, "{none:?}");
    }
    let fits = format!("{}words", "word ".repeat(31));
    assert_eq!((fits.chars().count(), clean(&fits, 160).as_deref()), (160, Some(fits.as_str())));
    assert_eq!(clean(&format!("{fits}s"), 160), None);
    assert_eq!(clean(&format!("{fits}s"), 161).map(|kept| kept.chars().count()), Some(161));
}

#[test]
fn a_quoted_line_is_printed_without_its_ends_or_not_at_all() {
    assert_eq!(line("\t  let x  = 1;  // two \t"), Some("let x  = 1;  // two"));
    assert_eq!(line("say(\"hello\");"), Some("say(\"hello\");"), "a quote is a line's own");
    let fits = format!("{}a", "ab ".repeat(33));
    assert_eq!((fits.chars().count(), line(&fits)), (100, Some(fits.as_str())));
    let (key, over) = (format!("let key = \"{}\";", key()), format!("{fits}b"));
    for none in [
        "",
        " \t ",
        "a\tb",
        "a\nb",
        "a\r",
        "zero\u{200b}width",
        "blank\u{2800}here",
        "a\u{a0}b",
        "// 3 more",
        key.as_str(),
        over.as_str(),
    ] {
        assert_eq!(line(none), None, "{none:?}");
    }
}
