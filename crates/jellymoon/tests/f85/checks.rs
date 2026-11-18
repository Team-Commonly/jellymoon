//! Each rule of a check (ADR 0023, "The catalogue"), asked of one run's work with a result written into the run's
//! folder by hand: a file there is anyone's to write, and it is checked as the run's own result is. The work
//! changed `code.rs`, made `new.txt` and `pic.bin`, removed `old.txt`, and has two files of which one side is
//! longer than is read.

use serde_json::{Value, json};

use super::{Made, result, told, write_kept};
use crate::support::{map_key, shows};

/// A comment of exactly a hundred characters, and one of a hundred and one.
fn hundred(more: &str) -> String {
    format!("// {}ab{more}", "word ".repeat(19))
}

/// `code.rs` as it was, and as the run left it.
fn code() -> [String; 2] {
    let was = "fn price(total: u32) -> u32 {\n    if total >= 100 { 0 } else { 8 }\n}\n// twice\n// twice\n// stays\n";
    let is = [
        "fn price(total: u32) -> u32 {",
        "    if total >= 50 { 0 } else { 8 }",
        "}",
        "// twice",
        "// stays",
        "// new",
        "// new",
        &hundred(""),
        &hundred("c"),
        "\t tabbed line \t",
        "let x  = 1;  // two spaces",
        &format!("let key = \"{}\";", map_key("kLmNo")),
        "zero\u{200b}width",
        "blank\u{2800}here",
        "// 3 more",
        "inner\ttab",
    ];
    [was.to_string(), is.join("\n") + "\n"]
}

