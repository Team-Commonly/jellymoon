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

/// Whether a value fits a schema, for the few words the schema of a last word uses: a type, an enum, an
/// object's properties, required parts and no other part, an array's items and most items, a text's most
/// characters. Any other word of a schema fails it, so a word added to the file is added here too.
fn fits(value: &Value, schema: &Value) -> bool {
    let known = ["type", "enum", "properties", "required", "additionalProperties", "items", "maxItems", "maxLength"];
    let schema = schema.as_object().unwrap();
    assert!(schema.keys().all(|word| known.contains(&word.as_str())), "{schema:?}");
    let most = |word: &str, n: usize| {
        schema.get(word).and_then(Value::as_u64).is_none_or(|most| u64::try_from(n).is_ok_and(|n| n <= most))
    };
    let typed = match (schema["type"].as_str().unwrap(), value) {
        ("string", Value::String(text)) => most("maxLength", text.chars().count()),
        ("array", Value::Array(items)) => {
            most("maxItems", items.len()) && items.iter().all(|i| fits(i, &schema["items"]))
        }
        ("object", Value::Object(parts)) => {
            let named = &schema["properties"];
            let required =
                schema["required"].as_array().unwrap().iter().all(|part| parts.contains_key(part.as_str().unwrap()));
            assert_eq!(schema["additionalProperties"], false, "every object of the schema is closed");
            required && parts.iter().all(|(name, part)| named.get(name).is_some_and(|inner| fits(part, inner)))
        }
        _ => false,
    };
    typed && schema.get("enum").is_none_or(|one_of| one_of.as_array().unwrap().contains(value))
}

#[test]
fn the_schema_file_is_json_its_bounds_are_core_s_and_both_worked_examples_fit_it_and_are_read_as_results() {
    use super::super::asks::{Last, Word, last, schema};
    let schema: Value = serde_json::from_str(schema()).unwrap();
    let piece = &schema["properties"]["result"]["properties"]["pieces"];
    let parts = &piece["items"]["properties"];
    assert_eq!(piece["maxItems"], KEPT);
    assert_eq!(
        (&parts["text"]["maxLength"], &parts["before"]["maxItems"]),
        (&json!(DID_MAX.max(SAYS_MAX)), &json!(LINES))
    );
    assert_eq!(
        (&parts["before"], &parts["type"]["enum"]),
        (&parts["after"], &json!(["did", "change", "decided", "unsure"]))
    );
    assert_eq!(parts["after"]["items"]["maxLength"], LINE_MAX);
    // The two worked examples of ADR 0023, typed by hand: each fits the schema, is `done`, and is a result of all
    // its pieces as core reads one.
    let shipping = json!({"state": "done", "result": {"pieces": [
        {"type": "did", "text": "Shipping is now free from 50, not 100. The test tries 49 and 50."},
        {"type": "change", "file": "src/shipping.rs", "before": ["    if total >= 100 { 0 } else { 8 }"],
         "after": ["    if total >= 50 { 0 } else { 8 }"]},
        {"type": "decided", "text": "An order of exactly 50 ships free."},
        {"type": "unsure", "text": "I could not run the test."},
        {"type": "change", "file": "README.md", "before": [], "after": ["Free shipping from 50."]},
    ]}});
    let page = json!({"state": "done", "result": {"pieces": [
        {"type": "did", "text": "There is a new page, index.html, with a heading and one line about Saturdays."},
        {"type": "change", "file": "index.html", "before": [],
         "after": ["<h1>We bake on Saturdays</h1>", "<p>Come by from 8 in the morning.</p>"]},
        {"type": "decided", "text": "I made up the opening time, 8 in the morning."},
        {"type": "decided", "text": "The page has no colours or pictures yet."},
    ]}});
    for (word, pieces) in [(shipping, 5), (page, 4)] {
        assert!(fits(&word, &schema), "{word}");
        let read: Word = serde_json::from_value(word.clone()).unwrap();
        assert_eq!(said(read.result().unwrap()).map(|said| said.pieces.len()), Some(pieces));
        assert_eq!(last(Some(read)), Last::Done);
    }
    // And what does not fit it: a fifth kind, a part no kind has, a line over its length, `done` alone fits.
    let one = |piece: Value| json!({"state": "done", "result": {"pieces": [piece]}});
    assert!(fits(&json!({"state": "done"}), &schema) && fits(&json!({"state": "asks", "question": "which?"}), &schema));
    for odd in [
        one(json!({"type": "checked", "text": "it works"})),
        one(json!({"type": "did", "text": "done", "how": "by hand"})),
        one(json!({"type": "change", "file": "a.txt", "after": ["x".repeat(LINE_MAX + 1)]})),
        json!({"state": "done", "result": {}}),
        json!({"state": "done", "more": 1}),
    ] {
        assert!(!fits(&odd, &schema), "{odd}");
    }
}
