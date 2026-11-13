//! `jellymoon run show --file <n>`: one file of the work as it now is, the whole text and never a diff, cleaned and
//! cut at a stated size. A file that is not text, a link, another repo's entry, a file the work removed and a number
//! that names no file are refused in fixed words, with no byte of the file.

use serde_json::json;

use super::{Made, WORK, finished, printed, refused};
use crate::support::{hub_key, key_header, map_key, pay_key, shows};

/// The refusal of `--file <n>`: one line that holds nothing of the work and no name of a file.
fn no(made: &Made, n: &str, why: &str) {
    let ran = made.show(&["--file", n]);
    let said = refused(&ran);
    assert!(said.contains(why), "{said}");
    for held in [WORK, "elsewhere", "secret-target", "tool", "gone.txt"] {
        assert!(!said.contains(held), "{said}");
    }
}

#[test]
fn a_file_is_printed_as_its_whole_text_with_its_lines_and_never_as_a_diff() {
    let made = finished();
    let text = "fn main() {\n    let said = \"hello\";\n\tprintln!(\"{said}\");\n}\n";
    made.rework(&[("100644", "a.txt", text.as_bytes())]);
    let before = made.held();
    let file = printed(&made.show(&["--file", "1"]));
    let want = json!({
        "run": made.run, "wish": "m1", "work": "waits", "n": 1, "name": "a.txt", "size": text.len(),
        "text": text.trim_end(), "cut": false, "most": 65536
    });
    assert_eq!(file, want, "the changed file whole, as it now is: no line of what it was, no mark of a change");
    assert_eq!(made.held(), before);
}

#[test]
fn a_text_is_cleaned_before_it_is_printed_until_cleaning_changes_nothing() {
    let made = finished();
    let (one, pay, hub) = (map_key("kLmNo"), pay_key(), hub_key());
    let pem = format!("{}\nMIIEow{WORK}\n{}\n", key_header(" "), key_header(" ").replace("BEGIN", "END"));
    let text = format!(
        "red \u{1b}[31malert\u{1b}[0m\nright\u{202e}to left\u{200b}\ntoken = {hub}\nkeys {one}{pay} end\n{pem}after\n"
    );
    made.rework(&[("100644", "notes.txt", text.as_bytes())]);
    let ran = made.show(&["--file", "1"]);
    let file = printed(&ran);
    let want = "red alert\nrightto left\ntoken = [redacted]\nkeys [redacted][redacted] end\n[redacted]\nafter";
    assert_eq!(file["text"], want, "a key straight after another is found by the second cleaning");
    for secret in [&one, &pay, &hub] {
        assert!(!shows(&ran.stdout, secret));
    }
    assert!(!ran.stdout.contains('\u{1b}') && !ran.stdout.contains("\\u001b") && !ran.stdout.contains('\u{202e}'));
    // Nine keys in a row are more than eight cleanings are done with: the text is refused, never half cleaned.
    let nine = format!("use {} here\n", map_key("dEfGh").repeat(9));
    made.rework(&[("100644", "notes.txt", nine.as_bytes())]);
    let said = made.show(&["--file", "1"]);
    assert!(refused(&said).contains("cannot be cleaned"), "{}", said.stderr);
    assert!(!shows(&said.stderr, &map_key("dEfGh")));
}

#[test]
fn a_text_over_the_stated_size_is_cut_before_its_last_word_on_a_character_and_says_so() {
    let made = finished();
    let exact = "w".repeat(65535) + "\n";
    // The 65536th byte is the middle of a character, and the last word before it is the head of a key.
    let key = hub_key();
    let long = format!("{} {}é and more {WORK}\n", "é".repeat(32757), &key[..20]);
    assert_eq!((exact.len(), &long.as_bytes()[65534..65537]), (65536, &[b'3', 0xc3, 0xa9][..]));
    made.rework(&[("100644", "exact.txt", exact.as_bytes()), ("100644", "long.txt", long.as_bytes())]);
    let listed = printed(&made.show(&[]));
    assert_eq!((&listed["files"][0]["kind"], &listed["files"][1]["kind"]), (&json!("text"), &json!("text")));
    let whole = printed(&made.show(&["--file", "1"]));
    assert_eq!(
        (&whole["cut"], whole["text"].as_str().unwrap().len(), &whole["size"]),
        (&json!(false), 65535, &json!(65536))
    );
    let ran = made.show(&["--file", "2"]);
    let cut = printed(&ran);
    assert_eq!((&cut["cut"], &cut["size"], &cut["most"]), (&json!(true), &json!(long.len()), &json!(65536)));
    assert_eq!(cut["text"], "é".repeat(32757), "the word the cut fell in is not shown: its rest was never read");
    assert!(!ran.stdout.contains(WORK) && !shows(&ran.stdout, &key));
}

#[test]
fn what_is_not_text_a_link_another_repos_entry_and_a_removed_file_are_refused_with_none_of_their_bytes() {
    let made = finished();
    let (nul, bad) = ([WORK.as_bytes(), &[0], b"tail"].concat(), [WORK.as_bytes(), &[0xff, 0xfe]].concat());
    // A character cut in two at the very end is not text when nothing was cut: the file ends there.
    let half = [WORK.as_bytes(), &[0xc3]].concat();
    made.rework(&[
        ("100644", "bin/tool", &nul),
        ("100644", "bin/tool-2", &bad),
        ("100644", "bin/tool-3", &half),
        ("0", "c.txt", b""),
        ("120000", "link", b"elsewhere/secret-target"),
        ("160000", "sub", b""),
    ]);
    let before = made.held();
    let kinds: Vec<String> =
        printed(&made.show(&[]))["files"].as_array().unwrap().iter().map(|file| file["kind"].to_string()).collect();
    assert_eq!(kinds.join(" "), r#""other" "other" "other" "text" "link" "repo""#);
    for n in ["1", "2", "3"] {
        no(&made, n, "is not text");
    }
    no(&made, "4", "was removed by the work");
    no(&made, "5", "is a link");
    no(&made, "6", "is another repo's entry");
    for n in ["0", "7", "8", "18446744073709551615"] {
        no(&made, n, "names no file");
    }
    assert_eq!(made.held(), before);
    made.nothing_on_disk(&["tool", "tool-2", "tool-3", "link", "sub", "secret-target"]);
}

#[test]
fn a_number_that_is_not_one_and_a_wish_id_that_is_not_one_are_not_taken() {
    let made = finished();
    for n in ["-1", "+1", "1.0", "one", "", " 1", "0x1", "99999999999999999999999"] {
        let ran = made.show(&["--file", n]);
        assert_eq!((ran.code, ran.stdout.as_str()), (2, ""), "{n:?}: {}", ran.stderr);
        assert!(ran.stderr.contains("--file takes a number"), "{}", ran.stderr);
    }
    for wish in ["m9", "-m1", "--file", "m1\n", "../m1", "refs/heads/main", ""] {
        let repo = made.repo.to_str().unwrap();
        let ran = made.w.run(&["run", "show", "--wish", wish, "--date", super::DAY, "--repo", repo]);
        assert!(refused(&ran).contains("has no made wish of that id"), "{wish:?}: {}", ran.stderr);
    }
}