/// The run: it ended with `done` alone, so what is checked is what a test then writes.
fn worked() -> Made {
    let [was, is] = code();
    let long = |first: &str| format!("{first}\n{}", "fill fill fill\n".repeat(4700)).into_bytes();
    let (hay, needle) = (long("hay"), long("needle"));
    let had = [
        ("code.rs", was.as_bytes()),
        ("old.txt", &b"old line\n"[..]),
        ("big.txt", &b"small\n"[..]),
        ("was-big.txt", &hay[..]),
    ];
    let left = [
        ("code.rs", is.as_bytes()),
        ("new.txt", &b"fresh line\n"[..]),
        ("pic.bin", &b"bin line\n\0\n"[..]),
        ("big.txt", &needle[..]),
        ("was-big.txt", &b"now small\n"[..]),
        ("zz 3 more.txt", &b"count line\n"[..]),
    ];
    let (made, said) = told(&had, &left, &["old.txt"], r#"{"state":"done"}"#);
    assert_eq!(said["outcome"], "completed", "{said}");
    assert_eq!(result(&made), Value::Null, "a run that said nothing of its work");
    made
}

fn change(file: &str, before: &[&str], after: &[&str]) -> Value {
    json!({"type": "change", "file": file, "before": before, "after": after})
}

/// What is printed of `pieces` written as the run's result.
fn of(made: &Made, pieces: &Value) -> Value {
    write_kept(made, &json!({ "pieces": pieces }).to_string());
    result(made)
}

/// The one piece as it is printed: it passed its check, and none was dropped.
fn passed(made: &Made, piece: &Value) -> Value {
    let told = of(made, &json!([piece]));
    assert_eq!((told["pieces"].as_array().map(Vec::len), &told["dropped"]), (Some(1), &json!(0)), "{piece}: {told}");
    told["pieces"][0].clone()
}

/// Fails unless the one piece is dropped whole, and counted.
fn dropped(made: &Made, piece: &Value) {
    assert_eq!(of(made, &json!([piece])), json!({"pieces": [], "dropped": 1}), "{piece}");
}

fn printed_change(n: usize, name: &str, made: bool, before: &[&str], after: &[&str]) -> Value {
    json!({"type": "change", "n": n, "name": name, "made": made, "before": before, "after": after})
}

const WAS: &str = "if total >= 100 { 0 } else { 8 }";
const IS: &str = "if total >= 50 { 0 } else { 8 }";

#[test]
fn a_change_is_printed_only_of_a_text_file_of_the_work_named_as_git_keeps_it() {
    let made = worked();
    assert_eq!(passed(&made, &change("code.rs", &[WAS], &[IS])), printed_change(2, "code.rs", false, &[WAS], &[IS]));
    // No file of the work: one that is nowhere, one of the repo the work left alone, and other spellings.
    dropped(&made, &change("README.md", &[], &["Free shipping from 50."]));
    dropped(&made, &change("c.txt", &[], &["c.txt as it was"]));
    for spelled in ["./code.rs", "CODE.RS", "code.rs ", "/code.rs", "code.rs\u{200b}", ""] {
        dropped(&made, &change(spelled, &[WAS], &[IS]));
    }
    // A file whose name a note would not print as git keeps it: it reads like a count.
    dropped(&made, &change("zz 3 more.txt", &[], &["count line"]));
    // A file that is not text, and a file one side of which is longer than is read.
    dropped(&made, &change("pic.bin", &[], &["bin line"]));
    dropped(&made, &change("big.txt", &[], &["needle"]));
    dropped(&made, &change("big.txt", &["small"], &[]));
    dropped(&made, &change("was-big.txt", &[], &["now small"]));
    // A file the run made takes no `before`; one it removed has no `after`.
    let new = passed(&made, &change("new.txt", &[], &["fresh line"]));
    assert_eq!(new, printed_change(3, "new.txt", true, &[], &["fresh line"]));
    dropped(&made, &change("new.txt", &["fresh line"], &[]));
    dropped(&made, &change("new.txt", &["old line"], &["fresh line"]));
    let old = passed(&made, &change("old.txt", &["old line"], &[]));
    assert_eq!(old, printed_change(4, "old.txt", false, &["old line"], &[]));
    dropped(&made, &change("old.txt", &[], &["old line"]));
    // Neither side need be given when it is empty.
    let bare = json!({"type": "change", "file": "new.txt", "after": ["fresh line"]});
    assert_eq!(passed(&made, &bare), new);
}

#[test]
fn every_quoted_line_is_a_whole_line_the_work_took_from_the_file_or_put_in_it() {
    let made = worked();
    let lines = |before: &[&str], after: &[&str]| change("code.rs", before, after);
    // A line the file has more often after the work is one it put there, and one it has less often one it took.
    passed(&made, &lines(&[], &["// new"]));
    passed(&made, &lines(&["// twice"], &[]));
    dropped(&made, &lines(&[], &["// twice"]));
    dropped(&made, &lines(&["// new"], &[]));
    // A line that is there as often as before is neither.
    dropped(&made, &lines(&["// stays"], &[]));
    dropped(&made, &lines(&[], &["// stays"]));
    dropped(&made, &lines(&["}"], &[IS]));
    // A line of the file as it was is no line of the file as it is, and the other way round.
    dropped(&made, &lines(&[], &[WAS]));
    dropped(&made, &lines(&[IS], &[]));
    // A part of a line is no line, and no other case of letters is the line.
    for part in ["total >= 50", "if total >= 50 { 0 } else { 8", "// ne", "// NEW", "//new"] {
        dropped(&made, &lines(&[], &[part]));
    }
    // One line that fails drops the piece, whatever the others are.
    dropped(&made, &lines(&[WAS], &[IS, "not in the file"]));
    dropped(&made, &lines(&[WAS, "not in the file"], &[IS]));
    // Spaces and tabs at a line's ends are no part of it, in the file or in the quote, and are not printed.
    let ends = passed(&made, &lines(&[&format!("\t  {WAS}")], &["  \ttabbed line", " // new\t"]));
    assert_eq!(ends, printed_change(2, "code.rs", false, &[WAS], &["tabbed line", "// new"]));
    // Spaces inside a line are the line's own.
    let inside = "let x  = 1;  // two spaces";
    assert_eq!(passed(&made, &lines(&[], &[inside]))["after"], json!([inside]));
    dropped(&made, &lines(&[], &["let x = 1; // two spaces"]));
}

#[test]
fn a_line_over_its_bound_or_one_that_would_not_be_read_as_it_is_drops_the_piece() {
    let made = worked();
    let lines = |before: &[&str], after: &[&str]| change("code.rs", before, after);
    let (fits, over) = (hundred(""), hundred("c"));
    assert_eq!((fits.chars().count(), over.chars().count()), (100, 101));
    // Three lines a side, a hundred characters a line, one line at least.
    passed(&made, &lines(&[WAS, "// twice"], &[IS, "// new", &fits]));
    dropped(&made, &lines(&[], &[IS, "// new", &fits, "let x  = 1;  // two spaces"]));
    dropped(&made, &lines(&[WAS, "// twice", "// twice", "// twice"], &[]));
    dropped(&made, &lines(&[], &[&over]));
    dropped(&made, &lines(&[], &[]));
    // A line the file holds, quoted as it is: with a character that draws as nothing, with a tab inside, with a
    // credential's shape, and one that reads like a note's count.
    let key = format!("let key = \"{}\";", map_key("kLmNo"));
    for unread in ["zero\u{200b}width", "blank\u{2800}here", "inner\ttab", key.as_str(), "// 3 more"] {
        dropped(&made, &lines(&[], &[unread]));
    }
    // And what is no line at all.
    for none in ["", "   ", "\t", "// new\n// new", "// new\r"] {
        dropped(&made, &lines(&[], &[none]));
    }
    // The first two changes are printed, and a third is left out: it failed no check, and is not counted.
    let (one, two, three) = (lines(&[], &[IS]), lines(&[], &["// new"]), change("new.txt", &[], &["fresh line"]));
    let told = of(&made, &json!([one, two, three]));
    let names: Vec<&Value> = told["pieces"].as_array().unwrap().iter().map(|piece| &piece["after"][0]).collect();
    assert_eq!((names, &told["dropped"]), (vec![&json!(IS), &json!("// new")], &json!(0)));
    let told = of(&made, &json!([lines(&[], &["not in the file"]), two, three]));
    assert_eq!((told["pieces"].as_array().unwrap().len(), &told["dropped"]), (1, &json!(1)), "{told}");
}

#[test]
fn a_text_passes_the_door_a_question_passes_and_is_dropped_never_cut() {
    let made = worked();
    let text = |kind: &str, text: &str| json!({"type": kind, "text": text});
    let key = map_key("kLmNo");
    for kind in ["did", "decided", "unsure"] {
        assert_eq!(passed(&made, &text(kind, "It retries twice now.")), text(kind, "It retries twice now."));
        // A credential's shape is printed as the mark, a `"` as a `'`, and a line break as a space.
        let marked = passed(&made, &text(kind, &format!("I set \"key\" to {key}\n in  the config.")));
        assert_eq!(marked, text(kind, "I set 'key' to [redacted] in the config."));
        assert!(!shows(&marked.to_string(), &key), "no eight characters in a row of the key are printed");
        for unread in [
            "",
            "  \n",
            key.as_str(),
            "zero\u{200b}width",
            "blank\u{2800}here",
            "red \u{1b}[31malert",
            "and 5 more",
            "with 12MORE of them",
        ] {
            dropped(&made, &text(kind, unread));
        }
        let most = if kind == "did" { 280 } else { 160 };
        let fits = format!("{}words", "word ".repeat(most / 5 - 1));
        assert_eq!(fits.chars().count(), most);
        assert_eq!(passed(&made, &text(kind, &fits)), text(kind, &fits));
        dropped(&made, &text(kind, &format!("{fits}s")));
        dropped(&made, &text(kind, &"long ".repeat(1000)));
    }
}

#[test]
fn the_pieces_are_printed_in_the_catalogue_s_order_and_each_kind_up_to_its_count() {
    let made = worked();
    let text = |kind: &str, text: &str| json!({"type": kind, "text": text});
    let written = json!([
        text("unsure", "u1"),
        text("decided", "d1"),
        change("new.txt", &[], &["fresh line"]),
        text("unsure", "u2"),
        text("did", "first"),
        text("decided", "d2"),
        text("did", "second"),
        text("decided", "d3"),
        text("unsure", "u3"),
        text("decided", "d4"),
        text("unsure", "u4"),
    ]);
    let want = json!({"pieces": [
        text("did", "first"),
        printed_change(3, "new.txt", true, &[], &["fresh line"]),
        text("decided", "d1"), text("decided", "d2"), text("decided", "d3"),
        text("unsure", "u1"), text("unsure", "u2"), text("unsure", "u3"),
    ], "dropped": 0});
    assert_eq!(of(&made, &written), want);
    // A piece that fails its check is counted and its place is not given to a later one of its kind.
    let failing = json!([text("did", ""), text("did", "second"), text("decided", "d1"), text("unsure", "5 more")]);
    assert_eq!(of(&made, &failing), json!({"pieces": [text("decided", "d1")], "dropped": 2}));
    // With nothing dropped none is counted, and with no piece and nothing dropped there is no result at all.
    assert_eq!(of(&made, &json!([text("decided", "d1")]))["dropped"], 0);
    assert_eq!(of(&made, &json!([])), Value::Null);
}
